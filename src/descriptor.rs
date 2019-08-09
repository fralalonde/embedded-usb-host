use core::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeed = 7,
    InterfacePower = 8,
}

impl TryFrom<u8> for DescriptorType {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Device),
            2 => Ok(Self::Configuration),
            3 => Ok(Self::String),
            4 => Ok(Self::Interface),
            5 => Ok(Self::Endpoint),
            6 => Ok(Self::DeviceQualifier),
            7 => Ok(Self::OtherSpeed),
            8 => Ok(Self::InterfacePower),
            _ => Err("invalid descriptor"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct DeviceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_sub_class: u8,
    pub b_device_protocol: u8,
    pub b_max_packet_size: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub bcd_device: u16,
    pub i_manufacturer: u8,
    pub i_product: u8,
    pub i_serial_number: u8,
    pub b_num_configurations: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct ConfigurationDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub w_total_length: u16,
    pub b_num_interfaces: u8,
    pub b_configuration_value: u8,
    pub i_configuration: u8,
    pub bm_attributes: u8,
    pub b_max_power: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct InterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_interface_number: u8,
    pub b_alternate_setting: u8,
    pub b_num_endpoints: u8,
    pub b_interface_class: u8,
    pub b_interface_sub_class: u8,
    pub b_interface_protocol: u8,
    pub i_interface: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct EndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_endpoint_address: u8,
    pub bm_attributes: u8,
    pub w_max_packet_size: u16,
    pub b_interval: u8,
}

#[cfg(test)]
mod test {
    use super::*;

    use core::mem;

    #[test]
    fn device_descriptor_layout() {
        assert_eq!(mem::size_of::<DeviceDescriptor>(), 18);
    }

    #[test]
    fn configuration_descriptor_layout() {
        assert_eq!(mem::size_of::<ConfigurationDescriptor>(), 9);
    }

    #[test]
    fn interface_descriptor_layout() {
        assert_eq!(mem::size_of::<InterfaceDescriptor>(), 9);
    }

    #[test]
    fn endpoint_descriptor_layout() {
        assert_eq!(mem::size_of::<EndpointDescriptor>(), 7);
    }
}
