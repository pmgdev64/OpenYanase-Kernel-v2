// src/vbe.rs

#[repr(C)]
pub struct Mb2TagFramebuffer {
    pub typ: u32,
    pub size: u32,
    pub framebuffer_addr: u64,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u8,
    pub framebuffer_type: u8,
    pub reserved: u16,
}

pub fn put_pixel(fb: &Mb2TagFramebuffer, x: u32, y: u32, color: u32) {
    if x >= fb.framebuffer_width || y >= fb.framebuffer_height {
        return;
    }
    
    let bpp = fb.framebuffer_bpp as u32;
    if bpp != 32 {
        return; 
    }
    
    let bytes_per_pixel = 4;
    let byte_offset = (y * fb.framebuffer_pitch) + (x * bytes_per_pixel);
    
    unsafe {
        let pixel_ptr = (fb.framebuffer_addr + byte_offset as u64) as *mut u32;
        if !pixel_ptr.is_null() {
            pixel_ptr.write_volatile(color);
        }
    }
}


pub fn bake_psf_to_texture(font_data: &[u8], output_buffer: &mut [u32], fg_color: u32) -> (u32, u32) {
    if font_data.len() < 4 {
        return (0, 0);
    }

    // Nhận diện loại Font PSF dựa trên Magic Bytes
    let is_psf1 = font_data[0] == 0x36 && font_data[1] == 0x04;
    let is_psf2 = font_data[0] == 0x72 && font_data[1] == 0xb5 && font_data[2] == 0x4a && font_data[3] == 0x86;

    let (glyph_width, glyph_height, glyphs_start_offset) = if is_psf1 {
        // Cấu trúc của PSF1
        (8usize, font_data[3] as usize, 4usize)
    } else if is_psf2 {
        if font_data.len() < 32 { return (0, 0); }
        // Cấu trúc của PSF2 (trích xuất thông số từ Header 32 bytes)
        let read_u32 = |offset: usize| -> u32 {
            (font_data[offset] as u32) |
            ((font_data[offset + 1] as u32) << 8) |
            ((font_data[offset + 2] as u32) << 16) |
            ((font_data[offset + 3] as u32) << 24)
        };
        (read_u32(28) as usize, read_u32(24) as usize, read_u32(8) as usize)
    } else {
        return (0, 0); // Không phải font PSF hợp lệ
    };

    if glyph_width == 0 || glyph_height == 0 {
        return (0, 0);
    }

    let total_glyphs = 256usize; 
    let chars_per_row = 16usize;
    let rows_count = (total_glyphs + chars_per_row - 1) / chars_per_row;

    let tex_width = (chars_per_row * glyph_width) as u32;
    let tex_height = (rows_count * glyph_height) as u32;

    // Tránh ghi đè nếu buffer thiếu dung lượng
    if output_buffer.len() < (tex_width * tex_height) as usize {
        return (0, 0); 
    }

    let bytes_per_row = (glyph_width + 7) / 8;
    let bytes_per_glyph = glyph_height * bytes_per_row;

    for i in 0..total_glyphs {
        let char_offset = glyphs_start_offset + (i * bytes_per_glyph);
        if char_offset + bytes_per_glyph > font_data.len() {
            break;
        }

        let cell_x = (i % chars_per_row) * glyph_width;
        let cell_y = (i / chars_per_row) * glyph_height;

        for row in 0..glyph_height {
            for col in 0..glyph_width {
                // Hỗ trợ xử lý đọc bit cho cả font lớn (nhiều hơn 1 byte mỗi hàng)
                let byte_idx = char_offset + (row * bytes_per_row) + (col / 8);
                let row_byte = font_data[byte_idx];
                let bit_set = (row_byte & (1 << (7 - (col % 8)))) != 0;
                
                let target_x = cell_x + col;
                let target_y = cell_y + row;
                let index = (target_y * tex_width as usize + target_x) as usize;

                if index < output_buffer.len() {
                    output_buffer[index] = if bit_set { fg_color } else { 0x00000000 };
                }
            }
        }
    }

    (tex_width, tex_height)
}

pub fn draw_char_from_baked_texture(
    fb: &Mb2TagFramebuffer,
    texture_buffer: &[u32],
    tex_width: u32,
    tex_height: u32, // Đã thêm param này
    ascii: u8,
    x: u32,
    y: u32,
) {
    if texture_buffer.is_empty() || tex_width == 0 || tex_height == 0 {
        return;
    }
    
    let chars_per_row = 16;
    // Tự động tính toán lại kích thước thật (thay vì cố định 8x16)
    let glyph_width = tex_width / 16; 
    let glyph_height = tex_height / 16;

    let index = ascii as u32;
    let cell_x = (index % chars_per_row) * glyph_width;
    let cell_y = (index / chars_per_row) * glyph_height;

    for row in 0..glyph_height {
        for col in 0..glyph_width {
            let src_x = cell_x + col;
            let src_y = cell_y + row;
            let tex_index = (src_y * tex_width + src_x) as usize;

            if tex_index < texture_buffer.len() {
                let color = texture_buffer[tex_index];
                if color != 0x00000000 {
                    put_pixel(fb, x + col, y + row, color);
                }
            }
        }
    }
}

pub fn draw_string_baked(
    fb: &Mb2TagFramebuffer,
    texture_buffer: &[u32],
    tex_width: u32,
    tex_height: u32,
    text: &str,
    mut x: u32,
    y: u32,
) {
    if texture_buffer.is_empty() || tex_width == 0 || tex_height == 0 {
        return;
    }
    
    let glyph_width = tex_width / 16;
    let start_x = x;
    for byte in text.bytes() {
        if byte == b'\n' {
            x = start_x;
            continue;
        }
        draw_char_from_baked_texture(fb, texture_buffer, tex_width, tex_height, byte, x, y);
        x += glyph_width;
    }
}

pub fn clear_screen(fb: &Mb2TagFramebuffer, color: u32) {
    for y in 0..core::cmp::min(fb.framebuffer_height, 50) {
        for x in 0..core::cmp::min(fb.framebuffer_width, 50) {
            put_pixel(fb, x, y, color);
        }
    }
}