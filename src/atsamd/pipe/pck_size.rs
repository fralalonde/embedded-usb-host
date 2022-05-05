/// § 32.8.7.3
/// Packet Size.
///
/// Offset: 0x04 & 0x14
/// Reset: 0xxxxxxxxx
/// Property: NA
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub(crate) struct PckSize(u32);

pub(crate) struct R {
    bits: u32,
}

pub(crate) struct W {
    bits: u32,
}

impl PckSize {
    pub fn read(self) -> R {
        R { bits: self.0 }
    }

    pub fn write<F>(&mut self, f: F)
    where
        F: FnOnce(&mut W) -> &mut W,
    {
        let mut w = W { bits: self.0 };
        f(&mut w);
        self.0 = w.bits;
    }

    pub fn modify<F>(&mut self, f: F)
    where
        for<'w> F: FnOnce(&R, &'w mut W) -> &'w mut W,
    {
        let r = R { bits: self.0 };
        let mut w = W { bits: self.0 };
        f(&r, &mut w);
        self.0 = w.bits;
    }
}

impl From<u32> for PckSize {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl R {
    /// Value in raw bits.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn auto_zlp(&self) -> AutoZLPR {
        let bits = {
            const POS: u8 = 31;
            const MASK: u32 = 1;
            ((self.bits >> POS) & MASK) == 1
        };

        AutoZLPR(bits)
    }

    pub fn size(&self) -> SizeR {
        let bits = {
            const POS: u8 = 28;
            const MASK: u32 = 0x7;
            ((self.bits >> POS) & MASK) as u8
        };

        SizeR::from(bits)
    }

    pub fn multi_packet_size(&self) -> MultiPacketSizeR {
        let bits = {
            const POS: u8 = 14;
            const MASK: u32 = 0x3fff;
            ((self.bits >> POS) & MASK) as u16
        };

        MultiPacketSizeR(bits)
    }

    // Documentation is wrong on this field. Actually 14 bits from
    // offset 0.
    pub fn byte_count(&self) -> ByteCountR {
        let bits = {
            const POS: u8 = 0;
            const MASK: u32 = 0x3fff;
            ((self.bits >> POS) & MASK) as u16
        };

        ByteCountR(bits)
    }
}

/// Automatic Zero Length Packet
///
/// This bit defines the automatic Zero Length Packet mode of the
/// pipe.
///
/// When enabled, the USB module will manage the ZLP handshake by
/// hardware. This bit is for OUT pipes only. When disabled the
/// handshake should be managed by firmware.
pub(crate) struct AutoZLPR(bool);
impl AutoZLPR {
    pub fn bit(&self) -> bool {
        self.0
    }

    pub fn bit_is_set(&self) -> bool {
        self.bit()
    }

    pub fn bit_is_clear(&self) -> bool {
        !self.bit()
    }
}

/// Pipe size
///
/// These bits contains the size of the pipe.
///
/// These bits are cleared upon sending a USB reset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SizeR {
    Bytes8,
    Bytes16,
    Bytes32,
    Bytes64,
    Bytes128,
    Bytes256,
    Bytes512,
    Bytes1024,
}

impl SizeR {
    pub fn bits(self) -> u8 {
        match self {
            Self::Bytes8 => 0x0,
            Self::Bytes16 => 0x1,
            Self::Bytes32 => 0x2,
            Self::Bytes64 => 0x3,
            Self::Bytes128 => 0x4,
            Self::Bytes256 => 0x5,
            Self::Bytes512 => 0x6,
            Self::Bytes1024 => 0x7,
        }
    }

    fn is_bytes8(self) -> bool {
        self == Self::Bytes8
    }
    fn is_bytes16(self) -> bool {
        self == Self::Bytes16
    }
    fn is_bytes32(self) -> bool {
        self == Self::Bytes32
    }
    fn is_bytes64(self) -> bool {
        self == Self::Bytes64
    }
    fn is_bytes128(self) -> bool {
        self == Self::Bytes128
    }
    fn is_bytes256(self) -> bool {
        self == Self::Bytes256
    }
    fn is_bytes512(self) -> bool {
        self == Self::Bytes512
    }
    fn is_bytes1024(self) -> bool {
        self == Self::Bytes1024
    }
}

impl From<u8> for SizeR {
    fn from(v: u8) -> Self {
        match v {
            0x0 => Self::Bytes8,
            0x1 => Self::Bytes16,
            0x2 => Self::Bytes32,
            0x3 => Self::Bytes64,
            0x4 => Self::Bytes128,
            0x5 => Self::Bytes256,
            0x6 => Self::Bytes512,
            0x7 => Self::Bytes1024,
            _ => panic!("pcksize between 0 and 7 only"),
        }
    }
}

/// Multi Packet IN or OUT size
///
/// These bits define the 14-bit value that is used for multi-packet
/// transfers.
///
/// For IN pipes, MULTI_PACKET_SIZE holds the total number of bytes
/// sent. MULTI_PACKET_SIZE should be written to zero when setting up
/// a new transfer.
///
/// For OUT pipes, MULTI_PACKET_SIZE holds the total data size for the
/// complete transfer. This value must be a multiple of the maximum
/// packet size.
pub(crate) struct MultiPacketSizeR(u16);
impl MultiPacketSizeR {
    pub fn bits(&self) -> u16 {
        self.0
    }
}

/// Byte Count
///
/// These bits define the 14-bit value that contains number of bytes
/// sent in the last OUT or SETUP transaction for an OUT pipe, or of
/// the number of bytes to be received in the next IN transaction for
/// an input pipe.
pub(crate) struct ByteCountR(u16);
impl ByteCountR {
    pub fn bits(&self) -> u16 {
        self.0
    }
}

impl W {
    /// Write raw bits.
    pub unsafe fn bits(&mut self, v: u32) -> &mut Self {
        self.bits = v;
        self
    }

    pub fn auto_zlp(&mut self) -> AutoZLPW {
        AutoZLPW { w: self }
    }

    pub fn size(&mut self) -> _SizeW {
        _SizeW { w: self }
    }

    pub fn multi_packet_size(&mut self) -> MultiPacketSizeW {
        MultiPacketSizeW { w: self }
    }

    pub fn byte_count(&mut self) -> ByteCountW {
        ByteCountW { w: self }
    }
}

pub(crate) struct AutoZLPW<'a> {
    w: &'a mut W,
}
impl<'a> AutoZLPW<'a> {
    pub fn bit(self, v: bool) -> &'a mut W {
        const POS: u8 = 31;
        const MASK: bool = true;
        self.w.bits &= !((MASK as u32) << POS);
        self.w.bits |= ((v & MASK) as u32) << POS;
        self.w
    }

    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }

    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum SizeW {
    Bytes8,
    Bytes16,
    Bytes32,
    Bytes64,
    Bytes128,
    Bytes256,
    Bytes512,
    Bytes1024,
}
impl SizeW {
    pub fn bits(self) -> u8 {
        match self {
            Self::Bytes8 => 0,
            Self::Bytes16 => 1,
            Self::Bytes32 => 2,
            Self::Bytes64 => 3,
            Self::Bytes128 => 4,
            Self::Bytes256 => 5,
            Self::Bytes512 => 6,
            Self::Bytes1024 => 7,
        }
    }
}

/// Proxy for `SizeW`
pub(crate) struct _SizeW<'a> {
    w: &'a mut W,
}
impl<'a> _SizeW<'a> {
    pub unsafe fn bits(self, v: u8) -> &'a mut W {
        const POS: u8 = 28;
        const MASK: u8 = 0x7;
        self.w.bits &= !(u32::from(MASK) << POS);
        self.w.bits |= u32::from(v & MASK) << POS;
        self.w
    }

    pub fn variant(self, v: SizeW) -> &'a mut W {
        unsafe { self.bits(v.bits()) }
    }

    pub fn bytes8(self) -> &'a mut W {
        self.variant(SizeW::Bytes8)
    }

    pub fn bytes16(self) -> &'a mut W {
        self.variant(SizeW::Bytes16)
    }

    pub fn bytes32(self) -> &'a mut W {
        self.variant(SizeW::Bytes32)
    }

    pub fn bytes64(self) -> &'a mut W {
        self.variant(SizeW::Bytes64)
    }

    pub fn bytes128(self) -> &'a mut W {
        self.variant(SizeW::Bytes128)
    }

    pub fn bytes256(self) -> &'a mut W {
        self.variant(SizeW::Bytes256)
    }

    pub fn bytes512(self) -> &'a mut W {
        self.variant(SizeW::Bytes512)
    }

    pub fn bytes1024(self) -> &'a mut W {
        self.variant(SizeW::Bytes1024)
    }
}

pub(crate) struct MultiPacketSizeW<'a> {
    w: &'a mut W,
}
impl<'a> MultiPacketSizeW<'a> {
    pub unsafe fn bits(self, v: u16) -> &'a mut W {
        assert!(v < 16_384);

        const POS: u8 = 14;
        const MASK: u16 = 0x3fff;
        self.w.bits &= !(u32::from(MASK) << POS);
        self.w.bits |= u32::from(v & MASK) << POS;
        self.w
    }
}

pub(crate) struct ByteCountW<'a> {
    w: &'a mut W,
}
impl<'a> ByteCountW<'a> {
    // Documentation is wrong on this field. Actually 14 bits from
    // offset 0.
    pub unsafe fn bits(self, v: u16) -> &'a mut W {
        assert!(v < 16_384);

        const POS: u8 = 0;
        const MASK: u16 = 0x3fff;
        self.w.bits &= !(u32::from(MASK) << POS);
        self.w.bits |= u32::from(v & MASK) << POS;
        self.w
    }
}
