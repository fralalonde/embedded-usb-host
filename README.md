# usb-host

This is a collection of traits that mediate between drivers for USB
peripherals and USB Host.

# Warning

This is still very early days, yet, and everything should be
considered experimental. However, it has been used to write a [driver
for a boot protocol keyboard](https://github.com/bjc/bootkbd), and can
presumably be used for other drivers as well.

The traits and structures defined in this crate are probably not
sufficient for general use. They are being kept purposefully minimal
to ease maintenance burden for implementors of these traits. If you
think there need to be further definitions, please open an issue (or,
even better, a pull request) on github describing the need.


A (partial) list of things that need to be changed, or at least looked
at, is maintained in `TODO.org`.

# Device driver crates.
  * [bootkbd](https://github.com/bjc/bootkbd) - A very simple
    boot-protocol keyboard driver.

# Host driver crates.
  * [atsamd-usb-host](https://github.com/bjc/atsamd-usb-host) - Host
    driver implementation for Atmel's SAMD* line of microcontrollers.
