use heapless::Vec;
use crate::{AddressPool, DeviceDescriptor, Driver, HostEvent, UsbError, UsbHost};
use crate::device::Device;
use crate::parser::DescriptorParser;

pub struct UsbStack<H> {
    host: H,
    drivers: &'static mut (dyn Driver + Sync + Send),
    addr_pool: AddressPool,
    devices: Vec<Device, 1>,
}

impl<H: UsbHost> UsbStack<H> {
    pub fn new(host: H, drivers: &'static mut (dyn Driver + Sync + Send)) -> Self {
        Self {
            host,
            drivers,
            addr_pool: AddressPool::new(),
            devices: Vec::new(),
        }
    }

    pub fn handle_irq(&mut self) {
        if let Some(host_event) = self.host.update(&mut self.addr_pool) {
            match host_event {
                HostEvent::Ready(device, desc) => {
                    debug!("USB Host Ready {:?}", device);
                    self.configure_dev(device, desc)
                }
                HostEvent::Reset => {
                    debug!("USB Host Reset");
                    // TODO clear pool, call drivers to unregister
                    self.addr_pool.reset();
                    self.devices.clear();
                }
                HostEvent::Tick => {
                    if let Err(err) = self.drivers.tick(&mut self.host) {
                        warn!("USB Driver error: {}", err)
                    }
                }
            }
        }

        // TODO set / unset usb midi port on attach / detach
    }

    pub fn configure_dev(&mut self, mut device: Device, desc: DeviceDescriptor) {
        debug!("USB New Device Descriptor {:?}", desc);

        let mut buf = [0u8; 256];
        match device.get_configuration_descriptors(&mut self.host, 0, &mut buf) {
            Ok(size) => {
                let mut conf = DescriptorParser::new(&buf[0..size]);
                match self.drivers.register(&mut self.host, &mut device, &desc, &mut conf) {
                    Ok(true) => info!("USB Driver registered device"),
                    Ok(false) => debug!("USB Driver rejected device"),
                    Err(e) => warn!("USB Driver Error {:?}", e)
                }
            }
            Err(e) => warn!("USB Device Config Descriptor Failed: {:?}", e)
        }

        if let Err(err) = self.devices.push(device) {
            warn!("USB Device configuration error: {}", err)
        }
    }
}

pub fn address_dev(
    host: &mut dyn UsbHost,
    addr_pool: &mut AddressPool,
) -> Result<(Device, DeviceDescriptor), UsbError> {
    let addr = addr_pool.take_next().ok_or(UsbError::Permanent("Out of USB addr"))?;
    // TODO determine correct packet size to use from descriptor
    let mut dev = Device::new(host.max_host_packet_size());
    let short_desc = dev.get_device_descriptor(host)?;

    dev.set_address(host, addr)?;
    debug!("USB Device Address Set to: {:?}", addr);

    Ok((dev, short_desc))
}