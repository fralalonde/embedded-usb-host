#![no_std]
#![no_main]

extern crate embedded_usb_host as usb_host;

use trinket_m0 as bsp;

use bsp::clock::GenericClockController;
use bsp::entry;
use bsp::pac::{interrupt, CorePeripherals, Peripherals};

use cortex_m::peripheral::NVIC;

use trinket_m0::clock::{ClockGenId, ClockSource};

use core::ops::{DerefMut};
use core::panic::PanicInfo;

use atsamd_hal as hal;


use hal::sercom::{
    v2::{
        uart::{self, BaudMode, Oversampling},
        Sercom0,
        Sercom2,
    },
    I2CMaster3,
    I2CMaster2,
    I2CMaster1,
    I2CMaster0,
};

use atsamd_hal::time::{Hertz};
use atsamd_hal::gpio::v2::*;
use atsamd_hal::sercom::UART0;

use embedded_usb_host::driver::UsbMidiDriver;

use hal::sercom::*;
use atsamd_hal::gpio::{self, *};

use atsamd_hal::gpio::PfD;

use atsamd_hal::rtc::Rtc;
use cortex_m::asm::delay;
use cortex_m_rt::exception;


use usb_host::{atsamd, Driver, HostEvent, Endpoint, UsbStack};
use usb_host::keyboard::BootKbdDriver;

use resource::{Local, Shared};

#[macro_use]
extern crate log;

use panic_probe as _;
use rtt_target::{rtt_init_print, rprintln};
use rtt_logger::RTTLogger;
use log::{ LevelFilter};

mod time;
mod resource;

static CORE: Local<CorePeripherals> = Local::uninit("CORE");

static BOOTKBD: Local<BootKbdDriver> = Local::uninit("BOOTKBD");

static USB_STACK: Shared<UsbStack<atsamd::HostController>> = Shared::uninit("USB_STACK");

static LOGGER: RTTLogger = RTTLogger::new(LevelFilter::Trace);

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CORE.init_static(CorePeripherals::take().unwrap());
    time::init(&mut core.SYST);

    rtt_init_print!();
    unsafe { log::set_logger_racy(&LOGGER); }
    log::set_max_level(log::LevelFilter::Trace);
    info!("Insert coin to play");

    // internal 32khz required for USB to complete swrst
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    let mut pins = bsp::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let timer_clock = clocks
        .configure_gclk_divider_and_source(ClockGenId::GCLK4, 1, ClockSource::OSC32K, false)
        .unwrap();

    let usb_pins = atsamd::HostPins::new(
        pins.usb_dm.into_floating_input(&mut pins.port),
        pins.usb_dp.into_floating_input(&mut pins.port),
        Some(pins.usb_sof.into_floating_input(&mut pins.port)),
        Some(pins.usb_host_enable.into_floating_input(&mut pins.port)),
    );

    let mut usb_host = atsamd::HostController::new(
        peripherals.USB,
        usb_pins,
        &mut pins.port,
        &mut clocks,
        &mut peripherals.PM,
        || time::now().ticks(),
        |ms| time::after_millis(ms).ticks(),
    );

    usb_host.reset_host();

    let mut usb_stack = UsbStack::new(usb_host);
    let bootkbd = BootKbdDriver::new();
    usb_stack.add_driver(BOOTKBD.init_static(bootkbd));
    USB_STACK.init_static(usb_stack);

    unsafe {
        core.NVIC.set_priority(interrupt::USB, 3);
        NVIC::unmask(interrupt::USB);
    }

    loop {
        red_led.toggle();
        delay(20_000_000);
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USB() {
    NVIC::mask(interrupt::USB);
    let mut usb_stack = USB_STACK.lock();
    // process any state changes and pending transfers
    usb_stack.update();
    unsafe { NVIC::unmask(interrupt::USB) }
}
