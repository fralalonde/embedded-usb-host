use hash32::Hasher;
use crate::{DevAddress, DescriptorType, Direction, EndpointDescriptor, RequestCode, UsbError, TransferType, UsbHost, Audio1EndpointDescriptor, RequestRecipient};

pub trait ControlEndpoint: Endpoint {
    fn control_get_descriptor(&mut self, host: &mut dyn UsbHost, desc_type: DescriptorType, idx: u8, buffer: &mut [u8]) -> Result<usize, UsbError>;

    fn control_set(&mut self, host: &mut dyn UsbHost, param: RequestCode, recip: RequestRecipient, lo_val: u8, hi_val: u8, index: u16) -> Result<(), UsbError>;
}

pub trait BulkEndpoint {
    fn bulk_in(&mut self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError>;

    fn bulk_out(&mut self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError>;
}

#[derive(Debug, PartialEq, Eq)]
#[derive(defmt::Format)]
// #[derive(hash32_derive::Hash32)]
pub struct SingleEp {
    device_address: DevAddress,
    max_packet_size: u16,
    endpoint_address: EpAddress,
    transfer_type: TransferType,
    in_toggle: bool,
    out_toggle: bool,
}

impl hash32::Hash for SingleEp {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write(&[self.device_address.into(), self.endpoint_address.into()])
    }
}

impl SingleEp {
    pub fn control(device_address: DevAddress, max_packet_size: u16) -> SingleEp {
        SingleEp {
            device_address,
            endpoint_address: 0.into(),
            transfer_type: TransferType::Control,
            max_packet_size,
            in_toggle: false,
            out_toggle: false,
        }
    }


    pub fn set_max_packet_size(&mut self, size: u16) {
        self.max_packet_size = size
    }

    pub fn set_device_address(&mut self, addr: DevAddress) {
        self.device_address = addr
    }
}

impl TryFrom<(&DevAddress, &EndpointDescriptor)> for SingleEp {
    type Error = UsbError;

    fn try_from(addr_ep_desc: (&DevAddress, &EndpointDescriptor)) -> Result<Self, Self::Error> {
        Ok(SingleEp {
            device_address: (*addr_ep_desc.0).into(),
            endpoint_address: addr_ep_desc.1.b_endpoint_address.into(),
            transfer_type: TransferType::from_repr(addr_ep_desc.1.bm_attributes).ok_or(UsbError::InvalidDescriptor)?,
            max_packet_size: addr_ep_desc.1.w_max_packet_size_lo as u16,
            in_toggle: false,
            out_toggle: false,
        })
    }
}

impl TryFrom<(&DevAddress, &Audio1EndpointDescriptor)> for SingleEp {
    type Error = UsbError;

    fn try_from(addr_ep_desc: (&DevAddress, &Audio1EndpointDescriptor)) -> Result<Self, Self::Error> {
        Ok(SingleEp {
            device_address: (*addr_ep_desc.0).into(),
            endpoint_address: addr_ep_desc.1.b_endpoint_address.into(),
            transfer_type: TransferType::from_repr(addr_ep_desc.1.bm_attributes).ok_or(UsbError::InvalidDescriptor)?,
            max_packet_size: addr_ep_desc.1.w_max_packet_size_lo as u16,
            in_toggle: false,
            out_toggle: false,
        })
    }
}

// impl BulkEndpoint for SingleEp {
//     fn bulk_in(&mut self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError> {
//         host.in_transfer(self, buffer)
//     }
//
//     fn bulk_out(&mut self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError> {
//         host.out_transfer(self, buffer)
//     }
// }

impl Endpoint for SingleEp {
    fn device_address(&self) -> DevAddress {
        self.device_address
    }

    fn endpoint_address(&self) -> EpAddress {
        self.endpoint_address
    }

    fn transfer_type(&self) -> TransferType {
        self.transfer_type
    }

    fn max_packet_size(&self) -> u16 {
        self.max_packet_size
    }

    fn in_toggle(&self) -> bool {
        self.in_toggle
    }

    fn set_in_toggle(&mut self, toggle: bool) {
        self.in_toggle = toggle
    }

    fn out_toggle(&self) -> bool {
        self.out_toggle
    }

    fn set_out_toggle(&mut self, toggle: bool) {
        self.out_toggle = toggle
    }
}

/// Bit 7 is the direction, with OUT = 0 and IN = 1
const ENDPOINT_DIRECTION_MASK: u8 = 0x80;

/// Bits 3..0 are the endpoint.rs number
const ENDPOINT_NUMBER_MASK: u8 = 0x0F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(defmt::Format)]
#[derive(hash32_derive::Hash32)]
pub struct EpAddress(u8);

impl EpAddress {
    /// Direction inferred from endpoint.rs address
    pub fn direction(&self) -> Direction {
        match self.0 & ENDPOINT_DIRECTION_MASK {
            0 => Direction::Out,
            _ => Direction::In
        }
    }
    /// Endpoint absolute number, irrespective of direction
    /// Two endpoints per interface can share the same absolute number (one IN, one OUT)
    pub fn absolute(&self) -> u8 {
        self.0 & ENDPOINT_NUMBER_MASK
    }
}

impl From<u8> for EpAddress {
    fn from(ep_addr: u8) -> Self {
        Self(ep_addr)
    }
}

impl From<EpAddress> for u8 {
    fn from(ep_addr: EpAddress) -> Self {
        ep_addr.0
    }
}

/// `Endpoint` defines the USB endpoint.rs for various transfers.
pub trait Endpoint {
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

    /// The maximum packet size for this endpoint.rs
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