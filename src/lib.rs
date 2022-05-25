//! This crate defines a set of traits for use on the host side of the
//! USB.
//!
//! The `USBHost` defines the Host Controller Interface that can be
//! used by the `Driver` interface.
//!
//! The `Driver` interface defines the set of functions necessary to
//! use devices plugged into the host.

#![no_std]

#[cfg(feature = "defmt")]
#[macro_use]
extern crate defmt;

#[cfg(feature = "log")]
#[macro_use]
extern crate log;

#[macro_use]
extern crate hash32_derive;

#[macro_use]
extern crate static_assertions;

pub mod address;
pub mod class;
pub mod control;
pub mod descriptor;
pub mod device;
pub mod driver;
pub mod endpoint;
pub mod host;
pub mod parser;
pub mod stack;

#[cfg(feature = "atsamd")]
pub mod atsamd;

#[cfg(feature = "stm32")]
pub mod stm32;

pub use address::*;
pub use class::*;
pub use control::*;
use core::mem;
pub use descriptor::*;
pub use device::*;
pub use driver::*;
pub use endpoint::*;
use hash32::Hasher;
use heapless::FnvIndexMap;
pub use host::*;
pub use parser::*;
pub use stack::*;

/// Errors that can be generated when attempting to do a USB transfer.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UsbError {
    AddressSet,
    OutOfAddresses,
    DescriptorTooBig,
    InvalidConfig,
    Control(DevAddress, RequestType, RequestCode, HostError),
    BulkIn(EpProps, HostError),
    BulkOut(EpProps, HostError),
    InvalidDescriptor,
    Driver,
    NoDriver,
    OutOfRange,
    TooManyDrivers,
    TooManyDevices,
    TooManyEndpoints,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(unused)]
pub enum HostError {
    // your. request. is. invalid.
    InvalidRequest,
    // NAK means "still no data" and is retryable for bulk
    Nak,
    // STALL means "no data" and finishes the transaction
    Stall,
    Fail,
    Toggle,
    Crc,
    Pid,
    DataPid,
    SoftTimeout,
    HardTimeout,
}

/// The type of transfer to use when talking to USB devices.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum_macros::FromRepr)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

const TRANSFER_TYPE_MASK: u8 = 0b00000011;

impl From<u8> for TransferType {
    fn from(ttype: u8) -> Self {
        unsafe { TransferType::from_repr(ttype & TRANSFER_TYPE_MASK).unwrap_unchecked() }
    }
}

impl hash32::Hash for TransferType {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
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

pub type ConfigNum = u8;
pub type InterfaceNum = u8;

pub trait MaxPacketSize {
    fn max_packet_size(&self) -> u16;
}

fn map_entry_mut<K: hash32::Hash + Eq + Copy, V, const N: usize, F: Fn() -> V>(
    map: &mut FnvIndexMap<K, V, N>, key: K, new: F,
) -> Option<&mut V> {
    if !map.contains_key(&key) {
        let _ = map.insert(key, new());
    }
    map.get_mut(&key)
}

fn to_slice_mut<T>(v: &mut T) -> &mut [u8] {
    let ptr = v as *mut T as *mut u8;
    unsafe { core::slice::from_raw_parts_mut(ptr, mem::size_of::<T>()) }
}

#[cfg(test)]
fn assert_offset<T>(name: &str, field: &T, base: usize, offset: usize) {
    let ptr = field as *const _ as usize;
    assert_eq!(ptr - base, offset, "{} register offset.", name);
}
