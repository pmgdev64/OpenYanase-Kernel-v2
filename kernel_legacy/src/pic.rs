// src/pic.rs
use crate::cpu::outb;

pub fn init() {
    unsafe {
        // ICW1
        outb(0x20, 0x11);
        outb(0xA0, 0x11);

        // ICW2: Remap IRQ0-7 -> Vector 32-39 | IRQ8-15 -> Vector 40-47
        outb(0x21, 0x20); 
        outb(0xA1, 0x28);

        // ICW3: Cascade
        outb(0x21, 0x04);
        outb(0xA1, 0x02);

        // ICW4: Mode 8086
        outb(0x21, 0x01);
        outb(0xA1, 0x01);

        // Master PIC: Unmask IRQ0 (Timer), IRQ1 (Keyboard), IRQ2 (Slave Cascade) -> 0xF8
        outb(0x21, 0xF8);

        // Slave PIC: Unmask IRQ12 (Mouse) -> 0xEF
        outb(0xA1, 0xEF);
    }
}

pub fn send_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(0xA0, 0x20);
        }
        outb(0x20, 0x20);
    }
}