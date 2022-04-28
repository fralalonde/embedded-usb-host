use crate::{Address, AddressPool, DescriptorType, DeviceDescriptor, Direction, Driver, Endpoint, HostEvent, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, to_slice_mut, TransferError, TransferType, UsbHost, WValue};
use crate::device::Device;

pub struct UsbStack<H> {
    host: H,
    drivers: &'static mut (dyn Driver + Sync + Send),
    addr_pool: AddressPool,
    // devices: Vec<Device, 16>,
}

impl<H: UsbHost> UsbStack<H> {
    pub fn new(host: H, drivers: &'static mut (dyn Driver + Sync + Send)) -> Self {
        Self {
            host,
            drivers,
            addr_pool: AddressPool::new(),
            // devices:
        }
    }

    pub fn handle_irq(&mut self) {
        if let Some(host_event) = self.host.update(&mut self.addr_pool) {
            match host_event {
                HostEvent::Ready(mut device, desc) => {
                    debug!("USB Host Ready {:?}", device);
                    self.register(device, desc)
                }
                HostEvent::Reset => {
                    debug!("USB Host Reset");
                    // TODO clear pool, call drivers for unregister
                    self.addr_pool.reset();
                }
                HostEvent::Tick => {
                    self.drivers.tick(&mut self.host);
                }
            }
        }

        // TODO set / unset usb midi port on attach / detach
    }

    pub fn register(&mut self, mut device: Device, desc: DeviceDescriptor) {
        debug!("USB New Device Descriptor {:?}", desc);
        // if self.drivers.want_device(&dev_desc) {
        //
        // }
        // TODO register device, call drivers for match
    }

    pub fn unregister(&mut self, mut device: Device) {
        // let dev_desc = device.get_device_descriptor(&mut self.host);
        // if self.drivers.want_device(&dev_desc) {}
        // TODO register device, call drivers for match
    }
}

// struct Addr0EP0 {
//     max_packet_size: u16,
//     in_toggle: bool,
//     out_toggle: bool,
// }

// impl Endpoint for Addr0EP0 {
//     fn device_address(&self) -> Address {
//         0
//     }(&self) -> u8 {
//         0
//     }
//
//     fn endpoint_num(&self) -> u8 {
//         0
//     }
//
//     fn transfer_type(&self) -> TransferType {
//         TransferType::Control
//     }
//
//     fn direction(&self) -> Direction {
//         Direction::In
//     }
//
//     fn max_packet_size(&self) -> u16 {
//         self.max_packet_size
//     }
//
//     fn in_toggle(&self) -> bool {
//         self.in_toggle
//     }
//
//     fn set_in_toggle(&mut self, toggle: bool) {
//         self.in_toggle = toggle;
//     }
//
//     fn out_toggle(&self) -> bool {
//         self.out_toggle
//     }
//
//     fn set_out_toggle(&mut self, toggle: bool) {
//         self.out_toggle = toggle;
//     }
// }

pub fn configure_dev(
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