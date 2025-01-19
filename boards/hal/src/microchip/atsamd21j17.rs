#![no_std]
#![no_main]

use crate::FlashInterface;
use atsamd21_hal as hal;
use core::ptr::write_volatile;
use cortex_m::asm;
use hal::pac::{Peripherals, NVMCTRL};

#[rustfmt::skip]
mod atsamd21j17_constants {
    pub const FLASH_PAGE_SIZE : u32 = 64;               // SAMD21 has 64-byte pages
    pub const FLASH_ROW_SIZE  : u32 = 256;              // SAMD21 erases in 4-page rows (256 bytes)
    pub const STACK_LOW       : u32 = 0x2000_0000;      // Start of SRAM
    pub const STACK_UP        : u32 = 0x2000_8000;      // End of SRAM (32KB)
    pub const RB_HDR_SIZE     : u32 = 0x100;
    pub const BASE_ADDR       : u32 = 0x4000;           // After bootloader
    pub const VTR_TABLE_SIZE  : u32 = 0x100;
    pub const FW_RESET_VTR    : u32 = BASE_ADDR + RB_HDR_SIZE + VTR_TABLE_SIZE + 0x89;
}

pub struct FlashWriterEraser {
    pub nvm: NVMCTRL,
}

impl FlashWriterEraser {
    pub fn new() -> Self {
        FlashWriterEraser {
            nvm: Peripherals::take().unwrap().NVMCTRL,
        }
    }
}

impl FlashInterface for FlashWriterEraser {
    fn hal_flash_erase(&self, addr: usize, len: usize) {
        let mut address = (addr & !0xFF) as u32; // Align to row boundary
        let remaining_bytes = len % FLASH_ROW_SIZE as usize;
        let mut num_rows = len / FLASH_ROW_SIZE as usize;

        if remaining_bytes != 0 {
            num_rows += 1;
        }

        while num_rows > 0 {
            // Wait for NVM to be ready
            while self.nvm.status.read().ready().bit_is_clear() {}

            // Execute erase row command
            unsafe {
                self.nvm.ctrla.write(|w| w.cmd().er()); // Erase Row command
                self.nvm.addr.write(|w| w.addr().bits(address >> 1)); // Address must be divided by 2
                self.nvm.ctrla.write(|w| w.cmdex().bits(0xA5)); // Command execution key
            }

            address += FLASH_ROW_SIZE;
            num_rows -= 1;
        }
    }

    fn hal_flash_write(&self, address: usize, data: *const u8, len: usize) {
        let mut idx = 0;
        let mut dst = address as *mut u32; // SAMD21 writes 32-bit words
        let src = data as *const u32;

        // Enable automatic write mode
        self.nvm.ctrlb.modify(|_, w| w.manw().clear_bit());

        while idx < len {
            // Wait for NVM to be ready
            while self.nvm.status.read().ready().bit_is_clear() {}

            if (len - idx) >= 4 {
                unsafe {
                    write_volatile(dst, *src.add(idx / 4));
                }
                dst = ((dst as u32) + 4) as *mut u32;
                idx += 4;
            } else {
                // Handle non-word-aligned writes
                let mut word_buffer = [0u8; 4];
                let remaining = len - idx;

                unsafe {
                    for i in 0..remaining {
                        word_buffer[i] = *data.add(idx + i);
                    }
                    write_volatile(dst, u32::from_le_bytes(word_buffer));
                }

                idx += remaining;
            }
        }
    }

    fn hal_flash_unlock(&self) {
        // SAMD21 doesn't need explicit unlock, but we'll clear the security bit if set
        if self.nvm.status.read().nvmlock().bit_is_set() {
            self.nvm.status.write(|w| w.nvmlock().clear_bit());
        }
    }

    fn hal_flash_lock(&self) {
        // Set write protection if needed
        self.nvm.ctrlb.modify(|_, w| w.rwweep().set_bit());
    }

    fn hal_init(&self) {
        // Configure NVM controller for optimal performance
        self.nvm
            .ctrlb
            .modify(|_, w| w.readmode().no_miss_penalty().sleepprm().wakeonaccess());
    }
}

pub fn preboot() {
    // Any specific pre-boot operations needed for SAMD21
}

struct RefinedUsize<const MIN: u32, const MAX: u32, const VAL: u32>(u32);

impl<const MIN: u32, const MAX: u32, const VAL: u32> RefinedUsize<MIN, MAX, VAL> {
    pub fn bounded_int(i: u32) -> Self {
        assert!(i >= MIN && i <= MAX);
        RefinedUsize(i)
    }

    pub fn single_valued_int(i: u32) -> Self {
        assert!(i == VAL);
        RefinedUsize(i)
    }
}

#[rustfmt::skip]
pub fn boot_from(fw_base_address: usize) -> ! {
    let address = fw_base_address as u32;
    let scb = hal::pac::SCB::ptr();
    unsafe {
        let sp = RefinedUsize::<STACK_LOW, STACK_UP, 0>::bounded_int(
            *(fw_base_address as *const u32)).0;
        let rv = RefinedUsize::<0, 0, FW_RESET_VTR>::single_valued_int(
            *((fw_base_address + 4) as *const u32)).0;
        let jump_vector = core::mem::transmute::<usize, extern "C" fn() -> !>(rv as usize);
        (*scb).vtor.write(address);
        cortex_m::register::msp::write(sp);
        jump_vector();
    }
    loop{}
}
