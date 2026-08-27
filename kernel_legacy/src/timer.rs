// src/timer.rs

use crate::cpu::outb;
use core::ptr::{read_volatile, write_volatile};

pub static mut TICKS: u64 = 0;
// Lưu lại tần số khởi tạo của Timer (mặc định 1000Hz = 1ms/tick)
pub static mut TIMER_FREQ: u32 = 1000;

pub fn init_timer(frequency_hz: u32) {
    let freq = if frequency_hz == 0 { 1000 } else { frequency_hz };
    unsafe {
        TIMER_FREQ = freq;
    }
    
    let divisor = 1_193_182 / freq;
    unsafe {
        outb(0x43, 0x36);
        outb(0x40, (divisor & 0xFF) as u8);
        outb(0x40, ((divisor >> 8) & 0xFF) as u8);
    }
}

pub fn on_tick() {
    unsafe {
        let current = read_volatile(&TICKS as *const u64);
        write_volatile(&mut TICKS as *mut u64, current.wrapping_add(1));
    }
}

pub fn get_ticks() -> u64 {
    unsafe { 
        read_volatile(&TICKS as *const u64) 
    }
}

pub fn sleep_ms(ms: u64) {
    let start = get_ticks();
    let freq = unsafe { TIMER_FREQ as u64 };
    
    // Quy đổi từ Milliseconds sang Ticks thực tế
    // Ví dụ: ms = 1000, freq = 60Hz -> ticks_to_wait = (1000 * 60) / 1000 = 60 ticks
    let ticks_to_wait = (ms * freq) / 1000;
    let target_ticks = ticks_to_wait.max(1); // Đảm bảo chờ ít nhất 1 tick

    while get_ticks() - start < target_ticks {
        unsafe {
            // Bắt buộc bật ngắt (sti) cùng lúc với hlt để CPU chắc chắn nhận IRQ0 tỉnh dậy
            core::arch::asm!("sti; hlt", options(nomem, nostack));
        }
    }
}