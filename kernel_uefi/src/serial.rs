// src/serial.rs
use core::fmt;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

const COM1: u16 = 0x3F8;

static SERIAL_LOCK: AtomicBool = AtomicBool::new(false);

pub fn serial_init() {
    unsafe {
        crate::cpu::outb(COM1 + 1, 0x00);
        crate::cpu::outb(COM1 + 3, 0x80);
        crate::cpu::outb(COM1 + 0, 0x03);
        crate::cpu::outb(COM1 + 1, 0x00);
        crate::cpu::outb(COM1 + 3, 0x03);
        crate::cpu::outb(COM1 + 2, 0xC7);
        crate::cpu::outb(COM1 + 4, 0x0B);
    }
}

pub fn serial_write_byte(b: u8) {
    unsafe {
        while (crate::cpu::inb(COM1 + 5) & 0x20) == 0 {}
        crate::cpu::outb(COM1, b);
    }
}

pub fn serial_write_str(s: &str) {
    while SERIAL_LOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    
    for b in s.bytes() {
        serial_write_byte(b);
    }
    
    SERIAL_LOCK.store(false, Ordering::Release);
}

pub fn serial_log(level: &str, msg: &str) {
    // TICKS is AtomicU64, no unsafe needed for load
    let ticks = crate::timer::TICKS.load(Ordering::Relaxed);
    let mut writer = SerialWriter;
    let _ = write!(&mut writer, "[{}] [{}] {}\r\n", ticks, level, msg);
}

pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write_str(s);
        Ok(())
    }
}

pub struct Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write_str(s);
        Ok(())
    }
}