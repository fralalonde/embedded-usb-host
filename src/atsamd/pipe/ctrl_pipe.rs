/// Host Control Pipe.
///
/// Offset: 0x0c
/// Reset: 0xXXXX
/// Property: PAC Write-Protection, Write-Synchronized, Read-Synchronized
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct CtrlPipe(u16);

pub struct R {
    bits: u16,
}

pub struct W {
    bits: u16,
}

impl CtrlPipe {
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

impl From<u16> for CtrlPipe {
    fn from(v: u16) -> Self {
        Self(v)
    }
}

impl R {
    /// Value in raw bits.
    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn permax(&self) -> PErMaxR {
        let bits = {
            const POS: u8 = 12;
            const MASK: u16 = 0xf;
            ((self.bits >> POS) & MASK) as u8
        };

        PErMaxR(bits)
    }

    pub fn pepnum(&self) -> PEpNumR {
        let bits = {
            const POS: u8 = 8;
            const MASK: u16 = 0xf;
            ((self.bits >> POS) & MASK) as u8
        };

        PEpNumR(bits)
    }

    pub fn pdaddr(&self) -> PDAddrR {
        let bits = {
            const POS: u8 = 0;
            const MASK: u16 = 0x3f;
            ((self.bits >> POS) & MASK) as u8
        };

        PDAddrR(bits)
    }
}

/// Pipe Error Max Number
///
/// These bits define the maximum number of error for this Pipe before
/// freezing the pipe automatically.
pub struct PErMaxR(u8);
impl PErMaxR {
    pub fn max(&self) -> u8 {
        self.0
    }
}

/// Pipe EndPoint Number
///
/// These bits define the number of endpoint for this Pipe.
pub struct PEpNumR(u8);
impl PEpNumR {
    pub fn epnum(&self) -> u8 {
        self.0
    }
}

/// Pipe Device Address
///
/// These bits define the Device Address for this pipe.
pub struct PDAddrR(u8);
impl PDAddrR {
    pub fn addr(&self) -> u8 {
        self.0
    }
}

impl W {
    /// Write raw bits.

    pub unsafe fn bits(&mut self, v: u16) -> &mut Self {
        self.bits = v;
        self
    }

    pub fn permax(&mut self) -> PErMaxW {
        PErMaxW { w: self }
    }

    pub fn pepnum(&mut self) -> PEpNumW {
        PEpNumW { w: self }
    }

    pub fn pdaddr(&mut self) -> PDAddrW {
        PDAddrW { w: self }
    }
}

pub struct PErMaxW<'a> {
    w: &'a mut W,
}
impl<'a> PErMaxW<'a> {
    pub unsafe fn bits(self, v: u8) -> &'a mut W {
        const POS: u8 = 12;
        const MASK: u8 = 0xf;
        self.w.bits &= !(u16::from(MASK) << POS);
        self.w.bits |= u16::from(v & MASK) << POS;
        self.w
    }

    pub fn set_max(self, v: u8) -> &'a mut W {
        unsafe { self.bits(v) }
    }
}

pub struct PEpNumW<'a> {
    w: &'a mut W,
}
impl<'a> PEpNumW<'a> {
    pub unsafe fn bits(self, v: u8) -> &'a mut W {
        const POS: u8 = 8;
        const MASK: u8 = 0xf;
        self.w.bits &= !(u16::from(MASK) << POS);
        self.w.bits |= u16::from(v & MASK) << POS;
        self.w
    }

    pub fn set_epnum(self, v: u8) -> &'a mut W {
        unsafe { self.bits(v) }
    }
}

pub struct PDAddrW<'a> {
    w: &'a mut W,
}
impl<'a> PDAddrW<'a> {
    pub unsafe fn bits(self, v: u8) -> &'a mut W {
        const POS: u8 = 0;
        const MASK: u8 = 0x3f;
        self.w.bits &= !(u16::from(MASK) << POS);
        self.w.bits |= u16::from(v & MASK) << POS;
        self.w
    }

    pub fn set_addr(self, v: u8) -> &'a mut W {
        unsafe { self.bits(v) }
    }
}
