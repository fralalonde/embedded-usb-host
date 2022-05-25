use crate::address::DevAddress;
use crate::{
    to_slice_mut, ConfigNum, ConfigurationDescriptor, ControlEndpoint, DataToggle, DescriptorParser, DescriptorType,
    DeviceClass, DeviceDescriptor, EndpointProperties, EpAddress, HostEndpoint, HostError, InterfaceNum, MaxPacketSize,
    RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, TransferType, UsbError, UsbHost, WValue,
};

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DeviceState {
    /// Device needs an address
    SetAddress,

    /// Device needs a configuration to be selected
    SetConfig(u64),

    /// HID
    /// Device needs an interface to be selected (maybe)
    SetInterface(InterfaceNum, u64),

    /// HID
    SetReport(InterfaceNum),
    /// HID
    SetIdle,

    /// Device is active and polled
    Running,

    /// No driver
    Orphan,
}

pub enum DeviceOps {
    SetAddress(DevAddress),
    SetConfig(ConfigNum),
    SetInterface(InterfaceNum),
    GetConfigDescriptor(ConfigNum),
    GetDeviceDescriptor,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Device {
    device_address: DevAddress,
    state: DeviceState,
    max_packet_len: u16,
    toggle: bool,
    error: Option<UsbError>,
}

impl hash32::Hash for Device {
    fn hash<H>(&self, state: &mut H)
    where
        H: hash32::Hasher,
    {
        self.device_address.hash(state)
    }
}

impl Device {
    pub fn new(max_bus_packet_size: u16) -> Self {
        Self {
            state: DeviceState::SetAddress,
            device_address: DevAddress::from(0),
            max_packet_len: max_bus_packet_size,
            error: None,
            toggle: false,
        }
    }

    pub fn state(&self) -> DeviceState {
        self.state
    }

    pub fn set_state(&mut self, state: DeviceState) {
        self.state = state;
        debug!("USB Dev: {:?}", state)
    }

    pub fn error(&self) -> Option<UsbError> {
        self.error
    }

    pub fn set_error(&mut self, error: UsbError) {
        self.error = Some(error)
    }

    pub fn get_device_descriptor(&mut self, host: &mut dyn UsbHost) -> Result<DeviceDescriptor, UsbError> {
        let mut dev_desc: DeviceDescriptor = DeviceDescriptor::default();
        self.control_get_descriptor(host, DescriptorType::Device, 0, to_slice_mut(&mut dev_desc))?;
        if dev_desc.b_max_packet_size < self.max_packet_len as u8 {
            self.max_packet_len = dev_desc.b_max_packet_size as u16;
        }
        Ok(dev_desc)
    }

    pub fn get_configuration_descriptors(
        &mut self, host: &mut dyn UsbHost, cfg_idx: u8, buffer: &mut [u8],
    ) -> Result<usize, UsbError> {
        let mut config_root: ConfigurationDescriptor = ConfigurationDescriptor::default();
        self.control_get_descriptor(host, DescriptorType::Configuration, cfg_idx, to_slice_mut(&mut config_root))?;
        if config_root.w_total_length as usize > buffer.len() {
            Err(UsbError::DescriptorTooBig)
        } else {
            self.control_get_descriptor(
                host,
                DescriptorType::Configuration,
                cfg_idx,
                &mut buffer[..config_root.w_total_length as usize],
            )
        }
    }

    pub fn set_address(&mut self, host: &mut dyn UsbHost, dev_addr: DevAddress) -> Result<(), UsbError> {
        if 0u8 == self.device_address.into() {
            self.control_set(host, RequestCode::SetAddress, RequestRecipient::Device, dev_addr.into(), 0, 0)
                .map_err(|err| UsbError::SetAddress(self.ep_props(), err))?;
            self.device_address = dev_addr;
            self.state = DeviceState::SetConfig(host.after_millis(10));
            Ok(())
        } else {
            Err(UsbError::AddressSet)
        }
    }

    pub fn set_configuration(&mut self, host: &mut dyn UsbHost, config_num: u8) -> Result<(), UsbError> {
        if config_num == 0 {
            return Err(UsbError::InvalidConfig);
        }
        self.control_set(host, RequestCode::SetConfiguration, RequestRecipient::Device, config_num, 0, 0)
            .map_err(|err| UsbError::SetConfiguration(self.ep_props(), err))?;
        Ok(())
    }

    pub fn set_interface(&mut self, host: &mut dyn UsbHost, iface_num: u8, protocol: u8) -> Result<(), UsbError> {
        host.control_transfer(
            self,
            RequestType::from((RequestDirection::HostToDevice, RequestKind::Class, RequestRecipient::Interface)),
            RequestCode::SetInterface,
            WValue::lo_hi(protocol, 0),
            u16::from(iface_num),
            None,
        )
        .map_err(|err| UsbError::SetInterface(self.ep_props(), err))?;
        Ok(())
    }
}

impl HostEndpoint for Device {}

impl EndpointProperties for Device {
    fn device_address(&self) -> DevAddress {
        self.device_address
    }

    fn endpoint_address(&self) -> EpAddress {
        EpAddress::from(0)
    }

    fn transfer_type(&self) -> TransferType {
        TransferType::Control
    }
}

impl MaxPacketSize for Device {
    fn max_packet_size(&self) -> u16 {
        self.max_packet_len
    }
}

impl DataToggle for Device {
    fn toggle(&self) -> bool {
        self.toggle
    }

    fn set_toggle(&mut self, toggle: bool) {
        self.toggle = toggle
    }
}

impl ControlEndpoint for Device {
    /// Retrieve descriptor(s)
    fn control_get_descriptor(
        &mut self, host: &mut dyn UsbHost, desc_type: DescriptorType, desc_index: u8, buffer: &mut [u8],
    ) -> Result<usize, UsbError> {
        let len = host
            .control_transfer(
                self,
                RequestType::from((RequestDirection::DeviceToHost, RequestKind::Standard, RequestRecipient::Device)),
                RequestCode::GetDescriptor,
                WValue::lo_hi(desc_index, desc_type as u8),
                0,
                Some(buffer),
            )
            .map_err(|err| UsbError::GetDescriptor(self.ep_props(), err))?;
        Ok(len)
    }

    /// Generic control write
    fn control_set(
        &mut self, host: &mut dyn UsbHost, code: RequestCode, recip: RequestRecipient, lo_val: u8, hi_val: u8,
        windex: u16,
    ) -> Result<(), HostError> {
        host.control_transfer(
            self,
            RequestType::from((RequestDirection::HostToDevice, RequestKind::Standard, recip)),
            code,
            WValue::lo_hi(lo_val, hi_val),
            windex,
            None,
        )?;
        Ok(())
    }

    /// Generic control write
    fn control_set_class(
        &mut self, host: &mut dyn UsbHost, code: RequestCode, recip: RequestRecipient, lo_val: u8, hi_val: u8,
        windex: u16,
    ) -> Result<(), HostError> {
        host.control_transfer(
            self,
            RequestType::from((RequestDirection::HostToDevice, RequestKind::Class, recip)),
            code,
            WValue::lo_hi(lo_val, hi_val),
            windex,
            None,
        )?;
        Ok(())
    }
}

/// Trait for drivers on the USB host.
pub trait Driver {
    fn name(&self) -> &str;

    fn accept(
        &self, device: &mut Device, conf: &mut DescriptorParser,
    ) -> Option<(DeviceClass, ConfigNum, InterfaceNum)>;

    fn register(&mut self, device: &mut Device, conf: &mut DescriptorParser) -> Result<(), UsbError>;

    fn unregister(&mut self, device: DevAddress);

    /// HID drivers may overload this to return `DeviceState::SetProtocol`
    fn state_after_config_set(&self, _host: &mut dyn UsbHost, _device: &mut Device) -> DeviceState {
        DeviceState::Running
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError>;
}
