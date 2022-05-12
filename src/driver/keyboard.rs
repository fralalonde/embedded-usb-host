//! Simple USB host-side driver for boot protocol keyboards.

use crate::{Driver, UsbError, HostEndpoint, EndpointDescriptor, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, UsbHost, WValue, DescriptorParser, Device, DeviceState, DescriptorRef, DeviceClass, ConfigNum, InterfaceNum, DevAddress, Endpoint, EndpointProperties, MaxPacketSize, to_slice_mut};


use heapless::{FnvIndexMap, Vec};

// How many total devices this driver can support.
const MAX_DEVICES: usize = 2;

// And how many endpoints we can support per-device.
const MAX_ENDPOINTS: usize = 2;

// The maximum size configuration descriptor we can handle.
const CONFIG_BUFFER_LEN: usize = 256;

static BOOT_KEYBOARD_PORT: Vec<u8, 16> = Vec::new();

/// Boot protocol keyboard driver for USB hosts.
pub struct BootKbdDriver {
    device_endpoints: FnvIndexMap<DevAddress, Endpoint, MAX_DEVICES>,
}

#[repr(u8)]
pub enum HidSubclass {
    NoBoot = 0,
    Boot = 1,
}

#[repr(u8)]
pub enum HidDevice {
    Keyboard = 1,
    Mouse = 2,
}

#[repr(u8)]
pub enum HidProtocol {
    Boot = 0,
    Report = 1,
}

const HID_PROTOCOL_KEYBOARD: u8 = 0x01;
const HID_PROTOCOL_MOUSE: u8 = 0x02;

impl Driver for BootKbdDriver {
    fn accept(&self, device: &mut Device, parser: &mut DescriptorParser) -> Option<(ConfigNum, InterfaceNum)> {
        let mut config_num = None;
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Configuration(cdesc) => {
                    config_num.replace(cdesc.b_configuration_value);
                }
                DescriptorRef::Interface(idesc) => {
                    if idesc.b_interface_class == DeviceClass::Hid as u8
                        && idesc.b_interface_sub_class == HidSubclass::Boot as u8
                        && idesc.b_interface_protocol == HidDevice::Keyboard as u8
                    {
                        if let Some(config_num) = config_num {
                            info!("{}", idesc);
                            return Some((config_num, idesc.b_interface_number));
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn register(&mut self, device: &mut Device, parser: &mut DescriptorParser) -> Result<(), UsbError> {
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Endpoint(edesc) => {
                    let new_ep = Endpoint::from_raw(device.device_address(), edesc.max_packet_size(), edesc.b_endpoint_address, edesc.bm_attributes);
                    self.device_endpoints.insert(device.device_address(), new_ep);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn unregister(&mut self, address: DevAddress) {
        self.device_endpoints.remove(&address);
    }

    fn state_after_config_set(&self, host: &mut dyn UsbHost, _device: &mut Device) -> DeviceState {
        // TODO get correct interface in here
        DeviceState::SetProtocol(0, host.after_millis(10))
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError> {
        for ep in self.device_endpoints.get_mut(&device.device_address()) {
            match device.state() {
                DeviceState::SetProtocol(iface, until) => if host.delay_done(until) {
                    device.set_interface(host, iface, HidProtocol::Boot as u8)?;
                    device.set_state(DeviceState::Running);
                }

                DeviceState::SetIdle => {
                    host.control_transfer(
                        ep,
                        RequestType::from((
                            RequestDirection::HostToDevice,
                            RequestKind::Class,
                            RequestRecipient::Interface,
                        )),
                        RequestCode::GetInterface,
                        WValue::lo_hi(0, 0),
                        0,
                        None,
                    )?;
                    device.set_state(DeviceState::SetReport(0))
                }

                DeviceState::SetReport(iface_num) => {
                    let mut report: [u8; 1] = [0];
                    host.control_transfer(
                        ep,
                        RequestType::from((
                            RequestDirection::HostToDevice,
                            RequestKind::Class,
                            RequestRecipient::Interface,
                        )),
                        RequestCode::SetConfiguration,
                        WValue::lo_hi(0, 2),
                        u16::from(iface_num),
                        Some(&mut report),
                    )?;
                    device.set_state(DeviceState::Running)
                }
                DeviceState::Running => {
                    let mut buf = 0u64;
                    match host.in_transfer(ep, to_slice_mut(&mut buf)) {
                        Ok(0) => {}
                        Ok(_size) => {
                            if buf > 0 {
                                info!("Got keys {:x}", buf)
                                // TODO cast to BootKbdPacket & decode
                            }
                        }
                        Err(_) => {}
                    }
                }
                state => {
                    warn!("Driver not handling device in state {}", state)
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
#[derive(defmt::Format)]
pub struct BootKbdPacket {
    modifiers: u8,
    r0: u8,
    keys: [u8; 6],
}

impl BootKbdDriver {
    pub fn new() -> Self {
        Self {
            device_endpoints: FnvIndexMap::new(),
        }
    }
}


// #[cfg(test)]
// mod test {
//     use super::*;
//
//     #[test]
//     fn add_remove_device() {
//         let mut driver = BootKeyboard::new(|_addr, _report| {});
//
//         let count = |driver: &mut BootKeyboard<_>| {
//             driver
//                 .devices
//                 .iter()
//                 .fold(0, |sum, dev| sum + dev.as_ref().map_or(0, |_| 1))
//         };
//         assert_eq!(count(&mut driver), 0);
//
//         driver.add_device(dummy_device(), 2).unwrap();
//         assert_eq!(count(&mut driver), 1);
//
//         driver.remove_device(2);
//         assert_eq!(count(&mut driver), 0);
//     }
//
//     #[test]
//     fn too_many_devices() {
//         let mut driver = BootKeyboard::new(|_addr, _report| {});
//
//         for i in 0..MAX_DEVICES {
//             driver.add_device(dummy_device(), (i + 1) as u8).unwrap();
//         }
//         assert!(driver
//             .add_device(dummy_device(), (MAX_DEVICES + 1) as u8)
//             .is_err());
//     }
//
//     #[test]
//     fn tick_propagates_errors() {
//         let mut dummyhost = DummyHost { fail: true };
//
//         let mut calls = 0;
//         let mut driver = BootKeyboard::new(|_addr, _report| calls += 1);
//
//         driver.add_device(dummy_device(), 1).unwrap();
//         driver.tick(0, &mut dummyhost).unwrap();
//         assert!(driver.tick(SETTLE_DELAY + 1, &mut dummyhost).is_err());
//     }
//
//     #[test]
//     fn parse_logitech_g105_config() {
//         // Config, Interface (0.0), HID, Endpoint, Interface (1.0), HID, Endpoint
//         let raw: &[u8] = &[
//             0x09, 0x02, 0x3b, 0x00, 0x02, 0x01, 0x04, 0xa0, 0x64, 0x09, 0x04, 0x00, 0x00, 0x01,
//             0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00, 0x07,
//             0x05, 0x81, 0x03, 0x08, 0x00, 0x0a, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x00, 0x00,
//             0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00, 0x07, 0x05, 0x82, 0x03,
//             0x08, 0x00, 0x0a,
//         ];
//         let mut parser = DescriptorParser::from(raw);
//
//         let config_desc = ConfigurationDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Configuration,
//             w_total_length: 59,
//             b_num_interfaces: 2,
//             b_configuration_value: 1,
//             i_configuration: 4,
//             bm_attributes: 0xa0,
//             b_max_power: 100,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Configuration(cdesc) = desc {
//             assert_eq!(*cdesc, config_desc, "Configuration descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 0,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x03,     // HID
//             b_interface_sub_class: 0x01, // Boot Interface,
//             b_interface_protocol: 0x01,  // Keyboard
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // Unknown descriptor just yields a byte slice.
//         let hid_desc1: &[u8] = &[0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00];
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(cdesc) = desc {
//             assert_eq!(cdesc, hid_desc1, "HID descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x81,
//             bm_attributes: 0x03,
//             w_max_packet_size: 0x08,
//             b_interval: 0x0a,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let interface_desc2 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 1,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x03,     // HID
//             b_interface_sub_class: 0x00, // No subclass
//             b_interface_protocol: 0x00,  // No protocol
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc2, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // Unknown descriptor just yields a byte slice.
//         let hid_desc2 = &[0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00];
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(cdesc) = desc {
//             assert_eq!(cdesc, hid_desc2, "HID descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let endpoint_desc2 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x82,
//             bm_attributes: 0x03,
//             w_max_packet_size: 0x08,
//             b_interval: 0x0a,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc2, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         assert!(parser.next().is_none(), "Extra descriptors.");
//     }
//
//     #[test]
//     fn logitech_g105_discovers_ep0() {
//         // Config, Interface (0.0), HID, Endpoint, Interface (1.0), HID, Endpoint
//         let raw: &[u8] = &[
//             0x09, 0x02, 0x3b, 0x00, 0x02, 0x01, 0x04, 0xa0, 0x64, 0x09, 0x04, 0x00, 0x00, 0x01,
//             0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00, 0x07,
//             0x05, 0x81, 0x03, 0x08, 0x00, 0x0a, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x00, 0x00,
//             0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00, 0x07, 0x05, 0x82, 0x03,
//             0x08, 0x00, 0x0a,
//         ];
//
//         let (got_inum, got) = ep_for_bootkbd(raw).expect("Looking for endpoint");
//         let want = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x81,
//             bm_attributes: 0x03,
//             w_max_packet_size: 0x08,
//             b_interval: 0x0a,
//         };
//         assert_eq!(got_inum, 0);
//         assert_eq!(*got, want);
//     }
//
//     fn dummy_device() -> DeviceDescriptor {
//         DeviceDescriptor {
//             b_length: mem::size_of::<DeviceDescriptor>() as u8,
//             b_descriptor_type: DescriptorType::Device,
//             bcd_usb: 0x0110,
//             b_device_class: 0,
//             b_device_sub_class: 0,
//             b_device_protocol: 0,
//             b_max_packet_size: 8,
//             id_vendor: 0xdead,
//             id_product: 0xbeef,
//             bcd_device: 0xf00d,
//             i_manufacturer: 1,
//             i_product: 2,
//             i_serial_number: 3,
//             b_num_configurations: 1,
//         }
//     }
//
//     #[test]
//     fn parse_keyboardio_config() {
//         let raw: &[u8] = &[
//             0x09, 0x02, 0x96, 0x00, 0x05, 0x01, 0x00, 0xa0, 0xfa, 0x08, 0x0b, 0x00, 0x02, 0x02,
//             0x02, 0x01, 0x00, 0x09, 0x04, 0x00, 0x00, 0x01, 0x02, 0x02, 0x00, 0x00, 0x05, 0x24,
//             0x00, 0x10, 0x01, 0x05, 0x24, 0x01, 0x01, 0x01, 0x04, 0x24, 0x02, 0x06, 0x05, 0x24,
//             0x06, 0x00, 0x01, 0x07, 0x05, 0x81, 0x03, 0x10, 0x00, 0x40, 0x09, 0x04, 0x01, 0x00,
//             0x02, 0x0a, 0x00, 0x00, 0x00, 0x07, 0x05, 0x02, 0x02, 0x40, 0x00, 0x00, 0x07, 0x05,
//             0x83, 0x02, 0x40, 0x00, 0x00, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00,
//             0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00, 0x07, 0x05, 0x84, 0x03, 0x40,
//             0x00, 0x01, 0x09, 0x04, 0x03, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x01,
//             0x01, 0x00, 0x01, 0x22, 0x72, 0x00, 0x07, 0x05, 0x85, 0x03, 0x40, 0x00, 0x01, 0x09,
//             0x04, 0x04, 0x00, 0x01, 0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x01, 0x01, 0x00, 0x01,
//             0x22, 0x3f, 0x00, 0x07, 0x05, 0x86, 0x03, 0x40, 0x00, 0x01,
//         ];
//         let mut parser = DescriptorParser::from(raw);
//
//         let config_desc = ConfigurationDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Configuration,
//             w_total_length: 150,
//             b_num_interfaces: 5,
//             b_configuration_value: 1,
//             i_configuration: 0,
//             bm_attributes: 0xa0,
//             b_max_power: 250,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Configuration(cdesc) = desc {
//             assert_eq!(*cdesc, config_desc, "Configuration descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // Interface Association Descriptor
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x08, 0x0b, 0x00, 0x02, 0x02, 0x02, 0x01, 0x00];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 0,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x02,     // Communications and CDC Control
//             b_interface_sub_class: 0x02, // Abstract Control Model
//             b_interface_protocol: 0x00,
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // Four communications descriptors.
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x05, 0x24, 0x00, 0x10, 0x01];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x05, 0x24, 0x01, 0x01, 0x01];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x04, 0x24, 0x02, 0x06];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x05, 0x24, 0x06, 0x00, 0x01];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x81,
//             bm_attributes: 0x03,
//             w_max_packet_size: 16,
//             b_interval: 64,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // CDC-Data interface.
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 1,
//             b_alternate_setting: 0,
//             b_num_endpoints: 2,
//             b_interface_class: 0x0a, // CDC-Data
//             b_interface_sub_class: 0x00,
//             b_interface_protocol: 0x00,
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x02,
//             bm_attributes: 0x02,
//             w_max_packet_size: 64,
//             b_interval: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x83,
//             bm_attributes: 0x02,
//             w_max_packet_size: 64,
//             b_interval: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID interface.
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 2,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x03, // HID
//             b_interface_sub_class: 0x00,
//             b_interface_protocol: 0x00,
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID Descriptor.
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x84,
//             bm_attributes: 0x03,
//             w_max_packet_size: 64,
//             b_interval: 1,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID interface.
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 3,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x03, // HID
//             b_interface_sub_class: 0x00,
//             b_interface_protocol: 0x00,
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID Descriptor.
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x72, 0x00];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x85,
//             bm_attributes: 0x03,
//             w_max_packet_size: 64,
//             b_interval: 1,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID interface.
//         let interface_desc1 = InterfaceDescriptor {
//             b_length: 9,
//             b_descriptor_type: DescriptorType::Interface,
//             b_interface_number: 4,
//             b_alternate_setting: 0,
//             b_num_endpoints: 1,
//             b_interface_class: 0x03,     // HID
//             b_interface_sub_class: 0x01, // Boot Interface
//             b_interface_protocol: 0x01,  // Keyboard
//             i_interface: 0,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Interface(cdesc) = desc {
//             assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         // HID Descriptor.
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Other(odesc) = desc {
//             let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x3f, 0x00];
//             assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
//         } else {
//             panic!("Wrong descriptor type.")
//         }
//
//         let endpoint_desc1 = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x86,
//             bm_attributes: 0x03,
//             w_max_packet_size: 64,
//             b_interval: 1,
//         };
//         let desc = parser.next().expect("Parsing configuration");
//         if let Descriptor::Endpoint(cdesc) = desc {
//             assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
//         } else {
//             panic!("Wrong descriptor type.");
//         }
//
//         assert!(parser.next().is_none(), "Extra descriptors.");
//     }
//
//     #[test]
//     fn keyboardio_discovers_bootkbd() {
//         let raw: &[u8] = &[
//             0x09, 0x02, 0x96, 0x00, 0x05, 0x01, 0x00, 0xa0, 0xfa, 0x08, 0x0b, 0x00, 0x02, 0x02,
//             0x02, 0x01, 0x00, 0x09, 0x04, 0x00, 0x00, 0x01, 0x02, 0x02, 0x00, 0x00, 0x05, 0x24,
//             0x00, 0x10, 0x01, 0x05, 0x24, 0x01, 0x01, 0x01, 0x04, 0x24, 0x02, 0x06, 0x05, 0x24,
//             0x06, 0x00, 0x01, 0x07, 0x05, 0x81, 0x03, 0x10, 0x00, 0x40, 0x09, 0x04, 0x01, 0x00,
//             0x02, 0x0a, 0x00, 0x00, 0x00, 0x07, 0x05, 0x02, 0x02, 0x40, 0x00, 0x00, 0x07, 0x05,
//             0x83, 0x02, 0x40, 0x00, 0x00, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00,
//             0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00, 0x07, 0x05, 0x84, 0x03, 0x40,
//             0x00, 0x01, 0x09, 0x04, 0x03, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x01,
//             0x01, 0x00, 0x01, 0x22, 0x72, 0x00, 0x07, 0x05, 0x85, 0x03, 0x40, 0x00, 0x01, 0x09,
//             0x04, 0x04, 0x00, 0x01, 0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x01, 0x01, 0x00, 0x01,
//             0x22, 0x3f, 0x00, 0x07, 0x05, 0x86, 0x03, 0x40, 0x00, 0x01,
//         ];
//
//         let (got_inum, got) = ep_for_bootkbd(raw).expect("Looking for endpoint");
//         let want = EndpointDescriptor {
//             b_length: 7,
//             b_descriptor_type: DescriptorType::Endpoint,
//             b_endpoint_address: 0x86,
//             bm_attributes: 0x03,
//             w_max_packet_size: 64,
//             b_interval: 1,
//         };
//         assert_eq!(got_inum, 4);
//         assert_eq!(*got, want);
//     }
//
//     struct DummyHost {
//         fail: bool,
//     }
//
//     impl UsbHost for DummyHost {
//         fn control_transfer(
//             &mut self,
//             _ep: &mut dyn Endpoint,
//             _bm_request_type: RequestType,
//             _b_request: RequestCode,
//             _w_value: WValue,
//             _w_index: u16,
//             _buf: Option<&mut [u8]>,
//         ) -> Result<usize, UsbError> {
//             if self.fail {
//                 Err(UsbError::Permanent("foo"))
//             } else {
//                 Ok(0)
//             }
//         }
//
//         fn in_transfer(
//             &mut self,
//             _ep: &mut dyn Endpoint,
//             _buf: &mut [u8],
//         ) -> Result<usize, UsbError> {
//             if self.fail {
//                 Err(UsbError::Permanent("foo"))
//             } else {
//                 Ok(0)
//             }
//         }
//
//         fn out_transfer(
//             &mut self,
//             _ep: &mut dyn Endpoint,
//             _buf: &[u8],
//         ) -> Result<usize, UsbError> {
//             if self.fail {
//                 Err(UsbError::Permanent("foo"))
//             } else {
//                 Ok(0)
//             }
//         }
//     }
// }
