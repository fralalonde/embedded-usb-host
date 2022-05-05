/// §32.8.7.4
/// Extended Register.
///
/// Offset: 0x08
/// Reset: 0xxxxxxxx
/// Property: NA
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub(crate) struct ExtReg(u16);

pub(crate) struct R {
    bits: u16,
}

pub(crate) struct W {
    bits: u16,
}

impl ExtReg {
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
}

impl From<u16> for ExtReg {
    fn from(v: u16) -> Self {
        Self(v)
    }
}

impl R {
    /// Value in raw bits.
    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn variable(&self) -> VariableR {
        let bits = {
            const POS: u8 = 4;
            const MASK: u16 = 0x7ff;
            (self.bits >> POS) & MASK
        };

        VariableR(bits)
    }

    pub fn subpid(&self) -> SubPIDR {
        let bits = {
            const POS: u8 = 0;
            const MASK: u16 = 0xf;
            ((self.bits >> POS) & MASK) as u8
        };

        SubPIDR(bits)
    }
}

/// Variable field send with extended token
///
/// These bits define the VARIABLE field sent with extended token. See
/// “Section 2.1.1 Protocol Extension Token in the reference document
/// ENGINEERING CHANGE NOTICE, USB 2.0 Link Power Management
/// Addendum.”
///
/// To support the USB2.0 Link Power Management addition the VARIABLE
/// field should be set as described below.
///
/// | VARIABLE       | Description           |
/// +----------------+-----------------------+
/// | VARIABLE[3:0]  | bLinkState[1]         |
/// | VARIABLE[7:4]  | BESL (See LPM ECN)[2] |
/// | VARIABLE[8]    | bRemoteWake[1]        |
/// | VARIABLE[10:9] | Reserved              |
///
/// [1] for a definition of LPM Token bRemoteWake and bLinkState
/// fields, refer to "Table 2-3 in the reference document ENGINEERING
/// CHANGE NOTICE, USB 2.0 Link Power Management Addendum"
///
/// [2] for a definition of LPM Token BESL field, refer to "Table 2-3
/// in the reference document ENGINEERING CHANGE NOTICE, USB 2.0 Link
/// Power Management Addendum" and "Table X-X1 in Errata for ECN USB
/// 2.0 Link Power Management.
pub(crate) struct VariableR(u16);
impl VariableR {
    pub fn bits(&self) -> u16 {
        self.0
    }
}

/// SUBPID field with extended token
///
/// These bits define the SUBPID field sent with extended token. See
/// “Section 2.1.1 Protocol Extension Token in the reference document
/// ENGINEERING CHANGE NOTICE, USB 2.0 Link Power Management
/// Addendum”.
///
/// To support the USB2.0 Link Power Management addition the SUBPID
/// field should be set as described in “Table 2.2 SubPID Types in the
/// reference document ENGINEERING CHANGE NOTICE, USB 2.0 Link Power
/// Management Addendum”.
pub(crate) struct SubPIDR(u8);
impl SubPIDR {
    pub fn bits(&self) -> u8 {
        self.0
    }
}

impl W {
    /// Write raw bits.
    pub unsafe fn bits(&mut self, v: u16) -> &mut Self {
        self.bits = v;
        self
    }

    pub fn variable(&mut self) -> VariableW {
        VariableW { w: self }
    }
    pub fn subpid(&mut self) -> SubPIDW {
        SubPIDW { w: self }
    }
}

pub(crate) struct VariableW<'a> {
    w: &'a mut W,
}
impl<'a> VariableW<'a> {
    pub unsafe fn bits(self, v: u16) -> &'a mut W {
        const POS: u8 = 4;
        const MASK: u16 = 0x7ff;
        self.w.bits &= !((MASK as u16) << POS);
        self.w.bits |= ((v & MASK) as u16) << POS;
        self.w
    }
}

pub(crate) struct SubPIDW<'a> {
    w: &'a mut W,
}
impl<'a> SubPIDW<'a> {
    pub unsafe fn bits(self, v: u16) -> &'a mut W {
        const POS: u8 = 0;
        const MASK: u16 = 0xf;
        self.w.bits &= !((MASK as u16) << POS);
        self.w.bits |= ((v & MASK) as u16) << POS;
        self.w
    }
}
