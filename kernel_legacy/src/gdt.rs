// src/gdt.rs
use core::arch::asm;

#[repr(C, packed)]
struct GdtPtr {
    limit: u16,
    base: u32,
}

static mut GDT: [u64; 3] = [0; 3];

pub fn init() {
    unsafe {
        GDT[0] = 0x0000000000000000; // Null Segment
        GDT[1] = 0x00CF9A000000FFFF; // Kernel Code Segment (0x08)
        GDT[2] = 0x00CF92000000FFFF; // Kernel Data Segment (0x10)

        let gdt_ptr = GdtPtr {
            limit: (core::mem::size_of_val(&GDT) - 1) as u16,
            base: GDT.as_ptr() as u32,
        };

        asm!(
            "lgdt [{ptr}]",
            "push 0x08",
            "lea {tmp}, [2f]",
            "push {tmp}",
            "retf",
            "2:",
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            ptr = in(reg) &gdt_ptr,
            tmp = out(reg) _
        );
    }
}