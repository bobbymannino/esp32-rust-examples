# ESP32 Rust Examples

This project is comprised of multiple mini ESP32 projects. The aim is to provide
a set of useful examples for concepts such as WiFi, bluetooth, UART, etc.

## Projects

**Key**

✅ = Completed

❌ = Not Started

🏗️ = In Progress

| Project                            | Description                                         | Progress |
| ---------------------------------- | --------------------------------------------------- | :------: |
| [UART](crates/uart/)               | Send and receive data over UART                     |    ✅    |
| [WiFi WPA2/WPA3](crates/wifi-wpa/) | Connect to WiFi using WPA2/WPA3 authentication      |    ✅    |
| WiFi WPA Enterprise                | Connect to WiFi using WPA Enterprise authentication |    ❌    |
| [Bluetooth](crates/bluetooth/)     | Connect to Bluetooth devices                        |    ✅    |
| [API Client](crates/api-client/)   | Execute HTTP requests                               |    ✅    |
| API Server                         | Host HTTP endpoints on the ESP32                    |    ❌    |
| [Power Modes](crates/power-modes/) | Different power and sleep modes                     |    ✅    |
| ADXL345                            | Implement controls for the ADXL345 component        |    ❌    |
| EC11                               | Implement controls for the EC11 component           |    ❌    |
