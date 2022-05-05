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

#[macro_use]
extern crate hash32_derive;

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

#[cfg(feature = "atsamd")]
pub mod atsamd;

use hash32::Hasher;
use heapless::FnvIndexMap;
pub use descriptor::*;
pub use device::*;
pub use parser::*;
pub use setup::*;
pub use address::*;
pub use stack::*;
pub use endpoint::*;
pub use host::*;
pub use driver::*;
pub use class::*;

#[cfg(feature = "atsamd")]
pub use atsamd::*;

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
    TooManyDevices,
    TooManyEndpoints,
}

/// The type of transfer to use when talking to USB devices.
///
/// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum_macros::FromRepr)]
#[derive(defmt::Format)]
#[repr(u8)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

impl hash32::Hash for TransferType {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write(&[*self as u8])
    }
}

/// The direction of the transfer with the USB device.
///
/// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Out,
    In,
}

fn map_entry_mut<K: hash32::Hash + Eq + Copy, V, const N: usize, F: Fn() -> V>(map: &mut FnvIndexMap<K, V, N>, key: K, new: F) -> Option<&mut V> {
    if !map.contains_key(&key) {
        let _ = map.insert(key, new());
    }
    map.get_mut(&key)
}