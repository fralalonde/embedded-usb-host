//! Simple USB host-side driver for USB keyboards supporting "boot" protocol (i.e. legacy PC BIOS compatible).
//! Tested working with classic Dell SK-8115
//! Does not handle newer keyboards such as HP KU-1156

use crate::{
    to_slice_mut, ConfigNum, DescriptorParser, DescriptorRef, DevAddress, Device, DeviceState, Driver, Endpoint,
    EndpointProperties, InterfaceNum, InterruptEndpoint, MaxPacketSize, UsbError, UsbHost,
};

use crate::class::DeviceClass;
use crate::hid::{HidDevice, HidProtocol, HidSubclass};
use heapless::FnvIndexMap;

// How many total devices this driver can support.
const MAX_DEVICES: usize = 2;

/// Boot protocol keyboard driver for USB hosts.
pub struct BootKbdDriver {
    device_endpoints: FnvIndexMap<DevAddress, Endpoint, MAX_DEVICES>,
}

impl Driver for BootKbdDriver {
    fn name(&self) -> &str {
        "BootKbd"
    }

    fn accept(
        &self, _device: &mut Device, parser: &mut DescriptorParser,
    ) -> Option<(DeviceClass, ConfigNum, InterfaceNum)> {
        let mut config_num = None;
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Configuration(cdesc) => {
                    info!("USB kbd conf {:?}", cdesc);
                    config_num.replace(cdesc.b_configuration_value);
                }
                DescriptorRef::Interface(idesc) => {
                    if idesc.b_interface_class == DeviceClass::Hid as u8
                        && idesc.b_interface_sub_class == HidSubclass::Boot as u8
                        && idesc.b_interface_protocol == HidDevice::Keyboard as u8
                    {
                        if let Some(config_num) = config_num {
                            info!("USB kbd iface {:?}", idesc);
                            return Some((DeviceClass::Hid, config_num, idesc.b_interface_number));
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
                    let new_ep = Endpoint::from_raw(
                        device.device_address(),
                        edesc.max_packet_size(),
                        edesc.b_endpoint_address,
                        edesc.bm_attributes,
                    );
                    if let Err(err) = self.device_endpoints.insert(device.device_address(), new_ep) {
                        warn!("Too many devices: {:?}", err)
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn unregister(&mut self, address: DevAddress) {
        // nothing we can do if this return None.
        let _ = self.device_endpoints.remove(&address);
    }

    fn state_after_config_set(&self, host: &mut dyn UsbHost, _device: &mut Device) -> DeviceState {
        DeviceState::SetInterface(0, host.after_millis(10))
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError> {
        for endpoint in self.device_endpoints.get_mut(&device.device_address()) {
            match device.state() {
                DeviceState::SetInterface(iface, until) => {
                    if host.delay_done(until) {
                        device.set_interface(host, iface, HidProtocol::Boot as u8)?;
                        device.set_state(DeviceState::Running);
                    }
                }

                DeviceState::Running => {
                    let mut buf = 0u64;
                    match endpoint.interrupt_in(host, to_slice_mut(&mut buf)) {
                        Ok(_size) => {
                            if buf > 0 {
                                // FIXME don't log, decode and pass to configured callback, see MIDI
                                info!("Got keys {:x}", buf)
                            }
                        }
                        Err(_) => {}
                    }
                }
                state => {
                    warn!("Driver not handling device in state {:?}", state)
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
