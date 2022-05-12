//! Simple USB host-side driver for boot protocol keyboards.

use crate::{Driver, UsbError, HostEndpoint, EndpointDescriptor, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, UsbHost, WValue, DescriptorParser, Device, DeviceState, DescriptorRef, DeviceClass, ConfigNum, InterfaceNum, DevAddress, Endpoint, EndpointProperties, MaxPacketSize, to_slice_mut};


use heapless::{FnvIndexMap, Vec};
use crate::hid::{HidDevice, HidProtocol, HidSubclass};

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
        DeviceState::SetProtocol(0, host.after_millis(10))
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError> {
        for ep in self.device_endpoints.get_mut(&device.device_address()) {
            match device.state() {
                DeviceState::SetProtocol(iface, until) => if host.delay_done(until) {
                    device.set_interface(host, iface, HidProtocol::Boot as u8)?;
                    device.set_state(DeviceState::Running);
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
