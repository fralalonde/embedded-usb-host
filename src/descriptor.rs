//! A collection of structures defining descriptors in the USB.
//!
//! These types are defined in §9.5 and 9.6 of the USB 2.0
//! specification.
//!
//! The structures defined herein are `repr(C)` and `repr(packed)`
//! when necessary to ensure that they are able to be directly
//! marshalled to the bus.

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
    use core::slice;

    #[test]
    fn device_descriptor_layout() {
        let len = mem::size_of::<DeviceDescriptor>();
        assert_eq!(len, 18);
        let desc = DeviceDescriptor {
            b_length: len as u8,
            b_descriptor_type: DescriptorType::Device,
            bcd_usb: 0x1001,
            b_device_class: 0xaa,
            b_device_sub_class: 0xbb,
            b_device_protocol: 0xcc,
            b_max_packet_size: 0xdd,
            id_vendor: 0xdead,
            id_product: 0xbeef,
            bcd_device: 0xf00d,
            i_manufacturer: 0x11,
            i_product: 0x22,
            i_serial_number: 0x33,
            b_num_configurations: 0x44,
        };
        let base = &desc as *const _ as usize;
        assert_offset("b_length", &desc.b_length, base, 0x00);
        assert_offset("b_descriptor_type", &desc.b_descriptor_type, base, 0x01);
        assert_offset("bcd_usb", &desc.bcd_usb, base, 0x02);
        assert_offset("b_device_class", &desc.b_device_class, base, 0x04);
        assert_offset("b_device_sub_class", &desc.b_device_sub_class, base, 0x05);
        assert_offset("b_device_protocol", &desc.b_device_protocol, base, 0x06);
        assert_offset("b_max_packet_size", &desc.b_max_packet_size, base, 0x07);
        assert_offset("id_vendor", &desc.id_vendor, base, 0x08);
        assert_offset("id_product", &desc.id_product, base, 0x0a);
        assert_offset("bcd_device", &desc.bcd_device, base, 0x0c);
        assert_offset("i_manufacturer", &desc.i_manufacturer, base, 0x0e);
        assert_offset("i_product", &desc.i_product, base, 0x0f);
        assert_offset("i_serial_number", &desc.i_serial_number, base, 0x10);
        assert_offset(
            "b_num_configurations",
            &desc.b_num_configurations,
            base,
            0x011,
        );

        let got = unsafe { slice::from_raw_parts(&desc as *const _ as *const u8, len) };
        let want = &[
            0x12, 0x01, 0x01, 0x10, 0xaa, 0xbb, 0xcc, 0xdd, 0xad, 0xde, 0xef, 0xbe, 0x0d, 0xf0,
            0x11, 0x22, 0x33, 0x44,
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn configuration_descriptor_layout() {
        let len = mem::size_of::<ConfigurationDescriptor>();
        assert_eq!(len, 9);
        let desc = ConfigurationDescriptor {
            b_length: len as u8,
            b_descriptor_type: DescriptorType::Configuration,
            w_total_length: 0xdead,
            b_num_interfaces: 0x22,
            b_configuration_value: 0x33,
            i_configuration: 0x44,
            bm_attributes: 0x55,
            b_max_power: 0x66,
        };
        let base = &desc as *const _ as usize;
        assert_offset("b_length", &desc.b_length, base, 0x00);
        assert_offset("b_descriptor_type", &desc.b_descriptor_type, base, 0x01);
        assert_offset("w_total_length", &desc.w_total_length, base, 0x02);
        assert_offset("b_num_interfaces", &desc.b_num_interfaces, base, 0x04);
        assert_offset(
            "b_configuration_value",
            &desc.b_configuration_value,
            base,
            0x05,
        );
        assert_offset("i_configuration", &desc.i_configuration, base, 0x06);
        assert_offset("bm_attributes", &desc.bm_attributes, base, 0x07);
        assert_offset("b_max_power", &desc.b_max_power, base, 0x08);

        let got = unsafe { slice::from_raw_parts(&desc as *const _ as *const u8, len) };
        let want = &[0x09, 0x02, 0xad, 0xde, 0x22, 0x33, 0x44, 0x55, 0x66];
        assert_eq!(got, want);
    }

    #[test]
    fn interface_descriptor_layout() {
        let len = mem::size_of::<InterfaceDescriptor>();
        assert_eq!(len, 9);
        let desc = InterfaceDescriptor {
            b_length: len as u8,
            b_descriptor_type: DescriptorType::Interface,
            b_interface_number: 0xee,
            b_alternate_setting: 0xaa,
            b_num_endpoints: 0xf7,
            b_interface_class: 0x11,
            b_interface_sub_class: 0x22,
            b_interface_protocol: 0x33,
            i_interface: 0x44,
        };
        let base = &desc as *const _ as usize;
        assert_offset("b_length", &desc.b_length, base, 0x00);
        assert_offset("b_descriptor_type", &desc.b_descriptor_type, base, 0x01);
        assert_offset("b_interface_number", &desc.b_interface_number, base, 0x02);
        assert_offset("b_alternate_setting", &desc.b_alternate_setting, base, 0x03);
        assert_offset("b_num_endpoints", &desc.b_num_endpoints, base, 0x04);
        assert_offset("b_interface_class", &desc.b_interface_class, base, 0x05);
        assert_offset(
            "b_interface_sub_class",
            &desc.b_interface_sub_class,
            base,
            0x06,
        );
        assert_offset(
            "b_interface_protocol",
            &desc.b_interface_protocol,
            base,
            0x07,
        );
        assert_offset("i_interface", &desc.i_interface, base, 0x08);

        let got = unsafe { slice::from_raw_parts(&desc as *const _ as *const u8, len) };
        let want = &[0x09, 0x04, 0xee, 0xaa, 0xf7, 0x11, 0x22, 0x33, 0x44];
        assert_eq!(got, want);
    }

    #[test]
    fn endpoint_descriptor_layout() {
        let len = mem::size_of::<EndpointDescriptor>();
        assert_eq!(len, 7);
        let desc = EndpointDescriptor {
            b_length: len as u8,
            b_descriptor_type: DescriptorType::Endpoint,
            b_endpoint_address: 2,
            bm_attributes: 0xae,
            w_max_packet_size: 0xdead,
            b_interval: 0x7a,
        };
        let base = &desc as *const _ as usize;
        assert_offset("b_length", &desc.b_length, base, 0x00);
        assert_offset("b_descriptor_type", &desc.b_descriptor_type, base, 0x01);
        assert_offset("b_endpoint_address", &desc.b_endpoint_address, base, 0x02);
        assert_offset("bm_attributes", &desc.bm_attributes, base, 0x03);
        assert_offset("w_max_packet_size", &desc.w_max_packet_size, base, 0x04);
        assert_offset("b_interval", &desc.b_interval, base, 0x06);

        let got = unsafe { slice::from_raw_parts(&desc as *const _ as *const u8, len) };
        let want = &[0x07, 0x05, 0x02, 0xae, 0xad, 0xde, 0x7a];
        assert_eq!(got, want);
    }

    fn assert_offset<T>(name: &str, field: &T, base: usize, offset: usize) {
        let ptr = field as *const _ as usize;
        assert_eq!(ptr - base, offset, "{} register offset.", name);
    }
}
