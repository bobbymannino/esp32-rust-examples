# API Server

This project demonstrates how the ESP32 can host an API server that can be
reached via HTTP requests from another device on the same network.

## Endpoints

| Method | Path       | Response                                         |
| ------ | ---------- | ------------------------------------------------ |
| GET    | `/api/rtc` | `{"rtc":1234}`, the RTC in seconds since the Unix epoch (UTC) |

> [!NOTE]
> Nothing sets the clock, so it counts up from 0 at boot rather than holding
> the real time.

## Usage

The credentials are read at compile time, so they never have to be committed.
Pass them in when you flash the firmware:

```sh
WIFI_SSID="My Network" WIFI_PASSWORD="hunter2" cargo run
```

Once it is connected the IP address is logged, and the endpoint can be called
from another device on the same network:

```sh
curl http://<ip>/api/rtc
```
