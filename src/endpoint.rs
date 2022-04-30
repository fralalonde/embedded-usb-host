use crate::{Address, DescriptorType, Direction, EndpointDescriptor, RequestCode, UsbError, TransferType, UsbHost, Audio1EndpointDescriptor};

pub trait ControlEndpoint {
    fn control_get_descriptor(&mut self, host: &mut dyn UsbHost, desc_type: DescriptorType, idx: u8, buffer: &mut [u8]) -> Result<usize, UsbError>;

    fn control_set(&mut self, host: &mut dyn UsbHost, param: RequestCode, lo_val: u8, hi_val: u8, index: u16) -> Result<(), UsbError>;
}

pub trait BulkEndpoint {
    fn bulk_in(&self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError>;

    fn bulk_out(&self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError>;
}

#[derive(Clone, Debug, PartialEq)]
#[derive(defmt::Format)]
pub struct SingleEp {
    pub device_address: Address,
    pub max_packet_size: u16,
    endpoint_address: u8,
    transfer_type: TransferType,
    in_toggle: bool,
    out_toggle: bool,
}

impl SingleEp {
    pub fn control(device_address: Address, max_packet_size: u16) -> SingleEp {
        SingleEp {
            device_address,
            endpoint_address: 0,
            transfer_type: TransferType::Control,
            max_packet_size,
            in_toggle: false,
            out_toggle: false,
        }
    }
}

impl TryFrom<(&Address, &EndpointDescriptor)> for SingleEp {
    type Error = UsbError;

    fn try_from(addr_ep_desc: (&Address, &EndpointDescriptor)) -> Result<Self, Self::Error> {
        Ok(SingleEp {
            device_address: (*addr_ep_desc.0).into(),
            endpoint_address: addr_ep_desc.1.b_endpoint_address,
            transfer_type: TransferType::from_repr(addr_ep_desc.1.bm_attributes).ok_or(UsbError::InvalidDescriptor)?,
            max_packet_size: addr_ep_desc.1.w_max_packet_size_lo as u16,
            in_toggle: false,
            out_toggle: false,
        })
    }
}

impl TryFrom<(&Address, &Audio1EndpointDescriptor)> for SingleEp {
    type Error = UsbError;

    fn try_from(addr_ep_desc: (&Address, &Audio1EndpointDescriptor)) -> Result<Self, Self::Error> {
        Ok(SingleEp {
            device_address: (*addr_ep_desc.0).into(),
            endpoint_address: addr_ep_desc.1.b_endpoint_address,
            transfer_type: TransferType::from_repr(addr_ep_desc.1.bm_attributes).ok_or(UsbError::InvalidDescriptor)?,
            max_packet_size: addr_ep_desc.1.w_max_packet_size_lo as u16,
            in_toggle: false,
            out_toggle: false,
        })
    }
}

impl BulkEndpoint for SingleEp {
    fn bulk_in(&self, host: &mut dyn UsbHost, buffer: &mut [u8]) -> Result<usize, UsbError> {
        todo!()
    }

    fn bulk_out(&self, host: &mut dyn UsbHost, buffer: &[u8]) -> Result<usize, UsbError> {
        todo!()
    }
}

impl Endpoint for SingleEp {
    fn device_address(&self) -> Address {
        self.device_address
    }

    fn endpoint_address(&self) -> u8 {
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

/// `Endpoint` defines the USB endpoint.rs for various transfers.
pub trait Endpoint {
    /// Address of the device owning this endpoint.rs
    fn device_address(&self) -> Address;

    /// Endpoint address, unique for the interface (includes direction bit)
    fn endpoint_address(&self) -> u8;

    /// Direction inferred from endpoint.rs address
    fn direction(&self) -> Direction {
        match self.endpoint_address() & ENDPOINT_DIRECTION_MASK  {
            0 => Direction::Out,
            _ => Direction::In
        }
    }

    /// Endpoint number, irrespective of direction
    /// Two endpoints per interface can share the same number (one IN, one OUT)
    fn endpoint_num(&self) -> u8 {
        self.endpoint_address() & ENDPOINT_NUMBER_MASK
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