#[allow(unused)]
pub mod addr;
#[allow(unused)]
pub mod ctrl_pipe;
#[allow(unused)]
pub mod ext_reg;
#[allow(unused)]
pub mod pck_size;
#[allow(unused)]
pub mod status_bk;
#[allow(unused)]
pub mod status_pipe;

use addr::Addr;
use ctrl_pipe::CtrlPipe;
use ext_reg::ExtReg;
use pck_size::PckSize;
use status_bk::StatusBk;
use status_pipe::StatusPipe;

use crate::{HostEndpoint, RequestCode, RequestDirection, RequestType, SetupPacket, TransferType, UsbError, WValue};

use atsamd_hal::target_device::usb::{
    self,
    host::{BINTERVAL, PCFG, PINTFLAG, PSTATUS, PSTATUSCLR, PSTATUSSET},
};

// Maximum time to wait for a control request with data to finish. cf §9.2.6.1 of USB 2.0.
const USB_TIMEOUT: u64 = 5000; // 5 Seconds

// samd21 only supports 8 pipes.
const MAX_PIPES: usize = 8;

// How many times to retry a transaction that has transient errors.
const NAK_LIMIT: usize = 15;

#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(defmt::Format)]
#[allow(unused)]
pub(crate) enum PipeErr {
    ShortPacket,
    InvalidPipe,
    InvalidToken,
    InvalidRequest,
    Stall,
    TransferFail,
    PipeErr,
    Flow,
    HardwareTimeout,
    DataToggle,
    SoftwareTimeout,
    Other(&'static str),
}

impl From<PipeErr> for UsbError {
    fn from(v: PipeErr) -> Self {
        match v {
            PipeErr::TransferFail => Self::Transient("Transfer failed"),
            PipeErr::Flow => Self::Transient("Data flow"),
            PipeErr::DataToggle => Self::Transient("Data toggle"),

            PipeErr::ShortPacket => Self::Permanent("Short packet"),
            PipeErr::InvalidPipe => Self::Permanent("Invalid pipe"),
            PipeErr::InvalidToken => Self::Permanent("Invalid token"),
            PipeErr::Stall => Self::Permanent("Stall"),
            PipeErr::PipeErr => Self::Permanent("Pipe error"),
            PipeErr::HardwareTimeout => Self::Permanent("Hardware timeout"),
            PipeErr::SoftwareTimeout => Self::Permanent("Software timeout"),
            PipeErr::Other(s) => Self::Permanent(s),
            PipeErr::InvalidRequest => Self::Permanent("Invalid request"),
        }
    }
}

impl From<&'static str> for PipeErr {
    fn from(v: &'static str) -> Self {
        Self::Other(v)
    }
}

pub(crate) struct PipeTable {
    tbl: [PipeDesc; MAX_PIPES],
}

impl PipeTable {
    pub(crate) fn new() -> Self {
        let tbl = {
            let mut tbl: [core::mem::MaybeUninit<PipeDesc>; MAX_PIPES] =
                unsafe { core::mem::MaybeUninit::uninit().assume_init() };

            for e in &mut tbl[..] {
                unsafe { core::ptr::write(e.as_mut_ptr(), PipeDesc::new()) }
            }

            unsafe { core::mem::transmute(tbl) }
        };
        Self { tbl }
    }

    pub(crate) fn pipe_for<'a, 'b>(
        &'a mut self,
        host: &'b mut usb::HOST,
        endpoint: &dyn HostEndpoint,
    ) -> Pipe<'a, 'b> {
        // Just use two pipes for now. 0 is always for control
        // endpoints, 1 for everything else.
        //
        // TODO: cache in-use pipes and return them without init if possible.
        let i = if endpoint.endpoint_address().absolute() == 0 { 0 } else { 1 };

        let pregs = PipeRegs::from(host, i);
        let pdesc = &mut self.tbl[i];

        pregs.cfg.write(|w| {
            let ptype = PType::from(endpoint.transfer_type()) as u8;
            unsafe { w.ptype().bits(ptype) }
        });

        pdesc.bank0.ctrl_pipe.write(|w| {
            w.pdaddr().set_addr(endpoint.device_address().into());
            w.pepnum().set_epnum(endpoint.endpoint_address().into())
        });
        pdesc.bank0.pcksize.write(|w| {
            let mps = endpoint.max_packet_size();
            if mps >= 1023 {
                w.size().bytes1024()
            } else if mps >= 512 {
                w.size().bytes512()
            } else if mps >= 256 {
                w.size().bytes256()
            } else if mps >= 128 {
                w.size().bytes128()
            } else if mps >= 64 {
                w.size().bytes64()
            } else if mps >= 32 {
                w.size().bytes32()
            } else if mps >= 16 {
                w.size().bytes16()
            } else {
                w.size().bytes8()
            }
        });
        Pipe {
            regs: pregs,
            desc: pdesc,
        }
    }
}

// TODO: hide regs/desc fields. Needed right now for init_pipe0.
pub(crate) struct Pipe<'a, 'b> {
    pub(crate) regs: PipeRegs<'b>,
    pub(crate) desc: &'a mut PipeDesc,
}

impl Pipe<'_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn control_transfer(
        &mut self,
        ep: &mut dyn HostEndpoint,
        bm_request_type: RequestType,
        b_request: RequestCode,
        w_value: WValue,
        w_index: u16,
        buf: Option<&mut [u8]>,
        after_millis: fn(u64) -> u64,
    ) -> Result<usize, PipeErr> {
        let buflen = buf.as_ref().map_or(0, |b| b.len() as u16);
        let mut setup_packet = SetupPacket {
            bm_request_type,
            b_request,
            w_value,
            w_index,
            w_length: buflen,
        };
        self.send(ep, PToken::Setup, &DataBuf::from(&mut setup_packet), NAK_LIMIT, after_millis)?;

        let direction = bm_request_type.direction().ok_or(PipeErr::InvalidRequest)?;
        let mut transfer_len = 0;

        if let Some(buf) = buf {
            // TODO: data stage, has up to 5,000ms (in 500ms
            // per-packet chunks) to complete. cf §9.2.6.4 of USB 2.0.

            transfer_len = match direction {
                RequestDirection::DeviceToHost => self.in_transfer(ep, buf, NAK_LIMIT, after_millis)?,
                RequestDirection::HostToDevice => self.out_transfer(ep, buf, NAK_LIMIT, after_millis)?
            }
        }

        // TODO: status stage has up to 50ms to complete. cf §9.2.6.4 of USB 2.0.
        self.desc.bank0.pcksize.modify(|_, w| {
            unsafe { w.byte_count().bits(0) };
            unsafe { w.multi_packet_size().bits(0) }
        });

        let token = match direction {
            RequestDirection::DeviceToHost => PToken::Out,
            RequestDirection::HostToDevice => PToken::In,
        };

        self.dispatch_retries(ep, token, NAK_LIMIT, after_millis)?;

        Ok(transfer_len)
    }

    fn send(&mut self, ep: &mut dyn HostEndpoint, token: PToken, buf: &DataBuf, nak_limit: usize, after_millis: fn(u64) -> u64) -> Result<(), PipeErr> {
        self.desc.bank0.addr.write(|w| unsafe { w.addr().bits(buf.ptr as u32) });
        // configure packet size PCKSIZE.SIZE
        self.desc.bank0.pcksize.modify(|_, w| {
            unsafe { w.byte_count().bits(buf.len as u16) };
            unsafe { w.multi_packet_size().bits(0) }
        });

        self.dispatch_retries(ep, token, nak_limit, after_millis)
    }

    pub fn in_transfer(&mut self, ep: &mut dyn HostEndpoint, buf: &mut [u8], nak_limit: usize, after_millis: fn(u64) -> u64) -> Result<usize, PipeErr> {
        // TODO: pull this from pipe descriptor for this addr/ep.
        let packet_size = 8;

        self.desc.bank0.pcksize.modify(|_, w| {
            unsafe { w.byte_count().bits(buf.len() as u16) };
            unsafe { w.multi_packet_size().bits(0) }
        });

        // Read until we get a short packet (indicating that there's
        // nothing left for us in this transaction) or the buffer is full.
        //
        // TODO: It is sometimes valid to get a short packet when
        // variable length data is desired by the driver. cf §5.3.2 of USB 2.0.
        let mut bytes_received = 0;
        while bytes_received < buf.len() {
            // Move the buffer pointer forward as we get data.
            self.desc.bank0.addr.write(|w| unsafe {
                w.addr()
                    .bits(buf.as_mut_ptr() as u32 + bytes_received as u32)
            });
            self.regs.statusclr.write(|w| w.bk0rdy().set_bit());

            self.dispatch_retries(ep, PToken::In, nak_limit, after_millis)?;
            let recvd = self.desc.bank0.pcksize.read().byte_count().bits() as usize;
            bytes_received += recvd;
            if recvd < packet_size {
                break;
            }

            // Don't allow writing past the buffer.
            assert!(bytes_received <= buf.len());
        }

        self.regs.statusset.write(|w| w.pfreeze().set_bit());
        if bytes_received < buf.len() {
            // TODO: honestly, this is probably a panic condition,
            // since whatever's in DataBuf.ptr is totally
            // invalid. Alternately, this function should be declared
            // `unsafe`. OR! Make the function take a mutable slice of
            // u8, and leave it up to the caller to figure out how to
            // deal with short packets.
            Err(PipeErr::ShortPacket)
        } else {
            Ok(bytes_received)
        }
    }

    pub fn out_transfer(&mut self, ep: &mut dyn HostEndpoint, buf: &[u8], nak_limit: usize, after_millis: fn(u64) -> u64) -> Result<usize, PipeErr> {
        self.desc.bank0.pcksize.modify(|_, w| {
            unsafe { w.byte_count().bits(buf.len() as u16) };
            unsafe { w.multi_packet_size().bits(0) }
        });

        let mut bytes_sent = 0;
        while bytes_sent < buf.len() {
            self.desc
                .bank0
                .addr
                .write(|w| unsafe { w.addr().bits(buf.as_ptr() as u32 + bytes_sent as u32) });
            self.dispatch_retries(ep, PToken::Out, nak_limit, after_millis)?;

            let sent = self.desc.bank0.pcksize.read().byte_count().bits() as usize;
            bytes_sent += sent;
            trace!("!! wrote {} of {}", bytes_sent, buf.len());
        }

        Ok(bytes_sent)
    }

    fn data_toggle(&mut self, ep: &mut dyn HostEndpoint, token: PToken) {
        let toggle = match token {
            PToken::In => {
                let t = !ep.in_toggle();
                ep.set_in_toggle(t);
                t
            }

            PToken::Out => {
                let t = !ep.out_toggle();
                ep.set_out_toggle(t);
                t
            }

            PToken::Setup => false,

            _ => !self.regs.status.read().dtgl().bit_is_set(),
        };

        if toggle {
            self.dtgl_set();
        } else {
            self.dtgl_clear();
        }
    }

    fn dtgl_set(&mut self) {
        self.regs.statusset.write(|w| w.dtgl().set_bit());
    }

    fn dtgl_clear(&mut self) {
        self.regs.statusclr.write(|w| unsafe {
            w.bits(1)
        });
    }

    // This is the only function that calls `millis`. If we can make
    // this just take the current timestamp, we can make this
    // non-blocking.
    fn dispatch_retries(
        &mut self,
        ep: &mut dyn HostEndpoint,
        token: PToken,
        retries: usize,
        after_millis: fn(u64) -> u64,
    ) -> Result<(), PipeErr> {
        self.dispatch_packet(ep, token);

        let until = after_millis(USB_TIMEOUT);
        // let mut last_err = PipeErr::SWTimeout;
        let mut naks = 0;
        loop {
            if after_millis(0) > until {
                return Err(PipeErr::SoftwareTimeout);
            }

            let res = self.dispatch_result(token);
            match res {
                Ok(true) => {
                    // Swap sequence bits on successful transfer.
                    if token == PToken::In {
                        ep.set_in_toggle(!ep.in_toggle());
                    } else if token == PToken::Out {
                        ep.set_out_toggle(!ep.out_toggle());
                    }
                    return Ok(());
                }
                Ok(false) => continue,

                Err(err) => {
                    match err {
                        PipeErr::DataToggle => self.data_toggle(ep, token),

                        // Flow error on interrupt pipes means we got a NAK = no data
                        PipeErr::Flow if ep.transfer_type() == TransferType::Interrupt => return Err(PipeErr::Flow),

                        PipeErr::Stall => return Err(PipeErr::Stall),

                        _any => {
                            naks += 1;
                            if naks > retries {
                                return Err(err);
                            }
                        }
                    }
                }
            }
        }
    }

    fn dispatch_packet(&mut self, ep: &mut dyn HostEndpoint, token: PToken) {
        self.regs
            .cfg
            .modify(|_, w| unsafe { w.ptoken().bits(token as u8) });
        self.regs.intflag.modify(|_, w| w.trfail().set_bit());
        self.regs.intflag.modify(|_, w| w.perr().set_bit());

        match token {
            PToken::Setup => {
                self.regs.intflag.write(|w| w.txstp().set_bit());
                self.regs.statusset.write(|w| w.bk0rdy().set_bit());

                // Toggles should be 1 for host and function's
                // sequence at end of setup transaction. cf §8.6.1 of USB 2.0.
                self.dtgl_clear();
                ep.set_in_toggle(true);
                ep.set_out_toggle(true);
            }
            PToken::In => {
                self.regs.statusclr.write(|w| w.bk0rdy().set_bit());
                if ep.in_toggle() {
                    self.dtgl_set();
                } else {
                    self.dtgl_clear();
                }
            }
            PToken::Out => {
                self.regs.intflag.write(|w| w.trcpt0().set_bit());
                self.regs.statusset.write(|w| w.bk0rdy().set_bit());
                if ep.out_toggle() {
                    self.dtgl_set();
                } else {
                    self.dtgl_clear();
                }
            }
            _ => {}
        }

        self.regs.statusclr.write(|w| w.pfreeze().set_bit());
    }

    fn dispatch_result(&mut self, token: PToken) -> Result<bool, PipeErr> {
        if self.is_transfer_complete(token)? {
            self.regs.statusset.write(|w| w.pfreeze().set_bit());
            Ok(true)
        } else if self.desc.bank0.status_bk.read().errorflow().bit_is_set() {
            Err(PipeErr::Flow)
        } else if self.desc.bank0.status_pipe.read().touter().bit_is_set() {
            Err(PipeErr::HardwareTimeout)
        } else if self.desc.bank0.status_pipe.read().dtgler().bit_is_set() {
            Err(PipeErr::DataToggle)
        } else if self.regs.intflag.read().trfail().bit_is_set() {
            self.regs.intflag.write(|w| w.trfail().set_bit());
            Err(PipeErr::TransferFail)
        } else {
            // Nothing wrong, but not done yet.
            Ok(false)
        }
    }

    fn is_transfer_complete(&mut self, token: PToken) -> Result<bool, PipeErr> {
        match token {
            PToken::Setup => {
                if self.regs.intflag.read().txstp().bit_is_set() {
                    self.regs.intflag.write(|w| w.txstp().set_bit());
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            PToken::In => {
                if self.regs.intflag.read().trcpt0().bit_is_set() {
                    self.regs.intflag.write(|w| w.trcpt0().set_bit());
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            PToken::Out => {
                if self.regs.intflag.read().trcpt0().bit_is_set() {
                    self.regs.intflag.write(|w| w.trcpt0().set_bit());
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Err(PipeErr::InvalidToken),
        }
    }
}

// TODO: merge into SVD for pipe cfg register.
#[derive(Copy, Clone, Debug, PartialEq)]
#[derive(defmt::Format)]
pub(crate) enum PToken {
    Setup = 0x0,
    In = 0x1,
    Out = 0x2,
    _Reserved = 0x3,
}

// TODO: merge into SVD for pipe cfg register.
#[allow(unused)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum PType {
    Disabled = 0x0,
    Control = 0x1,
    ISO = 0x2,
    Bulk = 0x3,
    Interrupt = 0x4,
    Extended = 0x5,
    _Reserved0 = 0x06,
    _Reserved1 = 0x07,
}

impl From<TransferType> for PType {
    fn from(v: TransferType) -> Self {
        match v {
            TransferType::Control => Self::Control,
            TransferType::Isochronous => Self::ISO,
            TransferType::Bulk => Self::Bulk,
            TransferType::Interrupt => Self::Interrupt,
        }
    }
}

#[derive(defmt::Format)]
struct DataBuf<'a> {
    ptr: *mut u8,
    len: usize,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a, T> From<&'a mut T> for DataBuf<'a> {
    fn from(v: &'a mut T) -> Self {
        Self {
            ptr: v as *mut T as *mut u8,
            len: core::mem::size_of::<T>(),
            _marker: core::marker::PhantomData,
        }
    }
}

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

// §32.8.7.1
pub(crate) struct PipeDesc {
    pub bank0: BankDesc,
    pub bank1: BankDesc,
}

// 2 banks: 32 bytes per pipe.
impl PipeDesc {
    pub fn new() -> Self {
        Self {
            bank0: BankDesc::new(),
            bank1: BankDesc::new(),
        }
    }
}

#[repr(C, packed)]
// 16 bytes per bank.
pub(crate) struct BankDesc {
    pub addr: Addr,
    pub pcksize: PckSize,
    pub extreg: ExtReg,
    pub status_bk: StatusBk,
    _reserved0: u8,
    pub ctrl_pipe: CtrlPipe,
    pub status_pipe: StatusPipe,
    _reserved1: u8,
}

impl BankDesc {
    fn new() -> Self {
        Self {
            addr: Addr::from(0),
            pcksize: PckSize::from(0),
            extreg: ExtReg::from(0),
            status_bk: StatusBk::from(0),
            _reserved0: 0,
            ctrl_pipe: CtrlPipe::from(0),
            status_pipe: StatusPipe::from(0),
            _reserved1: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bank_desc_sizes() {
        assert_eq!(core::mem::size_of::<Addr>(), 4, "Addr register size.");
        assert_eq!(core::mem::size_of::<PckSize>(), 4, "PckSize register size.");
        assert_eq!(core::mem::size_of::<ExtReg>(), 2, "ExtReg register size.");
        assert_eq!(
            core::mem::size_of::<StatusBk>(),
            1,
            "StatusBk register size."
        );
        assert_eq!(
            core::mem::size_of::<CtrlPipe>(),
            2,
            "CtrlPipe register size."
        );
        assert_eq!(
            core::mem::size_of::<StatusPipe>(),
            1,
            "StatusPipe register size."
        );

        // addr at 0x00 for 4
        // pcksize at 0x04 for 4
        // extreg at 0x08 for 2
        // status_bk at 0x0a for 2
        // ctrl_pipe at 0x0c for 2
        // status_pipe at 0x0e for 1
        assert_eq!(
            core::mem::size_of::<BankDesc>(),
            16,
            "Bank descriptor size."
        );
    }

    #[test]
    fn bank_desc_offsets() {
        let bd = BankDesc::new();
        let base = &bd as *const _ as usize;

        assert_offset("Addr", &bd.addr, base, 0x00);
        assert_offset("PckSize", &bd.pcksize, base, 0x04);
        assert_offset("ExtReg", &bd.extreg, base, 0x08);
        assert_offset("StatusBk", &bd.status_bk, base, 0x0a);
        assert_offset("CtrlPipe", &bd.ctrl_pipe, base, 0x0c);
        assert_offset("StatusPipe", &bd.status_pipe, base, 0x0e);
    }

    #[test]
    fn pipe_desc_size() {
        assert_eq!(core::mem::size_of::<PipeDesc>(), 32);
    }

    #[test]
    fn pipe_desc_offsets() {
        let pd = PipeDesc::new();
        let base = &pd as *const _ as usize;

        assert_offset("Bank0", &pd.bank0, base, 0x00);
        assert_offset("Bank1", &pd.bank1, base, 0x10);
    }

    fn assert_offset<T>(name: &str, field: &T, base: usize, offset: usize) {
        let ptr = field as *const _ as usize;
        assert_eq!(ptr - base, offset, "{} register offset.", name);
    }
}
