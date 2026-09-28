use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use anyhow::{Context, anyhow, bail};
use esp_idf_svc::{
    bt::{
        BdAddr, Ble, BtDriver, BtStatus,
        ble::{
            gap::{AdvertisingDataType, BleGapEvent, EspBleGap, GapSearchEvent, GapSearchResult, ScanParams},
            gatt::{
                GattInterface, GattStatus,
                client::{ConnectionId, EspGattc, GattCreateConnParams, GattcEvent},
            },
        },
    },
    hal::{delay::FreeRtos, modem::Modem, peripherals::Peripherals},
    nvs::EspDefaultNvsPartition,
};

/// The device to connect to is baked in at compile time, like the Wi-Fi credentials in the `wifi-wpa` example.
///
/// ```sh
/// BT_DEVICE_NAME="My Sensor" cargo run
/// ```
const DEVICE_NAME: &str = env!(
    "BT_DEVICE_NAME",
    "set BT_DEVICE_NAME to the advertised name of the device"
);

/// How long to scan for the device before giving up.
const SCAN_SECONDS: u32 = 10;

/// Upper bound on a whole connection attempt. Scanning and the stack's own connection timeout should both fire well
/// before this, it only exists so a lost event can never hang the program.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait before trying again after a failed connection attempt.
const RETRY_MS: u32 = 5_000;

/// Bluedroid supports several GATT client "apps" at once, each with its own interface. This example only needs one.
const APP_ID: u16 = 0;

type Driver = BtDriver<'static, Ble>;
type Gap = EspBleGap<'static, Ble, Arc<Driver>>;
type Gattc = EspGattc<'static, Ble, Arc<Driver>>;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    match run() {
        Ok(()) => {}
        Err(e) => {
            eprint!("{}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let client = BleClient::new(peripherals.modem, nvs)?;

    loop {
        match client.connect(DEVICE_NAME) {
            Ok(connection) => {
                log::info!(
                    "Connected to {DEVICE_NAME} at {} (connection {})",
                    connection.addr,
                    connection.conn_id
                );

                client.wait_for_disconnect();
                log::warn!("Connection to {DEVICE_NAME} dropped, reconnecting");
            }
            Err(e) => {
                log::error!("{e:#}, retrying in {}s", RETRY_MS / 1000);
                FreeRtos::delay_ms(RETRY_MS);
            }
        }
    }
}

/// An open link to a remote device.
#[derive(Clone, Copy, Debug)]
pub struct Connection {
    /// Identifies the link in every other GATT client call, e.g. reading or writing characteristics.
    pub conn_id: ConnectionId,
    pub addr: BdAddr,
}

/// Where the client is in the scan → connect → disconnect lifecycle.
#[derive(Debug)]
enum Phase {
    Idle,
    /// Looking for a device advertising this name.
    Scanning(String),
    /// Found the device and asked the stack to open a connection to it.
    Connecting(BdAddr),
    Connected(Connection),
    Failed(String),
}

impl Phase {
    fn is_pending(&self) -> bool {
        matches!(self, Self::Scanning(_) | Self::Connecting(_))
    }
}

struct State {
    gattc_if: Option<GattInterface>,
    phase: Phase,
}

/// A BLE central that connects to peripherals by their advertised name.
///
/// Bluedroid is entirely callback driven: every call below only *queues* work, and the result arrives later as a GAP
/// or GATT client event on the Bluetooth task. The event handlers record progress in [`State`] and wake up whoever is
/// waiting on the [`Condvar`], which is what lets [`BleClient::connect`] look like an ordinary blocking call.
#[derive(Clone)]
pub struct BleClient {
    gap: Arc<Gap>,
    gattc: Arc<Gattc>,
    state: Arc<Mutex<State>>,
    changed: Arc<Condvar>,
}

impl BleClient {
    /// Brings up the Bluetooth controller and registers a GATT client app with the stack.
    pub fn new(modem: Modem<'static>, nvs: EspDefaultNvsPartition) -> anyhow::Result<Self> {
        let driver = Arc::new(BtDriver::new(modem, Some(nvs))?);

        let client = Self {
            gap: Arc::new(EspBleGap::new(driver.clone())?),
            gattc: Arc::new(EspGattc::new(driver)?),
            state: Arc::new(Mutex::new(State {
                gattc_if: None,
                phase: Phase::Idle,
            })),
            changed: Arc::new(Condvar::new()),
        };

        let gap_client = client.clone();
        client.gap.subscribe(move |event| {
            if let Err(e) = gap_client.on_gap_event(event) {
                gap_client.fail(format!("{e:#}"));
            }
        })?;

        let gattc_client = client.clone();
        client.gattc.subscribe(move |(gattc_if, event)| {
            if let Err(e) = gattc_client.on_gattc_event(gattc_if, event) {
                gattc_client.fail(format!("{e:#}"));
            }
        })?;

        // Nothing can be opened until the stack hands back an interface for this app.
        client.gattc.register_app(APP_ID)?;
        let (state, _) = client
            .changed
            .wait_timeout_while(client.lock(), Duration::from_secs(5), |state| state.gattc_if.is_none())
            .unwrap();
        if state.gattc_if.is_none() {
            bail!("GATT client app was never registered");
        }
        drop(state);

        log::info!("Bluetooth ready");

        Ok(client)
    }

    /// Scans for a device advertising `name` and opens a connection to it, blocking until it is up or has failed.
    ///
    /// Returns straight away if already connected.
    pub fn connect(&self, name: &str) -> anyhow::Result<Connection> {
        let mut state = self.lock();

        match &state.phase {
            Phase::Connected(connection) => return Ok(*connection),
            phase if phase.is_pending() => bail!("a connection attempt is already in progress"),
            _ => {}
        }

        log::info!("Scanning for {name}");
        state.phase = Phase::Scanning(name.to_owned());
        // Scanning starts once the stack confirms these, in the `ScanParameterConfigured` event.
        if let Err(e) = self.gap.set_scan_params(&ScanParams::default()) {
            state.phase = Phase::Idle;
            return Err(e).context("failed to set scan parameters");
        }

        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, CONNECT_TIMEOUT, |state| state.phase.is_pending())
            .unwrap();

        match std::mem::replace(&mut state.phase, Phase::Idle) {
            Phase::Connected(connection) => {
                state.phase = Phase::Connected(connection);
                Ok(connection)
            }
            Phase::Failed(reason) => Err(anyhow!(reason)).context(format!("failed to connect to {name}")),
            Phase::Idle => bail!("{name} disconnected before the connection was established"),
            Phase::Scanning(_) => {
                let _ = self.gap.stop_scanning();
                bail!("timed out scanning for {name}")
            }
            Phase::Connecting(addr) => {
                // Cancels the pending connection.
                let _ = self.gap.disconnect(addr);
                bail!("timed out connecting to {name} at {addr}")
            }
        }
    }

    /// Drops the connection, if there is one. [`BleClient::wait_for_disconnect`] returns once it has gone.
    #[allow(dead_code)]
    pub fn disconnect(&self) -> anyhow::Result<()> {
        if let Phase::Connected(connection) = self.lock().phase {
            self.gap.disconnect(connection.addr)?;
        }

        Ok(())
    }

    /// Blocks for as long as a connection is open.
    pub fn wait_for_disconnect(&self) {
        let _state = self
            .changed
            .wait_while(self.lock(), |state| matches!(state.phase, Phase::Connected(_)))
            .unwrap();
    }

    fn on_gap_event(&self, event: BleGapEvent) -> anyhow::Result<()> {
        match event {
            BleGapEvent::ScanParameterConfigured(status) => {
                check_bt(status).context("failed to set scan parameters")?;
                self.gap.start_scanning(SCAN_SECONDS)?;
            }
            BleGapEvent::ScanStarted(status) => check_bt(status).context("failed to start scanning")?,
            BleGapEvent::ScanResult(GapSearchEvent::InquiryResult(result)) => self.on_scan_result(result)?,
            BleGapEvent::ScanResult(GapSearchEvent::InquiryComplete(_)) => {
                let mut state = self.lock();
                if let Phase::Scanning(name) = &state.phase {
                    state.phase = Phase::Failed(format!("{name} was not seen within {SCAN_SECONDS}s"));
                    self.changed.notify_all();
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn on_scan_result(&self, result: GapSearchResult) -> anyhow::Result<()> {
        let mut state = self.lock();

        let Phase::Scanning(target) = &state.phase else {
            return Ok(());
        };

        // Devices put their name either in the advertisement or the scan response, and may shorten it to fit.
        let name = result.ble_adv.and_then(|adv| {
            self.gap
                .resolve_adv_data_by_type(adv, AdvertisingDataType::NameCmpl)
                .or_else(|| self.gap.resolve_adv_data_by_type(adv, AdvertisingDataType::NameShort))
        });
        let name = name.and_then(|name| std::str::from_utf8(name).ok());

        log::debug!("Saw {} ({name:?}) at {} dBm", result.bda, result.rssi);

        if name != Some(target.as_str()) {
            return Ok(());
        }

        log::info!("Found {target} at {} ({} dBm), connecting", result.bda, result.rssi);

        let gattc_if = state.gattc_if.context("GATT client app is not registered")?;
        state.phase = Phase::Connecting(result.bda);

        // The radio is shared, so stop scanning before connecting.
        self.gap.stop_scanning()?;
        self.gattc
            .enh_open(gattc_if, &GattCreateConnParams::new(result.bda, result.ble_addr_type))?;

        Ok(())
    }

    fn on_gattc_event(&self, gattc_if: GattInterface, event: GattcEvent) -> anyhow::Result<()> {
        match event {
            GattcEvent::ClientRegistered { status, app_id } if app_id == APP_ID => {
                check_gatt(status).context("failed to register GATT client app")?;
                self.lock().gattc_if = Some(gattc_if);
                self.changed.notify_all();
            }
            GattcEvent::Open {
                status, conn_id, addr, ..
            } => {
                check_gatt(status).with_context(|| format!("failed to open a connection to {addr}"))?;

                self.lock().phase = Phase::Connected(Connection { conn_id, addr });
                self.changed.notify_all();
            }
            GattcEvent::Disconnected { addr, reason, .. } => {
                log::info!("Disconnected from {addr}: {reason:?}");

                let mut state = self.lock();
                // A failed attempt can also report a disconnect, keep the more useful reason it failed with.
                if !matches!(state.phase, Phase::Failed(_)) {
                    state.phase = Phase::Idle;
                }
                self.changed.notify_all();
            }
            _ => {}
        }

        Ok(())
    }

    /// Records that the connection attempt in progress went wrong, and wakes up [`BleClient::connect`] to report it.
    fn fail(&self, reason: String) {
        log::warn!("{reason}");

        let mut state = self.lock();
        if state.phase.is_pending() {
            state.phase = Phase::Failed(reason);
            self.changed.notify_all();
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }
}

fn check_bt(status: BtStatus) -> anyhow::Result<()> {
    match status {
        BtStatus::Success => Ok(()),
        status => bail!("{status:?}"),
    }
}

fn check_gatt(status: GattStatus) -> anyhow::Result<()> {
    match status {
        GattStatus::Ok => Ok(()),
        status => bail!("{status:?}"),
    }
}
