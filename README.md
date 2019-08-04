# usb-host

This is a collection of traits that mediate between drivers for USB
peripherals and USB Host.

This is still very early days, yet, and everything should be
considered experimental. However, it has been used to write a [driver
for a boot protocol keyboard](https://github.com/bjc/bootkbd), and can
presumably be used for other drivers as well.

# Device driver crates.
  * [bootkbd](https://github.com/bjc/bootkbd) - A very simple
    boot-protocol keyboard driver.

# Host driver crates.
  * [atsamd-usb-host](https://github.com/bjc/atsamd-usb-host) - Host
    driver implementation for Atmel's SAMD* line of microcontrollers.
