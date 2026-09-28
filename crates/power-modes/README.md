# Power Modes

This project demonstrates the 5 different types of power modes for an ESP32.

Each mode turns off more of the chip than the one before it. The less that is
running, the less current is drawn, but the longer it takes to get back to work
and the less state survives the sleep.

## Types of Power Modes

| Mode        | Running Components                          | Typical Current | Wake Up Time         |
| ----------- | ------------------------------------------- | --------------- | -------------------- |
| Active      | Everything                                  | 95 - 240 mA     | N/A                  |
| Modem Sleep | CPU, RAM, peripherals (radio off)           | 20 - 68 mA      | Immediate            |
| Light Sleep | RAM (retained), RTC, ULP (CPU paused)       | 0.8 mA          | < 1 ms               |
| Deep Sleep  | RTC controller, RTC memory, RTC peripherals | 10 - 150 µA     | Full reboot (~100ms) |
| Hibernate   | RTC timer and RTC GPIOs only                | 5 µA            | Full reboot (~100ms) |

> [!NOTE]
> Current figures are taken from the ESP32 datasheet and are for the chip alone.
> A dev board will draw a lot more because of the USB-to-UART chip, voltage
> regulator and power LED.

### Active

The CPU, Wi-Fi/Bluetooth radio and all peripherals are powered. Current draw
peaks when the radio is transmitting, which is where the 240 mA comes from.

### Modem Sleep

The CPU keeps running but the radio is switched off between Wi-Fi beacons
(DTIM intervals), so the station stays connected to the access point. This is
the default whenever Wi-Fi is in station mode, and is controlled with
`esp_wifi_set_ps`. The CPU frequency has the biggest effect on the current here
(~20 mA at 80 MHz, up to ~68 mA at 240 MHz).

### Light Sleep

The CPU is paused and its clock is gated, but RAM is kept powered so execution
carries on from the line after `esp_light_sleep_start` once it wakes. Any
Wi-Fi/Bluetooth connection is dropped unless the modem is configured to wake
the chip.

Wake sources:

- Timer
- Any GPIO
- UART
- Touch Pad
- ULP (Ultra Low Power) Coprocessor

### Deep Sleep

The CPUs, most of the RAM and all digital peripherals are powered off. Only the
RTC domain stays on, which includes the ULP co-processor and 8 KB of RTC memory.
Waking up is a full reboot, `main` runs again from the top, so anything that
needs to survive has to be stored in RTC memory (`#[link_section = ".rtc.data"]`)
or flash.

Wake sources:

- Timer
- One RTC GPIO (`ext0`)
- Multiple RTC GPIOs (`ext1`)
- Touch Pad
- ULP (Ultra Low Power) Coprocessor

### Hibernate

Deep sleep with the RTC peripherals and RTC memory also powered down, leaving
only the RTC timer and RTC GPIOs. There is no dedicated API for it, it is deep
sleep with the extra power domains turned off through `esp_sleep_pd_config`.
Nothing survives apart from what is in flash.

Wake sources:

- Timer
- Multiple RTC GPIOs (`ext1`)
