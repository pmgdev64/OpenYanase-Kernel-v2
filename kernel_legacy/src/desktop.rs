// src/desktop.rs

use crate::vbe::Mb2TagFramebuffer;
use crate::console::CONSOLE;
use crate::mouse;

const MAX_WIDTH: usize = 1024;
const MAX_HEIGHT: usize = 768;

static mut BACKBUFFER: [u32; MAX_WIDTH * MAX_HEIGHT] = [0; MAX_WIDTH * MAX_HEIGHT];
static mut WALLPAPER_LOADED: bool = false;

fn put_pixel_buf(x: i32, y: i32, color: u32) {
    if x >= 0 && x < MAX_WIDTH as i32 && y >= 0 && y < MAX_HEIGHT as i32 {
        let idx = (y as usize * MAX_WIDTH) + x as usize;
        unsafe {
            BACKBUFFER[idx] = color;
        }
    }
}

pub fn draw_rect(x: i32, y: i32, w: u32, h: u32, color: u32) {
    for dy in 0..h as i32 {
        let py = y + dy;
        if py < 0 || py >= MAX_HEIGHT as i32 { continue; }
        for dx in 0..w as i32 {
            let px = x + dx;
            if px < 0 || px >= MAX_WIDTH as i32 { continue; }
            put_pixel_buf(px, py, color);
        }
    }
}

pub fn draw_rect_border(x: i32, y: i32, w: u32, h: u32, thickness: u32, color: u32) {
    let t = thickness as i32;
    let wi = w as i32;
    let hi = h as i32;
    draw_rect(x, y, w, thickness, color);
    draw_rect(x, y + hi - t, w, thickness, color);
    draw_rect(x, y, thickness, h, color);
    draw_rect(x + wi - t, y, thickness, h, color);
}

fn draw_char(x: i32, y: i32, byte: u8, color_fg: u32) {
    unsafe {
        if CONSOLE.tex.is_null() || CONSOLE.tex_w == 0 { return; }
        let glyph_w = (CONSOLE.tex_w / 16) as i32;
        let glyph_h = (CONSOLE.tex_h / 16) as i32;
        let map_x = ((byte as u32) % 16) as i32 * glyph_w;
        let map_y = ((byte as u32) / 16) as i32 * glyph_h;

        for cy in 0..glyph_h {
            for cx in 0..glyph_w {
                let tex_idx = (((map_y + cy) * CONSOLE.tex_w as i32) + (map_x + cx)) as usize;
                let tex_color = *CONSOLE.tex.add(tex_idx);
                if tex_color != 0 {
                    put_pixel_buf(x + cx, y + cy, color_fg);
                }
            }
        }
    }
}

pub fn draw_string(mut x: i32, y: i32, text: &str, color: u32) {
    unsafe {
        if CONSOLE.tex_w == 0 { return; }
        let glyph_w = (CONSOLE.tex_w / 16) as i32;
        let glyph_h = (CONSOLE.tex_h / 16) as i32;
        let start_x = x;
        let mut current_y = y;

        for byte in text.bytes() {
            if byte == b'\n' {
                x = start_x;
                current_y += glyph_h;
                continue;
            }
            draw_char(x, current_y, byte, color);
            x += glyph_w;
        }
    }
}

fn draw_mouse_cursor(mx: i32, my: i32) {
    let cursor_bitmap: [&str; 16] = [
        "X           ",
        "XX          ",
        "X.X         ",
        "X..X        ",
        "X...X       ",
        "X....X      ",
        "X.....X     ",
        "X......X    ",
        "X.......X   ",
        "X.....XXXX  ",
        "X..X..X     ",
        "X.X X..X    ",
        "XX   X..X   ",
        "     X..X   ",
        "      XX    ",
        "            ",
    ];

    for (dy, row) in cursor_bitmap.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let px = mx + dx as i32;
            let py = my + dy as i32;
            match ch {
                'X' => put_pixel_buf(px, py, 0x000000),
                '.' => put_pixel_buf(px, py, 0xFFFFFF),
                _ => {}
            }
        }
    }
}

fn flush_dirty_to_screen(fb: &Mb2TagFramebuffer, width: usize, height: usize) {
    let pitch_u32 = (fb.framebuffer_pitch / 4) as usize;
    let fb_ptr = fb.framebuffer_addr as *mut u32;

    unsafe {
        let back = BACKBUFFER.as_ptr();

        for y in 0..height {
            let src_offset = y * MAX_WIDTH;
            let dst_ptr = fb_ptr.add(y * pitch_u32);
            
            // Copy entire row (faster than pixel-by-pixel comparison)
            core::ptr::copy_nonoverlapping(
                back.add(src_offset),
                dst_ptr,
                width
            );
        }
    }
}

/// Load wallpaper from initrd
pub fn load_wallpaper() -> bool {
    unsafe {
        if crate::initrd::INITRD_ADDR.is_null() {
            return false;
        }

        let fb = if CONSOLE.fb.is_null() {
            return false;
        } else {
            &*CONSOLE.fb
        };

        let fb_w = fb.framebuffer_width;
        let fb_h = fb.framebuffer_height;

        let mut name_buf = [0u8; 64];
        let name_len = get_wallpaper_filename(fb_w, fb_h, &mut name_buf);
        let name = core::str::from_utf8(&name_buf[..name_len]).unwrap_or("wallpaper.bmp");

        let names = [name, "wallpaper.bmp", "splash.bmp"];

        for wallpaper_name in names.iter() {
            if let Some(bmp_bytes) = crate::initrd::find_file_in_tar(crate::initrd::INITRD_ADDR, wallpaper_name) {
                draw_bmp_to_backbuffer(bmp_bytes, fb_w, fb_h);
                WALLPAPER_LOADED = true;
                return true;
            }
        }

        false
    }
}

/// Save current backbuffer as wallpaper (restore background)
fn restore_background() {
    unsafe {
        if WALLPAPER_LOADED {
            // Reload wallpaper from file
            load_wallpaper();
        } else {
            // Solid color fallback
            draw_rect(0, 0, 800, 600, 0x008080);
        }
    }
}

fn get_wallpaper_filename(width: u32, height: u32, buf: &mut [u8]) -> usize {
    let prefix = "wallpaper_";
    let suffix = ".bmp";
    
    let mut pos = 0;
    
    for &b in prefix.as_bytes() {
        if pos < buf.len() - 1 {
            buf[pos] = b;
            pos += 1;
        }
    }
    
    let mut num_buf = [0u8; 10];
    let num_len = u32_to_str(width, &mut num_buf);
    for i in 0..num_len {
        if pos < buf.len() - 1 {
            buf[pos] = num_buf[i];
            pos += 1;
        }
    }
    
    if pos < buf.len() - 1 {
        buf[pos] = b'x';
        pos += 1;
    }
    
    let num_len = u32_to_str(height, &mut num_buf);
    for i in 0..num_len {
        if pos < buf.len() - 1 {
            buf[pos] = num_buf[i];
            pos += 1;
        }
    }
    
    for &b in suffix.as_bytes() {
        if pos < buf.len() - 1 {
            buf[pos] = b;
            pos += 1;
        }
    }
    
    if pos < buf.len() {
        buf[pos] = 0;
    }
    
    pos
}

fn u32_to_str(mut num: u32, buf: &mut [u8]) -> usize {
    if num == 0 {
        buf[0] = b'0';
        return 1;
    }
    
    let mut temp = [0u8; 10];
    let mut i = 0;
    
    while num > 0 {
        temp[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    let mut pos = 0;
    for j in (0..i).rev() {
        buf[pos] = temp[j];
        pos += 1;
    }
    
    pos
}

fn draw_bmp_to_backbuffer(bmp_data: &[u8], fb_w: u32, fb_h: u32) {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return;
    }

    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes([bmp_data[offset], bmp_data[offset+1], bmp_data[offset+2], bmp_data[offset+3]])
    };
    let read_i32 = |offset: usize| -> i32 {
        i32::from_le_bytes([bmp_data[offset], bmp_data[offset+1], bmp_data[offset+2], bmp_data[offset+3]])
    };
    let read_u16 = |offset: usize| -> u16 {
        u16::from_le_bytes([bmp_data[offset], bmp_data[offset+1]])
    };

    let pixel_offset = read_u32(10) as usize;
    let bmp_width = read_i32(18) as u32;
    let mut bmp_height = read_i32(22);
    let bpp = read_u16(28);

    let top_down = bmp_height < 0;
    if top_down { bmp_height = -bmp_height; }
    let bmp_height = bmp_height as u32;

    if bpp != 24 && bpp != 32 { return; }

    let bytes_per_pixel = (bpp / 8) as u32;
    let row_size = ((bmp_width * bytes_per_pixel + 3) / 4) * 4;

    let fb_w = fb_w as usize;
    let fb_h = fb_h as usize;

    for fb_y in 0..fb_h {
        let bmp_y = (fb_y as u32 * bmp_height) / fb_h as u32;
        let row_idx = if top_down { bmp_y } else { bmp_height - 1 - bmp_y };
        let row_offset = pixel_offset + (row_idx * row_size) as usize;

        if row_offset + (bmp_width * bytes_per_pixel) as usize > bmp_data.len() {
            break;
        }

        for fb_x in 0..fb_w {
            let bmp_x = (fb_x as u32 * bmp_width) / fb_w as u32;
            let p = row_offset + (bmp_x * bytes_per_pixel) as usize;

            let b = bmp_data[p] as u32;
            let g = bmp_data[p+1] as u32;
            let r = bmp_data[p+2] as u32;

            let color = (r << 16) | (g << 8) | b;
            
            unsafe {
                let idx = fb_y * MAX_WIDTH + fb_x;
                if idx < BACKBUFFER.len() {
                    BACKBUFFER[idx] = color;
                }
            }
        }
    }
}

/// Vẽ toàn bộ desktop
fn draw_desktop(width: u32, height: u32, win_x: i32, win_y: i32, win_w: u32, win_h: u32, 
                start_menu_open: bool) {
    // 1. Background
    if !unsafe { WALLPAPER_LOADED } {
        draw_rect(0, 0, width, height, 0x008080);
    }

    // 2. Desktop icons
    draw_rect(30, 30, 48, 48, 0x005A9E);
    draw_rect_border(30, 30, 48, 48, 2, 0xFFFFFF);
    draw_string(20, 82, "Computer", 0xFFFFFF);

    draw_rect(30, 110, 48, 48, 0x2D2D2D);
    draw_rect_border(30, 110, 48, 48, 2, 0x00FF00);
    draw_string(22, 162, "Terminal", 0xFFFFFF);

    // 3. Taskbar
    let taskbar_h = 38;
    let taskbar_y = height as i32 - taskbar_h as i32;
    draw_rect(0, taskbar_y, width, taskbar_h, 0x222222);
    draw_rect(0, taskbar_y, width, 2, 0x444444);

    // 4. Start button
    let start_bg = if start_menu_open { 0x005A9E } else { 0x0078D7 };
    draw_rect(6, taskbar_y + 5, 75, 28, start_bg);
    draw_rect_border(6, taskbar_y + 5, 75, 28, 1, 0xFFFFFF);
    draw_string(22, taskbar_y + 12, "Start", 0xFFFFFF);

    // 5. Clock
    draw_rect(width as i32 - 85, taskbar_y + 5, 75, 28, 0x111111);
    draw_rect_border(width as i32 - 85, taskbar_y + 5, 75, 28, 1, 0x333333);
    draw_string(width as i32 - 72, taskbar_y + 12, "12:00", 0xFFFFFF);

    // 6. Start menu
    if start_menu_open {
        let menu_w: u32 = 150;
        let menu_h: u32 = 130;
        let menu_y = taskbar_y - menu_h as i32;
        draw_rect(6, menu_y, menu_w, menu_h, 0x2D2D2D);
        draw_rect_border(6, menu_y, menu_w, menu_h, 2, 0x0078D7);
        draw_string(16, menu_y + 15, "> OpenYanase OS", 0x00FF00);
        draw_string(16, menu_y + 45, "  Files", 0xFFFFFF);
        draw_string(16, menu_y + 75, "  Settings", 0xFFFFFF);
        draw_string(16, menu_y + 100, "  Exit GUI", 0xFF5555);
    }

    // 7. Window
    draw_rect(win_x + 6, win_y + 6, win_w, win_h, 0x004040);
    draw_rect(win_x, win_y, win_w, win_h, 0xC0C0C0);
    draw_rect_border(win_x, win_y, win_w, win_h, 2, 0xFFFFFF);

    draw_rect(win_x + 3, win_y + 3, win_w - 6, 28, 0x000080);
    draw_string(win_x + 10, win_y + 9, "OpenYanase VTTY Window", 0xFFFFFF);

    let btn_x = win_x + win_w as i32 - 25;
    let btn_y = win_y + 6;
    draw_rect(btn_x, btn_y, 20, 20, 0xE81123);
    draw_rect_border(btn_x, btn_y, 20, 20, 1, 0xFFFFFF);
    draw_string(btn_x + 6, btn_y + 2, "X", 0xFFFFFF);

    draw_rect(win_x + 10, win_y + 40, win_w - 20, win_h - 50, 0x000000);
    draw_rect_border(win_x + 10, win_y + 40, win_w - 20, win_h - 50, 2, 0x808080);

    draw_string(win_x + 20, win_y + 55, "OpenYanase Desktop Running via VBE.", 0x00FF00);
    draw_string(win_x + 20, win_y + 80, "Mouse Interaction Enabled!", 0x00FFFF);
    draw_string(win_x + 20, win_y + 105, "Drag titlebar to move window.", 0xAAAAAA);
}

pub fn run_desktop() {
    let fb = unsafe {
        if CONSOLE.fb.is_null() { return; }
        &*CONSOLE.fb
    };

    let width = fb.framebuffer_width;
    let height = fb.framebuffer_height;

    mouse::init_mouse();

    // Clear backbuffer
    unsafe {
        BACKBUFFER.fill(0);
    }

    // Load wallpaper once
    if !load_wallpaper() {
        draw_rect(0, 0, width, height, 0x008080);
    }

    let mut win_x: i32 = 180;
    let mut win_y: i32 = 70;
    let win_w: u32 = 480;
    let win_h: u32 = 340;

    let mut is_dragging = false;
    let mut drag_offset_x: i32 = 0;
    let mut drag_offset_y: i32 = 0;
    let mut start_menu_open = false;
    let mut prev_left_btn = false;
    let mut dirty = true;

    // Previous state for dirty tracking
    let mut prev_win_x = win_x;
    let mut prev_win_y = win_y;
    let mut prev_start_menu_open = false;

    // Draw initial desktop
    draw_desktop(width, height, win_x, win_y, win_w, win_h, start_menu_open);

    loop {
        let (key_code, mouse_changed) = mouse::poll_input(width, height);
        
        if let Some(scancode) = key_code {
            if scancode == 0x01 { break; }
        }

        if mouse_changed {
            dirty = true;
        }

        let (mx, my, left_btn, _right_btn) = mouse::get_mouse_state();

        if left_btn {
            if !prev_left_btn {
                let btn_x = win_x + win_w as i32 - 25;
                let btn_y = win_y + 6;
                if mx >= btn_x && mx < btn_x + 20 && my >= btn_y && my < btn_y + 20 {
                    break;
                }

                let taskbar_y = height as i32 - 38;
                if mx >= 6 && mx < 81 && my >= taskbar_y + 5 && my < taskbar_y + 33 {
                    start_menu_open = !start_menu_open;
                    dirty = true;
                } else if start_menu_open && (mx > 160 || my < taskbar_y - 140) {
                    start_menu_open = false;
                    dirty = true;
                }

                if mx >= win_x + 3 && mx < win_x + win_w as i32 - 30 && my >= win_y + 3 && my < win_y + 31 {
                    is_dragging = true;
                    drag_offset_x = mx - win_x;
                    drag_offset_y = my - win_y;
                }
            } else if is_dragging {
                win_x = mx - drag_offset_x;
                win_y = my - drag_offset_y;
                dirty = true;
            }
        } else {
            if is_dragging {
                is_dragging = false;
                dirty = true;
            }
        }
        prev_left_btn = left_btn;

        // Only redraw if something changed
        if dirty {
            // Restore background (clear trails)
            if unsafe { WALLPAPER_LOADED } {
                // Reload wallpaper to clear old window position
                load_wallpaper();
            } else {
                draw_rect(0, 0, width, height, 0x008080);
            }

            // Redraw everything
            draw_desktop(width, height, win_x, win_y, win_w, win_h, start_menu_open);

            // Draw mouse cursor on top
            draw_mouse_cursor(mx, my);

            // Flush to screen
            flush_dirty_to_screen(fb, width as usize, height as usize);

            // Update previous state
            prev_win_x = win_x;
            prev_win_y = win_y;
            prev_start_menu_open = start_menu_open;

            dirty = false;
        } else {
            unsafe { core::arch::asm!("nop"); }
        }
    }

    unsafe {
        CONSOLE.clear();
    }
    crate::println!("Exited Desktop Mode.");
}

// ==========================================
// PUBLIC FUNCTIONS FOR SYSCALLS
// ==========================================

pub fn redraw() {
    unsafe {
        if CONSOLE.fb.is_null() { return; }
        let fb = &*CONSOLE.fb;
        let width = fb.framebuffer_width as usize;
        let height = fb.framebuffer_height as usize;
        flush_dirty_to_screen(fb, width, height);
    }
}

pub fn clear() {
    unsafe {
        BACKBUFFER.fill(0);
    }
}

pub fn put_pixel(x: i32, y: i32, color: u32) {
    put_pixel_buf(x, y, color);
}

pub fn draw_wallpaper_from_bmp(bmp_data: &[u8]) -> bool {
    unsafe {
        if CONSOLE.fb.is_null() {
            return false;
        }
        let fb = &*CONSOLE.fb;
        let fb_w = fb.framebuffer_width;
        let fb_h = fb.framebuffer_height;
        
        draw_bmp_to_backbuffer(bmp_data, fb_w, fb_h);
        WALLPAPER_LOADED = true;
        true
    }
}

pub fn load_wallpaper_by_name(name: &str) -> bool {
    unsafe {
        if crate::initrd::INITRD_ADDR.is_null() {
            return false;
        }
        
        if let Some(bmp_bytes) = crate::initrd::find_file_in_tar(crate::initrd::INITRD_ADDR, name) {
            return draw_wallpaper_from_bmp(bmp_bytes);
        }
        false
    }
}