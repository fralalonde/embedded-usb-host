//! Structures and constants for control transfers

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
pub struct RequestType(u8);

const DIRECTION_MASK: u8 = 0b10000000;
const KIND_MASK: u8 = 0b1100000;
const RECIPIENT_MASK: u8 = 0b0000011;

impl RequestType {
    pub fn recipient(self) -> Option<RequestRecipient> {
        RequestRecipient::from_repr(self.0 & RECIPIENT_MASK)
    }

    pub fn set_recipient(&mut self, v: RequestRecipient) {
        self.0 |= v as u8 & RECIPIENT_MASK;
    }

    pub fn kind(self) -> Option<RequestKind> {
        RequestKind::from_repr(self.0 & KIND_MASK)
    }

    pub fn set_kind(&mut self, v: RequestKind) {
        self.0 |= v as u8 & KIND_MASK;
    }

    pub fn direction(self) -> Option<RequestDirection> {
        RequestDirection::from_repr(self.0 & DIRECTION_MASK)
    }

    pub fn set_direction(&mut self, v: RequestDirection) {
        self.0 |= v as u8 & DIRECTION_MASK;
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
    pub fn lo_hi(lo: u8, hi: u8) -> Self {
        Self(((hi as u16) << 8) + lo as u16)
    }

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

#[derive(Clone, Copy, Debug, PartialEq, strum_macros::FromRepr)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
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

use core::mem;
const_assert!(mem::size_of::<SetupPacket>() == 8);

#[cfg(test)]
mod test {
    use super::*;

    use crate::assert_offset;
    use core::mem;
    use core::slice;

    #[test]
    fn setup_packet_layout() {
        let sp = SetupPacket {
            bm_request_type: RequestType::from((
                RequestDirection::HostToDevice,
                RequestKind::Class,
                RequestRecipient::Endpoint,
            )),
            b_request: RequestCode::GetInterface,
            w_value: WValue::lo_hi(0xf0, 0x0d),
            w_index: 0xadde,
            w_length: 0xefbe,
        };
        let base = &sp as *const _ as usize;
        assert_offset("bm_request_type", &sp.bm_request_type, base, 0x00);
        assert_offset("b_request", &sp.b_request, base, 0x01);
        assert_offset("w_value", &sp.w_value, base, 0x02);
        assert_offset("w_index", &sp.w_index, base, 0x04);
        assert_offset("w_length", &sp.w_length, base, 0x06);

        let result = unsafe { slice::from_raw_parts(&sp as *const _ as *const u8, len) };
        let expected = &[0x22, 0x0a, 0xf0, 0x0d, 0xde, 0xad, 0xbe, 0xef];
        assert_eq!(result, expected);
    }
}
