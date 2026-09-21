# UART

This project demonstrates how to send and receive data over UART.

## Usage

To use the project flash the firmware onto the ESP:

```sh
cargo run
```

Then open a `screen` session on the port and type in letters from your keyboard.
You will see the letters show up twice. The first is the ESP log showing it has
received the data, and the second is the data echoed back to you on your
machine.

```sh
# Your USB port will be different
screen /dev/cu.usbserial-1130 115200
# To exit: <Ctrl-A> <k> <y>
```
