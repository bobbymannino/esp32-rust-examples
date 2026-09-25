# WiFi WPA2/WPA3

This project demonstrates how to join a WPA2 or WPA3 protected network as a
station, and how to stay on it.

The example scans for the network first so it can connect with the security the
access point actually advertises, rather than guessing. That one step is what
makes it work against a WPA2 network, a WPA3 network, and an access point
running both at once, without any changes.

> [!WARNING]
> Do not use any network that is below WPA2

## Usage

The credentials are read at compile time, so they never have to be committed.
Pass them in when you flash the firmware:

```sh
WIFI_SSID="My Network" WIFI_PASSWORD="hunter2" cargo run
```

> [!TIP]
> If you do not want the password in the terminal history, create a `.sh` script
> file with the variables in it and run that instead.
