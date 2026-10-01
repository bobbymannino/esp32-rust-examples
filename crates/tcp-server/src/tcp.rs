use std::{
    io::{ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    thread,
    time::Duration,
};

use anyhow::{Context as _, Result};

/// Stack given to each client's thread.
const CLIENT_STACK_BYTES: usize = 8 * 1024;

/// How long a client can stay silent before it is disconnected. Without this a client that vanishes without closing
/// the connection
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Accepts connections on `port` forever, echoing back whatever each client sends.
pub fn serve(port: u16) -> Result<()> {
    // Ipv4Addr::UNSPECIFIED = 0.0.0.0 which is open to devices on the same network
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)))
        .with_context(|| format!("failed to bind port {port}"))?;

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            // One failed to accept, such as running out of sockets, should not take the whole server down.
            Err(e) => {
                log::warn!("accept failed: {e}");
                continue;
            }
        };

        let spawned = thread::Builder::new().stack_size(CLIENT_STACK_BYTES).spawn(move || {
            if let Err(e) = handle_client(stream) {
                log::warn!("client error: {e:#}");
            }
        });
        if let Err(e) = spawned {
            // Usually out of memory. The stream was moved into the closure, so dropping it closes the connection.
            log::warn!("could not start a thread for the client: {e}");
        }
    }

    Ok(())
}

/// Echoes everything `stream` sends back to it until the client disconnects or goes idle.
fn handle_client(mut stream: TcpStream) -> Result<()> {
}
