use heapless::Vec;
use crate::{Address, AddressPool, DescriptorType, DeviceDescriptor, Direction, Driver, Endpoint, HostEvent, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, TransferError, TransferType, UsbHost, WValue};
use crate::device::Device;

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
                HostEvent::Ready(mut device, desc) => {
                    debug!("USB Host Ready {:?}", device);
                    self.configure_dev(device, desc)
                }
                HostEvent::Reset => {
                    debug!("USB Host Reset");
                    // TODO clear pool, call drivers for unregister
                    self.addr_pool.reset();
                    self.devices.clear();
                }
                HostEvent::Tick => {
                    self.drivers.tick(&mut self.host);
                }
            }
        }

        // TODO set / unset usb midi port on attach / detach
    }

    pub fn configure_dev(&mut self, mut device: Device, desc: DeviceDescriptor) {
        debug!("USB New Device Descriptor {:?}", desc);
        self.devices.push(device.clone());

        // if self.drivers.want_device(&dev_desc) {
        //
        // }
        // TODO register device, call drivers for match
    }
    
}

pub fn address_dev(
    host: &mut dyn UsbHost,
    addr_pool: &mut AddressPool,
) -> Result<(Device, DeviceDescriptor), TransferError> {


    let addr = addr_pool.take_next().ok_or(TransferError::Permanent("Out of USB addr"))?;
    // TODO determine correct packet size to use from descriptor
    let mut dev = Device::new(host.max_host_packet_size(), Address::from(0));
    let short_desc = dev.get_device_descriptor(host)?;

    dev.set_address(host, addr)?;
    debug!("USB Device Address Set to: {:?}", addr);
    let full_desc = dev.get_device_descriptor(host)?;

    Ok((dev, full_desc))
}