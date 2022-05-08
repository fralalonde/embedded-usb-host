use core::mem;

use crate::{ConfigurationDescriptor, ControlEndpoint, DescriptorType, DeviceDescriptor, Endpoint, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, SingleEp, UsbError, UsbHost, WValue, Audio1EndpointDescriptor, DescriptorParser, ConfigNum, EpAddress, TransferType, EndpointDescriptor};
use crate::address::{DevAddress};

#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(defmt::Format)]
pub enum DeviceState {
    Init,
    // settle for 10ms after address set
    AddressSet(u64),
    // settle for 10ms after config set
    ConfigSet(u64),
    ProtocolSet,
    // HID(HidStates)
    Running,
    Orphan,
    Error(UsbError),
}

#[derive(Debug, PartialEq)]
#[derive(defmt::Format)]
pub struct Device {
    state: DeviceState,
    device_address: DevAddress,
    max_packet_size: u16,
    in_toggle: bool,
    out_toggle: bool,
}

impl hash32::Hash for Device {
    fn hash<H>(&self, state: &mut H) where H: hash32::Hasher {
        self.device_address.hash(state)
    }
}

impl Device {
    pub fn new(max_bus_packet_size: u16) -> Self {
        Self {
            state: DeviceState::Init,
            device_address: DevAddress::from(0),
            max_packet_size: max_bus_packet_size,
            in_toggle: false,
            out_toggle: false
        }
    }

    pub fn state(&self) -> DeviceState {
        self.state
    }

    pub fn set_state(&mut self, state: DeviceState) {
        self.state = state
    }

    pub fn endpoint(&self, desc: &EndpointDescriptor) -> Result<SingleEp, UsbError> {
        SingleEp::try_from((&self.device_address, desc))
    }

    pub fn audio1_endpoint(&self, desc: &Audio1EndpointDescriptor) -> Result<SingleEp, UsbError> {
        SingleEp::try_from((&self.device_address, desc))
    }

    pub fn get_device_descriptor(&mut self, host: &mut dyn UsbHost) -> Result<DeviceDescriptor, UsbError> {
        let mut dev_desc: DeviceDescriptor = DeviceDescriptor::default();
        self.control_get_descriptor(host, DescriptorType::Device, 0, to_slice_mut(&mut dev_desc))?;
        if dev_desc.b_max_packet_size < self.max_packet_size as u8 {
            self.max_packet_size = dev_desc.b_max_packet_size as u16;
        }
        Ok(dev_desc)
    }

    pub fn get_configuration_descriptors(&mut self, host: &mut dyn UsbHost, cfg_idx: u8, buffer: &mut [u8]) -> Result<usize, UsbError> {
        let mut config_root: ConfigurationDescriptor = ConfigurationDescriptor::default();
        self.control_get_descriptor(host, DescriptorType::Configuration, cfg_idx, to_slice_mut(&mut config_root))?;
        if config_root.w_total_length as usize > buffer.len() {
            Err(UsbError::Permanent("USB Device config larger than buffer"))
        } else {
            self.control_get_descriptor(host, DescriptorType::Configuration, cfg_idx, &mut buffer[0..config_root.w_total_length as usize])
        }
    }

    pub fn set_address(&mut self, host: &mut dyn UsbHost, dev_addr: DevAddress) -> Result<(), UsbError> {
        if 0u8 == self.device_address.into() {
            self.control_set(host, RequestCode::SetAddress, RequestRecipient::Device, dev_addr.into(), 0, 0)?;
            self.device_address = dev_addr;
            self.state = DeviceState::AddressSet(host.after_millis(10));
            Ok(())
        } else {
            Err(UsbError::Permanent("Device Address Already Set"))
        }
    }

    pub fn set_configuration(&mut self, host: &mut dyn UsbHost, config_num: u8) -> Result<(), UsbError> {
        if config_num == 0 {
            return Err(UsbError::Permanent("Invalid device configuration number"));
        }
        self.control_set(host, RequestCode::SetConfiguration, RequestRecipient::Device, config_num, 0, 0)
    }

    pub fn set_interface(&mut self, host: &mut dyn UsbHost, iface_num: u8, alt_num: u8) -> Result<(), UsbError> {
        self.control_set(host, RequestCode::SetInterface, RequestRecipient::Interface, alt_num, 0, iface_num as u16)
    }
}

impl Endpoint for Device {
    fn device_address(&self) -> DevAddress {
        self.device_address
    }

    fn endpoint_address(&self) -> EpAddress {
        EpAddress::from(0)
    }

    fn transfer_type(&self) -> TransferType {
        TransferType::Control
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

impl ControlEndpoint for Device {
    /// Retrieve descriptor(s)
    fn control_get_descriptor(&mut self, host: &mut dyn UsbHost, desc_type: DescriptorType, desc_index: u8, buffer: &mut [u8]) -> Result<usize, UsbError> {
        host.control_transfer(
            self,
            RequestType::from((RequestDirection::DeviceToHost, RequestKind::Standard, RequestRecipient::Device)),
            RequestCode::GetDescriptor,
            WValue::from((desc_index, desc_type as u8)),
            0,
            Some(buffer))
    }

    /// Generic control write
    fn control_set(&mut self, host: &mut dyn UsbHost, param: RequestCode, recip: RequestRecipient, lo_val: u8, hi_val: u8, windex: u16) -> Result<(), UsbError> {
        host.control_transfer(
            self,
            RequestType::from((RequestDirection::HostToDevice, RequestKind::Standard, recip)),
            param,
            WValue::from((lo_val, hi_val)),
            windex,
            None)?;
        Ok(())
    }
}

fn to_slice_mut<T>(v: &mut T) -> &mut [u8] {
    let ptr = v as *mut T as *mut u8;
    unsafe { core::slice::from_raw_parts_mut(ptr, mem::size_of::<T>()) }
}

/// Trait for drivers on the USB host.
pub trait Driver {
    fn accept(&self, device: &mut Device, conf: &mut DescriptorParser) -> Option<ConfigNum>;
    fn register(&mut self, device: &mut Device, conf: &mut DescriptorParser) -> Result<(), UsbError>;

    fn unregister(&mut self, device: DevAddress);

    fn set_protocol(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError> {
        device.set_interface(host, 0, 0);
        Ok(())
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError>;
}