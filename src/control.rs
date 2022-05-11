//! A collection of structures for use in setting up devices during enumeration.
//!
//! These types are all defined in §9.3 of the USB 2.0 specification.
//!
//! The structures defined herein are `repr(C)` and `repr(packed)`
//! when necessary to ensure that they are able to be directly
//! marshalled to the bus.

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RequestType(u8);

impl RequestType {
    pub fn recipient(self) -> Option<RequestRecipient> {
        const POS: u8 = 0;
        const MASK: u8 = 0x1f;
        RequestRecipient::from_repr(self.0 & (MASK << POS))
    }

    pub fn set_recipient(&mut self, v: RequestRecipient) {
        const POS: u8 = 0;
        const MASK: u8 = 0x1f;
        self.0 &= !(MASK << POS);
        self.0 |= v as u8 & MASK;
    }

    pub fn kind(self) -> Option<RequestKind> {
        const POS: u8 = 5;
        const MASK: u8 = 0x3;
        RequestKind::from_repr(self.0 & (MASK << POS))
    }

    pub fn set_kind(&mut self, v: RequestKind) {
        const POS: u8 = 5;
        const MASK: u8 = 0x3;
        self.0 &= !(MASK << POS);
        self.0 |= v as u8 & MASK;
    }

    pub fn direction(self) -> Option<RequestDirection> {
        const POS: u8 = 7;
        const MASK: u8 = 0x1;
        RequestDirection::from_repr(self.0 & (MASK << POS))
    }

    pub fn set_direction(&mut self, v: RequestDirection) {
        const POS: u8 = 7;
        const MASK: u8 = 0x1;
        self.0 &= !(MASK << POS);
        self.0 |= v as u8 & MASK;
    }
}

impl From<(RequestDirection, RequestKind, RequestRecipient)> for RequestType {
    fn from(v: (RequestDirection, RequestKind, RequestRecipient)) -> Self {
        Self(v.0 as u8 | v.1 as u8 | v.2 as u8)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, strum_macros::FromRepr)]
#[repr(u8)]
pub enum RequestDirection {
    HostToDevice = 0x00,
    DeviceToHost = 0x80,
}

#[derive(Copy, Clone, Debug, PartialEq, strum_macros::FromRepr)]
#[repr(u8)]
pub enum RequestKind {
    Standard = 0x00,
    Class = 0x20,
    Vendor = 0x40,
}

#[derive(Copy, Clone, Debug, PartialEq, strum_macros::FromRepr)]
#[repr(u8)]
pub enum RequestRecipient {
    Device = 0x00,
    Interface = 0x01,
    Endpoint = 0x02,
    Other = 0x03,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct WValue(u16);
impl WValue {
    pub fn w_value_lo(self) -> u8 {
        const POS: u8 = 0;
        const MASK: u16 = 0xff;
        ((self.0 >> POS) & MASK) as u8
    }

    pub fn set_w_value_lo(&mut self, v: u8) {
        const POS: u8 = 0;
        const MASK: u8 = 0xff;
        self.0 &= !(u16::from(MASK) << POS);
        self.0 |= u16::from(v & MASK) << POS;
    }

    pub fn w_value_hi(self) -> u8 {
        const POS: u8 = 8;
        const MASK: u16 = 0xff;
        ((self.0 >> POS) & MASK) as u8
    }

    pub fn set_w_value_hi(&mut self, v: u8) {
        const POS: u8 = 8;
        const MASK: u8 = 0xff;
        self.0 &= !(u16::from(MASK) << POS);
        self.0 |= u16::from(v & MASK) << POS;
    }
}
impl From<(u8, u8)> for WValue {
    fn from(v: (u8, u8)) -> Self {
        let mut rc = Self(0);
        rc.set_w_value_lo(v.0);
        rc.set_w_value_hi(v.1);
        rc
    }
}

#[derive(Clone, Copy, Debug, PartialEq, strum_macros::FromRepr)]
pub enum RequestCode {
    GetStatus = 0,
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,

}

impl Default for RequestCode {
    fn default() -> Self {
        Self::GetStatus
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct SetupPacket {
    pub bm_request_type: RequestType,
    pub b_request: RequestCode,
    pub w_value: WValue,
    pub w_index: u16,
    pub w_length: u16,
}

#[cfg(test)]
mod test {
    use super::*;

    use core::mem;
    use core::slice;

    #[test]
    fn setup_packet_layout() {
        let len = mem::size_of::<SetupPacket>();
        assert_eq!(len, 8);
        let sp = SetupPacket {
            bm_request_type: RequestType::from((
                RequestDirection::HostToDevice,
                RequestKind::Class,
                RequestRecipient::Endpoint,
            )),
            b_request: RequestCode::GetInterface,
            w_value: WValue::from((0xf0, 0x0d)),
            w_index: 0xadde,
            w_length: 0xefbe,
        };
        let base = &sp as *const _ as usize;
        assert_offset("bm_request_type", &sp.bm_request_type, base, 0x00);
        assert_offset("b_request", &sp.b_request, base, 0x01);
        assert_offset("w_value", &sp.w_value, base, 0x02);
        assert_offset("w_index", &sp.w_index, base, 0x04);
        assert_offset("w_length", &sp.w_length, base, 0x06);

        let got = unsafe { slice::from_raw_parts(&sp as *const _ as *const u8, len) };
        let want = &[0x22, 0x0a, 0xf0, 0x0d, 0xde, 0xad, 0xbe, 0xef];
        assert_eq!(got, want);
    }

    fn assert_offset<T>(name: &str, field: &T, base: usize, offset: usize) {
        let ptr = field as *const _ as usize;
        assert_eq!(ptr - base, offset, "{} register offset.", name);
    }
}
