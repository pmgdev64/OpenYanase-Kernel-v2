// src/graphics/surface.rs
use crate::gop::Color;

#[derive(Clone, Copy)]
pub struct Surface {
    pub pixels: *mut u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
}

impl Surface {
    pub fn new(pixels: *mut u32, width: u32, height: u32, pitch: u32, bpp: u8) -> Self {
        Self { pixels, width, height, pitch, bpp }
    }

    #[inline]
    pub fn put_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height && !self.pixels.is_null() {
            let stride = (self.pitch / 4) as usize;
            let idx = (y as usize * stride) + x as usize;
            unsafe {
                core::ptr::write(self.pixels.add(idx), color.0);
            }
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x < self.width && y < self.height && !self.pixels.is_null() {
            let stride = (self.pitch / 4) as usize;
            let idx = (y as usize * stride) + x as usize;
            unsafe {
                Color(core::ptr::read(self.pixels.add(idx)))
            }
        } else {
            Color::BLACK
        }
    }

    pub fn fill(&mut self, color: Color) {
        if self.pixels.is_null() { return; }
        let stride = (self.pitch / 4) as usize;
        let total_pixels = stride * self.height as usize;
        unsafe {
            let ptr = self.pixels;
            for i in 0..total_pixels {
                core::ptr::write(ptr.add(i), color.0);
            }
        }
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: Color) {
        if self.pixels.is_null() || x >= self.width || y >= self.height { return; }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        let actual_w = (max_x - x) as usize;
        let stride = (self.pitch / 4) as usize;

        for dy in y..max_y {
            let row_start = (dy as usize * stride) + x as usize;
            unsafe {
                let ptr = self.pixels.add(row_start);
                for dx in 0..actual_w {
                    core::ptr::write(ptr.add(dx), color.0);
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: Color) {
        if w == 0 || h == 0 { return; }
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h - 1, w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w - 1, y, 1, h, color);
    }

    pub fn draw_line(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, color: Color) {
        let mut x1 = x1 as i32;
        let mut y1 = y1 as i32;
        let x2 = x2 as i32;
        let y2 = y2 as i32;

        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put_pixel(x1 as u32, y1 as u32, color);
            if x1 == x2 && y1 == y2 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x1 += sx;
            }
            if e2 <= dx {
                err += dx;
                y1 += sy;
            }
        }
    }

    pub fn draw_circle(&mut self, cx: u32, cy: u32, radius: u32, color: Color) {
        let mut x = 0;
        let mut y = radius as i32;
        let mut d = 3 - 2 * radius as i32;

        while x <= y {
            self.put_pixel(cx + x as u32, cy + y as u32, color);
            self.put_pixel(cx - x as u32, cy + y as u32, color);
            self.put_pixel(cx + x as u32, cy - y as u32, color);
            self.put_pixel(cx - x as u32, cy - y as u32, color);
            self.put_pixel(cx + y as u32, cy + x as u32, color);
            self.put_pixel(cx - y as u32, cy + x as u32, color);
            self.put_pixel(cx + y as u32, cy - x as u32, color);
            self.put_pixel(cx - y as u32, cy - x as u32, color);

            if d < 0 {
                d = d + 4 * x + 6;
            } else {
                d = d + 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
    }

    pub fn fill_circle(&mut self, cx: u32, cy: u32, radius: u32, color: Color) {
        for y in 0..=radius {
            let dy_sq = radius * radius - y * y;
            let mut dx = 0;
            while dx * dx <= dy_sq {
                dx += 1;
            }
            let dx = dx - 1;

            for x in 0..=dx {
                self.put_pixel(cx + x, cy + y, color);
                self.put_pixel(cx - x, cy + y, color);
                self.put_pixel(cx + x, cy - y, color);
                self.put_pixel(cx - x, cy - y, color);
            }
        }
    }

    pub fn copy_from(&mut self, src: &Surface) {
        if self.pixels.is_null() || src.pixels.is_null() { return; }
        let size = (self.height * (self.pitch / 4)) as usize;
        unsafe {
            core::ptr::copy_nonoverlapping(src.pixels, self.pixels, size);
        }
    }

    /// Blit chỉ một vùng chữ nhật (dùng để chỉ cập nhật vùng cursor thay vì full-frame)
    pub fn copy_rect_from(&mut self, src: &Surface, x: u32, y: u32, w: u32, h: u32) {
        if self.pixels.is_null() || src.pixels.is_null() { return; }
        let max_x = (x + w).min(self.width).min(src.width);
        let max_y = (y + h).min(self.height).min(src.height);
        if x >= max_x || y >= max_y { return; }

        let actual_w = (max_x - x) as usize;
        let dst_stride = (self.pitch / 4) as usize;
        let src_stride = (src.pitch / 4) as usize;

        unsafe {
            for row in y..max_y {
                let dst_row = self.pixels.add(row as usize * dst_stride + x as usize);
                let src_row = src.pixels.add(row as usize * src_stride + x as usize);
                core::ptr::copy_nonoverlapping(src_row, dst_row, actual_w);
            }
        }
    }

    pub fn draw_cursor(&mut self, cx: u32, cy: u32) {
        #[rustfmt::skip]
        const CURSOR: [&str; 19] = [
            "B           ",
            "BB          ",
            "BWB         ",
            "BWWB        ",
            "BWWWB       ",
            "BWWWWB      ",
            "BWWWWWB     ",
            "BWWWWWWB    ",
            "BWWWWWWWB   ",
            "BWWWWWWWWB  ",
            "BWWWWWBBBBB ",
            "BWWBWWB     ",
            "BWB BWWB    ",
            "BB   BWWB   ",
            "B     BWWB  ",
            "       BWWB ",
            "        BWB ",
            "        BB  ",
            "         B  ",
        ];

        let scale: u32 = 1;
        for (row, line) in CURSOR.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let color = match ch {
                    'B' => Some(Color::BLACK),
                    'W' => Some(Color::WHITE),
                    _ => None,
                };
                if let Some(c) = color {
                    let px = cx + (col as u32 * scale);
                    let py = cy + (row as u32 * scale);
                    if scale == 1 {
                        self.put_pixel(px, py, c);
                    } else {
                        self.fill_rect(px, py, scale, scale, c);
                    }
                }
            }
        }
    }
}