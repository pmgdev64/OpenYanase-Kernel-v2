// src/keyboard.rs
use core::arch::global_asm;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use crate::cpu::{inb, outb};

const KEY_BUFFER_SIZE: usize = 256;
static mut KEY_BUFFER: [char; KEY_BUFFER_SIZE] = ['\0'; KEY_BUFFER_SIZE];
static BUFFER_HEAD: AtomicUsize = AtomicUsize::new(0);
static BUFFER_TAIL: AtomicUsize = AtomicUsize::new(0);

static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
static ALT_PRESSED: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static KEYBOARD_LOCK: AtomicBool = AtomicBool::new(false);
static IS_E0: AtomicBool = AtomicBool::new(false);

global_asm!(
    r#"
    .global keyboard_interrupt_stub
    .extern handle_keyboard_interrupt

    keyboard_interrupt_stub:
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

        call handle_keyboard_interrupt

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
    fn keyboard_interrupt_stub();
}

// Scancode map Set 1
static SCANCODE_MAP: [char; 128] = [
    '\0', '\x1B', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\x08', '\t',
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n', '\0', 'a', 's',
    'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`', '\0', '\\', 'z', 'x', 'c', 'v',
    'b', 'n', 'm', ',', '.', '/', '\0', '*', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

static SCANCODE_SHIFT_MAP: [char; 128] = [
    '\0', '\x1B', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\x08', '\t',
    'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\n', '\0', 'A', 'S',
    'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"', '~', '\0', '|', 'Z', 'X', 'C', 'V',
    'B', 'N', 'M', '<', '>', '?', '\0', '*', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
    '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

// Special key scancodes
const SCANCODE_UP: u8 = 0x48;
const SCANCODE_DOWN: u8 = 0x50;
const SCANCODE_LEFT: u8 = 0x4B;
const SCANCODE_RIGHT: u8 = 0x4D;
const SCANCODE_HOME: u8 = 0x47;
const SCANCODE_END: u8 = 0x4F;
const SCANCODE_PAGEUP: u8 = 0x49;
const SCANCODE_PAGEDOWN: u8 = 0x51;
const SCANCODE_DELETE: u8 = 0x53;
const SCANCODE_INSERT: u8 = 0x52;
const SCANCODE_F1: u8 = 0x3B;
const SCANCODE_F2: u8 = 0x3C;

// Mã đại diện riêng biệt cho các phím điều hướng (Private Use Area)
pub const KEY_UP: char = '\u{E000}';
pub const KEY_DOWN: char = '\u{E001}';
pub const KEY_LEFT: char = '\u{E002}';
pub const KEY_RIGHT: char = '\u{E003}';
pub const KEY_HOME: char = '\u{E004}';
pub const KEY_END: char = '\u{E005}';
pub const KEY_PAGEUP: char = '\u{E006}';
pub const KEY_PAGEDOWN: char = '\u{E007}';
pub const KEY_INSERT: char = '\u{E008}';
pub const KEY_DELETE: char = '\u{E009}';
pub const KEY_F1: char = '\u{E00A}';
pub const KEY_F2: char = '\u{E00B}';

#[no_mangle]
pub extern "C" fn handle_keyboard_interrupt() {
    unsafe {
        let scancode = inb(0x60);

        // Phát hiện prefix 0xE0 cho phím mở rộng
        if scancode == 0xE0 {
            IS_E0.store(true, Ordering::Relaxed);
            outb(0x20, 0x20);
            return;
        }

        let is_e0 = IS_E0.swap(false, Ordering::Relaxed);

        // Modifier keys
        match scancode {
            0x2A | 0x36 => SHIFT_PRESSED.store(true, Ordering::Relaxed),  // Left/Right Shift
            0xAA | 0xB6 => SHIFT_PRESSED.store(false, Ordering::Relaxed),
            0x1D => {
                if !is_e0 { CTRL_PRESSED.store(true, Ordering::Relaxed); }
            }
            0x9D => {
                if !is_e0 { CTRL_PRESSED.store(false, Ordering::Relaxed); }
            }
            0x38 => {
                if !is_e0 { ALT_PRESSED.store(true, Ordering::Relaxed); }
            }
            0xB8 => {
                if !is_e0 { ALT_PRESSED.store(false, Ordering::Relaxed); }
            }
            0x3A => { // Caps Lock Press
                let caps = CAPS_LOCK.load(Ordering::Relaxed);
                CAPS_LOCK.store(!caps, Ordering::Relaxed);
            }
            _ => {}
        }

        // Key press (make code - scancode < 0x80)
        if scancode < 0x80 {
            let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
            let caps = CAPS_LOCK.load(Ordering::Relaxed);

            let ch = if is_e0 {
                // Xử lý các phím mũi tên/mở rộng khi có tiền tố E0
                match scancode {
                    SCANCODE_UP => Some(KEY_UP),
                    SCANCODE_DOWN => Some(KEY_DOWN),
                    SCANCODE_LEFT => Some(KEY_LEFT),
                    SCANCODE_RIGHT => Some(KEY_RIGHT),
                    SCANCODE_HOME => Some(KEY_HOME),
                    SCANCODE_END => Some(KEY_END),
                    SCANCODE_PAGEUP => Some(KEY_PAGEUP),
                    SCANCODE_PAGEDOWN => Some(KEY_PAGEDOWN),
                    SCANCODE_DELETE => Some(KEY_DELETE),
                    SCANCODE_INSERT => Some(KEY_INSERT),
                    _ => None,
                }
            } else {
                // Xử lý các phím thường / F1, F2
                match scancode {
                    SCANCODE_F1 => Some(KEY_F1),
                    SCANCODE_F2 => Some(KEY_F2),
                    _ => {
                        let idx = scancode as usize;
                        if idx < SCANCODE_MAP.len() {
                            let base_ch = SCANCODE_MAP[idx];
                            let shift_ch = SCANCODE_SHIFT_MAP[idx];

                            let ch = if base_ch.is_ascii_alphabetic() {
                                // Với chữ cái: Caps Lock đảo trạng thái của Shift (XOR)
                                if shift ^ caps { shift_ch } else { base_ch }
                            } else {
                                // Với ký tự đặc biệt/số: Caps Lock không ảnh hưởng
                                if shift { shift_ch } else { base_ch }
                            };

                            if ch != '\0' { Some(ch) } else { None }
                        } else {
                            None
                        }
                    }
                }
            };

            if let Some(c) = ch {
                push_char(c);
            }
        }

        // Send EOI to PIC
        outb(0x20, 0x20);
    }
}

unsafe fn ps2_wait_write() {
    let mut timeout = 100_000;
    while (inb(0x64) & 2) != 0 && timeout > 0 {
        timeout -= 1;
    }
}

unsafe fn ps2_wait_read() {
    let mut timeout = 100_000;
    while (inb(0x64) & 1) == 0 && timeout > 0 {
        timeout -= 1;
    }
}

pub fn init_keyboard() {
    unsafe {
        while KEYBOARD_LOCK.compare_exchange(false, true, 
            Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }

        // Flush keyboard buffer
        while (inb(0x64) & 1) != 0 {
            let _ = inb(0x60);
        }

        // Enable PS/2 port
        ps2_wait_write();
        outb(0x64, 0xAE);

        // Configure controller command byte
        ps2_wait_write();
        outb(0x64, 0x20); // Read Command Byte
        ps2_wait_read();
        let mut ccb = inb(0x60);

        ccb |= 0x01;  // Bit 0: Enable IRQ1
        ccb &= !0x10; // Bit 4: Clear to enable First PS/2 Clock
        ccb |= 0x40;  // Bit 6: Enable Translation (Hardware Translate Set 2 -> Set 1)

        ps2_wait_write();
        outb(0x64, 0x60); // Write Command Byte
        ps2_wait_write();
        outb(0x60, ccb);

        KEYBOARD_LOCK.store(false, Ordering::Release);
    }

    // Set interrupt handler for IRQ1 (vector 33)
    crate::idt::set_gate(33, keyboard_interrupt_stub as usize as u64);
}

pub fn push_char(ch: char) {
    while KEYBOARD_LOCK.compare_exchange(false, true, 
        Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let head = BUFFER_HEAD.load(Ordering::Relaxed);
    let next_head = (head + 1) % KEY_BUFFER_SIZE;
    let tail = BUFFER_TAIL.load(Ordering::Relaxed);

    if next_head != tail {
        unsafe {
            KEY_BUFFER[head] = ch;
        }
        BUFFER_HEAD.store(next_head, Ordering::Relaxed);
    }

    KEYBOARD_LOCK.store(false, Ordering::Release);
}

pub fn pop_char() -> Option<char> {
    while KEYBOARD_LOCK.compare_exchange(false, true, 
        Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }

    let tail = BUFFER_TAIL.load(Ordering::Relaxed);
    let head = BUFFER_HEAD.load(Ordering::Relaxed);

    let result = if tail == head {
        None
    } else {
        let ch = unsafe { KEY_BUFFER[tail] };
        BUFFER_TAIL.store((tail + 1) % KEY_BUFFER_SIZE, Ordering::Relaxed);
        Some(ch)
    };

    KEYBOARD_LOCK.store(false, Ordering::Release);
    result
}

pub fn has_key() -> bool {
    let tail = BUFFER_TAIL.load(Ordering::Relaxed);
    let head = BUFFER_HEAD.load(Ordering::Relaxed);
    tail != head
}

pub fn flush_keys() {
    while let Some(_) = pop_char() {}
}