// src/console.rs
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering, AtomicU8};

pub static IS_GFX_MODE: AtomicBool = AtomicBool::new(false);
static DISPLAY_MODE: AtomicU8 = AtomicU8::new(0);

const MAX_COLS: usize = 200;
const MAX_ROWS: usize = 160;
const SCROLLBACK_LINES: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Console = 0,
    Graphics = 1,
}

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
            return Some(byte as char);
        }

        if self.state == 0 {
            if byte >= 0xF0 {
                self.state = 1;
                self.codepoint = (byte & 0x07) as u32;
                self.bytes_left = 3;
            } else if byte >= 0xE0 {
                self.state = 1;
                self.codepoint = (byte & 0x0F) as u32;
                self.bytes_left = 2;
            } else if byte >= 0xC0 {
                self.state = 1;
                self.codepoint = (byte & 0x1F) as u32;
                self.bytes_left = 1;
            } else {
                self.state = 0;
                return None;
            }
        } else {
            if byte & 0xC0 != 0x80 {
                self.state = 0;
                return None;
            }
            self.codepoint = (self.codepoint << 6) | ((byte & 0x3F) as u32);
            self.bytes_left -= 1;

            if self.bytes_left == 0 {
                self.state = 0;
                return char::from_u32(self.codepoint);
            }
        }
        None
    }
}

pub struct ScrollbackBuffer {
    pub lines: [[u8; MAX_COLS]; SCROLLBACK_LINES],
    pub count: usize,
    pub head: usize,
    pub tail: usize,
}

impl ScrollbackBuffer {
    pub const fn new() -> Self {
        Self {
            lines: [[0; MAX_COLS]; SCROLLBACK_LINES],
            count: 0,
            head: 0,
            tail: 0,
        }
    }

    pub fn push(&mut self, line: &[u8]) {
        let len = line.len().min(MAX_COLS);
        let idx = self.head % SCROLLBACK_LINES;
        self.lines[idx][..len].copy_from_slice(&line[..len]);
        if len < MAX_COLS {
            self.lines[idx][len] = 0;
        }
        
        self.head += 1;
        if self.count < SCROLLBACK_LINES {
            self.count += 1;
        } else {
            self.tail += 1;
        }
    }

    pub fn get(&self, index: usize) -> &[u8] {
        let idx = (self.tail + index) % SCROLLBACK_LINES;
        let mut len = 0;
        while len < MAX_COLS && self.lines[idx][len] != 0 {
            len += 1;
        }
        &self.lines[idx][..len]
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

pub struct Console {
    pub fb_addr: *mut u32,
    pub fb_width: u32,
    pub fb_height: u32,
    pub fb_pitch: u32,
    pub tex: *const u32,
    pub tex_w: u32,
    pub tex_h: u32,
    pub col: u32,
    pub row: u32,
    pub cursor_active: bool,
    pub scroll_offset: u32,
    pub total_rows: u32,
    pub scrollback: ScrollbackBuffer,
    buf: [u8; MAX_ROWS * MAX_COLS],
    decoder: Utf8Decoder,
    lock: AtomicBool,
    pub fg_color: u32,
    pub bg_color: u32,
}

pub static mut CONSOLE: Console = Console {
    fb_addr: core::ptr::null_mut(),
    fb_width: 0,
    fb_height: 0,
    fb_pitch: 0,
    tex: core::ptr::null(),
    tex_w: 0,
    tex_h: 0,
    col: 0,
    row: 0,
    cursor_active: false,
    scroll_offset: 0,
    total_rows: 0,
    scrollback: ScrollbackBuffer::new(),
    buf: [0; MAX_ROWS * MAX_COLS],
    decoder: Utf8Decoder::new(),
    lock: AtomicBool::new(false),
    fg_color: 0xFFFFFFFF,
    bg_color: 0xFF000000,
};

impl Console {
    pub fn init(&mut self, fb_addr: *mut u32, width: u32, height: u32, pitch: u32, 
                tex: *const u32, tex_w: u32, tex_h: u32) {
        self.fb_addr = fb_addr;
        self.fb_width = width;
        self.fb_height = height;
        self.fb_pitch = pitch;
        self.tex = tex;
        self.tex_w = tex_w;
        self.tex_h = tex_h;
        self.col = 0;
        self.row = 0;
        self.cursor_active = false;
        self.scroll_offset = 0;
        self.total_rows = 0;
        self.fg_color = 0xFFFFFFFF;
        self.bg_color = 0xFF000000;
        self.scrollback = ScrollbackBuffer::new();
        self.buf = [0; MAX_ROWS * MAX_COLS];
        self.decoder = Utf8Decoder::new();
        self.lock = AtomicBool::new(false);
        
        self.clear();
    }

    #[inline(always)]
    pub fn put_pixel(&self, x: u32, y: u32, color: u32) {
        if self.fb_addr.is_null() { return; }
        if x < self.fb_width && y < self.fb_height {
            let offset = (y * (self.fb_pitch / 4)) + x;
            unsafe {
                self.fb_addr.add(offset as usize).write_volatile(color);
            }
        }
    }

    fn glyph_dims(&self) -> (u32, u32) {
        if self.tex.is_null() || self.tex_w == 0 || self.tex_h == 0 {
            return (8, 16);
        }
        let gw = self.tex_w / 16;
        let gh = self.tex_h / 16;
        if gw == 0 || gh == 0 {
            return (8, 16);
        }
        (gw, gh)
    }

    fn visible_cols(&self, glyph_w: u32) -> u32 {
        if self.fb_addr.is_null() || glyph_w == 0 { return 80; }
        (self.fb_width / glyph_w).min(MAX_COLS as u32).max(1)
    }

    fn visible_rows(&self, glyph_h: u32) -> u32 {
        if self.fb_addr.is_null() || glyph_h == 0 { return 25; }
        (self.fb_height / glyph_h).min(MAX_ROWS as u32).max(1)
    }

    pub fn clear(&mut self) {
        if self.fb_addr.is_null() { return; }
        
        while self.lock.compare_exchange(false, true, 
            core::sync::atomic::Ordering::Acquire, 
            core::sync::atomic::Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        
        self.hide_cursor();
        let size = (self.fb_height * (self.fb_pitch / 4)) as usize;
        unsafe {
            let ptr = self.fb_addr as *mut u32;
            for i in 0..size {
                ptr.add(i).write_volatile(self.bg_color);
            }
        }
        self.buf = [0; MAX_ROWS * MAX_COLS];
        self.col = 0;
        self.row = 0;
        self.scroll_offset = 0;
        self.decoder = Utf8Decoder::new();
        
        self.lock.store(false, core::sync::atomic::Ordering::Release);
    }

    fn scroll_up(&mut self) {
        if self.fb_addr.is_null() { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let rows = self.visible_rows(glyph_h);
        if rows == 0 { return; }

        while self.lock.compare_exchange(false, true, 
            core::sync::atomic::Ordering::Acquire, 
            core::sync::atomic::Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }

        let mut line_buf = [0u8; MAX_COLS];
        for c in 0..MAX_COLS {
            line_buf[c] = self.buf[c];
        }
        self.scrollback.push(&line_buf);
        self.total_rows += 1;

        for r in 0..(rows as usize - 1) {
            let src_idx = (r + 1) * MAX_COLS;
            let dst_idx = r * MAX_COLS;
            self.buf.copy_within(src_idx..src_idx + MAX_COLS, dst_idx);
        }

        let last_row = (rows as usize - 1) * MAX_COLS;
        self.buf[last_row..last_row + MAX_COLS].fill(0);

        let stride = (self.fb_pitch / 4) as usize;
        
        for y in 0..(self.fb_height - glyph_h) {
            let src_y = y + glyph_h;
            let dst_y = y;
            for x in 0..self.fb_width {
                let src_idx = (src_y as usize * stride) + x as usize;
                let dst_idx = (dst_y as usize * stride) + x as usize;
                unsafe {
                    let color = *self.fb_addr.add(src_idx);
                    *self.fb_addr.add(dst_idx) = color;
                }
            }
        }

        for y in (self.fb_height - glyph_h)..self.fb_height {
            for x in 0..self.fb_width {
                self.put_pixel(x, y, self.bg_color);
            }
        }

        let visible_rows = rows.min(MAX_ROWS as u32);
        for r in 0..visible_rows {
            let row_idx = r as usize * MAX_COLS;
            for c in 0..MAX_COLS {
                let ch = self.buf[row_idx + c];
                if ch != 0 {
                    self.draw_glyph_at(c as u32, r, ch);
                }
            }
        }
        
        self.row = rows - 1;
        
        self.lock.store(false, core::sync::atomic::Ordering::Release);
    }

    fn clear_cell(&self, col: u32, row: u32) {
        if self.fb_addr.is_null() { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }
        let px = col * glyph_w;
        let py = row * glyph_h;
        for cy in 0..glyph_h {
            for cx in 0..glyph_w {
                self.put_pixel(px + cx, py + cy, self.bg_color);
            }
        }
    }

    fn draw_glyph_at(&self, col: u32, row: u32, byte: u8) {
        if self.fb_addr.is_null() || self.tex.is_null() { 
            return; 
        }
        
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { 
            return; 
        }

        let px = col * glyph_w;
        let py = row * glyph_h;

        let map_x = ((byte as u32) % 16) * glyph_w;
        let map_y = ((byte as u32) / 16) * glyph_h;

        for cy in 0..glyph_h {
            for cx in 0..glyph_w {
                if px + cx < self.fb_width && py + cy < self.fb_height {
                    let tex_idx = ((map_y + cy) * self.tex_w + (map_x + cx)) as usize;
                    let max_idx = (self.tex_w * self.tex_h) as usize;
                    
                    if tex_idx < max_idx {
                        let color = unsafe { *self.tex.add(tex_idx) };
                        if color != 0x00000000 {
                            self.put_pixel(px + cx, py + cy, self.fg_color);
                        } else {
                            self.put_pixel(px + cx, py + cy, self.bg_color);
                        }
                    }
                }
            }
        }
    }

    pub fn redraw_all(&mut self) {
        if self.fb_addr.is_null() { return; }
        
        while self.lock.compare_exchange(false, true, 
            core::sync::atomic::Ordering::Acquire, 
            core::sync::atomic::Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        
        self.hide_cursor();
        let size = (self.fb_height * (self.fb_pitch / 4)) as usize;
        unsafe {
            let ptr = self.fb_addr as *mut u32;
            for i in 0..size {
                ptr.add(i).write_volatile(self.bg_color);
            }
        }

        let (_, glyph_h) = self.glyph_dims();
        let rows = self.visible_rows(glyph_h);
        for r in 0..rows.min(MAX_ROWS as u32) {
            for c in 0..MAX_COLS {
                let ch = self.buf[r as usize * MAX_COLS + c];
                if ch != 0 {
                    self.draw_glyph_at(c as u32, r, ch);
                }
            }
        }
        
        self.lock.store(false, core::sync::atomic::Ordering::Release);
    }

    pub fn set_font(&mut self, tex: *const u32, tex_w: u32, tex_h: u32) {
        self.hide_cursor();
        if !tex.is_null() && tex_w > 0 && tex_h > 0 {
            self.tex = tex;
            self.tex_w = tex_w;
            self.tex_h = tex_h;
        }
        self.redraw_all();
    }

    fn invert_cursor_pixels(&mut self) {
        if self.fb_addr.is_null() || self.tex_w == 0 { return; }
        let (glyph_w, glyph_h) = self.glyph_dims();
        if glyph_w == 0 || glyph_h == 0 { return; }

        let stride = (self.fb_pitch / 4) as usize;
        let base_x = self.col * glyph_w;
        let base_y = self.row * glyph_h;

        let cursor_y = base_y + glyph_h - 2;
        for cx in 0..glyph_w {
            let px = base_x + cx;
            if px < self.fb_width && cursor_y < self.fb_height {
                let idx = (cursor_y as usize) * stride + (px as usize);
                unsafe {
                    let current_color = *self.fb_addr.add(idx);
                    *self.fb_addr.add(idx) = current_color ^ 0x00FFFFFF;
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
        if is_gfx_mode() { return; }
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

    pub fn backspace(&mut self) {
        if is_gfx_mode() || self.fb_addr.is_null() || self.tex_w == 0 { return; }

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

    pub fn write_char(&mut self, ch: char) {
        if is_gfx_mode() || self.fb_addr.is_null() || self.tex.is_null() || self.tex_w == 0 {
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

        if ch == '\t' {
            let tab_size = 4;
            let spaces = tab_size - (self.col % tab_size);
            for _ in 0..spaces {
                self.write_char(' ');
            }
            return;
        }

        let byte = if ch as u32 <= 0x7F { ch as u8 } else { b'?' };
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
    }

    pub fn write_byte(&mut self, byte: u8) {
        if is_gfx_mode() || self.fb_addr.is_null() || self.tex.is_null() || self.tex_w == 0 {
            return;
        }
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

    pub fn scroll_up_offset(&mut self, lines: u32) {
        if self.scroll_offset + lines > self.scrollback.len() as u32 {
            self.scroll_offset = self.scrollback.len() as u32;
        } else {
            self.scroll_offset += lines;
        }
        self.redraw_with_scroll();
    }

    pub fn scroll_down_offset(&mut self, lines: u32) {
        if lines > self.scroll_offset {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset -= lines;
        }
        self.redraw_with_scroll();
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.redraw_all();
    }

    fn redraw_with_scroll(&mut self) {
        if self.fb_addr.is_null() { return; }
        
        while self.lock.compare_exchange(false, true, 
            core::sync::atomic::Ordering::Acquire, 
            core::sync::atomic::Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        
        self.hide_cursor();
        let size = (self.fb_height * (self.fb_pitch / 4)) as usize;
        unsafe {
            let ptr = self.fb_addr as *mut u32;
            for i in 0..size {
                ptr.add(i).write_volatile(self.bg_color);
            }
        }

        let (_, glyph_h) = self.glyph_dims();
        let rows = self.visible_rows(glyph_h);
        let scroll_start = self.scrollback.len() as u32 - self.scroll_offset;
        
        for r in 0..rows.min(MAX_ROWS as u32) {
            let line_idx = (scroll_start + r) as usize;
            let line = if line_idx < self.scrollback.len() {
                self.scrollback.get(line_idx)
            } else {
                &[]
            };
            
            for c in 0..MAX_COLS {
                let ch = if c < line.len() { line[c] } else { 0 };
                if ch != 0 {
                    self.draw_glyph_at(c as u32, r, ch);
                }
            }
        }
        
        self.lock.store(false, core::sync::atomic::Ordering::Release);
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub fn get_display_mode() -> DisplayMode {
    match DISPLAY_MODE.load(Ordering::Relaxed) {
        0 => DisplayMode::Console,
        1 => DisplayMode::Graphics,
        _ => DisplayMode::Console,
    }
}

pub fn set_display_mode(mode: DisplayMode) {
    DISPLAY_MODE.store(mode as u8, Ordering::Relaxed);
}

#[repr(C)]
pub struct ConsoleState {
    pub fb_addr: *mut u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub tex: *const u32,
    pub tex_w: u32,
    pub tex_h: u32,
    pub col: u32,
    pub row: u32,
    pub cursor_active: bool,
    pub scroll_offset: u32,
    pub total_rows: u32,
    pub fg_color: u32,
    pub bg_color: u32,
    pub buf: [u8; MAX_ROWS * MAX_COLS],
    pub saved: bool,
}

impl ConsoleState {
    pub const fn new() -> Self {
        Self {
            fb_addr: core::ptr::null_mut(),
            width: 0,
            height: 0,
            pitch: 0,
            tex: core::ptr::null(),
            tex_w: 0,
            tex_h: 0,
            col: 0,
            row: 0,
            cursor_active: false,
            scroll_offset: 0,
            total_rows: 0,
            fg_color: 0xFFFFFFFF,
            bg_color: 0xFF000000,
            buf: [0; MAX_ROWS * MAX_COLS],
            saved: false,
        }
    }
}

pub static mut CONSOLE_STATE: ConsoleState = ConsoleState::new();

pub unsafe fn save_console_state() {
    let console = &CONSOLE;
    let state = &mut CONSOLE_STATE;
    
    state.fb_addr = console.fb_addr;
    state.width = console.fb_width;
    state.height = console.fb_height;
    state.pitch = console.fb_pitch;
    state.tex = console.tex;
    state.tex_w = console.tex_w;
    state.tex_h = console.tex_h;
    state.col = console.col;
    state.row = console.row;
    state.cursor_active = console.cursor_active;
    state.scroll_offset = console.scroll_offset;
    state.total_rows = console.total_rows;
    state.fg_color = console.fg_color;
    state.bg_color = console.bg_color;
    state.buf.copy_from_slice(&console.buf);
    state.saved = true;
    
    set_display_mode(DisplayMode::Graphics);
}

pub unsafe fn restore_console_state() {
    if !CONSOLE_STATE.saved {
        return;
    }
    
    let state = &CONSOLE_STATE;
    let console = &mut CONSOLE;
    
    console.fb_addr = state.fb_addr;
    console.fb_width = state.width;
    console.fb_height = state.height;
    console.fb_pitch = state.pitch;
    console.tex = state.tex;
    console.tex_w = state.tex_w;
    console.tex_h = state.tex_h;
    console.col = state.col;
    console.row = state.row;
    console.cursor_active = state.cursor_active;
    console.scroll_offset = state.scroll_offset;
    console.total_rows = state.total_rows;
    console.fg_color = state.fg_color;
    console.bg_color = state.bg_color;
    console.buf.copy_from_slice(&state.buf);
    
    console.redraw_all();
    if console.cursor_active {
        console.show_cursor();
    }
    
    set_display_mode(DisplayMode::Console);
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
    if is_gfx_mode() { return; }
    use core::fmt::Write;
    unsafe {
        CONSOLE.write_fmt(args).unwrap();
    }
}

pub fn set_gfx_mode(enabled: bool) {
    IS_GFX_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_gfx_mode() -> bool {
    IS_GFX_MODE.load(Ordering::Relaxed)
}