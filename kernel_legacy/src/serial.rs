// src/serial.rs

use core::fmt;

const COM1: u16 = 0x3F8;

pub fn serial_init() {
    unsafe {
        port_outb(COM1 + 1, 0x00);
        port_outb(COM1 + 3, 0x80);
        port_outb(COM1 + 0, 0x03);
        port_outb(COM1 + 1, 0x00);
        port_outb(COM1 + 3, 0x03);
        port_outb(COM1 + 2, 0xC7);
        port_outb(COM1 + 4, 0x0B);
    }
}

pub fn serial_write_byte(b: u8) {
    unsafe {
        while (port_inb(COM1 + 5) & 0x20) == 0 {}
        port_outb(COM1, b);
    }
}

pub fn serial_write(s: &str) {
    for b in s.bytes() {
        serial_write_byte(b);
    }
}

unsafe fn port_outb(port: u16, val: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nostack, preserves_flags)
    );
}

unsafe fn port_inb(port: u16) -> u8 {
    let ret: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") ret,
        in("dx") port,
        options(nostack, preserves_flags)
    );
    ret
}

pub struct Serial;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write(s);
        Ok(())
    }
}

// Thêm vào src/serial.rs

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = $crate::serial::Serial.write_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ({
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    });
}