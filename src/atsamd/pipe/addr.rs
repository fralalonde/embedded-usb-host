/// § 32.8.7.2
/// Address of the Data Buffer.
///
/// Offset: 0x00 & 0x10
/// Reset: 0xxxxxxxxx
/// Property: NA
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Addr(u32);

pub struct R {
    bits: u32,
}

pub struct W {
    bits: u32,
}

impl Addr {
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

impl From<u32> for Addr {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl R {
    /// Value in raw bits.
    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn addr(&self) -> AddrR {
        AddrR(self.bits)
    }
}

/// Data Pointer Address Value
///
/// These bits define the data pointer address as an absolute double
/// word address in RAM. The two least significant bits must be zero
/// to ensure the descriptor is 32-bit aligned.
pub struct AddrR(u32);
impl AddrR {
    pub fn bits(&self) -> u32 {
        self.0
    }
}

impl W {
    /// Write raw bits.
    pub unsafe fn bits(&mut self, v: u32) -> &mut Self {
        self.bits = v;
        self
    }

    pub fn addr(&mut self) -> AddrW {
        AddrW { w: self }
    }
}

pub struct AddrW<'a> {
    w: &'a mut W,
}
impl<'a> AddrW<'a> {
    pub unsafe fn bits(self, v: u32) -> &'a mut W {
        // Address must be 32-bit aligned. cf §32.8.7.2 of SAMD21 data sheet.
        debug_assert!(v.trailing_zeros() >= 2);
        self.w.bits = v;
        self.w
    }

    // TODO: "safe" method take a pointer instead of raw u32
}
