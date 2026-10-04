// src/graphics/gfx.rs
use crate::gop::Color;
use crate::console::{CONSOLE, DisplayMode};
use crate::graphics::surface::Surface;
use crate::graphics::font::Font;
use crate::graphics::window::Window;
use core::sync::atomic::{AtomicBool, Ordering};

static mut GFX_ACTIVE: bool = false;
static mut FRONT_SURFACE: Option<Surface> = None;
static mut BACK_SURFACE: Option<Surface> = None;
static mut BACK_BUFFER_DATA: [u32; 1920 * 1080] = [0; 1920 * 1080];

pub static mut MOUSE_X: u32 = 100;
pub static mut MOUSE_Y: u32 = 100;
pub static mut MOUSE_LEFT_DOWN: bool = false;
pub static mut GFX_FONT: Option<Font> = None;

pub static GFX_DIRTY: AtomicBool = AtomicBool::new(true);

pub static mut SHOW_DEMO: bool = false;

static mut WIN1: Window = Window::new(1, 140, 90, 440, 340, "OpenYanase v2");
static mut WIN2: Window = Window::new(2, 420, 160, 380, 260, "Graphics");
static mut DRAG_WIN_ID: Option<u32> = None;
static mut DRAG_OFFSET_X: u32 = 0;
static mut DRAG_OFFSET_Y: u32 = 0;

static mut FRAMES: u32 = 0;
static mut CURRENT_FPS: u32 = 0;
static mut LAST_FPS_TIME: u64 = 0;

// Cache vị trí cursor đã vẽ lần trước — dùng để restore vùng nhỏ thay vì
// copy full buffer khi chỉ có cursor thay đổi.
const CURSOR_W: u32 = 12;
const CURSOR_H: u32 = 19;
static mut LAST_CURSOR_X: u32 = 0;
static mut LAST_CURSOR_Y: u32 = 0;
static mut CURSOR_DRAWN: bool = false;

#[inline(always)]
pub fn mark_dirty() {
    GFX_DIRTY.store(true, Ordering::Release);
}

#[inline(always)]
pub fn is_dirty() -> bool {
    GFX_DIRTY.load(Ordering::Acquire)
}

pub unsafe fn set_demo_mode(enabled: bool) {
    SHOW_DEMO = enabled;
    mark_dirty();
}

pub unsafe fn is_demo_mode() -> bool { SHOW_DEMO }

pub unsafe fn init_graphics(
    fb_addr: *mut u32, width: u32, height: u32, pitch: u32,
    font_atlas: *const u32, tex_w: u32, tex_h: u32,
) {
    let front = Surface::new(fb_addr, width, height, pitch, 32);
    let mut back = Surface::new(BACK_BUFFER_DATA.as_mut_ptr(), width, height, width * 4, 32);
    back.fill(Color::BLACK);

    WIN1.x = width / 8;
    WIN1.y = height / 10;
    WIN1.width = width / 3;
    WIN1.height = height / 2;
    WIN1.is_active = true;

    WIN2.x = width / 2;
    WIN2.y = height / 4;
    WIN2.width = width / 3;
    WIN2.height = height / 3;

    FRONT_SURFACE = Some(front);
    BACK_SURFACE = Some(back);
    GFX_FONT = Some(Font::new(font_atlas, tex_w, tex_h));
    GFX_ACTIVE = true;
    mark_dirty();
    SHOW_DEMO = false;
}

pub fn enter_graphics_mode() {
    unsafe {
        if !GFX_ACTIVE { return; }
        crate::console::save_console_state();
        crate::console::set_display_mode(DisplayMode::Graphics);
        CONSOLE.hide_cursor();
        mark_dirty();
        SHOW_DEMO = true;
        LAST_FPS_TIME = crate::timer::get_ticks();
        CURSOR_DRAWN = false;
        draw_graphics_demo();
        update_frame();
    }
}

pub fn exit_graphics_mode() {
    unsafe {
        if !GFX_ACTIVE { return; }
        if let Some(back) = BACK_SURFACE.as_mut() {
            back.fill(Color::BLACK);
        }
        if let Some(front) = FRONT_SURFACE.as_mut() {
            front.fill(Color::BLACK);
        }
        crate::console::set_display_mode(DisplayMode::Console);
        crate::console::restore_console_state();
        // Thoát gfx = về vtty, KHÔNG bật demo (demo chỉ bật ở enter_graphics_mode)
        SHOW_DEMO = false;
        mark_dirty();
        CURSOR_DRAWN = false;
    }
}

// === DRAW ===

pub fn draw_graphics_demo() {
    unsafe {
        if !GFX_ACTIVE || !SHOW_DEMO || !is_dirty() { return; }

        let current_tick = crate::timer::get_ticks();
        if current_tick - LAST_FPS_TIME >= 1000 {
            CURRENT_FPS = FRAMES;
            FRAMES = 0;
            LAST_FPS_TIME = current_tick;
        }
        FRAMES += 1;

        let back = match BACK_SURFACE.as_mut() {
            Some(s) => s,
            None => return,
        };
        let w = back.width;
        let h = back.height;
        let taskbar_h: u32 = 46;
        let desktop_h = h - taskbar_h;

        draw_desktop_background(back, w, desktop_h);
        draw_desktop_icons(back);
        draw_window_shadow(back, &WIN2);
        draw_window_shadow(back, &WIN1);
        if WIN1.is_active {
            render_custom_window(back, &WIN2);
            render_custom_window(back, &WIN1);
        } else {
            render_custom_window(back, &WIN1);
            render_custom_window(back, &WIN2);
        }
        draw_taskbar(back, w, h, taskbar_h);
    }
}

unsafe fn draw_desktop_background(back: &mut Surface, w: u32, desktop_h: u32) {
    let bands: u32 = 40;
    let band_h = (desktop_h / bands).max(1);
    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let r = (240.0 - t * 40.0) as u8;
        let g = (248.0 - t * 30.0) as u8;
        let b = 255;
        let y = i * band_h;
        let bh = if i == bands - 1 { desktop_h.saturating_sub(y) } else { band_h };
        back.fill_rect(0, y, w, bh, Color::rgb(r, g, b));
    }
}

unsafe fn draw_desktop_icons(back: &mut Surface) {
    let font = GFX_FONT.as_ref();
    let icons: [(&str, u32, u32, Color); 3] = [
        ("YanaseCore", 32, 32, Color::rgb(18, 140, 255)),
        ("Workspace", 32, 128, Color::rgb(255, 170, 0)),
        ("Trash.bin", 32, 224, Color::rgb(255, 90, 110)),
    ];

    for (label, x, y, accent) in icons.iter() {
        back.fill_rect(*x, *y, 64, 64, Color::rgb(255, 255, 255));
        back.draw_rect(*x, *y, 64, 64, Color::rgb(200, 210, 225));
        back.fill_rect(*x + 18, *y + 14, 28, 22, *accent);
        back.fill_rect(*x + 24, *y + 10, 16, 4, Color::rgb(230, 240, 255));
        if let Some(f) = font {
            let label_w = f.measure_string(label);
            let text_x = if label_w < 64 { *x + (64 - label_w) / 2 } else { *x };
            f.draw_string(back, label, text_x, *y + 70, Color::rgb(40, 50, 70));
        }
    }
}

unsafe fn draw_window_shadow(back: &mut Surface, win: &Window) {
    if !win.is_visible { return; }
    back.fill_rect(win.x + 4, win.y + 4, win.width, win.height, Color::rgb(210, 215, 225));
    back.fill_rect(win.x + 8, win.y + 8, win.width, win.height, Color::rgb(190, 200, 215));
}

unsafe fn render_custom_window(back: &mut Surface, win: &Window) {
    if !win.is_visible { return; }
    let header_bg = if win.is_active { Color::rgb(18, 140, 255) } else { Color::rgb(160, 170, 190) };
    let win_bg = Color::rgb(250, 252, 255);
    let border_color = if win.is_active { Color::rgb(10, 110, 210) } else { Color::rgb(200, 210, 225) };

    back.fill_rect(win.x, win.y, win.width, win.height, win_bg);
    back.fill_rect(win.x, win.y, win.width, 32, header_bg);
    if win.is_active {
        back.fill_rect(win.x, win.y, win.width, 2, Color::rgb(100, 200, 255));
    }

    let close_x = win.x + win.width - 28;
    back.fill_rect(close_x, win.y + 6, 20, 20, Color::rgb(240, 80, 100));
    back.draw_rect(win.x, win.y, win.width, win.height, border_color);

    if let Some(f) = GFX_FONT.as_ref() {
        f.draw_string(back, win.title, win.x + 12, win.y + 8, Color::WHITE);
        f.draw_string(back, "x", close_x + 6, win.y + 6, Color::WHITE);
    }
}

unsafe fn draw_taskbar(back: &mut Surface, w: u32, h: u32, taskbar_h: u32) {
    let taskbar_y = h - taskbar_h;
    let font = GFX_FONT.as_ref();

    back.fill_rect(0, taskbar_y, w, taskbar_h, Color::rgb(255, 255, 255));
    back.fill_rect(0, taskbar_y, w, 2, Color::rgb(200, 215, 235));

    let start_w: u32 = 90;
    back.fill_rect(10, taskbar_y + 7, start_w, taskbar_h - 14, Color::rgb(18, 140, 255));
    if let Some(f) = font {
        f.draw_string(back, "YANASE", 24, taskbar_y + 15, Color::WHITE);
    }

    let mut app_x = start_w + 24;
    let apps: [(&Window, &str); 2] = [(&WIN1, "System"), (&WIN2, "Demo")];
    for (win, label) in apps.iter() {
        if !win.is_visible { continue; }
        let active = win.is_active;
        let bg = if active { Color::rgb(235, 245, 255) } else { Color::rgb(245, 248, 252) };
        back.fill_rect(app_x, taskbar_y + 7, 100, taskbar_h - 14, bg);
        if active {
            back.fill_rect(app_x, taskbar_y + taskbar_h - 4, 100, 3, Color::rgb(18, 140, 255));
        }
        if let Some(f) = font {
            let text_col = if active { Color::rgb(10, 80, 180) } else { Color::rgb(100, 110, 130) };
            f.draw_string(back, label, app_x + 10, taskbar_y + 15, text_col);
        }
        app_x += 108;
    }

    if let Some(f) = font {
        let ticks = crate::timer::get_ticks();
        let secs = (ticks / 1000) % 86400;
        let hh = secs / 3600;
        let mm = (secs % 3600) / 60;
        let ss = secs % 60;
        let mut buf = [0u8; 16];
        let s = format_time(&mut buf, hh, mm, ss);
        let text_w = f.measure_string(s);
        let clock_x = w.saturating_sub(text_w + 20);
        back.fill_rect(clock_x - 10, taskbar_y + 7, text_w + 20, taskbar_h - 14, Color::rgb(240, 245, 252));
        f.draw_string(back, s, clock_x, taskbar_y + 15, Color::rgb(40, 60, 90));

        let mut fps_buf = [0u8; 16];
        let fps_len = format_fps(&mut fps_buf, CURRENT_FPS);
        let fps_str = core::str::from_utf8_unchecked(&fps_buf[..fps_len]);
        let fps_w = f.measure_string(fps_str);
        let fps_x = clock_x.saturating_sub(fps_w + 30);
        back.fill_rect(fps_x - 10, taskbar_y + 7, fps_w + 20, taskbar_h - 14, Color::rgb(240, 245, 252));
        f.draw_string(back, fps_str, fps_x, taskbar_y + 15, Color::rgb(18, 140, 255));
    }
}

// === UPDATE ===

/// Restore đúng vùng cursor cũ (~228 pixel) từ back buffer, không copy full.
unsafe fn restore_cursor_rect(front: &mut Surface, back: &Surface) {
    if !CURSOR_DRAWN { return; }
    let max_x = (LAST_CURSOR_X + CURSOR_W).min(front.width).min(back.width);
    let max_y = (LAST_CURSOR_Y + CURSOR_H).min(front.height).min(back.height);
    let mut y = LAST_CURSOR_Y;
    while y < max_y {
        let mut x = LAST_CURSOR_X;
        while x < max_x {
            let c = back.get_pixel(x, y);
            front.put_pixel(x, y, c);
            x += 1;
        }
        y += 1;
    }
}

pub unsafe fn update_frame() {
    // swap() atomically consume dirty flag.
    let had_dirty = GFX_DIRTY.swap(false, Ordering::AcqRel);
    if let (Some(front), Some(back)) = (FRONT_SURFACE.as_mut(), BACK_SURFACE.as_ref()) {
        if had_dirty {
            front.copy_from(back);
            CURSOR_DRAWN = false;
        } else {
            // Không dirty → chỉ cần xoá cursor cũ trước khi vẽ cursor mới
            restore_cursor_rect(front, back);
        }
        front.draw_cursor(MOUSE_X, MOUSE_Y);
        LAST_CURSOR_X = MOUSE_X;
        LAST_CURSOR_Y = MOUSE_Y;
        CURSOR_DRAWN = true;
    }
}

/// Chỉ update cursor, dùng trong sys_sleep yield. Rất nhẹ.
pub unsafe fn update_cursor_only() {
    if !GFX_ACTIVE { return; }
    if let (Some(front), Some(back)) = (FRONT_SURFACE.as_mut(), BACK_SURFACE.as_ref()) {
        restore_cursor_rect(front, back);
        front.draw_cursor(MOUSE_X, MOUSE_Y);
        LAST_CURSOR_X = MOUSE_X;
        LAST_CURSOR_Y = MOUSE_Y;
        CURSOR_DRAWN = true;
    }
}

pub unsafe fn clear_buffers() {
    if let Some(back) = BACK_SURFACE.as_mut() {
        back.fill(Color::BLACK);
    }
    if let Some(front) = FRONT_SURFACE.as_mut() {
        front.fill(Color::BLACK);
    }
    CURSOR_DRAWN = false;
    mark_dirty();
}

// === MOUSE ===

pub unsafe fn update_mouse_state(dx: i32, dy: i32, left_down: bool) {
    if let Some(s) = BACK_SURFACE.as_ref() {
        let new_x = (MOUSE_X as i32 + dx).clamp(0, s.width as i32 - 1);
        let new_y = (MOUSE_Y as i32 + dy).clamp(0, s.height as i32 - 1);

        let moved = new_x as u32 != MOUSE_X || new_y as u32 != MOUSE_Y;
        let button_changed = left_down != MOUSE_LEFT_DOWN;

        if moved {
            MOUSE_X = new_x as u32;
            MOUSE_Y = new_y as u32;
        }

        if moved || button_changed {
            mark_dirty();
        }

        let just_pressed = left_down && !MOUSE_LEFT_DOWN;
        let just_released = !left_down && MOUSE_LEFT_DOWN;

        if just_pressed {
            process_mouse_press(MOUSE_X, MOUSE_Y);
        }
        if left_down && DRAG_WIN_ID.is_some() {
            process_mouse_drag(MOUSE_X, MOUSE_Y);
        }
        if just_released {
            process_mouse_release();
        }

        MOUSE_LEFT_DOWN = left_down;
    }
}

pub unsafe fn update_mouse(dx: i32, dy: i32) {
    update_mouse_state(dx, dy, MOUSE_LEFT_DOWN);
}

// === HELPERS ===

unsafe fn process_mouse_press(mx: u32, my: u32) {
    let wins = if WIN1.is_active {
        [&mut WIN1 as *mut Window, &mut WIN2 as *mut Window]
    } else {
        [&mut WIN2 as *mut Window, &mut WIN1 as *mut Window]
    };

    for w_ptr in wins {
        let win = &mut *w_ptr;
        if !win.is_visible { continue; }

        if win.is_close_hit(mx, my) {
            win.is_visible = false;
            return;
        }

        if win.is_titlebar_hit(mx, my) {
            set_active_window(win.id);
            DRAG_WIN_ID = Some(win.id);
            DRAG_OFFSET_X = mx.saturating_sub(win.x);
            DRAG_OFFSET_Y = my.saturating_sub(win.y);
            win.is_dragging = true;
            return;
        }

        if win.contains_point(mx, my) {
            set_active_window(win.id);
            return;
        }
    }
}

unsafe fn process_mouse_drag(mx: u32, my: u32) {
    if let Some(id) = DRAG_WIN_ID {
        let win = if WIN1.id == id { &mut WIN1 } else { &mut WIN2 };
        win.x = mx.saturating_sub(DRAG_OFFSET_X);
        win.y = my.saturating_sub(DRAG_OFFSET_Y);
    }
}

unsafe fn process_mouse_release() {
    DRAG_WIN_ID = None;
    WIN1.is_dragging = false;
    WIN2.is_dragging = false;
}

unsafe fn set_active_window(id: u32) {
    WIN1.is_active = WIN1.id == id;
    WIN2.is_active = WIN2.id == id;
}

fn format_time(buf: &mut [u8; 16], hh: u64, mm: u64, ss: u64) -> &str {
    fn write_u2(buf: &mut [u8], pos: usize, v: u64) {
        buf[pos] = b'0' + (v / 10) as u8;
        buf[pos + 1] = b'0' + (v % 10) as u8;
    }
    write_u2(buf, 0, hh);
    buf[2] = b':';
    write_u2(buf, 3, mm);
    buf[5] = b':';
    write_u2(buf, 6, ss);
    core::str::from_utf8(&buf[..8]).unwrap_or("00:00:00")
}

fn format_fps(buf: &mut [u8; 16], fps: u32) -> usize {
    let mut pos = 0;
    let mut num = fps;
    let mut digits = [0u8; 10];
    let mut count = 0;
    if num == 0 {
        digits[0] = b'0';
        count = 1;
    } else {
        while num > 0 {
            digits[count] = b'0' + (num % 10) as u8;
            count += 1;
            num /= 10;
        }
    }
    for i in 0..count {
        buf[pos] = digits[count - 1 - i];
        pos += 1;
    }
    buf[pos] = b' '; pos += 1;
    buf[pos] = b'F'; pos += 1;
    buf[pos] = b'P'; pos += 1;
    buf[pos] = b'S'; pos += 1;
    pos
}

// === PUBLIC API ===

pub unsafe fn get_back_surface() -> Option<&'static mut Surface> {
    BACK_SURFACE.as_mut()
}

pub unsafe fn get_front_surface() -> Option<&'static mut Surface> {
    FRONT_SURFACE.as_mut()
}

pub unsafe fn get_font() -> Option<&'static Font> {
    GFX_FONT.as_ref()
}