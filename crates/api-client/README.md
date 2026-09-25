# API Client

This project demonstrates how an ESP32 can send a HTTP request to a server. In
this example there is a TypeScript API server with a single POST endpoint, this
endpoint just echoes the body of the request. The ESP32 is on a loop and every
5 seconds it will send a POST request with an incrementing counter.

## Usage

To run the API server, go into [api-server](api-server/) and run these 2
commands:

```sh
bun i
bun run index.ts
```

This will start a tiny web server with a single endpoint that will be used in
the ESP32 project.

To run the ESP32 project you must can use this command (substituting the
environment variables with your own values):

```sh
WIFI_SSID=<ssid> WIFI_PASSWORD=<password> IP=<API server ip> PORT=<API server port> cargo run
```
