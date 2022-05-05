use core::convert::TryInto;
use core::mem;

use crate::{ConfigurationDescriptor, ControlEndpoint, DescriptorType, DeviceDescriptor, Endpoint, EndpointDescriptor, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, SingleEp, UsbError, UsbHost, WValue, Audio1EndpointDescriptor, DescriptorParser};
use crate::address::{DevAddress};

#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(defmt::Format)]
enum DeviceState {
    Init,
    Addressed,
    StabilizingUntil(u64),
    GetConfig,
    ConfigSet,
    ProtocolSet,
    SetIdle,
    SetReport,
    Running,
}

#[derive(Debug, PartialEq)]
#[derive(defmt::Format)]
pub struct Device {
    state: DeviceState,
    control_ep: SingleEp,
}

impl hash32::Hash for Device {
    fn hash<H>(&self, state: &mut H) where H: hash32::Hasher {
        self.control_ep.hash(state)
    }
}

impl Device {
    pub fn new(max_bus_packet_size: u16) -> Self {
        Self {
            state: DeviceState::Init,
            control_ep: SingleEp::control(DevAddress::from(0), max_bus_packet_size)
        }
    }

    pub fn endpoint(&self, desc: &EndpointDescriptor) -> Result<SingleEp, UsbError> {
        (&self.control_ep.device_address(), desc).try_into()
    }

    pub fn audio1_endpoint(&self, desc: &Audio1EndpointDescriptor) -> Result<SingleEp, UsbError> {
        (&self.control_ep.device_address(), desc).try_into()
    }

    pub fn get_device_descriptor(&mut self, host: &mut dyn UsbHost) -> Result<DeviceDescriptor, UsbError> {
        let mut dev_desc: DeviceDescriptor = DeviceDescriptor::default();
        self.control_get_descriptor(host, DescriptorType::Device, 0, to_slice_mut(&mut dev_desc))?;
        if dev_desc.b_max_packet_size < self.control_ep.max_packet_size() as u8 {
            self.control_ep.set_max_packet_size(dev_desc.b_max_packet_size as u16);
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

    pub fn get_address(&self) -> DevAddress {
        self.control_ep.device_address()
    }

    pub fn set_address(&mut self, host: &mut dyn UsbHost, dev_addr: DevAddress) -> Result<(), UsbError> {
        if 0u8 == self.control_ep.device_address().into() {
            self.control_set(host, RequestCode::SetAddress, dev_addr.into(), 0, 0)?;
            self.control_ep.set_device_address(dev_addr);
            self.state = DeviceState::StabilizingUntil(host.now_ms() + 10);
            Ok(())
        } else {
            Err(UsbError::Permanent("Device Address Already Set"))
        }
    }

    // TODO unset_configuration()
    pub fn set_configuration(&mut self, host: &mut dyn UsbHost, config_num: u8) -> Result<(), UsbError> {
        if config_num == 0 {
            return Err(UsbError::Permanent("Invalid device configuration number"));
        }
        self.control_set(host, RequestCode::SetConfiguration, config_num, 0, 0)?;
        self.state = DeviceState::ConfigSet;
        Ok(())
    }

    // TODO get_interface()
    pub fn set_interface(&mut self, host: &mut dyn UsbHost, iface_num: u8, alt_num: u8) -> Result<(), UsbError> {
        self.control_set(host, RequestCode::SetInterface, alt_num, 0, iface_num as u16)?;
        self.state = DeviceState::ProtocolSet;
        Ok(())
    }
}

impl ControlEndpoint for Device {
    /// Retrieve descriptor(s)
    fn control_get_descriptor(&mut self, host: &mut dyn UsbHost, desc_type: DescriptorType, desc_index: u8, buffer: &mut [u8]) -> Result<usize, UsbError> {
        Ok(host.control_transfer(
            &mut self.control_ep,
            RequestType::from((RequestDirection::DeviceToHost, RequestKind::Standard, RequestRecipient::Device)),
            RequestCode::GetDescriptor,
            WValue::from((desc_index, desc_type as u8)),
            0,
            Some(buffer),
        )?)
    }

    /// Generic control write
    fn control_set(&mut self, host: &mut dyn UsbHost, param: RequestCode, lo_val: u8, hi_val: u8, index: u16) -> Result<(), UsbError> {
        host.control_transfer(
            &mut self.control_ep,
            RequestType::from((RequestDirection::HostToDevice, RequestKind::Standard, RequestRecipient::Device)),
            param,
            WValue::from((lo_val, hi_val)),
            index,
            None,
        )?;
        Ok(())
    }
}

fn to_slice_mut<T>(v: &mut T) -> &mut [u8] {
    let ptr = v as *mut T as *mut u8;
    unsafe { core::slice::from_raw_parts_mut(ptr, mem::size_of::<T>()) }
}

/// Trait for drivers on the USB host.
pub trait Driver {

    fn register(&mut self,  usbhost: &mut dyn UsbHost, device: &mut Device,  desc: &DeviceDescriptor, conf: &mut DescriptorParser) -> Result<bool, UsbError>;

    fn unregister(&mut self, device: &Device);

    fn tick(&mut self, host: &mut dyn UsbHost) -> Result<(), UsbError>;
}