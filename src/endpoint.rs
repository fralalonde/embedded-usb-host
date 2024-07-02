use crate::{DevAddress, Direction, MaxPacketSize, TransferType, UsbError, UsbHost};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Endpoint {
    props: EpProps,
    max_packet_len: u16,
    toggle: bool,
}

impl Endpoint {
    pub fn from_raw(
        device_address: DevAddress, max_packet_size: u16, b_endpoint_address: u8, bm_attributes: u8,
    ) -> Self {
        Endpoint {
            props: EpProps {
                dev_addr: device_address,
                ep_addr: EpAddress::from(b_endpoint_address),
                tr_type: TransferType::from(bm_attributes),
            },
            max_packet_len: max_packet_size,
            toggle: false,
        }
    }

    pub fn set_max_packet_size(&mut self, size: u16) {
        self.max_packet_len = size
    }

    pub fn set_device_address(&mut self, addr: DevAddress) {
        self.props.dev_addr = addr
    }
}

/// Read-only utility structure that captures an endpoint's static properties.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct EpProps {
    dev_addr: DevAddress,
    ep_addr: EpAddress,
    tr_type: TransferType,
}

impl EndpointProperties for EpProps {
    fn device_address(&self) -> DevAddress {
        self.dev_addr
    }

    fn endpoint_address(&self) -> EpAddress {
        self.ep_addr
    }

    fn transfer_type(&self) -> TransferType {
        self.tr_type
    }
}

impl EndpointProperties for Endpoint {
    fn ep_props(&self) -> EpProps {
        self.props
    }

    fn device_address(&self) -> DevAddress {
        self.props.dev_addr
    }

    fn endpoint_address(&self) -> EpAddress {
        self.props.ep_addr
    }

    fn transfer_type(&self) -> TransferType {
        self.props.tr_type
    }
}

impl MaxPacketSize for Endpoint {
    fn max_packet_size(&self) -> u16 {
        self.max_packet_len
    }
}

impl DataToggle for Endpoint {
    fn toggle(&self) -> bool {
        self.toggle
    }

    fn set_toggle(&mut self, toggle: bool) {
        self.toggle = toggle
    }
}

/// Bit 7 is the direction, with OUT = 0 and IN = 1
const ENDPOINT_DIRECTION_MASK: u8 = 0x80;

/// Bits 3..0 are the endpoint.rs number
const ENDPOINT_NUMBER_MASK: u8 = 0x0F;

/// Max endpoint address is 0x7F - [0..63] + direction bit
const ENDPOINT_ADDRESS_MASK: u8 = ENDPOINT_DIRECTION_MASK + ENDPOINT_NUMBER_MASK;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EpAddress(u8);

impl EpAddress {
    /// Direction inferred from endpoint.rs address
    pub fn direction(&self) -> Direction {
        match self.0 & ENDPOINT_DIRECTION_MASK {
            0 => Direction::Out,
            _ => Direction::In,
        }
    }

    /// Absolute endpoint number, irrespective of direction
    /// Two endpoints per interface can share the same absolute number (one In, one Out)
    pub fn absolute(&self) -> u8 {
        self.0 & ENDPOINT_NUMBER_MASK
    }
}

impl From<u8> for EpAddress {
    fn from(addr: u8) -> Self {
        Self(addr & ENDPOINT_ADDRESS_MASK)
    }
}

impl From<EpAddress> for u8 {
    fn from(addr: EpAddress) -> Self {
        addr.0
    }
}

impl HostEndpoint for Endpoint {}

pub trait EndpointProperties {
    fn ep_props(&self) -> EpProps {
        EpProps {
            dev_addr: self.device_address(),
            ep_addr: self.endpoint_address(),
            tr_type: self.transfer_type(),
        }
    }

    /// Address of the device owning this endpoint.rs
    fn device_address(&self) -> DevAddress;

    /// Endpoint address, unique for the interface (includes direction bit)
    fn endpoint_address(&self) -> EpAddress;

    /// Endpoint address, unique for the interface (includes direction bit)
    fn direction(&self) -> Direction {
        self.endpoint_address().direction()
    }

    /// The type of transfer this endpoint.rs uses
    fn transfer_type(&self) -> TransferType;
}

pub trait DataToggle {
    /// The data toggle sequence bit for the next transfer from the
    /// device to the host.
    fn toggle(&self) -> bool;

    /// The `USBHost` will, when required, update the data toggle
    /// sequence bit for the next device to host transfer.
    fn set_toggle(&mut self, toggle: bool);

    fn flip_toggle(&mut self) -> bool {
        let flipped = !self.toggle();
        self.set_toggle(flipped);
        flipped
    }
}

pub trait BulkEndpoint: HostEndpoint + Sized {
    fn bulk_in(&mut self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError> {
        if self.transfer_type() != TransferType::Bulk {
            return Err(UsbError::TransferTypeMismatch);
        }
        if self.direction() != Direction::In {
            return Err(UsbError::DirectionMismatch);
        }
        host.in_transfer(self as &mut dyn HostEndpoint, buffer)
            .map_err(|err| UsbError::BulkIn(self.ep_props(), err))
    }

    fn bulk_out(&mut self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError> {
        if self.transfer_type() != TransferType::Bulk {
            return Err(UsbError::TransferTypeMismatch);
        }
        if self.direction() != Direction::Out {
            return Err(UsbError::DirectionMismatch);
        }
        host.out_transfer(self, buffer)
            .map_err(|err| UsbError::BulkOut(self.ep_props(), err))
    }
}

impl BulkEndpoint for Endpoint {}

pub trait InterruptEndpoint: HostEndpoint + Sized {
    fn interrupt_in(&mut self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError> {
        if self.transfer_type() != TransferType::Interrupt {
            return Err(UsbError::TransferTypeMismatch);
        }
        if self.direction() != Direction::In {
            return Err(UsbError::DirectionMismatch);
        }
        host.in_transfer(self as &mut dyn HostEndpoint, buffer)
            .map_err(|err| UsbError::Interrupt(self.ep_props(), err))
    }
}

impl InterruptEndpoint for Endpoint {}

pub trait HostEndpoint: DataToggle + MaxPacketSize + EndpointProperties {}
