// src/timer.rs
use core::arch::global_asm;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use core::fmt::Write;
use crate::cpu::outb;
use crate::idt;
use crate::serial;

pub static TICKS: AtomicU64 = AtomicU64::new(0);
pub static mut TIMER_FREQ: u32 = 1000;
static TIMER_LOCK: AtomicBool = AtomicBool::new(false);

global_asm!(
    r#"
    .global timer_interrupt_stub
    .extern handle_timer_interrupt

    timer_interrupt_stub:
        cld
        push rax
        push rcx
        push rdx
        push rsi
        push rdi
        push r8
        push r9
        push r10
        push r11

        call handle_timer_interrupt

        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rdx
        pop rcx
        pop rax

        iretq
    "#
);

extern "C" {
    fn timer_interrupt_stub();
}

#[no_mangle]
pub extern "C" fn handle_timer_interrupt() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        outb(0x20, 0x20);
    }
}

pub fn init_timer(frequency_hz: u32) {
    while TIMER_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let freq = if frequency_hz == 0 { 1000 } else { frequency_hz };
    unsafe { TIMER_FREQ = freq; }

    let divisor = 1_193_182 / freq;
    unsafe {
        outb(0x43, 0x36);
        outb(0x40, (divisor & 0xFF) as u8);
        outb(0x40, ((divisor >> 8) & 0xFF) as u8);
    }

    idt::set_gate(32, timer_interrupt_stub as usize as u64);
    
    let mut msg_buffer = [0u8; 64];
    let mut writer = MsgWriter(&mut msg_buffer);
    let _ = write!(&mut writer, "Timer initialized at {}Hz", freq);
    // Convert to string safely
    let msg = core::str::from_utf8(&msg_buffer).unwrap_or("Timer initialized");
    // Find null terminator
    let msg_len = msg.as_bytes().iter().position(|&b| b == 0).unwrap_or(msg.len());
    let msg = &msg[..msg_len];
    serial::serial_log("INFO", msg);

    TIMER_LOCK.store(false, Ordering::Release);
}

struct MsgWriter<'a>(&'a mut [u8]);

impl<'a> core::fmt::Write for MsgWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let len = bytes.len().min(self.0.len() - 1);
        self.0[..len].copy_from_slice(&bytes[..len]);
        self.0[len] = 0;
        Ok(())
    }
}

pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn sleep(ms: u64) {
    let start = get_ticks();
    while get_ticks() - start < ms {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}