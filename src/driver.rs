use crate::{DeviceDescriptor, UsbHost};

/// Types of errors that can be returned from a `Driver`.
#[derive(Copy, Clone, Debug)]
#[derive(defmt::Format)]
pub enum DriverError {
    /// An error that may be retried.
    Retry(u8, &'static str),

    /// A permanent error.
    Permanent(u8, &'static str),
}

/// Trait for drivers on the USB host.
pub trait Driver: core::fmt::Debug {
    /// Does this driver want `device`?
    ///
    /// Answering `true` to this not necessarily mean the driver will get `device`.
    fn want_device(&self, desc: &DeviceDescriptor) -> bool;

    /// Add `device` with address `address` to the driver's registry,
    /// if necessary.
    fn add_device(&mut self, desc: DeviceDescriptor, address: u8) -> Result<(), DriverError>;

    /// Remove the device at address `address` from the driver's
    /// registry, if necessary.
    fn remove_device(&mut self, address: u8);

    /// Called regularly by the USB host to allow the driver to do any
    /// work necessary on its registered devices.
    ///
    /// `millis` is the current time, in milliseconds from some
    /// arbitrary starting point. It should be expected that after a
    /// long enough run-time, this value will wrap.
    ///
    /// `usbhost` may be used for communication with the USB when
    /// required.
    fn tick(&mut self, usbhost: &mut dyn UsbHost) -> Result<(), DriverError>;
}