// src/graphics/font.rs
use crate::graphics::surface::Surface;
use crate::gop::Color;

/// Trộn màu fg lên bg theo alpha 0..=255 (kết quả opaque 0xFFRRGGBB).
/// Dùng chung cho console, gfx (Font::draw_char) và splash.
#[inline(always)]
pub fn blend_rgb(fg: u32, bg: u32, a: u32) -> u32 {
    if a == 0 {
        return bg;
    }
    if a >= 255 {
        return fg;
    }
    let inv = 255 - a;
    let mix = |sh: u32| -> u32 {
        let t = ((fg >> sh) & 0xFF) * a + ((bg >> sh) & 0xFF) * inv + 128;
        (t + (t >> 8)) >> 8 // ~ t / 255, làm tròn
    };
    0xFF00_0000 | (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

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
        // Sanitize: glyph dims phải chia hết cho lưới 16x16.
        // Nếu tex_width < 16 (do corruption hoặc chưa init), fallback 128x256.
        let safe_w = if tex_width >= 16 { (tex_width / 16) * 16 } else { 128 };
        let safe_h = if tex_height >= 16 { (tex_height / 16) * 16 } else { 256 };

        let glyph_width = safe_w / 16;
        let glyph_height = safe_h / 16;

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
            tex_width: safe_w,
            tex_height: safe_h,
            glyph_width,
            glyph_height,
        }
    }

    pub fn draw_char(&self, surface: &mut Surface, ch: u8, x: u32, y: u32, color: Color) {
        if self.texture.is_null() || surface.pixels.is_null() {
            return;
        }

        // FIX: toạ độ ngoài màn hình (x/y rác, hoặc số âm bị cast sang u32)
        // thì bỏ qua cả ký tự, không cộng để tránh overflow panic.
        if x >= surface.width || y >= surface.height {
            return;
        }

        let glyph = &self.glyphs[ch as usize];
        if glyph.width == 0 || glyph.height == 0 {
            return;
        }

        let tex_stride = self.tex_width as usize;
        let tex_total = (self.tex_width as usize).saturating_mul(self.tex_height as usize);
        if tex_total == 0 {
            return;
        }

        for row in 0..glyph.height {
            let py = match y.checked_add(row) {
                Some(v) if v < surface.height => v,
                _ => break,
            };
            let tex_row_base = ((glyph.y + row) as usize).saturating_mul(tex_stride);

            for col in 0..glyph.width {
                let px = match x.checked_add(col) {
                    Some(v) if v < surface.width => v,
                    _ => break,
                };

                let tex_idx = tex_row_base.wrapping_add((glyph.x + col) as usize);
                if tex_idx >= tex_total {
                    continue;
                }

                let pixel = unsafe { core::ptr::read_volatile(self.texture.add(tex_idx)) };

                // Atlas lưu coverage ở kênh alpha: PSF = 0 hoặc 255, YFN = 0..255.
                let a = pixel >> 24;
                if a == 0 {
                    continue;
                }
                if a >= 255 {
                    surface.put_pixel(px, py, color);
                } else {
                    let bg = surface.get_pixel(px, py).0;
                    surface.put_pixel(px, py, Color(blend_rgb(color.0, bg, a)));
                }
            }
        }
    }

    pub fn draw_string(&self, surface: &mut Surface, text: &str, x: u32, y: u32, color: Color) {
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x >= surface.width {
                break;
            }
            if ch == '\n' {
                continue;
            }
            if ch == '\t' {
                cur_x = cur_x.saturating_add(self.glyph_width.saturating_mul(4));
                continue;
            }

            let byte = if (ch as u32) <= 0x7F { ch as u8 } else { b'?' };
            self.draw_char(surface, byte, cur_x, y, color);
            cur_x = cur_x.saturating_add(self.glyph_width);
        }
    }

    pub fn measure_string(&self, text: &str) -> u32 {
        let mut width: u32 = 0;
        for ch in text.chars() {
            if ch == '\n' || ch == '\t' {
                continue;
            }
            width = width.saturating_add(self.glyph_width);
        }
        width
    }
}

// ============================================================
//  bake_psf — auto-detect PSF1/PSF2, bake vào texture atlas
//
//  PSF1: magic 0x36 0x04, header 4 byte, glyph width cố định 8
//        [0] magic0, [1] magic1, [2] mode, [3] char_size (=height)
//
//  PSF2: magic 0x72 0xB5 0x4A 0x86, header 32 byte
//        [4..8]   version       (u32 LE)
//        [8..12]  header_size   (u32 LE)
//        [12..16] flags         (u32 LE)
//        [16..20] length        (u32 LE) - số glyph
//        [20..24] char_size     (u32 LE) - byte/glyph
//        [24..28] height        (u32 LE) - pixel
//        [28..32] width         (u32 LE) - pixel
//
//  Layout texture: 16 cột × 16 hàng glyph (256 glyph max)
//  Trả về (tex_w, tex_h), (0, 0) nếu lỗi
// ============================================================
pub fn bake_psf(psf_data: &[u8], out_buffer: &mut [u32], color: Color) -> (u32, u32) {
    use crate::serial;

    if psf_data.len() < 4 {
        serial::serial_write_str("FONT: Data too small\r\n");
        return (0, 0);
    }

    // YFN1: font khử răng cưa (8-bit alpha) do tools/ttf2yfn.py sinh ra.
    if psf_data.len() >= 16 && &psf_data[0..4] == b"YFN1" {
        return bake_yfn(psf_data, out_buffer);
    }

    // --- Detect format & extract parameters ---
    let (glyph_width, glyph_height, glyphs_start, bytes_per_glyph, bytes_per_row, num_glyphs):
        (usize, usize, usize, usize, usize, usize);

    if psf_data[0] == 0x36 && psf_data[1] == 0x04 {
        // ============ PSF1 ============
        if psf_data.len() < 4 { return (0, 0); }
        let h = psf_data[3] as usize;
        if h == 0 || h > 32 {
            serial::serial_write_str("FONT: PSF1 bad height\r\n");
            return (0, 0);
        }
        glyph_width = 8;
        glyph_height = h;
        glyphs_start = 4;
        bytes_per_row = 1;
        bytes_per_glyph = h;
        num_glyphs = 256;
        serial::serial_write_str("FONT: PSF1 detected\r\n");

    } else if psf_data[0] == 0x72 && psf_data[1] == 0xB5
           && psf_data[2] == 0x4A && psf_data[3] == 0x86 {
        // ============ PSF2 ============
        if psf_data.len() < 32 {
            serial::serial_write_str("FONT: PSF2 header truncated\r\n");
            return (0, 0);
        }
        let version     = u32::from_le_bytes([psf_data[4], psf_data[5], psf_data[6], psf_data[7]]);
        let header_size = u32::from_le_bytes([psf_data[8], psf_data[9], psf_data[10], psf_data[11]]);
        let _flags      = u32::from_le_bytes([psf_data[12], psf_data[13], psf_data[14], psf_data[15]]);
        let length      = u32::from_le_bytes([psf_data[16], psf_data[17], psf_data[18], psf_data[19]]);
        let char_size   = u32::from_le_bytes([psf_data[20], psf_data[21], psf_data[22], psf_data[23]]);
        let h           = u32::from_le_bytes([psf_data[24], psf_data[25], psf_data[26], psf_data[27]]);
        let w           = u32::from_le_bytes([psf_data[28], psf_data[29], psf_data[30], psf_data[31]]);

        if version != 0 {
            serial::serial_write_str("FONT: PSF2 unsupported version\r\n");
            return (0, 0);
        }
        if header_size < 32 || (header_size as usize) > psf_data.len() {
            serial::serial_write_str("FONT: PSF2 bad header_size\r\n");
            return (0, 0);
        }
        if w == 0 || w > 32 || h == 0 || h > 32 {
            serial::serial_write_str("FONT: PSF2 bad glyph size\r\n");
            return (0, 0);
        }

        glyph_width = w as usize;
        glyph_height = h as usize;
        glyphs_start = header_size as usize;
        bytes_per_row = (glyph_width + 7) / 8;
        bytes_per_glyph = char_size as usize;

        if bytes_per_glyph < bytes_per_row * glyph_height {
            serial::serial_write_str("FONT: PSF2 char_size too small\r\n");
            return (0, 0);
        }

        num_glyphs = (length.min(256)) as usize;
        serial::serial_write_str("FONT: PSF2 detected\r\n");

    } else {
        serial::serial_write_str("FONT: Unknown PSF format\r\n");
        return (0, 0);
    }

    // --- Texture layout: 16 × 16 grid ---
    let tex_w = 16 * glyph_width;
    let tex_h = 16 * glyph_height;
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

    // --- Bake glyphs ---
    for glyph_idx in 0..num_glyphs {
        let char_offset = glyphs_start + glyph_idx * bytes_per_glyph;
        if char_offset + bytes_per_glyph > psf_data.len() {
            break;
        }

        let char_x = (glyph_idx % 16) * glyph_width;
        let char_y = (glyph_idx / 16) * glyph_height;

        for row in 0..glyph_height {
            let row_base = char_offset + row * bytes_per_row;

            for col in 0..glyph_width {
                let byte_idx = row_base + col / 8;
                if byte_idx >= psf_data.len() {
                    continue;
                }
                let bit_idx = 7 - (col % 8);
                let bit = (psf_data[byte_idx] >> bit_idx) & 1;

                if bit != 0 {
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

    serial::serial_write_str("FONT: Bake OK\r\n");
    (tex_w as u32, tex_h as u32)
}

// ============================================================
//  bake_yfn — YFN1: header 16 byte rồi N glyph, mỗi glyph w*h byte alpha.
//    [0..4) "YFN1"  [4..8) width  [8..12) height  [12..16) num_glyphs
//  Atlas: 16×16 glyph, pixel = (alpha << 24) | 0x00FFFFFF
//  Trả về (tex_w, tex_h), (0, 0) nếu lỗi.
// ============================================================
fn bake_yfn(data: &[u8], out_buffer: &mut [u32]) -> (u32, u32) {
    use crate::serial;

    let rd = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
    let w = rd(4) as usize;
    let h = rd(8) as usize;
    let n = (rd(12) as usize).min(256);

    if w == 0 || w > 32 || h == 0 || h > 32 || n == 0 {
        serial::serial_write_str("FONT: YFN1 bad header\r\n");
        return (0, 0);
    }

    let glyph_bytes = w * h;
    if 16 + n * glyph_bytes > data.len() {
        serial::serial_write_str("FONT: YFN1 truncated\r\n");
        return (0, 0);
    }

    let tex_w = 16 * w;
    let tex_h = 16 * h;
    let needed = tex_w * tex_h;
    if needed > out_buffer.len() {
        serial::serial_write_str("FONT: Buffer too small\r\n");
        return (0, 0);
    }

    for i in 0..needed {
        out_buffer[i] = 0x0000_0000;
    }

    for g in 0..n {
        let base = 16 + g * glyph_bytes;
        let gx = (g % 16) * w;
        let gy = (g / 16) * h;
        for y in 0..h {
            for x in 0..w {
                let a = data[base + y * w + x] as u32;
                if a != 0 {
                    out_buffer[(gy + y) * tex_w + gx + x] = (a << 24) | 0x00FF_FFFF;
                }
            }
        }
    }

    serial::serial_write_str("FONT: YFN1 (antialiased) bake OK\r\n");
    (tex_w as u32, tex_h as u32)
}

// ============================================================
//  create_default_font — fallback khi không load được PSF
// ============================================================
pub fn create_default_font(buffer: &mut [u32]) -> (u32, u32) {
    let tex_w: usize = 128;
    let tex_h: usize = 256;

    let needed = tex_w * tex_h;
    if needed > buffer.len() {
        return (0, 0);
    }

    for i in 0..needed {
        buffer[i] = 0x00000000;
    }

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