use core::fmt::{Formatter, Pointer};

use cortex_m::peripheral::{SYST};
use cortex_m::peripheral::syst::SystClkSource;

use fugit::{Duration, Instant};

use crate::{Local};

const SYSTICK_CYCLES: u32 = 40_000_000;

pub type SysInstant = Instant<u64, 1, SYSTICK_CYCLES>;
pub type SysDuration = Duration<u64, 1, SYSTICK_CYCLES>;

pub struct SysClock {
    syst: &'static mut SYST,
    past_cycles: u64,
}

impl core::fmt::Debug for SysClock {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.syst.fmt(f)
    }
}

static CLOCK: Local<SysClock> = Local::uninit("CLOCK");

pub fn init(syst: &'static mut SYST) {
    CLOCK.init_static(SysClock::new(syst));
}

pub fn now() -> SysInstant {
    CLOCK.now()
}

pub fn after_millis(millis: u64) -> SysInstant {
    now() + SysDuration::millis(millis)
}

const MAX_RVR: u32 = 0x00FF_FFFF;

const SYST_CSR_COUNTFLAG: u32 = 1 << 16;

impl SysClock {
    fn new(syst: &'static mut SYST) -> Self {
        syst.disable_interrupt();
        syst.disable_counter();
        syst.clear_current();

        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(MAX_RVR);

        syst.enable_counter();
        // only if using #[exception] SysTick() (we don't)
        // syst.enable_interrupt();

        Self {
            syst,
            past_cycles: 0,
        }
    }

    pub(crate) const fn zero() -> SysInstant {
        SysInstant::from_ticks(0)
    }

    fn now(&self) -> SysInstant {
        SysInstant::from_ticks(self.cycles())
    }

    fn cycles(&self) -> u64 {
        // systick cvr counts DOWN
        let elapsed_cycles = MAX_RVR - self.syst.cvr.read();

        // blatantly ripped from SYST.has_wrapped()
        // see https://github.com/rust-embedded/cortex-m/issues/438
        if self.syst.csr.read() & SYST_CSR_COUNTFLAG != 0 {
            // This is ok because I hereby declare it to be so.
            // TODO u64 are not atomic. use u32 += 1 with MAX_RVR pow2 - 1 then shift left upon read.
            unsafe { *(&self.past_cycles as *const u64 as *mut u64) += MAX_RVR as u64; }
        }
        self.past_cycles as u64 + elapsed_cycles as u64
    }
}


