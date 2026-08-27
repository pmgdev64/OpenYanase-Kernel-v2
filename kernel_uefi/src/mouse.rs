// src/mouse.rs
use core::arch::global_asm;
use crate::cpu::{inb, outb};
use crate::graphics::gfx;

static mut MOUSE_CYCLE: u8 = 0;
static mut MOUSE_BYTE: [u8; 3] = [0; 3];

global_asm!(
    r#"
    .global mouse_interrupt_stub
    .extern handle_mouse_interrupt

    mouse_interrupt_stub:
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

        call handle_mouse_interrupt

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
    fn mouse_interrupt_stub();
}

unsafe fn mouse_wait(type_val: u8) {
    let mut timeout = 100_000;
    while timeout > 0 {
        let status = inb(0x64);
        if type_val == 0 && (status & 1) != 0 { return; }
        if type_val == 1 && (status & 2) == 0 { return; }
        timeout -= 1;
    }
}

unsafe fn mouse_write(data: u8) {
    mouse_wait(1);
    outb(0x64, 0xD4);
    mouse_wait(1);
    outb(0x60, data);
}

unsafe fn mouse_read() -> u8 {
    mouse_wait(0);
    inb(0x60)
}

pub fn init_mouse() {
    unsafe {
        mouse_wait(1);
        outb(0x64, 0xA8); // Enable auxiliary device (mouse clock)

        mouse_wait(1);
        outb(0x64, 0x20); // Read Command Byte
        mouse_wait(0);
        let old_status = inb(0x60);

        // Giữ nguyên bit keyboard (bit0 IRQ1, bit4 clock1),
        // thêm bit1 (IRQ12 enable) và clear bit5 (enable clock2)
        let status = (old_status | 0x02) & !0x20;

        mouse_wait(1);
        outb(0x64, 0x60); // Write Command Byte
        mouse_wait(1);
        outb(0x60, status);

        mouse_write(0xFF); // Reset
        let _ack = mouse_read();
        let _bat = mouse_read();
        let _id  = mouse_read();

        mouse_write(0xF6); // Set defaults
        let _ack2 = mouse_read();

        mouse_write(0xF4); // Enable data reporting
        let _ack3 = mouse_read();
    }

    crate::idt::set_gate(44, mouse_interrupt_stub as usize as u64);

    // Unmask IRQ12 ở slave PIC (bit4 = IRQ 8+4=12)
    unsafe {
        let mask = inb(0xA1);
        outb(0xA1, mask & !(1 << 4));
    }
}

#[no_mangle]
pub extern "C" fn handle_mouse_interrupt() {
    unsafe {
        let status = inb(0x64);
        if (status & 0x01) != 0 && (status & 0x20) != 0 {
            let data = inb(0x60);
            match MOUSE_CYCLE {
                0 => {
                    // Bit3 phải =1, bit6/7 (overflow) phải =0 để hợp lệ; nếu không, drop và tự resync
                    if (data & 0x08) != 0 && (data & 0xC0) == 0 {
                        MOUSE_BYTE[0] = data;
                        MOUSE_CYCLE = 1;
                    }
                }
                1 => {
                    MOUSE_BYTE[1] = data;
                    MOUSE_CYCLE = 2;
                }
                2 => {
                    MOUSE_BYTE[2] = data;
                    MOUSE_CYCLE = 0;

                    let b0 = MOUSE_BYTE[0];
                    let b1 = MOUSE_BYTE[1];
                    let b2 = MOUSE_BYTE[2];

                    let dx = if (b0 & 0x10) != 0 { b1 as i32 - 256 } else { b1 as i32 };
                    let dy = if (b0 & 0x20) != 0 { b2 as i32 - 256 } else { b2 as i32 };
                    let left_click = (b0 & 0x01) != 0;

                    gfx::update_mouse_state(dx, -dy, left_click);
                }
                _ => MOUSE_CYCLE = 0,
            }
        }
        outb(0xA0, 0x20);
        outb(0x20, 0x20);
    }
}