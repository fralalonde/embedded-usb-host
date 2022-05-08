use core::borrow::BorrowMut;
use core::cell::{RefCell, RefMut};
use core::ops::DerefMut;
use heapless::Vec;
use crate::{AddressPool, DevAddress, DeviceDescriptor, DeviceState, Driver, Endpoint, HostEvent, UsbError, UsbHost};

use crate::device::Device;
use crate::parser::DescriptorParser;

pub struct UsbStack<H> {
    host: RefCell<H>,
    drivers: Vec<RefCell<&'static mut (dyn Driver + Sync + Send)>, 4>,
    addr_pool: RefCell<AddressPool>,
    devices: Vec<RefCell<(Device, Option<DriverIdx>)>, 16>,
}

pub type DriverIdx = u8;

impl<H: UsbHost> UsbStack<H> {
    pub fn new(host: H) -> Self {
        Self {
            host: RefCell::new(host),
            drivers: Vec::new(),
            addr_pool: RefCell::new(AddressPool::new()),
            devices: Vec::new(),
        }
    }

    /// Drivers are added on startup, never removed
    pub fn add_driver(&mut self, driver: &'static mut (dyn Driver + Sync + Send)) {
        self.drivers.push(RefCell::new(driver)).or(Err(UsbError::TooManyDrivers)).unwrap()
    }

    pub fn update(&mut self) {
        let mut host = self.host.borrow_mut();
        if let Some(host_event) = host.update() {
            match host_event {
                HostEvent::Ready => {
                    let root_dev = Device::new(host.max_host_packet_size());
                    self.devices.push(RefCell::new((root_dev, None)));
                    info!("Added root device");
                }
                HostEvent::Reset => {
                    // Note: root dev address will always be 1
                    for cell in &self.devices {
                        let mut bor = cell.borrow_mut();
                        // let (dev, driver_idx) = bor.0;
                        if let Some(driver_idx) = bor.1 {
                            let driver = &self.drivers[driver_idx as usize];
                            driver.borrow_mut().unregister(bor.0.device_address());
                        }
                    }
                    self.devices.clear();
                    debug!("USB Host Reset");
                    self.addr_pool.borrow_mut().reset();
                }
            }
        }

        // Perform device upkeep
        // for cell in &self.devices {
        for cell in &self.devices {
            if let Err(err) = self.update_dev(&mut *host, cell) {
                warn!("USB Device Error: {}", err);
                cell.borrow_mut().0.set_state(DeviceState::Error(err));
            }
        }
    }

    pub fn update_dev(&self, host: &mut dyn UsbHost, cell: &RefCell<(Device, Option<DriverIdx>)>) -> Result<(), UsbError> {
        let mut cell = cell.borrow_mut();
        let state = cell.0.state();

        match state {
            DeviceState::Init => {
                info!("Device init");
                let _dev_desc = self.address_dev(host, &mut cell.0)?;
                // TODO determine what happens if address set fails
                cell.0.set_state(DeviceState::AddressSet(host.after_millis(10)))
            }

            DeviceState::AddressSet(until) if host.delay_done(until) => {
                if let Some(match_idx) = self.configure_dev(host, &mut cell.0)? {
                    cell.1 = Some(match_idx);
                    cell.0.set_state(DeviceState::ConfigSet(host.after_millis(10)));
                } else {
                    cell.0.set_state(DeviceState::Orphan);
                }
            }

            DeviceState::ConfigSet(until) if host.delay_done(until) => {
                // TODO let driver handle set interface?
                cell.0.set_interface(host, 1, 1)?;
                // TODO handle class states e.g. HID set_report, etc.
                cell.0.set_state(DeviceState::Running);
            }

            DeviceState::Running => {
                let idx = cell.1.expect("No driver for running device?");
                self.drivers[idx as usize].borrow_mut().run(host, &mut cell.0)?;
            }

            _ => {}
        }
        Ok(())
    }

    pub fn configure_dev(&self, host: &mut dyn UsbHost, device: &mut Device) -> Result<Option<DriverIdx>, UsbError> {
        let mut buf = [0u8; 256];
        let size = device.get_configuration_descriptors(host, 0, &mut buf)?;

        let mut conf = DescriptorParser::new(&buf[0..size]);
        for (idx, driver) in self.drivers.iter().enumerate() {
            let mut driver = driver.borrow_mut();
            if let Some(conf_num) = driver.accept(device, &mut conf) {
                device.set_configuration(host, conf_num)?;
                driver.register(device, &mut conf);
                info!("USB Driver registered device");
                return Ok(Some(idx as DriverIdx));
            }
        }
        Ok(None)
    }

    fn address_dev(&self, host: &mut dyn UsbHost, dev: &mut Device) -> Result<DeviceDescriptor, UsbError> {
        let addr = self.addr_pool.borrow_mut().take_next().ok_or(UsbError::Permanent("Out of USB addr"))?;
        // TODO determine correct packet size to use from descriptor
        let short_desc = dev.get_device_descriptor(host)?;
        debug!("USB New Device Descriptor {:?}", short_desc);

        dev.set_address(host, addr)?;
        debug!("USB Device Address Set: {:?}", addr);
        Ok(short_desc)
    }
}

