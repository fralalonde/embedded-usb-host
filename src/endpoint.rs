use crate::{
    DescriptorType, DevAddress, Direction, MaxPacketSize, RequestCode, RequestRecipient,
    TransferType, UsbError, UsbHost,
};
use hash32::Hasher;

pub trait ControlEndpoint: HostEndpoint {
    fn control_get_descriptor(
        &mut self,
        host: &mut dyn UsbHost,
        desc_type: DescriptorType,
        idx: u8,
        buffer: &mut [u8],
    ) -> Result<usize, UsbError>;

    fn control_set(
        &mut self,
        host: &mut dyn UsbHost,
        code: RequestCode,
        recip: RequestRecipient,
        lo_val: u8,
        hi_val: u8,
        index: u16,
    ) -> Result<(), UsbError>;

    fn control_set_class(
        &mut self,
        host: &mut dyn UsbHost,
        code: RequestCode,
        recip: RequestRecipient,
        lo_val: u8,
        hi_val: u8,
        windex: u16,
    ) -> Result<(), UsbError>;
}

pub trait BulkEndpoint {
    fn bulk_in(&mut self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError>;

    fn bulk_out(&mut self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError>;
}

#[derive(Debug, defmt::Format)]
pub struct Endpoint {
    props: EpProps,
    max_packet_size: u16,
    toggle: bool,
}

impl hash32::Hash for Endpoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.props.hash(state)
    }
}

impl Endpoint {
    pub fn from_raw(
        device_address: DevAddress,
        max_packet_size: u16,
        b_endpoint_address: u8,
        bm_attributes: u8,
    ) -> Self {
        Endpoint {
            props: EpProps {
                device_address,
                endpoint_address: EpAddress::from(b_endpoint_address),
                transfer_type: TransferType::from(bm_attributes),
            },
            max_packet_size,
            toggle: false,
        }
    }

    pub fn props(&self) -> &EpProps {
        &self.props
    }

    pub fn set_max_packet_size(&mut self, size: u16) {
        self.max_packet_size = size
    }

    pub fn set_device_address(&mut self, addr: DevAddress) {
        self.props.device_address = addr
    }
}

/// Read-only utility structure that captures an endpoint's static properties.
#[derive(Debug, Clone, Copy, Eq, PartialEq, defmt::Format, hash32_derive::Hash32)]
pub struct EpProps {
    device_address: DevAddress,
    endpoint_address: EpAddress,
    transfer_type: TransferType,
}

impl EndpointProperties for EpProps {
    fn device_address(&self) -> DevAddress {
        self.device_address
    }

    fn endpoint_address(&self) -> EpAddress {
        self.endpoint_address
    }

    fn transfer_type(&self) -> TransferType {
        self.transfer_type
    }
}

impl EndpointProperties for Endpoint {
    fn device_address(&self) -> DevAddress {
        self.props.device_address
    }

    fn endpoint_address(&self) -> EpAddress {
        self.props.endpoint_address
    }

    fn transfer_type(&self) -> TransferType {
        self.props.transfer_type
    }
}

impl MaxPacketSize for Endpoint {
    fn max_packet_size(&self) -> u16 {
        self.max_packet_size
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format, hash32_derive::Hash32)]
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
}

pub trait HostEndpoint: DataToggle + MaxPacketSize + EndpointProperties {}
