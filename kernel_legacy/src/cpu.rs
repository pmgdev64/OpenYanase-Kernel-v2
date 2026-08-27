// src/cpu.rs
use core::arch::asm;

// === 8-BIT PORT I/O ===
pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}

// === 16-BIT PORT I/O (Dùng cho AC97 Audio Mixer NAM) ===
pub unsafe fn outw(port: u16, value: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack, preserves_flags)
    );
}

pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!(
        "in ax, dx",
        out("ax") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}

// === 32-BIT PORT I/O (Dùng cho PCI Config Space & AC97 Bus Master NABM) ===
pub unsafe fn outl(port: u16, value: u32) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack, preserves_flags)
    );
}

pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!(
        "in eax, dx",
        out("eax") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}

// === CPU CONTROLS & WAITS ===
pub unsafe fn io_wait() {
    outb(0x80, 0);
}

pub unsafe fn hlt() {
    asm!("hlt", options(nomem, nostack));
}

pub unsafe fn cli() {
    asm!("cli", options(nomem, nostack));
}

pub unsafe fn sti() {
    asm!("sti", options(nomem, nostack));
}