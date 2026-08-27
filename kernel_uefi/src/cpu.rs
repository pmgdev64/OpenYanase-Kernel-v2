// src/cpu.rs
use core::arch::asm;

pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

pub unsafe fn inb(port: u16) -> u8 {
    let mut value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}

pub unsafe fn io_wait() {
    outb(0x80, 0);
}