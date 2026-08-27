// src/graphics/gfx.rs
use crate::gop::Color;
use crate::console::{CONSOLE, DisplayMode};
use crate::graphics::surface::Surface;
use crate::graphics::font::Font;
use crate::graphics::window::Window;
use crate::serial;

static mut GFX_ACTIVE: bool = false;
static mut FRONT_SURFACE: Option<Surface> = None;
static mut BACK_SURFACE: Option<Surface> = None;

static mut BACK_BUFFER_DATA: [u32; 1920 * 1080] = [0; 1920 * 1080];

pub static mut MOUSE_X: u32 = 100;
pub static mut MOUSE_Y: u32 = 100;
static mut MOUSE_LEFT_DOWN: bool = false;

static mut WIN1: Window = Window::new(1, 100, 100, 400, 300, "OpenYanase GUI");
static mut WIN2: Window = Window::new(2, 350, 200, 350, 220, "Graphics Demo");

static mut DRAG_WIN_ID: Option<u32> = None;
static mut DRAG_OFFSET_X: u32 = 0;
static mut DRAG_OFFSET_Y: u32 = 0;

static mut GFX_FONT: Option<Font> = None;

static mut GFX_DIRTY: bool = true;

pub unsafe fn mark_dirty() {
    GFX_DIRTY = true;
}

pub unsafe fn init_graphics(
    fb_addr: *mut u32,
    width: u32,
    height: u32,
    pitch: u32,
    font_atlas: *const u32,
    tex_w: u32,
    tex_h: u32,
) {
    let front = Surface::new(fb_addr, width, height, pitch, 32);
    let back = Surface::new(BACK_BUFFER_DATA.as_mut_ptr(), width, height, width * 4, 32);

    WIN1.x = width / 6;
    WIN1.y = height / 8;
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
    GFX_DIRTY = true;

    serial::serial_write_str("INFO: [GFX] Subsystem initialized with Double Buffering\r\n");
}

pub fn enter_graphics_mode() {
    unsafe {
        if !GFX_ACTIVE { return; }
        crate::console::save_console_state();
        crate::console::set_display_mode(DisplayMode::Graphics);
        CONSOLE.hide_cursor();
        GFX_DIRTY = true;
        draw_graphics_demo();
    }
}

pub fn exit_graphics_mode() {
    unsafe {
        if !GFX_ACTIVE { return; }
        crate::console::set_display_mode(DisplayMode::Console);
        crate::console::restore_console_state();
    }
}

pub fn draw_graphics_demo() {
    unsafe {
        if !GFX_ACTIVE { return; }
        if !GFX_DIRTY { return; }
        GFX_DIRTY = false;

        let back = match BACK_SURFACE.as_mut() {
            Some(s) => s,
            None => return,
        };
        let font = GFX_FONT.as_ref();
        let w = back.width;
        let h = back.height;
        let taskbar_h: u32 = 40;
        let desktop_h = h - taskbar_h;

        draw_desktop_background(back, w, desktop_h);
        draw_desktop_icons(back, font);

        draw_window_shadow(back, &WIN2);
        draw_window_shadow(back, &WIN1);

        if WIN1.is_active {
            WIN2.render(back, font);
            WIN1.render(back, font);
        } else {
            WIN1.render(back, font);
            WIN2.render(back, font);
        }

        draw_taskbar(back, font, w, h, taskbar_h);

        back.draw_cursor(MOUSE_X, MOUSE_Y);

        if let Some(front) = FRONT_SURFACE.as_mut() {
            front.copy_from(back);
        }
    }
}

unsafe fn draw_desktop_background(back: &mut Surface, w: u32, desktop_h: u32) {
    let bands: u32 = 24;
    let band_h = (desktop_h / bands).max(1);

    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let r = (18.0 + t * 40.0) as u8;
        let g = (22.0 + t * 30.0) as u8;
        let b = (48.0 + t * 70.0) as u8;
        let y = i * band_h;
        let bh = if i == bands - 1 { desktop_h.saturating_sub(y) } else { band_h };
        back.fill_rect(0, y, w, bh, Color::rgb(r, g, b));
    }
}

unsafe fn draw_desktop_icons(back: &mut Surface, font: Option<&Font>) {
    let icons: [(&str, u32, u32); 3] = [
        ("My PC", 24, 24),
        ("Files", 24, 104),
        ("Trash", 24, 184),
    ];

    for (label, x, y) in icons.iter() {
        back.fill_rect(*x, *y, 56, 56, Color::rgb(70, 90, 130));
        back.draw_rect(*x, *y, 56, 56, Color::rgb(110, 130, 170));
        back.fill_rect(*x + 20, *y + 16, 16, 20, Color::rgb(230, 230, 235));

        if let Some(f) = font {
            f.draw_string(back, label, x.saturating_sub(4), *y + 60, Color::rgb(230, 230, 235));
        }
    }
}

unsafe fn draw_window_shadow(back: &mut Surface, win: &Window) {
    if !win.is_visible { return; }
    let shadow_offset = 6;
    back.fill_rect(
        win.x + shadow_offset,
        win.y + shadow_offset,
        win.width,
        win.height,
        Color::rgb(8, 8, 12),
    );
}

unsafe fn draw_taskbar(back: &mut Surface, font: Option<&Font>, w: u32, h: u32, taskbar_h: u32) {
    let taskbar_y = h - taskbar_h;

    back.fill_rect(0, taskbar_y, w, taskbar_h, Color::rgb(15, 17, 23));
    back.fill_rect(0, taskbar_y, w, 2, Color::rgb(45, 50, 65));

    let start_w: u32 = 90;
    back.fill_rect(6, taskbar_y + 6, start_w, taskbar_h - 12, Color::rgb(0, 122, 204));
    back.fill_rect(6, taskbar_y + 6, start_w, 3, Color::rgb(40, 160, 235));

    if let Some(f) = font {
        f.draw_string(back, "Start", 24, taskbar_y + 14, Color::WHITE);
    }

    let mut app_x = start_w + 20;
    let apps: [(&Window, &str); 2] = [(&WIN1, "GUI"), (&WIN2, "Demo")];
    for (win, label) in apps.iter() {
        if !win.is_visible { continue; }
        let active = win.is_active;
        let bg = if active { Color::rgb(35, 40, 55) } else { Color::rgb(22, 24, 32) };
        back.fill_rect(app_x, taskbar_y + 6, 80, taskbar_h - 12, bg);
        if active {
            back.fill_rect(app_x, taskbar_y + taskbar_h - 4, 80, 3, Color::rgb(0, 150, 255));
        }
        if let Some(f) = font {
            f.draw_string(back, label, app_x + 8, taskbar_y + 14, Color::rgb(210, 210, 215));
        }
        app_x += 88;
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
        f.draw_string(back, s, w.saturating_sub(text_w + 20), taskbar_y + 14, Color::rgb(220, 220, 225));
    }
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

pub unsafe fn update_mouse_state(dx: i32, dy: i32, left_down: bool) {
    if let Some(s) = BACK_SURFACE.as_ref() {
        let new_x = (MOUSE_X as i32 + dx).clamp(0, s.width as i32 - 1);
        let new_y = (MOUSE_Y as i32 + dy).clamp(0, s.height as i32 - 1);

        if new_x as u32 != MOUSE_X || new_y as u32 != MOUSE_Y {
            MOUSE_X = new_x as u32;
            MOUSE_Y = new_y as u32;
            GFX_DIRTY = true;
        }

        let just_pressed = left_down && !MOUSE_LEFT_DOWN;
        let just_released = !left_down && MOUSE_LEFT_DOWN;

        if just_pressed {
            process_mouse_press(MOUSE_X, MOUSE_Y);
            GFX_DIRTY = true;
        }

        if left_down && DRAG_WIN_ID.is_some() {
            process_mouse_drag(MOUSE_X, MOUSE_Y);
            GFX_DIRTY = true;
        }

        if just_released {
            process_mouse_release();
            GFX_DIRTY = true;
        }

        MOUSE_LEFT_DOWN = left_down;
    }
}

pub unsafe fn update_mouse(dx: i32, dy: i32) {
    update_mouse_state(dx, dy, MOUSE_LEFT_DOWN);
}

unsafe fn process_mouse_press(mx: u32, my: u32) {
    let wins = if WIN1.is_active {
        [ &mut WIN1 as *mut Window, &mut WIN2 as *mut Window ]
    } else {
        [ &mut WIN2 as *mut Window, &mut WIN1 as *mut Window ]
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