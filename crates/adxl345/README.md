# ADXL345

This project demonstrates how to use an ADXL345. It creates an easy to interact
with struct so you can read measurements while maintaining a clean DX.

## Wiring

The ADXL345 talks to the ESP32 over I2C at address `0x53`.

| ADXL345 | ESP32  | Notes                               |
| ------- | ------ | ----------------------------------- |
| SDA     | GPIO21 | I2C data, internal pull-up enabled  |
| SCL     | GPIO22 | I2C clock, internal pull-up enabled |
| INT1    | GPIO23 | Wakes the ESP32 from light sleep    |

## How it works

On boot the driver checks the device ID, sets the range to ±4g in full
resolution mode and turns on measure mode. Readings are returned in mg.

The main loop then:

1. Reads the X, Y and Z acceleration and logs it.
2. If any axis is above 1g, it keeps reading every 100ms.
3. Otherwise it arms the activity interrupt with a 1g threshold and puts the
   ESP32 into light sleep.
4. When the ADXL345 detects movement it pulls INT1 high, which wakes the ESP32.
   The interrupt source register is read (which clears it) and logged, and the
   loop starts again.

## Usage

```sh
cargo run
```
