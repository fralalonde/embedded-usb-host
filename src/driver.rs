use crate::{ConfigurationDescriptor, DeviceDescriptor, UsbError, UsbHost};
use crate::device::Device;
use crate::parser::DescriptorParser;

// /// Types of errors that can be returned from a `Driver`.
// #[derive(Copy, Clone, Debug)]
// #[derive(defmt::Format)]
// pub enum DriverError {
//     /// An error that may be retried.
//     Retry(u8, &'static str),
//
//     /// A permanent error.
//     Permanent(u8, &'static str),
// }

/// Trait for drivers on the USB host.
pub trait Driver: core::fmt::Debug {
    // /// Does this driver want `device`?
    // ///
    // /// Answering `true` to this not necessarily mean the driver will get `device`.
    // fn want_device(&self, desc: &DeviceDescriptor) -> bool;

    /// return Ok(true) if driver took device (stop looking for other drivers)
    fn register(&mut self,  usbhost: &mut dyn UsbHost, device: &mut Device,  desc: &DeviceDescriptor, conf: &mut DescriptorParser) -> Result<bool, UsbError>;

    /// Remove the device at address `address` from the driver's
    /// registry, if necessary.
    fn unregister(&mut self, device: &Device);

    /// Called regularly by the USB host to allow the driver to do any
    /// work necessary on its registered devices.
    ///
    /// `millis` is the current time, in milliseconds from some
    /// arbitrary starting point. It should be expected that after a
    /// long enough run-time, this value will wrap.
    ///
    /// `usbhost` may be used for communication with the USB when
    /// required.
    fn tick(&mut self, usbhost: &mut dyn UsbHost) -> Result<(), UsbError>;
}