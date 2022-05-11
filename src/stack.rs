use core::cell::{RefCell};
use heapless::Vec;
use crate::{AddressPool, DeviceDescriptor, DeviceState, Driver, EndpointProperties, HostEvent, InterfaceNum, UsbError, UsbHost, DescriptorParser, Device};

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
                    for dev_drv in self.devices.iter().map(|d| d.borrow_mut()) {
                        // let mut bor = dev_drv.borrow_mut();
                        // let (dev, driver_idx) = bor.0;
                        if let Some(driver_idx) = dev_drv.1 {
                            let driver = &self.drivers[driver_idx as usize];
                            driver.borrow_mut().unregister(dev_drv.0.device_address());
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
                let mut dev = &mut cell.borrow_mut().0;
                warn!("USB Device Failed: {}, Error: {}", dev.state(), err);
                dev.set_error(err);
            }
        }
    }

    pub fn update_dev(&self, host: &mut dyn UsbHost, cell: &RefCell<(Device, Option<DriverIdx>)>) -> Result<(), UsbError> {
        let mut dev_drv = cell.borrow_mut();

        if dev_drv.0.error().is_some() {
            return Ok(());
        }

        let driver = dev_drv.1.map(|idx| self.drivers[idx as usize].borrow_mut());

        let state = dev_drv.0.state();
        match state {
            DeviceState::SetAddress => {
                info!("Device init");
                let _dev_desc = self.address_dev(host, &mut dev_drv.0)?;
                // TODO determine what happens if address set fails
                dev_drv.0.set_state(DeviceState::SetConfig(host.after_millis(10)))
            }

            DeviceState::SetConfig(until) => if host.delay_done(until) {
                if let Some((match_idx, iface)) = self.configure_dev(host, &mut dev_drv.0)? {
                    dev_drv.1 = Some(match_idx);
                    dev_drv.0.set_state(DeviceState::SetInterface(iface, host.after_millis(10)));
                } else {
                    dev_drv.0.set_state(DeviceState::Orphan);
                }
            }

            DeviceState::SetInterface(iface, until) => if host.delay_done(until) {
                // TODO let driver handle set interface?
                if let Err(err) = dev_drv.0.set_interface(host, iface, iface) {
                    info!("USB Set Interface failed")
                } else {
                    info!("USB Set Interface WORKED")
                }
                // TODO handle class states e.g. HID set_report, etc.
                if let Some(mut driver) = driver {
                    dev_drv.0.set_state(driver.next_state_after_interface_set());
                } else {
                    return Err(UsbError::NoDriver);
                }
            }

            DeviceState::Orphan => {}

            // Other state is handled by driver
            _ => {
                if let Some(mut driver) = driver {
                    driver.run(host, &mut dev_drv.0)?;
                } else {
                    return Err(UsbError::NoDriver);
                }
            }
        }
        Ok(())
    }

    pub fn configure_dev(&self, host: &mut dyn UsbHost, device: &mut Device) -> Result<Option<(DriverIdx, InterfaceNum)>, UsbError> {
        let mut buf = [0u8; 256];
        let size = device.get_configuration_descriptors(host, 0, &mut buf)?;

        let mut desc_parser = DescriptorParser::new(&buf[0..size]);
        for (idx, driver) in self.drivers.iter().enumerate() {
            let mut driver = driver.borrow_mut();
            if let Some((conf_num, iface_num)) = driver.accept(device, &mut desc_parser) {
                device.set_configuration(host, conf_num)?;
                desc_parser.rewind();
                driver.register(device, &mut desc_parser);
                info!("USB Driver registered device");
                return Ok(Some((idx as DriverIdx, iface_num)));
            }
            desc_parser.rewind();
        }
        Ok(None)
    }

    fn address_dev(&self, host: &mut dyn UsbHost, dev: &mut Device) -> Result<DeviceDescriptor, UsbError> {
        let mut addr_pool = self.addr_pool.borrow_mut();
        let addr = addr_pool.take_next().ok_or(UsbError::Permanent("Out of USB addr"))?;
        // TODO determine correct packet size to use from descriptor
        let short_desc = dev.get_device_descriptor(host)?;
        debug!("USB New Device Descriptor {:?}", short_desc);

        if let Err(err) = dev.set_address(host, addr) {
            addr_pool.put_back(addr);
            return Err(err);
        }
        debug!("USB Device Address Set: {:?}", addr);
        Ok(short_desc)
    }
}

