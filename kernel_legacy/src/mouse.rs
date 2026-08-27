// src/mouse.rs

use crate::cpu::{inb, outb};

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    cycle: u8,
    packet: [u8; 3],
}

pub static mut MOUSE: MouseState = MouseState {
    x: 400,
    y: 300,
    left_button: false,
    right_button: false,
    middle_button: false,
    cycle: 0,
    packet: [0; 3],
};

// Trạng thái nút bấm ở lần "take_*_click()" gần nhất — dùng để phát hiện
// edge (vừa nhấn xuống), phục vụ app desktop (click-to-open icon, nút bấm UI...)
// mà không bị trigger lặp lại liên tục khi giữ chuột.
static mut PREV_LEFT_FOR_CLICK: bool = false;
static mut PREV_RIGHT_FOR_CLICK: bool = false;

fn mouse_wait_write() {
    let mut timeout = 100_000;
    while timeout > 0 {
        if (unsafe { inb(0x64) } & 2) == 0 { return; }
        timeout -= 1;
    }
}

fn mouse_wait_read() {
    let mut timeout = 100_000;
    while timeout > 0 {
        if (unsafe { inb(0x64) } & 1) != 0 { return; }
        timeout -= 1;
    }
}

fn mouse_write_cmd(cmd: u8) {
    mouse_wait_write();
    unsafe { outb(0x64, 0xD4); }
    mouse_wait_write();
    unsafe { outb(0x60, cmd); }
}

fn mouse_read_data() -> u8 {
    mouse_wait_read();
    unsafe { inb(0x60) }
}

pub fn init_mouse() {
    mouse_wait_write();
    unsafe { outb(0x64, 0xA8); }

    mouse_wait_write();
    unsafe { outb(0x64, 0x20); }
    mouse_wait_read();
    let status = (unsafe { inb(0x60) }) | 2;

    mouse_wait_write();
    unsafe { outb(0x64, 0x60); }
    mouse_wait_write();
    unsafe { outb(0x60, status); }

    mouse_write_cmd(0xF6);
    let _ = mouse_read_data();

    mouse_write_cmd(0xF4);
    let _ = mouse_read_data();
}

pub fn handle_mouse_byte(data: u8, screen_w: u32, screen_h: u32) -> bool {
    unsafe {
        match MOUSE.cycle {
            0 => {
                if (data & 0x08) != 0 {
                    MOUSE.packet[0] = data;
                    MOUSE.cycle = 1;
                }
            }
            1 => {
                MOUSE.packet[1] = data;
                MOUSE.cycle = 2;
            }
            2 => {
                MOUSE.packet[2] = data;
                MOUSE.cycle = 0;

                let flags = MOUSE.packet[0];
                let mut dx = MOUSE.packet[1] as i32;
                let mut dy = MOUSE.packet[2] as i32;

                if (flags & 0x10) != 0 { dx -= 256; }
                if (flags & 0x20) != 0 { dy -= 256; }

                let left = (flags & 0x01) != 0;
                let right = (flags & 0x02) != 0;
                let middle = (flags & 0x04) != 0;

                if dx != 0 || dy != 0 || left != MOUSE.left_button || right != MOUSE.right_button {
                    MOUSE.x += dx;
                    MOUSE.y -= dy;

                    if MOUSE.x < 0 { MOUSE.x = 0; }
                    if MOUSE.x >= screen_w as i32 { MOUSE.x = screen_w as i32 - 1; }
                    if MOUSE.y < 0 { MOUSE.y = 0; }
                    if MOUSE.y >= screen_h as i32 { MOUSE.y = screen_h as i32 - 1; }

                    MOUSE.left_button = left;
                    MOUSE.right_button = right;
                    MOUSE.middle_button = middle;
                    return true; // Trả về true nếu có sự thay đổi
                }
            }
            _ => MOUSE.cycle = 0,
        }
    }
    false
}

// Trả về tuple (Mã phím, Có thay đổi chuột hay không)
pub fn poll_input(screen_w: u32, screen_h: u32) -> (Option<u8>, bool) {
    let mut key_code = None;
    let mut mouse_changed = false;

    while (unsafe { inb(0x64) } & 1) != 0 {
        let status = unsafe { inb(0x64) };
        let data = unsafe { inb(0x60) };

        if (status & 0x20) != 0 {
            if handle_mouse_byte(data, screen_w, screen_h) {
                mouse_changed = true;
            }
        } else {
            key_code = Some(data);
        }
    }

    (key_code, mouse_changed)
}

pub fn get_mouse_state() -> (i32, i32, bool, bool) {
    unsafe {
        (MOUSE.x, MOUSE.y, MOUSE.left_button, MOUSE.right_button)
    }
}

/// Edge-detect: trả về true đúng 1 lần khi nút trái vừa chuyển từ nhả -> nhấn
/// kể từ lần gọi trước. Dùng cho app desktop (click icon, click nút UI...)
/// thay vì phải tự so sánh trạng thái qua nhiều frame.
pub fn take_left_click() -> bool {
    unsafe {
        let now = MOUSE.left_button;
        let clicked = now && !PREV_LEFT_FOR_CLICK;
        PREV_LEFT_FOR_CLICK = now;
        clicked
    }
}

/// Tương tự take_left_click() nhưng cho nút phải (context menu, v.v.)
pub fn take_right_click() -> bool {
    unsafe {
        let now = MOUSE.right_button;
        let clicked = now && !PREV_RIGHT_FOR_CLICK;
        PREV_RIGHT_FOR_CLICK = now;
        clicked
    }
}