#![no_std]
#![no_main]
#![allow(non_snake_case)]


// use defmt_rtt as _;
use cortex_m_rt::entry;
use panic_probe as _;

use atsamd21j::hal::{clock::GenericClockController, delay::Delay, pac::Peripherals, prelude::*};
use atsamd21j::pac::CorePeripherals;

use rustBoot_hal::atsamd::atsamd21j17::FlashWriterEraser;
use rustBoot_update::update::{update_flash::FlashUpdater, UpdateInterface};

// SCB Application Interrupt and Reset Control Register Definitions
const SCB_AIRCR_VECTKEY_POS: u32 = 16; // SCB AIRCR: VECTKEY Position
const SCB_AIRCR_PRIGROUP_POS: u32 = 8; // SCB AIRCR: PRIGROUP Position
const SCB_AIRCR_PRIGROUP_MSK: u32 = 7u32 << SCB_AIRCR_PRIGROUP_POS; // SCB AIRCR: PRIGROUP Mask
const SCB_AIRCR_SYSRESETREQ_POS: u32 = 2; // SCB AIRCR: SYSRESETREQ Position
const SCB_AIRCR_SYSRESETREQ_MSK: u32 = 1u32 << SCB_AIRCR_SYSRESETREQ_POS; // SCB AIRCR: SYSRESETREQ Mask

/// System Reset
///
/// Initiates a system reset request to reset the MCU.
#[inline]
pub fn nvic_systemreset() -> ! {
    let core_peripherals = CorePeripherals::take().unwrap();
    let scb = core_peripherals.SCB;
    cortex_m::asm::dsb();
    unsafe {
        scb.aircr.write(
            (0x5FA << SCB_AIRCR_VECTKEY_POS)
                | (scb.aircr.read() & SCB_AIRCR_PRIGROUP_MSK)
                | SCB_AIRCR_SYSRESETREQ_MSK,
        );
    }
    cortex_m::asm::dsb();
    loop {}
}

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    
    let pins = peripherals.PORT.split();
    let mut led = pins.pa17.into_push_pull_output(); // Assuming PA17 is the LED pin
    
    let mut delay = Delay::new(core.SYST, &mut clocks);
    let mut count = 0u8;

    // flash green led
    while count < 5 {
        delay.delay_ms(250u16);
        led.set_high().unwrap();
        delay.delay_ms(250u16);
        led.set_low().unwrap();
        delay.delay_ms(250u16);
        count += 1;
    }

    let flash_writer = FlashWriterEraser::new(peripherals.NVMCTRL);
    let updater = FlashUpdater::new(flash_writer);
    match updater.update_trigger() {
        Ok(_v) => {}
        Err(e) => panic!("couldnt trigger update: {}", e),
    }

    nvic_systemreset();
}
