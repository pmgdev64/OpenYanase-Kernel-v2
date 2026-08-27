// src/idt.rs

use crate::pic;
use crate::keyboard;
use crate::mouse;
use crate::cpu::inb;
use core::arch::global_asm;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry { base_low: u16, sel: u16, always0: u8, flags: u8, base_high: u16 }

#[repr(C, packed)]
struct IdtPtr { limit: u16, base: u32 }

static mut IDT: [IdtEntry; 256] = [IdtEntry { base_low: 0, sel: 0, always0: 0, flags: 0, base_high: 0 }; 256];
static mut IDT_PTR: IdtPtr = IdtPtr { limit: 0, base: 0 };

extern "C" {
    fn isr_timer_wrapper();
    fn isr_keyboard_wrapper();
    fn isr_mouse_wrapper();
    fn isr_double_fault_wrapper();
    fn isr_general_protection_wrapper();
    fn isr_page_fault_wrapper();
}

global_asm!(
    r#"
    .global isr_timer_wrapper
    isr_timer_wrapper:
        pushad
        call irq0_timer_handler
        popad
        iretd

    .global isr_keyboard_wrapper
    isr_keyboard_wrapper:
        pushad
        call irq1_keyboard_handler
        popad
        iretd
        
    .global isr_mouse_wrapper
    isr_mouse_wrapper:
        pushad
        call irq12_mouse_handler
        popad
        iretd

    .global isr_double_fault_wrapper
    isr_double_fault_wrapper:
        pushad
        call double_fault_handler
        popad
        iretd

    .global isr_general_protection_wrapper
    isr_general_protection_wrapper:
        pushad
        call general_protection_handler
        popad
        iretd

    .global isr_page_fault_wrapper
    isr_page_fault_wrapper:
        pushad
        call page_fault_handler
        popad
        iretd
    "#
);

#[no_mangle]
pub extern "C" fn irq0_timer_handler() {
    crate::timer::on_tick();
    pic::send_eoi(0);
}

#[no_mangle]
pub extern "C" fn irq1_keyboard_handler() {
    unsafe {
        let scancode = inb(0x60);
        keyboard::handle_scancode(scancode);
    }
    pic::send_eoi(1);
}

#[no_mangle]
pub extern "C" fn irq12_mouse_handler() {
    unsafe {
        let status = inb(0x64);
        if (status & 0x20) != 0 {
            let data = inb(0x60);
            // Lấy đúng resolution thật từ framebuffer đang active thay vì
            // hard-code 800x600 — hard-code sai khiến toạ độ chuột bị clamp
            // lệch trên mọi resolution khác 800x600, cursor vẽ ra ngoài
            // framebuffer nên không bao giờ hiển thị được.
            let (w, h) = if !crate::console::CONSOLE.fb.is_null() {
                let fb = &*crate::console::CONSOLE.fb;
                (fb.framebuffer_width, fb.framebuffer_height)
            } else {
                (800, 600)
            };
            mouse::handle_mouse_byte(data, w, h);
        }
    }
    pic::send_eoi(12);
}

// ==========================================
// EXCEPTION HANDLERS WITH BUGCHECK
// ==========================================

#[no_mangle]
pub extern "C" fn double_fault_handler() {
    crate::bugcheck::bugcheck(crate::bugcheck::BugCheckCode::DoubleFault);
}

#[no_mangle]
pub extern "C" fn general_protection_handler() {
    crate::bugcheck::bugcheck(crate::bugcheck::BugCheckCode::PageTableFailure);
}

#[no_mangle]
pub extern "C" fn page_fault_handler() {
    crate::bugcheck::bugcheck(crate::bugcheck::BugCheckCode::PageFaultFatal);
}

fn set_gate(num: usize, base: u32, sel: u16, flags: u8) {
    unsafe {
        IDT[num].base_low = (base & 0xFFFF) as u16;
        IDT[num].base_high = ((base >> 16) & 0xFFFF) as u16;
        IDT[num].sel = sel;
        IDT[num].always0 = 0;
        IDT[num].flags = flags;
    }
}

pub fn init() {
    unsafe {
        IDT_PTR.limit = (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16;
        IDT_PTR.base = IDT.as_ptr() as u32;

        set_gate(32, isr_timer_wrapper as u32, 0x08, 0x8E);
        set_gate(33, isr_keyboard_wrapper as u32, 0x08, 0x8E);
        set_gate(44, isr_mouse_wrapper as u32, 0x08, 0x8E);

        set_gate(8, isr_double_fault_wrapper as u32, 0x08, 0x8E);
        set_gate(13, isr_general_protection_wrapper as u32, 0x08, 0x8E);
        set_gate(14, isr_page_fault_wrapper as u32, 0x08, 0x8E);

        core::arch::asm!("lidt [{}]", in(reg) &IDT_PTR, options(readonly, nostack, preserves_flags));
    }
}