// src/syscall.rs
use crate::idt;
use crate::serial;
use core::arch::global_asm;

global_asm!(
    r#"
    .global syscall_interrupt_stub
    .extern handle_syscall

    syscall_interrupt_stub:
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

        mov rdi, [rsp + 8*8]
        mov rsi, [rsp + 8*4]
        mov rdx, [rsp + 8*5]
        mov rcx, [rsp + 8*6]
        mov r8,  [rsp + 8*7]
        mov r9,  [rsp + 8*3]

        sub rsp, 8
        call handle_syscall
        add rsp, 8

        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rdx
        pop rcx
        add rsp, 8
        iretq
    "#
);

extern "C" {
    fn syscall_interrupt_stub();
}

pub fn init_syscall() {
    idt::set_gate_dpl(0x80, syscall_interrupt_stub as usize as u64, 0xEE);
    serial::serial_write_str("INFO: Syscall (int 0x80) initialized with DPL=3\r\n");
}

const MAX_PRINT_LEN: u64 = 4096;
const SANDBOX_MAX_TICK_SLEEP: u64 = 60000;
const MAX_DRAW_SIZE: i64 = 4096;
const MAX_CSTR_SCAN: u64 = 2048;
const MAX_EXEC_NAME: u64 = 120;

const PTR_LOW: u64 = 0x1000;
const PTR_HIGH: u64 = 0x0000_7FFF_FFFF_FFFF;

#[inline(always)]
fn ptr_ok(ptr: u64) -> bool {
    ptr >= PTR_LOW && ptr < PTR_HIGH
}

/// App truyền 0x00RRGGBB → ép alpha=0xFF. Không phụ thuộc format framebuffer.
#[inline(always)]
fn opaque(color: i64) -> u32 {
    ((color as u64 & 0x00FF_FFFF) | 0xFF00_0000) as u32
}

unsafe fn scan_cstr(ptr: u64) -> Option<u64> {
    if !ptr_ok(ptr) { return None; }
    let p = ptr as *const u8;
    let mut n = 0u64;
    while n < MAX_CSTR_SCAN {
        let b = core::ptr::read_volatile(p.add(n as usize));
        if b == 0 { return Some(n); }
        n += 1;
    }
    None
}

#[no_mangle]
pub extern "C" fn handle_syscall(
    id: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64,
) -> i64 {
    match id {
        1 | 21 => sys_print(a1, a2),

        // 2  = fill_rect (mặc định cho hầu hết app)
        // 22 = fill_rect (alias)
        // 24 = draw_rect OUTLINE — dùng cho gfx.draw_rect trong app
        2 | 22 => sys_fill_rect(a1 as i64, a2 as i64, a3 as i64, a4 as i64, a5 as i64),
        24     => sys_draw_rect_outline(a1 as i64, a2 as i64, a3 as i64, a4 as i64, a5 as i64),

        3 | 11 | 23 | 29 => sys_get_tick(),
        4 | 12 | 30 => sys_sleep(a1),
        5 | 13 | 25 => sys_exit(a1 as i32),
        6 | 14 => sys_get_pid(),
        7 | 15 => sys_get_time(),
        8 | 16 => sys_beep(a1),
        9 | 17 => sys_get_width(),
        10 | 18 => sys_get_height(),

        19 | 20 => sys_gfx_clear(a1 as u32, a2 as u32),
        26 => sys_gfx_draw_text(a1 as i64, a2 as i64, a3, a4 as i64),
        27 => sys_gfx_flush(),
        28 => sys_init_gfx(),

        40 => sys_get_mouse_x(),
        41 => sys_get_mouse_y(),
        42 => sys_is_mouse_pressed(),
        43 => sys_draw_cursor(),

        50 => sys_key_read(),
        51 => sys_console_ctl(a1),
        52 => sys_exec(a1, a2),

        53 => sys_console_clear(),
        54 => sys_write_char(a1),
        55 => sys_println(a1),
        56 => sys_read_key_blocking(),
        57 => sys_console_backspace(),

        107 => sys_get_tick(),

        113 => { crate::acpi::shutdown(); }
        114 => { crate::acpi::reboot(); }

        120 => sys_ipc_create_port(a1 as u32, a2),
        121 => sys_ipc_send(a1, a2, a3, a4),
        122 => sys_ipc_recv(a1, a2),
        125 => sys_shm_create(a1, a2),
        126 => sys_shm_map(a1, a2, a3),

        _ => 0,
    }
}

// ============================================================
// PRINT
// ============================================================
fn sys_print(ptr: u64, len: u64) -> i64 {
    if !ptr_ok(ptr) { return -1; }
    let actual = if len == 0 {
        match unsafe { scan_cstr(ptr) } { Some(n) => n, None => return -1 }
    } else {
        if len > MAX_PRINT_LEN { return -1; }
        if ptr.checked_add(len).is_none() { return -1; }
        len
    };
    if actual == 0 || actual > MAX_PRINT_LEN { return -1; }

    unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, actual as usize);
        if let Ok(s) = core::str::from_utf8(slice) {
            use core::fmt::Write;
            let _ = crate::console::CONSOLE.write_str(s);
            return 0;
        }
    }
    -1
}

fn sys_println(ptr: u64) -> i64 {
    let r = sys_print(ptr, 0);
    if r == 0 { unsafe { crate::console::CONSOLE.write_char('\n'); } }
    r
}

fn sys_write_char(c: u64) -> i64 {
    if c > 0x10FFFF { return -1; }
    match char::from_u32(c as u32) {
        Some(ch) => { unsafe { crate::console::CONSOLE.write_char(ch); } 0 }
        None => -1,
    }
}

fn sys_console_clear() -> i64 { unsafe { crate::console::CONSOLE.clear(); } 0 }
fn sys_console_backspace() -> i64 { unsafe { crate::console::CONSOLE.backspace(); } 0 }

// ============================================================
// GRAPHICS
// ============================================================

/// Fill hình chữ nhật đặc. Đây là syscall chính cho gfx.fill_rect trong app.
fn sys_fill_rect(x: i64, y: i64, w: i64, h: i64, color: i64) -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Graphics { return 0; }
    let x = x.clamp(0, MAX_DRAW_SIZE) as u32;
    let y = y.clamp(0, MAX_DRAW_SIZE) as u32;
    let w = w.clamp(0, MAX_DRAW_SIZE) as u32;
    let h = h.clamp(0, MAX_DRAW_SIZE) as u32;
    let color = opaque(color);

    unsafe {
        if let Some(surface) = crate::graphics::gfx::get_back_surface() {
            surface.fill_rect(x, y, w, h, crate::gop::Color(color));
            crate::graphics::gfx::mark_dirty();
        }
    }
    0
}

/// Chỉ vẽ 4 cạnh (outline). Dùng cho gfx.draw_rect trong app.
fn sys_draw_rect_outline(x: i64, y: i64, w: i64, h: i64, color: i64) -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Graphics { return 0; }
    let x = x.clamp(0, MAX_DRAW_SIZE) as u32;
    let y = y.clamp(0, MAX_DRAW_SIZE) as u32;
    let w = w.clamp(0, MAX_DRAW_SIZE) as u32;
    let h = h.clamp(0, MAX_DRAW_SIZE) as u32;
    if w == 0 || h == 0 { return 0; }
    let color = opaque(color);

    unsafe {
        if let Some(surface) = crate::graphics::gfx::get_back_surface() {
            surface.draw_rect(x, y, w, h, crate::gop::Color(color));
            crate::graphics::gfx::mark_dirty();
        }
    }
    0
}

fn sys_get_tick() -> i64 { crate::timer::get_ticks() as i64 }

fn sys_sleep(ms: u64) -> i64 {
    let ms = ms.min(SANDBOX_MAX_TICK_SLEEP);
    let start = crate::timer::get_ticks();
    while crate::timer::get_ticks().wrapping_sub(start) < ms {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)); }
        unsafe {
            if crate::console::get_display_mode() == crate::console::DisplayMode::Graphics {
                crate::graphics::gfx::update_cursor_only();
            }
        }
        if crate::timer::get_ticks().wrapping_sub(start) > ms + 5000 { break; }
    }
    0
}

fn sys_exit(code: i32) -> i64 { crate::process::request_exit(code); 0 }

fn sys_get_pid() -> i64 {
    unsafe { if let Some(pid) = crate::process::CURRENT_PID { pid as i64 } else { -1 } }
}

fn sys_get_time() -> i64 { (crate::timer::get_ticks() / 1000) as i64 }

fn sys_beep(freq: u64) -> i64 {
    if freq == 0 || freq > 20000 { return -1; }
    unsafe {
        let divisor = 1193180 / freq as u32;
        let status = crate::cpu::inb(0x61);
        crate::cpu::outb(0x61, status | 0x03);
        crate::cpu::outb(0x43, 0xB6);
        crate::cpu::outb(0x42, (divisor & 0xFF) as u8);
        crate::cpu::outb(0x42, ((divisor >> 8) & 0xFF) as u8);
        crate::timer::sleep(200);
        crate::cpu::outb(0x61, status & !0x03);
    }
    0
}

fn sys_get_width() -> i64 {
    unsafe { let w = crate::console::CONSOLE.fb_width; if w != 0 { w as i64 } else { 800 } }
}

fn sys_get_height() -> i64 {
    unsafe { let h = crate::console::CONSOLE.fb_height; if h != 0 { h as i64 } else { 600 } }
}

fn sys_init_gfx() -> i64 {
    unsafe {
        if crate::console::get_display_mode() == crate::console::DisplayMode::Console {
            crate::console::CONSOLE.hide_cursor();
            crate::console::save_console_state();
        }
        crate::graphics::gfx::set_demo_mode(false);
        crate::console::set_display_mode(crate::console::DisplayMode::Graphics);
        if let Some(surface) = crate::graphics::gfx::get_back_surface() {
            surface.fill_rect(0, 0, surface.width, surface.height,
                              crate::gop::Color(0xFF00_0000));
        }
        crate::graphics::gfx::mark_dirty();
    }
    0
}

fn sys_gfx_clear(color: u32, _mode: u32) -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Graphics { return 0; }
    let color = (color & 0x00FF_FFFF) | 0xFF00_0000;
    unsafe {
        crate::graphics::gfx::set_demo_mode(false);
        if let Some(surface) = crate::graphics::gfx::get_back_surface() {
            surface.fill_rect(0, 0, surface.width, surface.height, crate::gop::Color(color));
        }
        crate::graphics::gfx::mark_dirty();
    }
    0
}

fn sys_gfx_draw_text(x: i64, y: i64, str_ptr: u64, color: i64) -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Graphics { return 0; }
    if x < 0 || y < 0 || x > 8192 || y > 8192 { return 0; }
    if !ptr_ok(str_ptr) { return 0; }

    let color = opaque(color);

    unsafe {
        let ptr = str_ptr as *const u8;
        let mut len = 0;
        while len < 256 {
            let b = *ptr.add(len);
            if b == 0 { break; }
            if b < 0x09 || (b > 0x0D && b < 0x20) || b > 0x7E { break; }
            len += 1;
        }
        if len == 0 { return 0; }

        let slice = core::slice::from_raw_parts(ptr, len);
        if let Ok(text) = core::str::from_utf8(slice) {
            if let Some(surface) = crate::graphics::gfx::get_back_surface() {
                if let Some(font) = crate::graphics::gfx::get_font() {
                    font.draw_string(
                        surface, text,
                        x as u32, y as u32,
                        crate::gop::Color(color),
                    );
                    crate::graphics::gfx::mark_dirty();
                }
            }
        }
    }
    0
}

fn sys_gfx_flush() -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Graphics { return 0; }
    unsafe { crate::graphics::gfx::update_frame(); }
    0
}

fn sys_get_mouse_x() -> i64 { unsafe { crate::graphics::gfx::MOUSE_X as i64 } }
fn sys_get_mouse_y() -> i64 { unsafe { crate::graphics::gfx::MOUSE_Y as i64 } }
fn sys_is_mouse_pressed() -> i64 {
    unsafe { if crate::graphics::gfx::MOUSE_LEFT_DOWN { 1 } else { 0 } }
}
fn sys_draw_cursor() -> i64 { 0 }

// ============================================================
// I/O
// ============================================================
fn sys_key_read() -> i64 {
    match crate::keyboard::pop_char() { Some(c) => c as u32 as i64, None => -1 }
}

fn sys_read_key_blocking() -> i64 {
    loop {
        if let Some(c) = crate::keyboard::pop_char() { return c as u32 as i64; }
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
            core::arch::asm!("hlt", options(nomem, nostack));
            core::arch::asm!("cli", options(nomem, nostack));
        }
    }
}

fn sys_console_ctl(op: u64) -> i64 {
    if crate::console::get_display_mode() != crate::console::DisplayMode::Console { return -1; }
    unsafe {
        match op {
            1 => { crate::console::CONSOLE.backspace(); 0 }
            2 => { crate::console::CONSOLE.clear(); 0 }
            3 => { crate::console::CONSOLE.force_show_cursor(); 0 }
            4 => { crate::console::CONSOLE.hide_cursor(); 0 }
            _ => -1,
        }
    }
}

const SHELL_PACKAGE: &str = "globalsys/shell.abp";
const MAX_EXEC_DEPTH: u32 = 3;
static mut EXEC_DEPTH: u32 = 0;

fn sys_exec(ptr: u64, len: u64) -> i64 {
    if !crate::process::current_is(SHELL_PACKAGE) { return -4; }
    if !ptr_ok(ptr) { return -1; }

    let actual = if len == 0 {
        match unsafe { scan_cstr(ptr) } { Some(n) => n, None => return -1 }
    } else {
        if len > MAX_EXEC_NAME { return -1; }
        len
    };
    if actual == 0 || actual > MAX_EXEC_NAME { return -1; }

    let mut name_buf = [0u8; 128];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr as *const u8, name_buf.as_mut_ptr(), actual as usize);
    }
    let mut name_len = actual as usize;

    let has_ext = name_len >= 4
        && name_buf[name_len - 4] == b'.'
        && name_buf[name_len - 3] == b'a'
        && name_buf[name_len - 2] == b'b'
        && name_buf[name_len - 1] == b'p';

    if !has_ext {
        if name_len + 4 > 127 { return -1; }
        name_buf[name_len] = b'.'; name_len += 1;
        name_buf[name_len] = b'a'; name_len += 1;
        name_buf[name_len] = b'b'; name_len += 1;
        name_buf[name_len] = b'p'; name_len += 1;
    }

    let name = match core::str::from_utf8(&name_buf[..name_len]) {
        Ok(s) => s, Err(_) => return -1,
    };

    unsafe {
        if EXEC_DEPTH >= MAX_EXEC_DEPTH { return -2; }
        let tar = crate::initrd::INITRD_ADDR;
        if tar.is_null() { return -1; }
        EXEC_DEPTH += 1;
        let r = crate::abp::run_abp_file(tar, name);
        EXEC_DEPTH -= 1;
        if r.is_ok() { 0 } else { -3 }
    }
}

fn sys_ipc_create_port(_p: u32, _s: u64) -> i64 { -1 }
fn sys_ipc_send(_a: u64, _b: u64, _c: u64, _d: u64) -> i64 { -1 }
fn sys_ipc_recv(_a: u64, _b: u64) -> i64 { -1 }
fn sys_shm_create(_s: u64, _f: u64) -> i64 { -1 }
fn sys_shm_map(_i: u64, _a: u64, _f: u64) -> i64 { -1 }