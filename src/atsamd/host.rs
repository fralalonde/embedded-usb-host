use crate::{HostEndpoint, HostError, HostEvent, RequestCode, RequestType, UsbError, UsbHost, WValue};

use crate::atsamd::pipe::table::PipeTable;

use bsp::hal;
use hal::prelude::*;

use atsamd_hal::{
    calibration::{usb_transn_cal, usb_transp_cal, usb_trim_cal},
    clock::{ClockGenId, ClockSource, GenericClockController},
    gpio::{self},
    target_device::{PM, USB},
};
use gpio::v2::{Floating, Input, Output};
use embedded_hal::digital::v2::OutputPin;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HostState {
    Init,
    Disconnected,
    BusReset,
    BusSettleUntil(u64),
    Connected,
    Error,
}

pub struct HostPins {
    dm_pin: gpio::v2::PA24,
    dp_pin: gpio::v2::PA25,
    sof_pin: Option<gpio::v2::PA23>,
    host_enable_pin: Option<gpio::v2::PA28>,
}

impl HostPins {
    pub fn new(
        dm_pin: gpio::v2::PA24, dp_pin: gpio::v2::PA25,
        sof_pin: Option<gpio::v2::PA23>, host_enable_pin: Option<gpio::v2::PA28>,
    ) -> Self {
        Self {
            dm_pin,
            dp_pin,
            sof_pin,
            host_enable_pin,
        }
    }
}

pub struct HostController {
    usb: USB,
    state: HostState,

    pipe_table: PipeTable,

    _dm_pad: gpio::v2::PA24,
    _dp_pad: gpio::v2::PA25,
    _sof_pad: Option<gpio::v2::PA23>,
    host_enable_pin: Option<gpio::v2::PA28>,
    now: fn() -> u64,
    after_millis: fn(u64) -> u64,
}

impl HostController {
    pub fn new(
        usb: USB, pins: HostPins, port: &mut gpio::Port, clocks: &mut GenericClockController, power: &mut PM,
        now: fn() -> u64, after_millis: fn(u64) -> u64,
    ) -> Self {
        power.apbbmask.modify(|_, w| w.usb_().set_bit());

        clocks.configure_gclk_divider_and_source(ClockGenId::GCLK6, 1, ClockSource::DFLL48M, false);
        let gclk6 = clocks.get_gclk(ClockGenId::GCLK6).expect("Could not get clock 6");
        clocks.usb(&gclk6);

        HostController {
            usb,
            state: HostState::Init,
            pipe_table: PipeTable::new(),

            _dm_pad: pins.dm_pin/*.into_function_g(port)*/,
            _dp_pad: pins.dp_pin/*.into_function_g(port),*/,
            _sof_pad: pins.sof_pin/*.map(|p| p.into_function_g(port))*/,
            host_enable_pin: pins.host_enable_pin.into_open_drain_output(port),
            now,
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
    }
}

impl UsbHost for HostController {
    fn update(&mut self) -> Option<HostEvent> {
        let prev_state = self.state;
        let mut host_event = None;
        let irq = self.next_irq();

        match (irq, self.state) {
            (Some(HostIrq::Detached), _) => self.state = HostState::Init,
            (Some(HostIrq::Attached), HostState::Disconnected) => {
                self.usb.host().ctrlb.modify(|_, w| w.busreset().set_bit());
                self.state = HostState::BusReset;
            }
            (Some(HostIrq::Reset), HostState::BusReset) => {
                // Seems unnecessary, since SOFE will be set immediately after reset according to §32.6.3.3.
                self.usb.host().ctrlb.modify(|_, w| w.sofe().set_bit());
                // USB spec requires 20ms delay after bus reset.
                self.state = HostState::BusSettleUntil((self.after_millis)(20));
            }
            (Some(HostIrq::HostStartOfFrame), HostState::BusSettleUntil(until)) if self.now() >= until => {
                self.state = HostState::Connected;
                host_event = Some(HostEvent::Ready);
            }
            _ => {}
        };

        if self.state == HostState::Init {
            self.reset_host();
            self.state = HostState::Disconnected;
            host_event = Some(HostEvent::Reset);
        }

        if prev_state != self.state {
            debug!("USB Host: {:?}", self.state)
        }
        host_event
    }

    fn max_host_packet_size(&self) -> u16 {
        match self.usb.host().status.read().speed().bits() {
            0x0 => 64,
            _ => 8,
        }
    }

    fn now(&self) -> u64 {
        (self.now)()
    }

    fn after_millis(&self, ms: u64) -> u64 {
        (self.after_millis)(ms)
    }

    fn control_transfer(
        &mut self, endpoint: &mut dyn HostEndpoint, bm_request_type: RequestType, b_request: RequestCode,
        w_value: WValue, w_index: u16, buf: Option<&mut [u8]>,
    ) -> Result<usize, HostError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), endpoint);
        let len =
            pipe.control_transfer(endpoint, bm_request_type, b_request, w_value, w_index, buf, self.after_millis)?;
        Ok(len)
    }

    fn in_transfer(&mut self, endpoint: &mut dyn HostEndpoint, buf: &mut [u8]) -> Result<usize, HostError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), endpoint);
        let len = pipe.in_transfer(endpoint, buf, self.after_millis)?;
        Ok(len)
    }

    fn out_transfer(&mut self, endpoint: &mut dyn HostEndpoint, buf: &[u8]) -> Result<usize, HostError> {
        let mut pipe = self.pipe_table.pipe_for(self.usb.host_mut(), endpoint);
        let len = pipe.out_transfer(endpoint, buf, self.after_millis)?;
        Ok(len)
    }
}
