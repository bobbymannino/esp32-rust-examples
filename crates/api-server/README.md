# API Server

This project demonstrates how the ESP32 can host an API server that can be
reached via HTTP requests from another device on the same network.

## Endpoints

| Method | Path       | Request                                    | Response                                                      |
| ------ | ---------- | ------------------------------------------ | ------------------------------------------------------------- |
| GET    | `/api/rtc` |                                            | `{"rtc":1234}`, the RTC in seconds since the Unix epoch (UTC) |
| POST   | `/api/rtc` | `text/plain` seconds since the Unix epoch  | `{"rtc":1234}`, the RTC after it has been set                 |

A bad POST gets `415` if the body is not `text/plain`, `413` if it is too long,
and `400` if it is not a whole number of seconds.

> [!NOTE]
> The clock counts up from 0 at boot until something sets it, and it resets on
> every reboot.

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

To set the RTC to the current time:

```sh
curl -H "Content-Type: text/plain" -d "$(date +%s)" http://<ip>/api/rtc
```
