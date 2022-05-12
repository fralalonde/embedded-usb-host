use crate::atsamd::pipe::{MAX_PIPES, Pipe, PipeDesc, PipeType};
use crate::atsamd::pipe::regs::PipeRegs;
use crate::{EndpointProperties, HostEndpoint, MaxPacketSize};
use atsamd_hal::target_device::usb;

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

    pub(crate) fn pipe_for<'a, 'b>(&'a mut self, host: &'b mut usb::HOST, endpoint: &dyn HostEndpoint) -> Pipe<'a, 'b> {

        let i = if endpoint.endpoint_address().absolute() == 0 { 0 } else { 1 };

        let pregs = PipeRegs::from(host, i);
        let pdesc = &mut self.tbl[i];

        pregs.cfg.write(|w| {
            let ptype = PipeType::from(endpoint.transfer_type()) as u8;
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
