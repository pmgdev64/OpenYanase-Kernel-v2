// src/graphics/font.rs
use crate::graphics::surface::Surface;
use crate::gop::Color;

#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
    pub advance: u32,
}

pub struct Font {
    pub glyphs: [Glyph; 256],
    pub texture: *const u32,
    pub tex_width: u32,
    pub tex_height: u32,
    pub glyph_width: u32,
    pub glyph_height: u32,
}

impl Font {
    pub fn new(texture: *const u32, tex_width: u32, tex_height: u32) -> Self {
        let glyph_width = tex_width / 16;
        let glyph_height = tex_height / 16;
        
        let mut glyphs = [Glyph { width: 0, height: 0, x: 0, y: 0, advance: 0 }; 256];
        
        for i in 0..256 {
            let x = (i as u32 % 16) * glyph_width;
            let y = (i as u32 / 16) * glyph_height;
            glyphs[i] = Glyph {
                width: glyph_width,
                height: glyph_height,
                x,
                y,
                advance: glyph_width,
            };
        }
        
        Self {
            glyphs,
            texture,
            tex_width,
            tex_height,
            glyph_width,
            glyph_height,
        }
    }

    pub fn draw_char(&self, surface: &mut Surface, ch: u8, x: u32, y: u32, color: Color) {
        if self.texture.is_null() || surface.pixels.is_null() { return; }
        
        let glyph = &self.glyphs[ch as usize];
        let tex_stride = self.tex_width;
        
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let tex_idx = ((glyph.y + row) * tex_stride + (glyph.x + col)) as usize;
                let pixel = unsafe { *self.texture.add(tex_idx) };
                
                if pixel != 0x00000000 {
                    surface.put_pixel(x + col, y + row, color);
                }
            }
        }
    }

    pub fn draw_string(&self, surface: &mut Surface, text: &str, x: u32, y: u32, color: Color) {
        let mut cur_x = x;
        for ch in text.chars() {
            if ch == '\n' { continue; }
            if ch == '\t' { cur_x += self.glyph_width * 4; continue; }
            
            let byte = if ch as u32 <= 0x7F { ch as u8 } else { b'?' };
            self.draw_char(surface, byte, cur_x, y, color);
            cur_x += self.glyph_width;
        }
    }

    pub fn measure_string(&self, text: &str) -> u32 {
        let mut width = 0;
        for ch in text.chars() {
            if ch == '\n' || ch == '\t' { continue; }
            width += self.glyph_width;
        }
        width
    }
}

/// Bake PSF1 font (8x16) to texture - DÙNG usize CHO TẤT CẢ
pub fn bake_psf_to_texture(psf_data: &[u8], out_buffer: &mut [u32], color: Color) -> (u32, u32) {
    use crate::serial;
    
    if psf_data.len() < 4 {
        serial::serial_write_str("FONT: PSF data too small\r\n");
        return (0, 0);
    }
    
    if psf_data[0] != 0x36 || psf_data[1] != 0x04 {
        serial::serial_write_str("FONT: Not PSF1 format\r\n");
        return (0, 0);
    }
    
    let glyph_height = psf_data[3] as u32;
    if glyph_height == 0 {
        return (0, 0);
    }
    
    // TẤT CẢ ĐỀU DÙNG usize
    let glyph_width: usize = 8;
    let glyph_height_us: usize = glyph_height as usize;
    let tex_w: usize = 128; // 16 * 8
    let tex_h: usize = glyph_height_us * 16;
    
    let needed = tex_w * tex_h;
    if needed > out_buffer.len() {
        serial::serial_write_str("FONT: Buffer too small\r\n");
        return (0, 0);
    }
    
    // Clear buffer
    for i in 0..needed {
        out_buffer[i] = 0x00000000;
    }
    
    let color_val = color.0;
    let glyphs_start: usize = 4;
    let bytes_per_glyph = glyph_height_us;
    
    for glyph_idx in 0..256 {
        let char_offset = glyphs_start + (glyph_idx * bytes_per_glyph);
        if char_offset + bytes_per_glyph > psf_data.len() {
            break;
        }
        
        let char_x = (glyph_idx % 16) * glyph_width;
        let char_y = (glyph_idx / 16) * glyph_height_us;
        
        for row in 0..glyph_height_us {
            let byte = psf_data[char_offset + row];
            for col in 0..8 {
                if (byte & (1 << (7 - col))) != 0 {
                    let px = char_x + col;
                    let py = char_y + row;
                    let idx = py * tex_w + px;
                    if idx < needed {
                        out_buffer[idx] = color_val;
                    }
                }
            }
        }
    }
    
    serial::serial_write_str("FONT: Baked successfully\r\n");
    (tex_w as u32, tex_h as u32)
}

/// Tạo font mặc định 8x16 - DÙNG usize CHO TẤT CẢ
pub fn create_default_font(buffer: &mut [u32]) -> (u32, u32) {
    let tex_w: usize = 128;
    let tex_h: usize = 256;
    
    let needed = tex_w * tex_h;
    if needed > buffer.len() {
        return (0, 0);
    }
    
    // Clear buffer
    for i in 0..needed {
        buffer[i] = 0x00000000;
    }
    
    // Tạo font đơn giản 8x16
    for ch in 32..127 {
        let char_x = ((ch % 16) * 8) as usize;
        let char_y = ((ch / 16) * 16) as usize;
        
        for y in 0..16 {
            for x in 0..8 {
                let idx = (char_y + y) * tex_w + (char_x + x);
                if idx < needed {
                    if y == 0 || y == 15 || x == 0 || x == 7 {
                        buffer[idx] = 0xFFFFFFFF;
                    } else if y > 2 && y < 14 && x > 1 && x < 6 {
                        buffer[idx] = 0xFFFFFFFF;
                    }
                }
            }
        }
    }
    
    (tex_w as u32, tex_h as u32)
}