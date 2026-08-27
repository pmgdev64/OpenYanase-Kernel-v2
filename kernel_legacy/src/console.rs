// src/console.rs

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::vbe::{Mb2TagFramebuffer, put_pixel};

// Cờ toàn cục để vô hiệu hóa TTY khi đang ở Graphics Mode
pub static IS_GFX_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_gfx_mode(enabled: bool) {
    IS_GFX_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_gfx_mode() -> bool {
    IS_GFX_MODE.load(Ordering::Relaxed)
}

// Kích thước tối đa của lưới ký tự
const MAX_COLS: usize = 200;
const MAX_ROWS: usize = 160;

// UTF-8 Decoder state
#[derive(Clone, Copy)]
struct Utf8Decoder {
    state: u32,
    codepoint: u32,
    bytes_left: u32,
}

impl Utf8Decoder {
    const fn new() -> Self {
        Utf8Decoder {
            state: 0,
            codepoint: 0,
            bytes_left: 0,
        }
    }

    fn decode(&mut self, byte: u8) -> Option<char> {
        if byte < 0x80 {
            // ASCII
            return Some(byte as char);
        }

        if self.state == 0 {
            // Start of UTF-8 sequence
            if byte >= 0xF0 {
                // 4-byte sequence
                self.state = 1;
                self.codepoint = (byte & 0x07) as u32;
                self.bytes_left = 3;
            } else if byte >= 0xE0 {
                // 3-byte sequence
                self.state = 1;
                self.codepoint = (byte & 0x0F) as u32;
                self.bytes_left = 2;
            } else if byte >= 0xC0 {
                // 2-byte sequence
                self.state = 1;
                self.codepoint = (byte & 0x1F) as u32;
                self.bytes_left = 1;
            } else {
                // Invalid UTF-8
                self.state = 0;
                return None;
            }
        } else {
            // Continuation byte
            if byte & 0xC0 != 0x80 {
                self.state = 0;
                return None;
            }
            self.codepoint = (self.codepoint << 6) | ((byte & 0x3F) as u32);
            self.bytes_left -= 1;

            if self.bytes_left == 0 {
                self.state = 0;
                if let Some(c) = char::from_u32(self.codepoint) {
                    return Some(c);
                }
            }
        }
        None
    }
}

pub struct Console {
    pub fb: *const Mb2TagFramebuffer,
    pub tex: *const u32,
    pub tex_w: u32,
    pub tex_h: u32,
    pub col: u32,
    pub row: u32,
    pub cursor_active: bool,
    buf: [u8; MAX_ROWS * MAX_COLS],
    // UTF-8 decoder state
    decoder: Utf8Decoder,
}

pub static mut CONSOLE: Console = Console {
    fb: core::ptr::null(),
    tex: core::ptr::null(),
    tex_w: 0,
    tex_h: 0,
    col: 0,
    row: 0,
    cursor_active: false,
    buf: [0; MAX_ROWS * MAX_COLS],
    decoder: Utf8Decoder::new(),
};

impl Console {
    pub fn init(&mut self, fb: *const Mb2TagFramebuffer, tex: *const u32, tex_w: u32, tex_h: u32) {
        self.fb = fb;
        self.tex = tex;
        self.tex_w = tex_w;
        self.tex_h = tex_h;
        self.col = 0;
        self.row = 0;
        self.cursor_active = false;
        self.buf = [0; MAX_ROWS * MAX_COLS];
        self.decoder = Utf8Decoder::new();
        self.clear();
    }

    fn glyph_dims(&self) -> (u32, u32) {
        if self.tex_w == 0 || self.tex_h == 0 {
            return (0, 0);
        }
        (self.tex_w / 16, self.tex_h / 16)
    }

    fn visible_cols(&self, glyph_w: u32) -> u32 {
        if self.fb.is_null() || glyph_w == 0 { return 0; }
        let fb = unsafe { &*self.fb };
        (fb.framebuffer_width / glyph_w).min(MAX_COLS as u32)
    }

    fn visible_rows(&self, glyph_h: u32) -> u32 {
        if self.fb.is_null() || glyph_h == 0 { return 0; }
        let fb = unsafe { &*self.fb };
        (fb.framebuffer_height / glyph_h).min(MAX_ROWS as u32)
    }

    pub fn clear(&mut self) {
        if self.fb.is_null() { return; }
        self.hide_cursor();
        let fb = unsafe { &*self.fb };
        let size = (fb.framebuffer_height * fb.framebuffer_pitch) as usize;
        unsafe {
            core::ptr::write_bytes(fb.framebuffer_addr as *mut u8, 0, size);
        }
        self.buf = [0; MAX_ROWS * MAX_COLS];
        self.col = 0;
        self.row = 0;
        self.decoder = Utf8Decoder::new();
    }

    fn scroll_up(&mut self) {
        if self.fb.is_null() { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let fb = unsafe { &*self.fb };
        let rows = self.visible_rows(glyph_h);
        
        for r in 0..(rows as usize - 1) {
            let src_idx = (r + 1) * MAX_COLS;
            let dst_idx = r * MAX_COLS;
            for c in 0..MAX_COLS {
                self.buf[dst_idx + c] = self.buf[src_idx + c];
            }
        }
        
        let last_row = (rows as usize - 1) * MAX_COLS;
        for c in 0..MAX_COLS {
            self.buf[last_row + c] = 0;
        }

        let fb_w = fb.framebuffer_width;
        let fb_h = fb.framebuffer_height;
        let pitch = fb.framebuffer_pitch;
        let fb_ptr = fb.framebuffer_addr as *mut u32;

        for y in 0..(fb_h - glyph_h) {
            let src_y = y + glyph_h;
            let dst_y = y;
            for x in 0..fb_w {
                let src_idx = (src_y * pitch / 4) as usize + x as usize;
                let dst_idx = (dst_y * pitch / 4) as usize + x as usize;
                unsafe {
                    let color = *fb_ptr.add(src_idx);
                    *fb_ptr.add(dst_idx) = color;
                }
            }
        }

        for y in (fb_h - glyph_h)..fb_h {
            for x in 0..fb_w {
                put_pixel(fb, x, y, 0x00000000);
            }
        }

        for r in 0..rows {
            let row_idx = r as usize * MAX_COLS;
            for c in 0..MAX_COLS {
                let ch = self.buf[row_idx + c];
                if ch != 0 {
                    self.draw_glyph_at(c as u32, r, ch);
                }
            }
        }
    }

    fn clear_cell(&self, col: u32, row: u32) {
        if self.fb.is_null() { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }
        let fb = unsafe { &*self.fb };
        let px = col * glyph_w;
        let py = row * glyph_h;
        for cy in 0..glyph_h {
            for cx in 0..glyph_w {
                if px + cx < fb.framebuffer_width && py + cy < fb.framebuffer_height {
                    put_pixel(fb, px + cx, py + cy, 0x00000000);
                }
            }
        }
    }

    fn draw_glyph_at(&self, col: u32, row: u32, byte: u8) {
        if self.fb.is_null() || self.tex.is_null() { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let fb = unsafe { &*self.fb };
        let px = col * glyph_w;
        let py = row * glyph_h;

        let map_x = ((byte as u32) % 16) * glyph_w;
        let map_y = ((byte as u32) / 16) * glyph_h;

        for cy in 0..glyph_h {
            for cx in 0..glyph_w {
                if px + cx < fb.framebuffer_width && py + cy < fb.framebuffer_height {
                    let tex_idx = ((map_y + cy) * self.tex_w + (map_x + cx)) as usize;
                    let color = unsafe { *self.tex.add(tex_idx) };
                    put_pixel(fb, px + cx, py + cy, color);
                }
            }
        }
    }

    fn redraw_all(&mut self) {
        if self.fb.is_null() { return; }
        self.hide_cursor();
        let fb = unsafe { &*self.fb };
        let size = (fb.framebuffer_height * fb.framebuffer_pitch) as usize;
        unsafe {
            core::ptr::write_bytes(fb.framebuffer_addr as *mut u8, 0, size);
        }

        let rows = self.visible_rows(self.glyph_dims().1);
        for r in 0..rows.min(MAX_ROWS as u32) {
            for c in 0..MAX_COLS {
                let ch = self.buf[r as usize * MAX_COLS + c];
                if ch != 0 {
                    self.draw_glyph_at(c as u32, r, ch);
                }
            }
        }
    }

    pub fn set_font(&mut self, tex: *const u32, tex_w: u32, tex_h: u32) {
        self.hide_cursor();
        self.tex = tex;
        self.tex_w = tex_w;
        self.tex_h = tex_h;
        self.redraw_all();
    }

    fn invert_cursor_pixels(&mut self) {
        if self.fb.is_null() || self.tex_w == 0 { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let fb = unsafe { &*self.fb };
        let stride = (fb.framebuffer_pitch / 4) as usize;
        let fb_ptr = fb.framebuffer_addr as *mut u32;

        let base_x = self.col * glyph_w;
        let base_y = self.row * glyph_h;

        for cy in (glyph_h.saturating_sub(2))..glyph_h {
            for cx in 0..glyph_w {
                let px = base_x + cx;
                let py = base_y + cy;
                if px < fb.framebuffer_width && py < fb.framebuffer_height {
                    let idx = (py as usize) * stride + (px as usize);
                    unsafe {
                        let current_color = *fb_ptr.add(idx);
                        *fb_ptr.add(idx) = current_color ^ 0x00FFFFFF;
                    }
                }
            }
        }
    }

    pub fn hide_cursor(&mut self) {
        if self.cursor_active {
            self.invert_cursor_pixels();
            self.cursor_active = false;
        }
    }

    pub fn show_cursor(&mut self) {
        if is_gfx_mode() {
            return;
        }
        if !self.cursor_active {
            self.invert_cursor_pixels();
            self.cursor_active = true;
        }
    }

    pub fn toggle_cursor(&mut self) {
        if is_gfx_mode() {
            if self.cursor_active {
                self.hide_cursor();
            }
            return;
        }
        if self.cursor_active {
            self.hide_cursor();
        } else {
            self.show_cursor();
        }
    }

    pub fn draw_cursor(&mut self, visible: bool) {
        if visible {
            self.show_cursor();
        } else {
            self.hide_cursor();
        }
    }

    pub fn backspace(&mut self) {
        if is_gfx_mode() || self.fb.is_null() || self.tex_w == 0 { return; }

        self.hide_cursor();

        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            let (glyph_w, _) = self.glyph_dims();
            self.col = self.visible_cols(glyph_w).saturating_sub(1);
        } else {
            return;
        }

        if (self.row as usize) < MAX_ROWS && (self.col as usize) < MAX_COLS {
            self.buf[self.row as usize * MAX_COLS + self.col as usize] = 0;
        }
        self.clear_cell(self.col, self.row);
    }

    /// Write a UTF-8 character (handles multi-byte sequences)
    pub fn write_char(&mut self, ch: char) {
        if is_gfx_mode() || self.fb.is_null() || self.tex.is_null() || self.tex_w == 0 {
            return;
        }

        self.hide_cursor();

        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let cols = self.visible_cols(glyph_w);
        let rows = self.visible_rows(glyph_h);

        if ch == '\n' {
            self.col = 0;
            self.row += 1;
            if self.row >= rows {
                self.scroll_up();
                self.row = rows - 1;
            }
            return;
        }

        if ch == '\x08' {
            self.backspace();
            return;
        }

        // Only handle ASCII for now (font only has ASCII glyphs)
        // For Unicode, we could render as replacement char or skip
        if ch as u32 <= 0x7F {
            let byte = ch as u8;
            if (self.row as usize) < MAX_ROWS && (self.col as usize) < MAX_COLS {
                self.buf[self.row as usize * MAX_COLS + self.col as usize] = byte;
            }
            self.draw_glyph_at(self.col, self.row, byte);

            self.col += 1;
            if self.col >= cols {
                self.col = 0;
                self.row += 1;
                if self.row >= rows {
                    self.scroll_up();
                    self.row = rows - 1;
                }
            }
        } else {
            // Unicode character not in font - draw as '?'
            if (self.row as usize) < MAX_ROWS && (self.col as usize) < MAX_COLS {
                self.buf[self.row as usize * MAX_COLS + self.col as usize] = b'?';
            }
            self.draw_glyph_at(self.col, self.row, b'?');
            self.col += 1;
            if self.col >= cols {
                self.col = 0;
                self.row += 1;
                if self.row >= rows {
                    self.scroll_up();
                    self.row = rows - 1;
                }
            }
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        if is_gfx_mode() || self.fb.is_null() || self.tex.is_null() || self.tex_w == 0 {
            return;
        }

        // Try to decode UTF-8 sequence
        if let Some(ch) = self.decoder.decode(byte) {
            self.write_char(ch);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        if is_gfx_mode() { return; }
        for ch in s.chars() {
            self.write_char(ch);
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if is_gfx_mode() {
        return;
    }
    use core::fmt::Write;
    unsafe {
        CONSOLE.write_fmt(args).unwrap();
    }
}