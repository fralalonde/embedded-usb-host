//! This crate defines a set of traits for use on the host side of the
//! USB.
//!
//! The `USBHost` defines the Host Controller Interface that can be
//! used by the `Driver` interface.
//!
//! The `Driver` interface defines the set of functions necessary to
//! use devices plugged into the host.

#![no_std]

#[macro_use]
extern crate defmt;

pub mod descriptor;
pub mod setup;
pub mod address;
pub mod class;
pub mod device;
pub mod parser;
pub mod stack;
pub mod endpoint;
pub mod host;
pub mod driver;

use core::mem;
pub use descriptor::*;
pub use setup::*;
pub use address::*;
pub use stack::*;
pub use endpoint::*;
pub use host::*;
pub use driver::*;

/// Errors that can be generated when attempting to do a USB transfer.
#[derive(Debug)]
#[derive(defmt::Format)]
pub enum UsbError {
    /// An error that may be retried.
    Transient(&'static str),

    /// A permanent error.
    Permanent(&'static str),

    InvalidDescriptor,
    Driver,
}

/// The type of transfer to use when talking to USB devices.
///
/// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq, strum_macros::FromRepr)]
#[derive(defmt::Format)]
#[repr(u8)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

/// The direction of the transfer with the USB device.
///
/// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Out,
    In,
}




