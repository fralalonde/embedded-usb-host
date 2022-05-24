# embedded-usb-host

An all-in-one host-side USB stack implementation destined for embedded (`no_std`) use.

Includes host driver for SAMD chips (for now). 

Includes class drivers for keyboard and MIDI devices.

## Status

Work in progress, alpha-level code but compiles and runs.

See https://github.com/fralalonde/usbmidi-host for sample application.

Not yet published on crates.io for maturity reasons.

## Missing
- Fix MIDI functionality
- STM32 support would be very nice.
- More class drivers (Hub, Mouse, etc.)
- Multi-pipe support

## Thanks

Based on https://github.com/bjc/bootkbd and https://github.com/bjc/atsamd-usb-host
