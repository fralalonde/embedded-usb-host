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

use core::mem;
pub use descriptor::*;
pub use setup::*;
pub use address::*;
pub use stack::*;

/// Errors that can be generated when attempting to do a USB transfer.
#[derive(Debug)]
#[derive(defmt::Format)]
pub enum TransferError {
    /// An error that may be retried.
    Retry(&'static str),

    /// A permanent error.
    Permanent(&'static str),
}

#[derive(Debug)]
#[derive(defmt::Format)]
pub enum HostEvent {
    Reset,
    Ready(device::Device),
    Tick,
}

/// Trait for host controller interface.
pub trait USBHost {
    fn max_host_packet_size(&self) -> u16;

    /// Issue a control transfer with an optional data stage to
    /// `ep`. The data stage direction is determined by the direction
    /// of `bm_request_type`.
    ///
    /// On success, the amount of data transferred into `buf` is
    /// returned.
    fn control_transfer(
        &mut self,
        ep: &mut dyn Endpoint,
        bm_request_type: RequestType,
        b_request: RequestCode,
        w_value: WValue,
        w_index: u16,
        buf: Option<&mut [u8]>,
    ) -> Result<usize, TransferError>;

    /// Issue a transfer from `ep` to the host.
    ///
    /// On success, the amount of data transferred into `buf` is
    /// returned.
    fn in_transfer(
        &mut self,
        ep: &mut dyn Endpoint,
        buf: &mut [u8],
    ) -> Result<usize, TransferError>;

    /// Issue a transfer from the host to `ep`.
    ///
    /// On success, the amount of data transferred from `buf` is
    /// returned. This should always be equal to `buf.len()`.
    fn out_transfer(&mut self, ep: &mut dyn Endpoint, buf: &[u8]) -> Result<usize, TransferError>;
}

/// The type of transfer to use when talking to USB devices.
///
/// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(defmt::Format)]
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

pub trait ControlEndpoint {
    fn control_get_descriptor(&self, host: &mut dyn USBHost, desc_type: DescriptorType, idx: u8, buffer: &mut [u8]) -> Result<usize, TransferError>;

    fn control_set(&self, host: &mut dyn USBHost, param: RequestCode, lo_val: u8, hi_val: u8, index: u16) -> Result<(), TransferError>;
}

pub trait BulkEndpoint {
    fn bulk_in(&self, host: &mut dyn USBHost, buffer: &mut [u8]) -> Result<usize, TransferError>;

    fn bulk_out(&self, host: &mut dyn USBHost, buffer: &[u8]) -> Result<usize, TransferError>;
}


#[derive(defmt::Format)]
pub struct SingleEp {
    pub device_address: Address,
    pub endpoint_address: u8,
    pub transfer_type: TransferType,
    pub max_packet_size: u16,
}

impl BulkEndpoint for SingleEp {
     fn bulk_in(&self, host: &mut dyn USBHost, buffer: &mut [u8]) -> Result<usize, TransferError> {
        todo!()
    }

     fn bulk_out(&self, host: &mut dyn USBHost, buffer: &[u8]) -> Result<usize, TransferError> {
        todo!()
    }
}

// impl Endpoint for SingleEp {
//     fn device_address(&self) -> Address {
//         self.device_address
//     }
//
//     fn endpoint_address(&self) -> u8 {
//         self.endpoint_address
//     }
//
//     fn transfer_type(&self) -> TransferType {
//         self.transfer_type
//     }
//
//     fn max_packet_size(&self) -> u16 {
//         self.max_packet_size
//     }
// }


/// `Endpoint` defines the USB endpoint for various transfers.
pub trait Endpoint {
    /// Address of the device owning this endpoint. Must be between 0
    /// and 127.
    fn address(&self) -> u8;

    /// Endpoint number, irrespective of direction. (e.g., for both
    /// endpoint addresses, `0x81` and `0x01`, this function would
    /// return `0x01`).
    fn endpoint_num(&self) -> u8;

    /// The type of transfer this endpoint uses.
    fn transfer_type(&self) -> TransferType;

    /// The direction of transfer this endpoint accepts.
    fn direction(&self) -> Direction;

    /// The maximum packet size for this endpoint.
    fn max_packet_size(&self) -> u16;

    /// The data toggle sequence bit for the next transfer from the
    /// device to the host.
    fn in_toggle(&self) -> bool;

    /// The `USBHost` will, when required, update the data toggle
    /// sequence bit for the next device to host transfer.
    fn set_in_toggle(&mut self, toggle: bool);

    /// The data toggle sequence bit for the next transfer from the
    /// host to the device.
    fn out_toggle(&self) -> bool;

    /// The `USBHost` will, when required, update the data toggle
    /// sequence bit for the next host to device transfer.
    fn set_out_toggle(&mut self, toggle: bool);
}

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
    /// Answering `true` to this not necessarily mean the driver will
    /// get `device`.
    fn want_device(&self, device: &DeviceDescriptor) -> bool;

    /// Add `device` with address `address` to the driver's registry,
    /// if necessary.
    fn add_device(&mut self, device: DeviceDescriptor, address: u8) -> Result<(), DriverError>;

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
    fn tick(&mut self, millis: u64, usbhost: &mut dyn USBHost) -> Result<(), DriverError>;
}

pub(crate) fn to_slice_mut<T>(v: &mut T) -> &mut [u8] {
    let ptr = v as *mut T as *mut u8;
    unsafe { core::slice::from_raw_parts_mut(ptr, mem::size_of::<T>()) }
}
