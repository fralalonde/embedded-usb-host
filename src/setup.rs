use core::convert::{TryFrom, TryInto};

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct RequestType(u8);
impl RequestType {
    pub fn recipient(&self) -> Result<RequestRecipient, &'static str> {
        const POS: u8 = 0;
        const MASK: u8 = 0x1f;
        (self.0 & (MASK << POS)).try_into()
    }

    pub fn set_recipient(&mut self, v: RequestRecipient) {
        const POS: u8 = 0;
        const MASK: u8 = 0x1f;
        self.0 &= !(MASK << POS);
        self.0 |= v as u8 & MASK;
    }

    pub fn kind(&self) -> Result<RequestKind, &'static str> {
        const POS: u8 = 5;
        const MASK: u8 = 0x3;
        (self.0 & (MASK << POS)).try_into()
    }

    pub fn set_kind(&mut self, v: RequestKind) {
        const POS: u8 = 5;
        const MASK: u8 = 0x3;
        self.0 &= !(MASK << POS);
        self.0 |= v as u8 & MASK;
    }

    pub fn direction(&self) -> Result<RequestDirection, &'static str> {
        const POS: u8 = 7;
        const MASK: u8 = 0x1;
        (self.0 & (MASK << POS)).try_into()
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestDirection {
    HostToDevice = 0x00,
    DeviceToHost = 0x80,
}
impl TryFrom<u8> for RequestDirection {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::HostToDevice),
            0x80 => Ok(Self::DeviceToHost),
            _ => Err("direction can only be 0x00 or 0x80"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestKind {
    Standard = 0x00,
    Class = 0x20,
    Vendor = 0x40,
}
impl TryFrom<u8> for RequestKind {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::Standard),
            0x20 => Ok(Self::Class),
            0x40 => Ok(Self::Vendor),
            _ => Err("type can only be 0x00, 0x20, or 0x40"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RequestRecipient {
    Device = 0x00,
    Interface = 0x01,
    Endpoint = 0x02,
    Other = 0x03,
}
impl TryFrom<u8> for RequestRecipient {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::Device),
            0x01 => Ok(Self::Interface),
            0x02 => Ok(Self::Endpoint),
            0x03 => Ok(Self::Other),
            _ => Err("recipient can only be between 0 and 3"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct WValue(u16);
impl WValue {
    pub fn w_value_lo(&self) -> u8 {
        const POS: u8 = 0;
        const MASK: u16 = 0xff;
        ((self.0 >> POS) & MASK) as u8
    }

    pub fn set_w_value_lo(&mut self, v: u8) {
        const POS: u8 = 0;
        const MASK: u8 = 0xff;
        self.0 &= !((MASK as u16) << POS);
        self.0 |= ((v & MASK) as u16) << POS;
    }

    pub fn w_value_hi(&self) -> u8 {
        const POS: u8 = 8;
        const MASK: u16 = 0xff;
        ((self.0 >> POS) & MASK) as u8
    }

    pub fn set_w_value_hi(&mut self, v: u8) {
        const POS: u8 = 8;
        const MASK: u8 = 0xff;
        self.0 &= !((MASK as u16) << POS);
        self.0 |= ((v & MASK) as u16) << POS;
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

#[derive(Clone, Copy, Debug, PartialEq)]
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
impl TryFrom<u8> for RequestCode {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::GetStatus),
            1 => Ok(Self::ClearFeature),
            3 => Ok(Self::SetFeature),
            5 => Ok(Self::SetAddress),
            6 => Ok(Self::GetDescriptor),
            7 => Ok(Self::SetDescriptor),
            8 => Ok(Self::GetConfiguration),
            9 => Ok(Self::SetConfiguration),
            10 => Ok(Self::GetInterface),
            11 => Ok(Self::SetInterface),
            12 => Ok(Self::SynchFrame),
            _ => Err("invalid request value"),
        }
    }
}
impl Default for RequestCode {
    fn default() -> Self {
        Self::GetStatus
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct SetupPacket {
    pub bm_request_type: RequestType,
    pub b_request: RequestCode,
    pub w_value: WValue,
    pub w_index: u16,
    pub w_length: u16,
}
