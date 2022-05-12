use crate::atsamd::pipe::MAX_PIPES;
use atsamd_hal::target_device::usb::{
    self,
    host::{BINTERVAL, PCFG, PINTFLAG, PSTATUS, PSTATUSCLR, PSTATUSSET},
};

pub(crate) struct PipeRegs<'a> {
    pub(crate) cfg: &'a mut PCFG,
    pub(crate) binterval: &'a mut BINTERVAL,
    pub(crate) statusclr: &'a mut PSTATUSCLR,
    pub(crate) statusset: &'a mut PSTATUSSET,
    pub(crate) status: &'a mut PSTATUS,
    pub(crate) intflag: &'a mut PINTFLAG,
}

impl<'a> PipeRegs<'a> {
    pub(crate) fn from(host: &'a mut usb::HOST, i: usize) -> PipeRegs {
        assert!(i < MAX_PIPES);
        match i {
            0 => Self {
                cfg: &mut host.pcfg0,
                binterval: &mut host.binterval0,
                statusclr: &mut host.pstatusclr0,
                statusset: &mut host.pstatusset0,
                status: &mut host.pstatus0,
                intflag: &mut host.pintflag0,
            },
            1 => Self {
                cfg: &mut host.pcfg1,
                binterval: &mut host.binterval1,
                statusclr: &mut host.pstatusclr1,
                statusset: &mut host.pstatusset1,
                status: &mut host.pstatus1,
                intflag: &mut host.pintflag1,
            },
            2 => Self {
                cfg: &mut host.pcfg2,
                binterval: &mut host.binterval2,
                statusclr: &mut host.pstatusclr2,
                statusset: &mut host.pstatusset2,
                status: &mut host.pstatus2,
                intflag: &mut host.pintflag2,
            },
            3 => Self {
                cfg: &mut host.pcfg3,
                binterval: &mut host.binterval3,
                statusclr: &mut host.pstatusclr3,
                statusset: &mut host.pstatusset3,
                status: &mut host.pstatus3,
                intflag: &mut host.pintflag3,
            },
            4 => Self {
                cfg: &mut host.pcfg4,
                binterval: &mut host.binterval4,
                statusclr: &mut host.pstatusclr4,
                statusset: &mut host.pstatusset4,
                status: &mut host.pstatus4,
                intflag: &mut host.pintflag4,
            },
            5 => Self {
                cfg: &mut host.pcfg5,
                binterval: &mut host.binterval5,
                statusclr: &mut host.pstatusclr5,
                statusset: &mut host.pstatusset5,
                status: &mut host.pstatus5,
                intflag: &mut host.pintflag5,
            },
            6 => Self {
                cfg: &mut host.pcfg6,
                binterval: &mut host.binterval6,
                statusclr: &mut host.pstatusclr6,
                statusset: &mut host.pstatusset6,
                status: &mut host.pstatus6,
                intflag: &mut host.pintflag6,
            },
            7 => Self {
                cfg: &mut host.pcfg7,
                binterval: &mut host.binterval7,
                statusclr: &mut host.pstatusclr7,
                statusset: &mut host.pstatusset7,
                status: &mut host.pstatus7,
                intflag: &mut host.pintflag7,
            },
            _ => unreachable!(),
        }
    }
}
