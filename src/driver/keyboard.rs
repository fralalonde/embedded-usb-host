//! Simple USB host-side driver for boot protocol keyboards.

use crate::{ConfigurationDescriptor, DescriptorType, DeviceDescriptor, Direction, Driver, UsbError, Endpoint, EndpointDescriptor, InterfaceDescriptor, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, TransferType, UsbHost, WValue, DescriptorParser};

use core::convert::TryFrom;
use core::mem::{self, MaybeUninit};
use core::ptr;
use heapless::Vec;

// How long to wait before talking to the device again after setting
// its address. cf §9.2.6.3 of USB 2.0
const SETTLE_DELAY: usize = 2;

// How many total devices this driver can support.
const MAX_DEVICES: usize = 32;

// And how many endpoints we can support per-device.
const MAX_ENDPOINTS: usize = 2;

// The maximum size configuration descriptor we can handle.
const CONFIG_BUFFER_LEN: usize = 256;

/// Boot protocol keyboard driver for USB hosts.
pub struct BootKeyboard<F> {
    devices: Vec<Option<Device>, MAX_DEVICES>,
    callback: F,
}

impl<F> core::fmt::Debug for BootKeyboard<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BootKeyboard")
    }
}

impl<F> BootKeyboard<F>
    where
        F: FnMut(u8, &[u8]),
{

    pub fn new(callback: F) -> Self {
        Self { devices: Vec::new(), callback }
    }
}

impl<F> Driver for BootKeyboard<F>
    where F: FnMut(u8, &[u8]),
{
    fn register(&mut self, usbhost: &mut dyn UsbHost, device: &mut crate::Device, desc: &DeviceDescriptor, conf: &mut DescriptorParser) -> Result<bool, UsbError> {
        if let Some(ref mut d) = self.devices.iter_mut().find(|d| d.is_none()) {
            **d = Some(Device::new(address, device.b_max_packet_size));
            Ok(true)
        } else {
            Err(UsbError::Permanent("Too many kbd"))
        }
    }

    fn unregister(&mut self, device: &crate::Device) {
        if let Some(ref mut d) = self
            .devices
            .iter_mut()
            .find(|d| d.as_ref().map_or(false, |dd| dd.addr == address))
        {
            **d = None;
        }
    }

    fn tick(&mut self, host: &mut dyn UsbHost) -> Result<(), UsbError> {
        for dev in self.devices.iter_mut().filter_map(|d| d.as_mut()) {
            if let Err(TransferError::Permanent(e)) = dev.fsm(millis, host, &mut self.callback) {
                return Err(UsbError::Permanent(dev.addr, e));
            }
        }
    }

    // fn tick(&mut self, host: &mut dyn UsbHost) -> Result<(), UsbError> {

    //     Ok(())
    // }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum DeviceState {
    Addressed,
    WaitForSettle(usize),
    GetConfig,
    SetConfig(u8),
    SetProtocol,
    SetIdle,
    SetReport,
    Running,
}

struct Device {
    addr: u8,
    ep0: EP,
    endpoints: [Option<EP>; MAX_ENDPOINTS],
    state: DeviceState,
}

impl Device {
    fn new(addr: u8, max_packet_size: u8) -> Self {
        let endpoints: [Option<EP>; MAX_ENDPOINTS] = {
            let mut eps: [MaybeUninit<Option<EP>>; MAX_ENDPOINTS] =
                unsafe { mem::MaybeUninit::uninit().assume_init() };
            for ep in &mut eps[..] {
                unsafe { ptr::write(ep.as_mut_ptr(), None) }
            }
            unsafe { mem::transmute(eps) }
        };

        Self {
            addr,
            ep0: EP::new(
                addr,
                0,
                0,
                TransferType::Control,
                Direction::In,
                u16::from(max_packet_size),
            ),
            endpoints,
            state: DeviceState::Addressed,
        }
    }

    fn fsm(
        &mut self,
        millis: usize,
        host: &mut dyn UsbHost,
        callback: &mut dyn FnMut(u8, &[u8]),
    ) -> Result<(), TransferError> {
        // TODO: either we need another `control_transfer` that
        // doesn't take data, or this `none` value needs to be put in
        // the usb-host layer. None of these options are good.
        let none: Option<&mut [u8]> = None;
        unsafe {
            static mut LAST_STATE: DeviceState = DeviceState::Addressed;
            if LAST_STATE != self.state {
                info!("{:?} -> {:?}", LAST_STATE, self.state);
                LAST_STATE = self.state;
            }
        }

        match self.state {
            DeviceState::Addressed => {
                self.state = DeviceState::WaitForSettle(millis + SETTLE_DELAY)
            }

            DeviceState::WaitForSettle(until) => {
                // TODO: This seems unnecessary. We're not using the device descriptor at all.
                if millis > until {
                    let mut dev_desc: MaybeUninit<DeviceDescriptor> = MaybeUninit::uninit();
                    let buf = unsafe { to_slice_mut(&mut dev_desc) };
                    let len = host.control_transfer(
                        &mut self.ep0,
                        RequestType::from((
                            RequestDirection::DeviceToHost,
                            RequestKind::Standard,
                            RequestRecipient::Device,
                        )),
                        RequestCode::GetDescriptor,
                        WValue::from((0, DescriptorType::Device as u8)),
                        0,
                        Some(buf),
                    )?;
                    assert!(len == mem::size_of::<DeviceDescriptor>());
                    self.state = DeviceState::GetConfig
                }
            }

            DeviceState::GetConfig => {
                let mut conf_desc: MaybeUninit<ConfigurationDescriptor> = MaybeUninit::uninit();
                let desc_buf = unsafe { to_slice_mut(&mut conf_desc) };
                let len = host.control_transfer(
                    &mut self.ep0,
                    RequestType::from((
                        RequestDirection::DeviceToHost,
                        RequestKind::Standard,
                        RequestRecipient::Device,
                    )),
                    RequestCode::GetDescriptor,
                    WValue::from((0, DescriptorType::Configuration as u8)),
                    0,
                    Some(desc_buf),
                )?;
                assert!(len == mem::size_of::<ConfigurationDescriptor>());
                let conf_desc = unsafe { conf_desc.assume_init() };

                if (conf_desc.w_total_length as usize) > CONFIG_BUFFER_LEN {
                    trace!("config descriptor: {:?}", conf_desc);
                    return Err(TransferError::Permanent("config descriptor too large"));
                }

                // TODO: do a real allocation later. For now, keep a
                // large-ish static buffer and take an appropriately
                // sized slice into it for the transfer.
                let mut config =
                    unsafe { MaybeUninit::<[u8; CONFIG_BUFFER_LEN]>::uninit().assume_init() };
                let config_buf = &mut config[..conf_desc.w_total_length as usize];
                let len = host.control_transfer(
                    &mut self.ep0,
                    RequestType::from((
                        RequestDirection::DeviceToHost,
                        RequestKind::Standard,
                        RequestRecipient::Device,
                    )),
                    RequestCode::GetDescriptor,
                    WValue::from((0, DescriptorType::Configuration as u8)),
                    0,
                    Some(config_buf),
                )?;
                assert!(len == conf_desc.w_total_length as usize);
                let (interface_num, ep) =
                    ep_for_bootkbd(config_buf).expect("no boot keyboard found");
                info!("Boot keyboard found on {:?}", ep);

                self.endpoints[0] = Some(EP::new(
                    self.addr,
                    ep.b_endpoint_address & 0x7f,
                    interface_num,
                    TransferType::Interrupt,
                    Direction::In,
                    ep.w_max_packet_size,
                ));

                // TODO: browse configs and pick the "best" one. But
                // this should always be ok, at least.
                self.state = DeviceState::SetConfig(1)
            }

            DeviceState::SetConfig(config_index) => {
                host.control_transfer(
                    &mut self.ep0,
                    RequestType::from((
                        RequestDirection::HostToDevice,
                        RequestKind::Standard,
                        RequestRecipient::Device,
                    )),
                    RequestCode::SetConfiguration,
                    WValue::from((config_index, 0)),
                    0,
                    none,
                )?;

                self.state = DeviceState::SetProtocol;
            }

            DeviceState::SetProtocol => {
                if let Some(ref ep) = self.endpoints[0] {
                    host.control_transfer(
                        &mut self.ep0,
                        RequestType::from((
                            RequestDirection::HostToDevice,
                            RequestKind::Class,
                            RequestRecipient::Interface,
                        )),
                        RequestCode::SetInterface,
                        WValue::from((0, 0)),
                        u16::from(ep.interface_num),
                        None,
                    )?;

                    self.state = DeviceState::SetIdle;
                } else {
                    return Err(TransferError::Permanent("no boot keyboard"));
                }
            }

            DeviceState::SetIdle => {
                host.control_transfer(
                    &mut self.ep0,
                    RequestType::from((
                        RequestDirection::HostToDevice,
                        RequestKind::Class,
                        RequestRecipient::Interface,
                    )),
                    RequestCode::GetInterface,
                    WValue::from((0, 0)),
                    0,
                    none,
                )?;
                self.state = DeviceState::SetReport;
            }

            DeviceState::SetReport => {
                if let Some(ref mut ep) = self.endpoints[0] {
                    let mut r: [u8; 1] = [0];
                    let report = &mut r[..];
                    let res = host.control_transfer(
                        &mut self.ep0,
                        RequestType::from((
                            RequestDirection::HostToDevice,
                            RequestKind::Class,
                            RequestRecipient::Interface,
                        )),
                        RequestCode::SetConfiguration,
                        WValue::from((0, 2)),
                        u16::from(ep.interface_num),
                        Some(report),
                    );

                    if let Err(e) = res {
                        warn!("couldn't set report: {:?}", e)
                    }

                    // If we made it this far, thins should be ok, so
                    // throttle the logging.
                    log::set_max_level(LevelFilter::Info);
                    self.state = DeviceState::Running
                } else {
                    return Err(TransferError::Permanent("no boot keyboard"));
                }
            }

            DeviceState::Running => {
                if let Some(ref mut ep) = self.endpoints[0] {
                    let mut b: [u8; 8] = [0; 8];
                    let buf = &mut b[..];
                    match host.in_transfer(ep, buf) {
                        Err(TransferError::Permanent(msg)) => {
                            error!("reading report: {}", msg);
                            return Err(TransferError::Permanent(msg));
                        }
                        Err(TransferError::Retry(_)) => return Ok(()),
                        Ok(_) => {
                            callback(self.addr, buf);
                        }
                    }
                } else {
                    return Err(TransferError::Permanent("no boot keyboard"));
                }
            }
        }

        Ok(())
    }
}

/// If a boot protocol keyboard is found, return its interface number and endpoint.
fn ep_for_bootkbd(buf: &[u8]) -> Option<(u8, &EndpointDescriptor)> {
    let mut parser = DescriptorParser::from(buf);
    let mut interface_found = None;
    while let Some(desc) = parser.next() {
        if let Descriptor::Interface(idesc) = desc {
            if idesc.b_interface_class == 0x03
                && idesc.b_interface_sub_class == 0x01
                && idesc.b_interface_protocol == 0x01
            {
                interface_found = Some(idesc.b_interface_number);
            } else {
                interface_found = None;
            }
        } else if let Descriptor::Endpoint(edesc) = desc {
            if let Some(interface_num) = interface_found {
                return Some((interface_num, edesc));
            }
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_remove_device() {
        let mut driver = BootKeyboard::new(|_addr, _report| {});

        let count = |driver: &mut BootKeyboard<_>| {
            driver
                .devices
                .iter()
                .fold(0, |sum, dev| sum + dev.as_ref().map_or(0, |_| 1))
        };
        assert_eq!(count(&mut driver), 0);

        driver.add_device(dummy_device(), 2).unwrap();
        assert_eq!(count(&mut driver), 1);

        driver.remove_device(2);
        assert_eq!(count(&mut driver), 0);
    }

    #[test]
    fn too_many_devices() {
        let mut driver = BootKeyboard::new(|_addr, _report| {});

        for i in 0..MAX_DEVICES {
            driver.add_device(dummy_device(), (i + 1) as u8).unwrap();
        }
        assert!(driver
            .add_device(dummy_device(), (MAX_DEVICES + 1) as u8)
            .is_err());
    }

    #[test]
    fn tick_propagates_errors() {
        let mut dummyhost = DummyHost { fail: true };

        let mut calls = 0;
        let mut driver = BootKeyboard::new(|_addr, _report| calls += 1);

        driver.add_device(dummy_device(), 1).unwrap();
        driver.tick(0, &mut dummyhost).unwrap();
        assert!(driver.tick(SETTLE_DELAY + 1, &mut dummyhost).is_err());
    }

    #[test]
    fn parse_logitech_g105_config() {
        // Config, Interface (0.0), HID, Endpoint, Interface (1.0), HID, Endpoint
        let raw: &[u8] = &[
            0x09, 0x02, 0x3b, 0x00, 0x02, 0x01, 0x04, 0xa0, 0x64, 0x09, 0x04, 0x00, 0x00, 0x01,
            0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00, 0x07,
            0x05, 0x81, 0x03, 0x08, 0x00, 0x0a, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x00, 0x00,
            0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00, 0x07, 0x05, 0x82, 0x03,
            0x08, 0x00, 0x0a,
        ];
        let mut parser = DescriptorParser::from(raw);

        let config_desc = ConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Configuration,
            w_total_length: 59,
            b_num_interfaces: 2,
            b_configuration_value: 1,
            i_configuration: 4,
            bm_attributes: 0xa0,
            b_max_power: 100,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Configuration(cdesc) = desc {
            assert_eq!(*cdesc, config_desc, "Configuration descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x03,     // HID
            b_interface_sub_class: 0x01, // Boot Interface,
            b_interface_protocol: 0x01,  // Keyboard
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // Unknown descriptor just yields a byte slice.
        let hid_desc1: &[u8] = &[0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00];
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(cdesc) = desc {
            assert_eq!(cdesc, hid_desc1, "HID descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x81,
            bm_attributes: 0x03,
            w_max_packet_size: 0x08,
            b_interval: 0x0a,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let interface_desc2 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 1,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x03,     // HID
            b_interface_sub_class: 0x00, // No subclass
            b_interface_protocol: 0x00,  // No protocol
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc2, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // Unknown descriptor just yields a byte slice.
        let hid_desc2 = &[0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00];
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(cdesc) = desc {
            assert_eq!(cdesc, hid_desc2, "HID descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let endpoint_desc2 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x82,
            bm_attributes: 0x03,
            w_max_packet_size: 0x08,
            b_interval: 0x0a,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc2, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        assert!(parser.next().is_none(), "Extra descriptors.");
    }

    #[test]
    fn logitech_g105_discovers_ep0() {
        // Config, Interface (0.0), HID, Endpoint, Interface (1.0), HID, Endpoint
        let raw: &[u8] = &[
            0x09, 0x02, 0x3b, 0x00, 0x02, 0x01, 0x04, 0xa0, 0x64, 0x09, 0x04, 0x00, 0x00, 0x01,
            0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x41, 0x00, 0x07,
            0x05, 0x81, 0x03, 0x08, 0x00, 0x0a, 0x09, 0x04, 0x01, 0x00, 0x01, 0x03, 0x00, 0x00,
            0x00, 0x09, 0x21, 0x10, 0x01, 0x00, 0x01, 0x22, 0x85, 0x00, 0x07, 0x05, 0x82, 0x03,
            0x08, 0x00, 0x0a,
        ];

        let (got_inum, got) = ep_for_bootkbd(raw).expect("Looking for endpoint");
        let want = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x81,
            bm_attributes: 0x03,
            w_max_packet_size: 0x08,
            b_interval: 0x0a,
        };
        assert_eq!(got_inum, 0);
        assert_eq!(*got, want);
    }

    fn dummy_device() -> DeviceDescriptor {
        DeviceDescriptor {
            b_length: mem::size_of::<DeviceDescriptor>() as u8,
            b_descriptor_type: DescriptorType::Device,
            bcd_usb: 0x0110,
            b_device_class: 0,
            b_device_sub_class: 0,
            b_device_protocol: 0,
            b_max_packet_size: 8,
            id_vendor: 0xdead,
            id_product: 0xbeef,
            bcd_device: 0xf00d,
            i_manufacturer: 1,
            i_product: 2,
            i_serial_number: 3,
            b_num_configurations: 1,
        }
    }

    #[test]
    fn parse_keyboardio_config() {
        let raw: &[u8] = &[
            0x09, 0x02, 0x96, 0x00, 0x05, 0x01, 0x00, 0xa0, 0xfa, 0x08, 0x0b, 0x00, 0x02, 0x02,
            0x02, 0x01, 0x00, 0x09, 0x04, 0x00, 0x00, 0x01, 0x02, 0x02, 0x00, 0x00, 0x05, 0x24,
            0x00, 0x10, 0x01, 0x05, 0x24, 0x01, 0x01, 0x01, 0x04, 0x24, 0x02, 0x06, 0x05, 0x24,
            0x06, 0x00, 0x01, 0x07, 0x05, 0x81, 0x03, 0x10, 0x00, 0x40, 0x09, 0x04, 0x01, 0x00,
            0x02, 0x0a, 0x00, 0x00, 0x00, 0x07, 0x05, 0x02, 0x02, 0x40, 0x00, 0x00, 0x07, 0x05,
            0x83, 0x02, 0x40, 0x00, 0x00, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00,
            0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00, 0x07, 0x05, 0x84, 0x03, 0x40,
            0x00, 0x01, 0x09, 0x04, 0x03, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x01,
            0x01, 0x00, 0x01, 0x22, 0x72, 0x00, 0x07, 0x05, 0x85, 0x03, 0x40, 0x00, 0x01, 0x09,
            0x04, 0x04, 0x00, 0x01, 0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x01, 0x01, 0x00, 0x01,
            0x22, 0x3f, 0x00, 0x07, 0x05, 0x86, 0x03, 0x40, 0x00, 0x01,
        ];
        let mut parser = DescriptorParser::from(raw);

        let config_desc = ConfigurationDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Configuration,
            w_total_length: 150,
            b_num_interfaces: 5,
            b_configuration_value: 1,
            i_configuration: 0,
            bm_attributes: 0xa0,
            b_max_power: 250,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Configuration(cdesc) = desc {
            assert_eq!(*cdesc, config_desc, "Configuration descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // Interface Association Descriptor
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x08, 0x0b, 0x00, 0x02, 0x02, 0x02, 0x01, 0x00];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 0,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x02,     // Communications and CDC Control
            b_interface_sub_class: 0x02, // Abstract Control Model
            b_interface_protocol: 0x00,
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // Four communications descriptors.
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x05, 0x24, 0x00, 0x10, 0x01];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x05, 0x24, 0x01, 0x01, 0x01];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x04, 0x24, 0x02, 0x06];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x05, 0x24, 0x06, 0x00, 0x01];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x81,
            bm_attributes: 0x03,
            w_max_packet_size: 16,
            b_interval: 64,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // CDC-Data interface.
        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 1,
            b_alternate_setting: 0,
            b_num_endpoints: 2,
            b_interface_class: 0x0a, // CDC-Data
            b_interface_sub_class: 0x00,
            b_interface_protocol: 0x00,
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x02,
            bm_attributes: 0x02,
            w_max_packet_size: 64,
            b_interval: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x83,
            bm_attributes: 0x02,
            w_max_packet_size: 64,
            b_interval: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID interface.
        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 2,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x03, // HID
            b_interface_sub_class: 0x00,
            b_interface_protocol: 0x00,
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID Descriptor.
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x84,
            bm_attributes: 0x03,
            w_max_packet_size: 64,
            b_interval: 1,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID interface.
        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 3,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x03, // HID
            b_interface_sub_class: 0x00,
            b_interface_protocol: 0x00,
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID Descriptor.
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x72, 0x00];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x85,
            bm_attributes: 0x03,
            w_max_packet_size: 64,
            b_interval: 1,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID interface.
        let interface_desc1 = InterfaceDescriptor {
            b_length: 9,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 4,
            b_alternate_setting: 0,
            b_num_endpoints: 1,
            b_interface_class: 0x03,     // HID
            b_interface_sub_class: 0x01, // Boot Interface
            b_interface_protocol: 0x01,  // Keyboard
            i_interface: 0,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Interface(cdesc) = desc {
            assert_eq!(*cdesc, interface_desc1, "Interface descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        // HID Descriptor.
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Other(odesc) = desc {
            let odesc1: &[u8] = &[0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x3f, 0x00];
            assert_eq!(odesc, odesc1, "Interface descriptor mismatch");
        } else {
            panic!("Wrong descriptor type.")
        }

        let endpoint_desc1 = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x86,
            bm_attributes: 0x03,
            w_max_packet_size: 64,
            b_interval: 1,
        };
        let desc = parser.next().expect("Parsing configuration");
        if let Descriptor::Endpoint(cdesc) = desc {
            assert_eq!(*cdesc, endpoint_desc1, "Endpoint descriptor mismatch.");
        } else {
            panic!("Wrong descriptor type.");
        }

        assert!(parser.next().is_none(), "Extra descriptors.");
    }

    #[test]
    fn keyboardio_discovers_bootkbd() {
        let raw: &[u8] = &[
            0x09, 0x02, 0x96, 0x00, 0x05, 0x01, 0x00, 0xa0, 0xfa, 0x08, 0x0b, 0x00, 0x02, 0x02,
            0x02, 0x01, 0x00, 0x09, 0x04, 0x00, 0x00, 0x01, 0x02, 0x02, 0x00, 0x00, 0x05, 0x24,
            0x00, 0x10, 0x01, 0x05, 0x24, 0x01, 0x01, 0x01, 0x04, 0x24, 0x02, 0x06, 0x05, 0x24,
            0x06, 0x00, 0x01, 0x07, 0x05, 0x81, 0x03, 0x10, 0x00, 0x40, 0x09, 0x04, 0x01, 0x00,
            0x02, 0x0a, 0x00, 0x00, 0x00, 0x07, 0x05, 0x02, 0x02, 0x40, 0x00, 0x00, 0x07, 0x05,
            0x83, 0x02, 0x40, 0x00, 0x00, 0x09, 0x04, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00,
            0x09, 0x21, 0x01, 0x01, 0x00, 0x01, 0x22, 0x35, 0x00, 0x07, 0x05, 0x84, 0x03, 0x40,
            0x00, 0x01, 0x09, 0x04, 0x03, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x09, 0x21, 0x01,
            0x01, 0x00, 0x01, 0x22, 0x72, 0x00, 0x07, 0x05, 0x85, 0x03, 0x40, 0x00, 0x01, 0x09,
            0x04, 0x04, 0x00, 0x01, 0x03, 0x01, 0x01, 0x00, 0x09, 0x21, 0x01, 0x01, 0x00, 0x01,
            0x22, 0x3f, 0x00, 0x07, 0x05, 0x86, 0x03, 0x40, 0x00, 0x01,
        ];

        let (got_inum, got) = ep_for_bootkbd(raw).expect("Looking for endpoint");
        let want = EndpointDescriptor {
            b_length: 7,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 0x86,
            bm_attributes: 0x03,
            w_max_packet_size: 64,
            b_interval: 1,
        };
        assert_eq!(got_inum, 4);
        assert_eq!(*got, want);
    }

    struct DummyHost {
        fail: bool,
    }

    impl UsbHost for DummyHost {
        fn control_transfer(
            &mut self,
            _ep: &mut dyn Endpoint,
            _bm_request_type: RequestType,
            _b_request: RequestCode,
            _w_value: WValue,
            _w_index: u16,
            _buf: Option<&mut [u8]>,
        ) -> Result<usize, TransferError> {
            if self.fail {
                Err(TransferError::Permanent("foo"))
            } else {
                Ok(0)
            }
        }

        fn in_transfer(
            &mut self,
            _ep: &mut dyn Endpoint,
            _buf: &mut [u8],
        ) -> Result<usize, TransferError> {
            if self.fail {
                Err(TransferError::Permanent("foo"))
            } else {
                Ok(0)
            }
        }

        fn out_transfer(
            &mut self,
            _ep: &mut dyn Endpoint,
            _buf: &[u8],
        ) -> Result<usize, TransferError> {
            if self.fail {
                Err(TransferError::Permanent("foo"))
            } else {
                Ok(0)
            }
        }
    }
}
