// src/cpu.rs
use core::arch::asm;

// ==========================================
// PORT I/O OPERATIONS (8-bit, 16-bit, 32-bit)
// ==========================================

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

pub unsafe fn io_wait() {
    outb(0x80, 0);
}

// ==========================================
// CPU CONTROL INSTRUCTIONS (CLI, STI, HLT)
// ==========================================

pub unsafe fn cli() {
    asm!("cli", options(nomem, nostack, preserves_flags));
}

pub unsafe fn sti() {
    asm!("sti", options(nomem, nostack, preserves_flags));
}

pub unsafe fn hlt() {
    asm!("hlt", options(nomem, nostack, preserves_flags));
}

// ==========================================
// CONTROL REGISTERS (CR0, CR2, CR3, CR4)
// ==========================================

pub unsafe fn read_cr0() -> usize {
    let val: usize;
    asm!("mov {}, cr0", out(reg) val, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn write_cr0(val: usize) {
    asm!("mov cr0, {}", in(reg) val, options(nomem, nostack));
}

pub unsafe fn read_cr2() -> usize {
    let val: usize;
    asm!("mov {}, cr2", out(reg) val, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn read_cr3() -> usize {
    let val: usize;
    asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn write_cr3(val: usize) {
    asm!("mov cr3, {}", in(reg) val, options(nomem, nostack));
}

pub unsafe fn read_cr4() -> usize {
    let val: usize;
    asm!("mov {}, cr4", out(reg) val, options(nomem, nostack, preserves_flags));
    val
}

pub unsafe fn write_cr4(val: usize) {
    asm!("mov cr4, {}", in(reg) val, options(nomem, nostack));
}

/// Invalidate Page (Làm sạch bộ đệm TLB cho một địa chỉ ảo cụ thể)
pub unsafe fn invlpg(addr: usize) {
    asm!("invlpg [{}]", in(reg) addr, options(nostack));
}

// ==========================================
// MODEL-SPECIFIC REGISTERS (MSRs)
// ==========================================

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = (value & 0xFFFFFFFF) as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack)
    );
}