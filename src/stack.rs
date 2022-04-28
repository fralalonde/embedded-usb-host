use crate::{Address, AddressPool, DescriptorType, DeviceDescriptor, Direction, Driver, Endpoint, RequestCode, RequestDirection, RequestKind, RequestRecipient, RequestType, to_slice_mut, TransferError, TransferType, USBHost, WValue};
use crate::device::Device;


struct Addr0EP0 {
    max_packet_size: u16,
    in_toggle: bool,
    out_toggle: bool,
}

impl Endpoint for Addr0EP0 {
    fn address(&self) -> u8 {
        0
    }

    fn endpoint_num(&self) -> u8 {
        0
    }

    fn transfer_type(&self) -> TransferType {
        TransferType::Control
    }

    fn direction(&self) -> Direction {
        Direction::In
    }

    fn max_packet_size(&self) -> u16 {
        self.max_packet_size
    }

    fn in_toggle(&self) -> bool {
        self.in_toggle
    }

    fn set_in_toggle(&mut self, toggle: bool) {
        self.in_toggle = toggle;
    }

    fn out_toggle(&self) -> bool {
        self.out_toggle
    }

    fn set_out_toggle(&mut self, toggle: bool) {
        self.out_toggle = toggle;
    }
}

pub fn configure_dev(
    host: &mut dyn USBHost,
    addr_pool: &mut AddressPool,
) -> Result<Device, TransferError> {
    let none: Option<&mut [u8]> = None;
    let host_packet_size: u16 = host.max_host_packet_size();
    let mut a0ep0 = Addr0EP0 {
        max_packet_size: host_packet_size,
        in_toggle: true,
        out_toggle: true,
    };
    let mut dev_desc: DeviceDescriptor = DeviceDescriptor::default();

    host.control_transfer(
        &mut a0ep0,
        RequestType::from((
            RequestDirection::DeviceToHost,
            RequestKind::Standard,
            RequestRecipient::Device,
        )),
        RequestCode::GetDescriptor,
        WValue::from((0, DescriptorType::Device as u8)),
        0,
        Some(unsafe { to_slice_mut(&mut dev_desc) }),
    )?;

    let addr = addr_pool.take_next().ok_or(TransferError::Permanent("Out of USB addr"))?;
    // TODO determine correct packet size to use from descriptor
    let mut dev = Device::new(host_packet_size, addr);

    host.control_transfer(
        &mut a0ep0,
        RequestType::from((
            RequestDirection::HostToDevice,
            RequestKind::Standard,
            RequestRecipient::Device,
        )),
        RequestCode::SetAddress,
        WValue::from((addr.into(), 0)),
        0,
        none,
    )?;
    debug!("USB Device Address Set to: {:?}", addr);

    Ok(dev)
}