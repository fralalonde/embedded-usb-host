use crate::atsamd::pipe::{PipeErr, PipeTable};

use crate::{AddressPool, Endpoint, HostEvent, RequestCode, RequestType, stack, UsbError, UsbHost, WValue};

use atsamd_hal::{
    calibration::{usb_transn_cal, usb_transp_cal, usb_trim_cal},
    clock::{ClockGenId, ClockSource, GenericClockController},
    gpio::{self, Floating, Input, OpenDrain, Output},
    target_device::{PM, USB},
};
use embedded_hal::digital::v2::OutputPin;

#[derive(Debug)]
#[derive(defmt::Format)]
pub enum HostIrq {
    Detached,
    Attached,
    RamAccess,
    UpstreamResume,
    DownResume,
    WakeUp,
    Reset,
    HostStartOfFrame,
}

const NAK_LIMIT: usize = 15;

#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(defmt::Format)]
pub enum HostState {
    Initialize,
    WaitForDevice,

    WaitResetComplete,
    WaitSOF(u64),

    Ready,
    Error,
}

pub struct HostController {
    usb: USB,
    state: HostState,

    // Need chunk of RAM for USB pipes, which gets used with DESCADD register.
    pipe_table: PipeTable,

    _dm_pad: gpio::Pa24<gpio::PfG>,
    _dp_pad: gpio::Pa25<gpio::PfG>,
    _sof_pad: Option<gpio::Pa23<gpio::PfG>>,
    host_enable_pin: Option<gpio::Pa28<Output<OpenDrain>>>,
    after_millis: fn(u64) -> u64,
}

pub struct HostPins {
    dm_pin: gpio::Pa24<Input<Floating>>,
    dp_pin: gpio::Pa25<Input<Floating>>,
    sof_pin: Option<gpio::Pa23<Input<Floating>>>,
    host_enable_pin: Option<gpio::Pa28<Input<Floating>>>,
}

impl HostPins {
    pub fn new(
        dm_pin: gpio::Pa24<Input<Floating>>,
        dp_pin: gpio::Pa25<Input<Floating>>,
        sof_pin: Option<gpio::Pa23<Input<Floating>>>,
        host_enable_pin: Option<gpio::Pa28<Input<Floating>>>,
    ) -> Self {
        Self {
            dm_pin,
            dp_pin,
            sof_pin,
            host_enable_pin,
        }
    }
}

impl HostController {
    pub fn new(
        usb: USB,
        pins: HostPins,
        port: &mut gpio::Port,
        clocks: &mut GenericClockController,
        power: &mut PM,
        after_millis: fn(u64) -> u64,
    ) -> Self {
        power.apbbmask.modify(|_, w| w.usb_().set_bit());

        clocks.configure_gclk_divider_and_source(ClockGenId::GCLK6, 1, ClockSource::DFLL48M, false);
        let gclk6 = clocks.get_gclk(ClockGenId::GCLK6).expect("Could not get clock 6");
        clocks.usb(&gclk6);

        HostController {
            usb,
            state: HostState::Initialize,
            pipe_table: PipeTable::new(),

            _dm_pad: pins.dm_pin.into_function_g(port),
            _dp_pad: pins.dp_pin.into_function_g(port),
            _sof_pad: pins.sof_pin.map(|p| p.into_function_g(port)),
            host_enable_pin: pins.host_enable_pin.map(|p| p.into_open_drain_output(port)),
            after_millis,
        }
    }

    /// Low-Level USB Host Interrupt service method
    /// Any Event returned by should be sent to process_event()
    /// then fsm_tick() should be called for each event or once if no event at all
    fn next_irq(&self) -> Option<HostIrq> {
        let flags = self.usb.host().intflag.read();

        if flags.ddisc().bit_is_set() {
            self.usb.host().intflag.write(|w| w.ddisc().set_bit());
            Some(HostIrq::Detached)
        } else if flags.dconn().bit_is_set() {
            self.usb.host().intflag.write(|w| w.dconn().set_bit());
            Some(HostIrq::Attached)
        } else if flags.ramacer().bit_is_set() {
            self.usb.host().intflag.write(|w| w.ramacer().set_bit());
            Some(HostIrq::RamAccess)
        } else if flags.uprsm().bit_is_set() {
            self.usb.host().intflag.write(|w| w.uprsm().set_bit());
            Some(HostIrq::UpstreamResume)
        } else if flags.dnrsm().bit_is_set() {
            self.usb.host().intflag.write(|w| w.dnrsm().set_bit());
            Some(HostIrq::DownResume)
        } else if flags.wakeup().bit_is_set() {
            self.usb.host().intflag.write(|w| w.wakeup().set_bit());
            Some(HostIrq::WakeUp)
        } else if flags.rst().bit_is_set() {
            self.usb.host().intflag.write(|w| w.rst().set_bit());
            Some(HostIrq::Reset)
        } else if flags.hsof().bit_is_set() {
            self.usb.host().intflag.write(|w| w.hsof().set_bit());
            Some(HostIrq::HostStartOfFrame)
        } else {
            None
        }
    }

    pub fn reset_host(&mut self) {
        self.usb.host().ctrla.write(|w| w.swrst().set_bit());
        while self.usb.host().syncbusy.read().swrst().bit_is_set() {}
        self.usb.host().ctrla.modify(|_, w| w.mode().host());

        unsafe {
            self.usb.host().padcal.write(|w| {
                w.transn().bits(usb_transn_cal());
                w.transp().bits(usb_transp_cal());
                w.trim().bits(usb_trim_cal())
            });
            self.usb.host().descadd.write(|w| w.bits(&self.pipe_table as *const _ as u32));
        }

        self.usb.host().ctrlb.modify(|_, w| w.spdconf().normal());
        self.usb.host().ctrla.modify(|_, w| w.runstdby().set_bit());

        if let Some(host_enable_pin) = &mut self.host_enable_pin {
            host_enable_pin.set_high().expect("USB Reset [host enable pin]");
        }

        self.usb.host().intenset.write(|w| {
            w.dconn().set_bit();
            w.ddisc().set_bit();
            w.wakeup().set_bit();
            w.ramacer().set_bit();
            w.uprsm().set_bit();
            w.dnrsm().set_bit();
            w.rst().set_bit();
            w.hsof().set_bit()
        });

        self.usb.host().ctrla.modify(|_, w| w.enable().set_bit());
        while self.usb.host().syncbusy.read().enable().bit_is_set() {}
        self.usb.host().ctrlb.modify(|_, w| w.vbusok().set_bit());
        debug!("USB Host Reset");
    }
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
            PipeErr::HWTimeout => Self::Permanent("Hardware timeout"),
            PipeErr::SWTimeout => Self::Permanent("Software timeout"),
            PipeErr::Other(s) => Self::Permanent(s),
        }
    }
}

impl UsbHost for HostController {
    fn max_host_packet_size(&self) -> u16 {
        match self.usb.host().status.read().speed().bits() {
            0x0 => 64,
            _ => 8,
        }
    }

    fn update(&mut self, addr_pool: &mut AddressPool) -> Option<HostEvent> {
        let prev_state = self.state;
        let mut host_event = None;
        let irq = self.next_irq();

        match (irq, self.state) {
            (Some(HostIrq::Detached), _) => self.state = HostState::Initialize,
            (Some(HostIrq::Attached), HostState::WaitForDevice) => {
                self.usb.host().ctrlb.modify(|_, w| w.busreset().set_bit());
                self.state = HostState::WaitResetComplete;
            }
            (Some(HostIrq::Reset), HostState::WaitResetComplete) => {
                // Seems unneccesary, since SOFE will be set immediately after reset according to §32.6.3.3.
                self.usb.host().ctrlb.modify(|_, w| w.sofe().set_bit());
                // USB spec requires 20ms of SOF after bus reset.
                self.state = HostState::WaitSOF((self.after_millis)(20));
            }
            (Some(HostIrq::HostStartOfFrame), HostState::WaitSOF(until)) if self.now() >= until => {
                self.state = HostState::Ready;
                match stack::address_dev(self, addr_pool) {
                    Ok((device, desc)) => {
                        debug!("USB Ready {:?}", device);
                        self.state = HostState::Ready;
                        host_event = Some(HostEvent::Ready(device, desc));
                    }
                    Err(e) => {
                        warn!("Enumeration error: {:?}", e);
                        self.state = HostState::Error
                    }
                }
            }
            (Some(HostIrq::HostStartOfFrame), HostState::Ready) => {
                host_event = Some(HostEvent::Tick);
            }
            _ => {}
        };

        if self.state == HostState::Initialize {
            self.reset_host();
            self.state = HostState::WaitForDevice;
            host_event = Some(HostEvent::Reset);
        }

        if prev_state != self.state {
            trace!("USB new task state {:?}", self.state)
        }
        host_event
    }

    fn control_transfer(&mut self, ep: &mut dyn Endpoint, bm_request_type: RequestType,
                        b_request: RequestCode, w_value: WValue, w_index: u16, buf: Option<&mut [u8]>,
    ) -> Result<usize, UsbError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), ep);
        let len = pipe.control_transfer(ep, bm_request_type, b_request, w_value, w_index, buf, self.after_millis)?;
        Ok(len)
    }

    fn in_transfer(&mut self, ep: &mut dyn Endpoint, buf: &mut [u8]) -> Result<usize, UsbError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), ep);
        let len = pipe.in_transfer(ep, buf, NAK_LIMIT, self.after_millis)?;
        Ok(len)
    }

    fn out_transfer(&mut self, ep: &mut dyn Endpoint, buf: &[u8]) -> Result<usize, UsbError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), ep);
        let len = pipe.out_transfer(ep, buf, NAK_LIMIT, self.after_millis)?;
        Ok(len)
    }

    fn after_millis(&self, ms: u64) -> u64 {
        (self.after_millis)(ms)
    }
}
