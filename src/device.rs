use core::convert::TryInto;

use crate::{ConfigurationDescriptor, ControlEndpoint, DescriptorType, EndpointDescriptor, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, to_slice_mut, TransferError, TransferType, USBHost, WValue};
use crate::address::{Address};
use crate::Endpoint;

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
enum DeviceState {
    // Init,
    // Settling(u64),
    Addressed,
    GetConfig,
    SetConfig(u8),
    SetProtocol,
    SetIdle,
    SetReport,
    Running,
}

#[derive(Debug, defmt::Format)]
pub struct Device {
    state: DeviceState,
    address: Address,
    max_packet_size: u16,
}


// /// Control endpoint for device
// impl Endpoint for Device {
//     fn device_address(&self) -> Address {
//         self.address
//     }
//
//     fn endpoint_address(&self) -> u8 {
//         0
//     }
//
//     fn transfer_type(&self) -> TransferType {
//         TransferType::Control
//     }
//
//     fn max_packet_size(&self) -> u16 {
//         self.max_packet_size
//     }
// }

impl Device {
    pub fn new(max_bus_packet_size: u16, addr: Address) -> Self {
        Self {
            state: DeviceState::Addressed,
            address: addr,
            max_packet_size: max_bus_packet_size,
        }
    }

    // pub fn endpoint(&self, desc: &EndpointDescriptor) -> Result<SingleEp, TransferError> {
    //     (self.address, desc).try_into()
    // }

    // pub fn get_device_descriptor(&mut self, host: &mut dyn USBHost) -> Result<DeviceDescriptor, TransferError> {
    //     let mut dev_desc: DeviceDescriptor = DeviceDescriptor::default();
    //     self.control_get_descriptor(host, DescriptorType::Device, 0, to_slice_mut(&mut dev_desc))?;
    //     if dev_desc.b_max_packet_size < self.max_packet_size as u8 {
    //         self.max_packet_size = dev_desc.b_max_packet_size as u16;
    //     }
    //     Ok(dev_desc)
    // }

    // pub fn get_configuration_descriptors(&mut self, host: &mut dyn USBHost, cfg_idx: u8, buffer: &mut [u8]) -> Result<usize, TransferError> {
    //     let mut config_root: ConfigurationDescriptor = ConfigurationDescriptor::default();
    //     self.control_get_descriptor(host, DescriptorType::Configuration, cfg_idx, to_slice_mut(&mut config_root))?;
    //     if config_root.w_total_length as usize > buffer.len() {
    //         Err(TransferError::Permanent("Device config larger than buffer"))
    //     } else {
    //         self.control_get_descriptor(host, DescriptorType::Configuration, cfg_idx, buffer)
    //     }
    // }

    pub fn get_address(&self) -> Address {
        self.address
    }

    // pub fn set_address(&mut self, host: &mut dyn USBHost, dev_addr: Address) -> Result<(), TransferError> {
    //     if 0u8 == self.address.into() {
    //         self.control_set(host, RequestCode::SetAddress, dev_addr.into(), 0, 0)?;
    //         self.address = dev_addr;
    //         Ok(())
    //     } else {
    //         Err(TransferError::Permanent("Device Address Already Set"))
    //     }
    // }

    // TODO unset_configuration()
    // pub fn set_configuration(&mut self, host: &mut dyn USBHost, config_num: u8) -> Result<(), TransferError> {
    //     if config_num == 0 {
    //         return Err(TransferError::Permanent("Invalid device configuration number"));
    //     }
    //     self.control_set(host, RequestCode::SetConfiguration, config_num, 0, 0)
    // }

    // TODO get_interface()
    // pub fn set_interface(&mut self, host: &mut dyn USBHost, iface_num: u8, alt_num: u8) -> Result<(), TransferError> {
    //     self.control_set(host, RequestCode::SetInterface, alt_num, 0, iface_num as u16)
    // }
}

// impl ControlEndpoint for Device {
//     /// Retrieve descriptor(s)
//     fn control_get_descriptor(&self, host: &mut dyn USBHost, desc_type: DescriptorType, desc_index: u8, buffer: &mut [u8]) -> Result<usize, TransferError> {
//         Ok(host.control_transfer(
//             self,
//             RequestType::from((RequestDirection::DeviceToHost, RequestKind::Standard, RequestRecipient::Device)),
//             RequestCode::GetDescriptor,
//             WValue::from((desc_index, desc_type as u8)),
//             0,
//             Some(buffer),
//         )?)
//     }
//
//     /// Generic control write
//     fn control_set(&self, host: &mut dyn USBHost, param: RequestCode, lo_val: u8, hi_val: u8, index: u16) -> Result<(), TransferError> {
//         host.control_transfer(
//             self,
//             RequestType::from((RequestDirection::HostToDevice, RequestKind::Standard, RequestRecipient::Device)),
//             param,
//             WValue::from((lo_val, hi_val)),
//             index,
//             None,
//         )?;
//         Ok(())
//     }
// }


