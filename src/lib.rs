#![no_std]

mod descriptor;
mod setup;

pub use descriptor::*;
pub use setup::*;

#[derive(Debug)]
pub enum TransferError {
    Retry(&'static str),
    Permanent(&'static str),
}

pub trait USBHost {
    fn control_transfer(
        &mut self,
        ep: &mut dyn Endpoint,
        bm_request_type: RequestType,
        b_request: RequestCode,
        w_value: WValue,
        w_index: u16,
        buf: Option<&mut [u8]>,
    ) -> Result<usize, TransferError>;

    fn in_transfer(
        &mut self,
        ep: &mut dyn Endpoint,
        buf: &mut [u8],
    ) -> Result<usize, TransferError>;

    fn out_transfer(&mut self, ep: &mut dyn Endpoint, buf: &[u8]) -> Result<usize, TransferError>;
}

// cf §9.6.6 of USB 2.0
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TransferType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

// ibid
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Out,
    In,
}

pub trait Endpoint {
    fn address(&self) -> u8;

    fn endpoint_num(&self) -> u8;

    fn transfer_type(&self) -> TransferType;

    fn direction(&self) -> Direction;

    fn max_packet_size(&self) -> u16;

    fn in_toggle(&self) -> bool;

    fn set_in_toggle(&mut self, toggle: bool);

    fn out_toggle(&self) -> bool;

    fn set_out_toggle(&mut self, toggle: bool);
}

#[derive(Copy, Clone, Debug)]
pub enum DriverError {
    Retry(u8, &'static str),
    Permanent(u8, &'static str),
}
pub trait Driver: core::fmt::Debug {
    fn want_device(&self, device: &DeviceDescriptor) -> bool;

    fn add_device(&mut self, device: DeviceDescriptor, address: u8) -> Result<(), DriverError>;

    fn remove_device(&mut self, address: u8);

    fn tick(&mut self, millis: usize, usbhost: &mut dyn USBHost) -> Result<(), DriverError>;
}

// TODO: There needs to be a per-interface/function driver trait, as
// well, since that's how most drivers will actually work.
//
// As a result, the host driver has to at least get the full
// configuration.
