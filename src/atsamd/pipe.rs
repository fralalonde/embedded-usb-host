#[allow(unused)]
pub mod addr;
#[allow(unused)]
pub mod ctrl_pipe;
#[allow(unused)]
pub mod ext_reg;
#[allow(unused)]
pub mod pck_size;
#[allow(unused)]
pub mod regs;
#[allow(unused)]
pub mod status_bk;
#[allow(unused)]
pub mod status_pipe;
#[allow(unused)]
pub mod table;

use addr::Addr;
use ctrl_pipe::CtrlPipe;
use ext_reg::ExtReg;
use pck_size::PckSize;
use status_bk::StatusBk;
use status_pipe::StatusPipe;

use crate::{
    to_slice_mut, HostEndpoint, RequestCode, RequestDirection, RequestType, SetupPacket,
    TransferType, WValue,
};

use crate::atsamd::error::PipeErr;
use regs::PipeRegs;

// Maximum time to wait for a control request with data to finish. cf §9.2.6.1 of USB 2.0.
const USB_TIMEOUT: u64 = 5000; // 5 Seconds

// samd21 only supports 8 pipes.
const MAX_PIPES: usize = 8;

// How many times to retry a transaction that has transient errors.
const NAK_LIMIT: usize = 15;

// TODO: hide regs/desc fields. Needed right now for init_pipe0.
pub(crate) struct Pipe<'a, 'b> {
    regs: PipeRegs<'b>,
    desc: &'a mut PipeDesc,
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
        self.send(
            ep,
            PipeToken::Setup,
            to_slice_mut(&mut setup_packet),
            after_millis,
        )?;

        let direction = bm_request_type.direction().ok_or(PipeErr::InvalidRequest)?;
        let mut transfer_len = 0;

        if let Some(buf) = buf {
            transfer_len = match direction {
                RequestDirection::DeviceToHost => self.in_transfer(ep, buf, after_millis)?,
                RequestDirection::HostToDevice => self.out_transfer(ep, buf, after_millis)?,
            }
        }

        self.bank0_size(0);

        let token = match direction {
            RequestDirection::DeviceToHost => PipeToken::Out,
            RequestDirection::HostToDevice => PipeToken::In,
        };

        self.sync_tx(ep, token, after_millis)?;

        Ok(transfer_len)
    }

    fn bank0_size(&mut self, len: usize) {
        self.desc.bank0.pcksize.modify(|_, w| {
            unsafe { w.byte_count().bits(len as u16) };
            unsafe { w.multi_packet_size().bits(0) }
        });
    }

    fn send(
        &mut self,
        ep: &mut dyn HostEndpoint,
        token: PipeToken,
        buf: &mut [u8],
        after_millis: fn(u64) -> u64,
    ) -> Result<(), PipeErr> {
        self.desc
            .bank0
            .addr
            .write(|w| unsafe { w.addr().bits(buf.as_ptr() as u32) });
        self.bank0_size(buf.len());
        self.sync_tx(ep, token, after_millis)
    }

    pub fn in_transfer(
        &mut self,
        ep: &mut dyn HostEndpoint,
        buf: &mut [u8],
        after_millis: fn(u64) -> u64,
    ) -> Result<usize, PipeErr> {
        // TODO: pull this from pipe descriptor for this addr/ep.
        let packet_size = 8;
        self.bank0_size(buf.len());

        let mut bytes_received = 0;
        while bytes_received < buf.len() {
            // Move the buffer pointer forward as we get data.
            self.desc.bank0.addr.write(|w| unsafe {
                w.addr().bits(buf.as_mut_ptr() as u32 + bytes_received as u32)
            });
            self.regs.statusclr.write(|w| w.bk0rdy().set_bit());

            self.sync_tx(ep, PipeToken::In, after_millis)?;
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
            Err(PipeErr::ShortPacket)
        } else {
            Ok(bytes_received)
        }
    }

    pub fn out_transfer(
        &mut self,
        ep: &mut dyn HostEndpoint,
        buf: &[u8],
        after_millis: fn(u64) -> u64,
    ) -> Result<usize, PipeErr> {
        self.bank0_size(buf.len());

        let mut bytes_sent = 0;
        while bytes_sent < buf.len() {
            self.desc.bank0.addr.write(|w| unsafe { w.addr().bits(buf.as_ptr() as u32 + bytes_sent as u32) });
            self.sync_tx(ep, PipeToken::Out, after_millis)?;

            let sent = self.desc.bank0.pcksize.read().byte_count().bits() as usize;
            bytes_sent += sent;
            trace!("!! wrote {} of {}", bytes_sent, buf.len());
        }

        Ok(bytes_sent)
    }

    fn data_toggle(&mut self, ep: &mut dyn HostEndpoint, token: PipeToken) {
        let toggle = match token {
            PipeToken::In => {
                let t = !ep.toggle();
                ep.set_toggle(t);
                t
            }

            PipeToken::Out => {
                let t = !ep.toggle();
                ep.set_toggle(t);
                t
            }

            PipeToken::Setup => false,

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
        self.regs.statusclr.write(|w| unsafe { w.bits(1) });
    }

    // This is the only function that calls `millis`. If we can make
    // this just take the current timestamp, we can make this
    // non-blocking.
    fn sync_tx(
        &mut self,
        ep: &mut dyn HostEndpoint,
        token: PipeToken,
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
                    if matches!(token, PipeToken::In | PipeToken::Out) {
                        // Save endpoint toggle state on successful transfer.
                        ep.set_toggle(!ep.toggle());
                    }
                    return Ok(());
                }
                Ok(false) => continue,

                Err(err) => {
                    match err {
                        PipeErr::DataToggle => self.data_toggle(ep, token),

                        // Flow error on interrupt pipes means we got a NAK = no data
                        PipeErr::Flow if matches!(ep.transfer_type(), TransferType::Interrupt) => {
                            return Err(PipeErr::Flow);
                        }

                        PipeErr::Stall => return Err(PipeErr::Stall),

                        _any => {
                            naks += 1;
                            if naks > NAK_LIMIT {
                                return Err(err);
                            }
                        }
                    }
                }
            }
        }
    }

    fn dispatch_packet(&mut self, ep: &mut dyn HostEndpoint, token: PipeToken) {
        self.regs.cfg.modify(|_, w| unsafe { w.ptoken().bits(token as u8) });
        self.regs.intflag.modify(|_, w| w.trfail().set_bit());
        self.regs.intflag.modify(|_, w| w.perr().set_bit());
        self.regs.intflag.modify(|_, w| w.stall().set_bit());

        match token {
            PipeToken::Setup => {
                self.regs.intflag.write(|w| w.txstp().set_bit());
                self.regs.statusset.write(|w| w.bk0rdy().set_bit());

                // Toggles should be 1 for host and function's
                // sequence at end of setup transaction. cf §8.6.1 of USB 2.0.
                self.dtgl_clear();
                ep.set_toggle(true);
            }
            PipeToken::In => {
                self.regs.statusclr.write(|w| w.bk0rdy().set_bit());
                if ep.toggle() {
                    self.dtgl_set();
                } else {
                    self.dtgl_clear();
                }
            }
            PipeToken::Out => {
                self.regs.intflag.write(|w| w.trcpt0().set_bit());
                self.regs.statusset.write(|w| w.bk0rdy().set_bit());
                if ep.toggle() {
                    self.dtgl_set();
                } else {
                    self.dtgl_clear();
                }
            }
            _ => {}
        }

        self.regs.statusclr.write(|w| w.pfreeze().set_bit());
    }

    fn dispatch_result(&mut self, token: PipeToken) -> Result<bool, PipeErr> {
        if self.is_transfer_complete(token) {
            self.regs.statusset.write(|w| w.pfreeze().set_bit());
            Ok(true)
        } else if self.desc.bank0.status_bk.read().errorflow().bit_is_set() {
            Err(PipeErr::Flow)
        } else if self.desc.bank0.status_pipe.read().touter().bit_is_set() {
            Err(PipeErr::HardwareTimeout)
        } else if self.desc.bank0.status_pipe.read().dtgler().bit_is_set() {
            Err(PipeErr::DataToggle)
        } else if self.regs.intflag.read().stall().bit_is_set() {
            self.regs.intflag.write(|w| w.stall().set_bit());
            Err(PipeErr::Stall)
        } else if self.regs.intflag.read().trfail().bit_is_set() {
            self.regs.intflag.write(|w| w.trfail().set_bit());
            Err(PipeErr::TransferFail)
        } else {
            // Nothing wrong, but not done yet.
            Ok(false)
        }
    }

    fn is_transfer_complete(&mut self, token: PipeToken) -> bool {
        match token {
            PipeToken::Setup =>
                if self.regs.intflag.read().txstp().bit_is_set() {
                    self.regs.intflag.write(|w| w.txstp().set_bit());
                    return true;
                }
            PipeToken::In | PipeToken::Out =>
                if self.regs.intflag.read().trcpt0().bit_is_set() {
                    self.regs.intflag.write(|w| w.trcpt0().set_bit());
                    return true;
                }
        }
        false
    }
}

// TODO: merge into SVD for pipe cfg register.
#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
pub(crate) enum PipeToken {
    Setup = 0x0,
    In = 0x1,
    Out = 0x2,
    // _Reserved = 0x3,
}

// TODO: merge into SVD for pipe cfg register.
#[allow(unused)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum PipeType {
    Disabled = 0x0,
    Control = 0x1,
    ISO = 0x2,
    Bulk = 0x3,
    Interrupt = 0x4,
    Extended = 0x5,
    _Reserved0 = 0x06,
    _Reserved1 = 0x07,
}

impl From<TransferType> for PipeType {
    fn from(v: TransferType) -> Self {
        match v {
            TransferType::Control => Self::Control,
            TransferType::Isochronous => Self::ISO,
            TransferType::Bulk => Self::Bulk,
            TransferType::Interrupt => Self::Interrupt,
        }
    }
}

// §32.8.7.1
pub(crate) struct PipeDesc {
    pub bank0: BankDesc,
    // TODO use bank1 for double buffered
    #[allow(unused)]
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
