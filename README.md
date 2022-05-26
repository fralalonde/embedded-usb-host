# embedded-usb-host
An all-in-one host-side Rust USB stack implementation destined for embedded (`no_std`) use.

Includes host driver for SAMD chips (for now). 

Includes class drivers for keyboard and MIDI devices.

## Status
Work in progress, alpha-level code but compiles and runs.

Confirmed working with classic Dell SK-8115 keyboard, see examples/bootkbd

Also See https://github.com/fralalonde/usbmidi-host for sample MIDI application.

Not yet published on crates.io for obvious maturity reasons.

## Missing
- Concurrent, non-blocking transfers (yep, still all single threaded now)
- STM32 support would be _nice_
- Get string descriptors from device (forked UTF16 is ready!)
- More class drivers (Hub, Mouse, etc.)
- Alternative Async API
- Harmonize with `usb-device` crate for full OTG madness

## Known bugs
- Double fault on device reconnect (this is new)
- MIDI not coming in (Arturia Beatstep)
- HP keyboard (KU-1156) fails to get descriptor, why?

## Reference
- [USB Complete The Developer's Guide 4th Ed](https://doc.lagout.org/science/0_Computer%20Science/9_Others/9_Misc/USB%20Complete%20The%20Developer's%20Guide%204th%20Ed.pdf)
- [Atmel-42181-SAM-D21_Datasheet](https://community.atmel.com/sites/default/files/forum_attachments/SAM-D21-Family-Datasheet-DS40001882B.pdf)
- [osdev wiki](https://wiki.osdev.org/USB#Functions)
- [microsoft-usb-blog](https://techcommunity.microsoft.com/t5/microsoft-usb-blog/how-does-usb-stack-enumerate-a-device/ba-p/270685)

## Thanks
ATSAMD USB and bootkbd drivers based on https://github.com/bjc/bootkbd and https://github.com/bjc/atsamd-usb-host
