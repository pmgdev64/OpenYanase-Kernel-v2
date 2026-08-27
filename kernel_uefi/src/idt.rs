// src/idt.rs
use crate::cpu::{outb, io_wait};

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attributes: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attributes: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub fn set_handler(&mut self, handler_addr: u64, selector: u16, flags: u8) {
        self.selector = selector;
        self.offset_low = handler_addr as u16;
        self.offset_mid = (handler_addr >> 16) as u16;
        self.offset_high = (handler_addr >> 32) as u32;
        self.type_attributes = flags;
        self.ist = 0;
        self.reserved = 0;
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];
static mut IDTR: Idtr = Idtr { limit: 0, base: 0 };

unsafe fn pic_remap() {
    outb(0x20, 0x11); io_wait();
    outb(0xA0, 0x11); io_wait();

    outb(0x21, 0x20); io_wait();
    outb(0xA1, 0x28); io_wait();

    outb(0x21, 0x04); io_wait();
    outb(0xA1, 0x02); io_wait();

    outb(0x21, 0x01); io_wait();
    outb(0xA1, 0x01); io_wait();

    // Master: mở IRQ0(timer), IRQ1(kbd), IRQ2(cascade bắt buộc cho slave PIC)
    outb(0x21, 0xF8);
    // Slave: mask hết ban đầu, mouse.rs sẽ tự unmask IRQ12
    outb(0xA1, 0xFF);
}

pub fn init_idt() {
    unsafe {
        pic_remap();

        IDTR = Idtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: IDT.as_ptr() as u64,
        };

        core::arch::asm!("lidt [{}]", in(reg) &IDTR, options(readonly, nostack, preserves_flags));
    }
}

pub fn set_gate(vector: usize, handler_addr: u64) {
    unsafe {
        IDT[vector].set_handler(handler_addr, 0x08, 0x8E); // DPL=0
    }
}

pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

pub fn set_gate_dpl(vector: usize, handler_addr: u64, flags: u8) {
    unsafe {
        IDT[vector].set_handler(handler_addr, 0x08, flags);
    }
}