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