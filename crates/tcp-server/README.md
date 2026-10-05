# TCP Server

This project demonstrates how the ESP32 can host a TCP server that other devices
on the same network can connect to. It is an echo server: everything a client
sends is sent straight back.

Each client gets its own thread, so several can be connected at once. A client
that sends nothing for 30 seconds is disconnected.

> [!NOTE]
> lwIP allows 10 sockets by default (`CONFIG_LWIP_MAX_SOCKETS`) and the
> listener uses one of them, so at most 9 clients can be connected at a time.

## Usage

The credentials and port are read at compile time, so they never have to be
committed. Pass them in when you flash the firmware:

```sh
WIFI_SSID="My Network" WIFI_PASSWORD="password" PORT=8080 cargo run
```

Once it is connected the IP address is logged, and the server can be reached
from another device on the same network:

```sh
nc <ip> 8080
```

Type a line and press enter to have it echoed back.
