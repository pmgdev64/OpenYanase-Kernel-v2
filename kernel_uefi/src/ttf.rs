// src/ttf.rs
extern crate alloc;

use ab_glyph::{FontRef, Font, Scale, point};
use crate::graphics::surface::Surface;
use crate::gop::Color;

#[derive(Copy, Clone, Default)]
pub struct GlyphMetrics {
    pub tex_x: u32,
    pub tex_y: u32,
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub advance_x: f32,
}

pub struct TtfAtlas {
    pub tex_w: u32,
    pub tex_h: u32,
    pub metrics: [GlyphMetrics; 256],
}

impl TtfAtlas {
    /// Đọc byte array của file .ttf và nướng các ký tự ASCII (32-255) vào một Texture Atlas (buffer 1D).
    pub unsafe fn bake(
        ttf_data: &[u8],
        font_size: f32,
        atlas_buffer: &mut [u32],
        tex_w: u32,
        tex_h: u32,
    ) -> Option<Self> {
        // Parse font từ mảng byte (yêu cầu bộ Global Allocator đã được khởi tạo trong kernel)
        let font = FontRef::try_from_slice(ttf_data).ok()?;
        let scale = Scale::uniform(font_size);
        
        let mut atlas = Self {
            tex_w,
            tex_h,
            metrics: [GlyphMetrics::default(); 256],
        };

        // Clear atlas buffer (trong suốt hoàn toàn: Alpha = 0)
        for i in 0..(tex_w * tex_h) as usize {
            atlas_buffer[i] = 0x00000000;
        }

        let mut current_x = 0;
        let mut current_y = 0;
        let line_height = font_size as u32 + 8; // Padding dọc an toàn

        for ch_u32 in 32..255 {
            let ch = core::char::from_u32(ch_u32).unwrap_or('?');
            let glyph = font.glyph_id(ch).with_scale_and_position(scale, point(0.0, 0.0));
            
            if let Some(outlined) = font.outline_glyph(glyph.clone()) {
                let bounds = outlined.px_bounds();
                let g_width = bounds.width() as u32;
                let g_height = bounds.height() as u32;

                // Xuống hàng trên texture nếu sắp tràn lề ngang
                if current_x + g_width >= tex_w {
                    current_x = 0;
                    current_y += line_height;
                }

                if current_y + g_height >= tex_h {
                    crate::println!("kernel: WARN: TTF Atlas buffer is too small for size {}!", font_size);
                    break; 
                }

                atlas.metrics[ch_u32 as usize] = GlyphMetrics {
                    tex_x: current_x,
                    tex_y: current_y,
                    width: g_width,
                    height: g_height,
                    offset_x: bounds.min.x,
                    offset_y: bounds.min.y,
                    advance_x: font.as_scaled(scale).h_advance(glyph.id),
                };

                // Vẽ coverage (0.0 -> 1.0) vào kênh Alpha của buffer
                outlined.draw(|gx, gy, v| {
                    let px = current_x + gx;
                    let py = current_y + gy;
                    let idx = (py * tex_w + px) as usize;
                    
                    if idx < atlas_buffer.len() {
                        let alpha = (v * 255.0) as u32;
                        // Lưu dưới dạng màu trắng, kênh alpha giữ cường độ nét chữ
                        atlas_buffer[idx] = (alpha << 24) | 0x00FFFFFF; 
                    }
                });

                // Cách ký tự tiếp theo 2px để chống lem viền khi render
                current_x += g_width + 2; 
            } else {
                // Các ký tự rỗng (như dấu cách) không có outline, chỉ lưu advance_x
                atlas.metrics[ch_u32 as usize].advance_x = font.as_scaled(scale).h_advance(glyph.id);
            }
        }
        
        Some(atlas)
    }

    /// Render text trực tiếp từ Atlas buffer lên Surface (hỗ trợ Alpha Blending)
    pub fn draw_text(
        &self,
        atlas_buffer: &[u32],
        surface: &mut Surface,
        text: &str,
        mut x: f32,
        y: f32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let baseline = y;

        for ch in text.chars() {
            let ch_idx = ch as usize;
            if ch_idx >= 256 { continue; }

            let metric = &self.metrics[ch_idx];

            let draw_x = (x + metric.offset_x) as i32;
            let draw_y = (baseline + metric.offset_y) as i32;

            for gy in 0..metric.height {
                for gx in 0..metric.width {
                    let atlas_idx = ((metric.tex_y + gy) * self.tex_w + (metric.tex_x + gx)) as usize;
                    let tex_pixel = atlas_buffer[atlas_idx];
                    
                    let alpha = tex_pixel >> 24;
                    if alpha > 0 { 
                        let target_x = draw_x + gx as i32;
                        let target_y = draw_y + gy as i32;
                        
                        if target_x >= 0 && target_y >= 0 {
                            // Chế độ 1: Alpha Cut-off (Nhanh nhất, không khử răng cưa)
                            // Phù hợp nếu Surface của ông không có hàm get_pixel()
                            if alpha > 128 {
                                surface.put_pixel(target_x as u32, target_y as u32, Color::rgb(r, g, b));
                            }
                            
                            /* Chế độ 2: Alpha Blending (Mượt, khử răng cưa) - Bỏ comment nếu Surface hỗ trợ get_pixel
                            let bg_color = surface.get_pixel(target_x as u32, target_y as u32); // Giả sử trả về u32 (0xAARRGGBB)
                            let inv_alpha = 255 - alpha;
                            
                            let bg_r = (bg_color >> 16) & 0xFF;
                            let bg_g = (bg_color >> 8) & 0xFF;
                            let bg_b = bg_color & 0xFF;
                            
                            let final_r = ((r as u32 * alpha) + (bg_r * inv_alpha)) >> 8;
                            let final_g = ((g as u32 * alpha) + (bg_g * inv_alpha)) >> 8;
                            let final_b = ((b as u32 * alpha) + (bg_b * inv_alpha)) >> 8;
                            
                            surface.put_pixel(target_x as u32, target_y as u32, Color::rgb(final_r as u8, final_g as u8, final_b as u8));
                            */
                        }
                    }
                }
            }
            x += metric.advance_x;
        }
    }
}