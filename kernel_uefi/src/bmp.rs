use crate::graphics::surface::Surface;
use crate::gop::Color;
use crate::gop::GraphicsOutput;

// Hàm helper để trích xuất pixel từ mọi định dạng BMP (1, 4, 8, 16, 24, 32 bpp)
fn get_bmp_pixel(bmp_data: &[u8], row_offset: usize, bmp_x: u32, bpp: u16, palette_offset: usize) -> Option<(u8, u8, u8)> {
    match bpp {
        32 => {
            let p = row_offset + (bmp_x as usize * 4);
            if p + 3 < bmp_data.len() { Some((bmp_data[p+2], bmp_data[p+1], bmp_data[p])) } else { None }
        },
        24 => {
            let p = row_offset + (bmp_x as usize * 3);
            if p + 2 < bmp_data.len() { Some((bmp_data[p+2], bmp_data[p+1], bmp_data[p])) } else { None }
        },
        16 => {
            // Hỗ trợ 16-bit chuẩn (RGB555)
            let p = row_offset + (bmp_x as usize * 2);
            if p + 1 < bmp_data.len() {
                let val = u16::from_le_bytes([bmp_data[p], bmp_data[p+1]]);
                let r = (((val >> 10) & 0x1F) * 255 / 31) as u8;
                let g = (((val >> 5) & 0x1F) * 255 / 31) as u8;
                let b = ((val & 0x1F) * 255 / 31) as u8;
                Some((r, g, b))
            } else { None }
        },
        8 => {
            // 8-bit có sử dụng bảng màu (Palette)
            let p = row_offset + bmp_x as usize;
            if p < bmp_data.len() {
                let idx = bmp_data[p] as usize;
                let pal = palette_offset + idx * 4;
                if pal + 2 < bmp_data.len() { Some((bmp_data[pal+2], bmp_data[pal+1], bmp_data[pal])) } else { None }
            } else { None }
        },
        4 => {
            // 4-bit (2 pixel trong 1 byte)
            let p = row_offset + (bmp_x as usize / 2);
            if p < bmp_data.len() {
                let byte = bmp_data[p];
                let idx = if bmp_x % 2 == 0 { byte >> 4 } else { byte & 0x0F } as usize;
                let pal = palette_offset + idx * 4;
                if pal + 2 < bmp_data.len() { Some((bmp_data[pal+2], bmp_data[pal+1], bmp_data[pal])) } else { None }
            } else { None }
        },
        1 => {
            // 1-bit đơn sắc (8 pixel trong 1 byte)
            let p = row_offset + (bmp_x as usize / 8);
            if p < bmp_data.len() {
                let byte = bmp_data[p];
                let bit_idx = 7 - (bmp_x % 8);
                let idx = ((byte >> bit_idx) & 1) as usize;
                let pal = palette_offset + idx * 4;
                if pal + 2 < bmp_data.len() { Some((bmp_data[pal+2], bmp_data[pal+1], bmp_data[pal])) } else { None }
            } else { None }
        },
        _ => None
    }
}

pub fn draw_bmp_to_surface_scaled(
    surface: &mut Surface,
    bmp_data: &[u8],
    dest_x: u32,
    dest_y: u32,
    target_w: u32,
    target_h: u32,
) {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return;
    }

    let pixel_offset = u32::from_le_bytes([bmp_data[10], bmp_data[11], bmp_data[12], bmp_data[13]]) as usize;
    let dib_size = u32::from_le_bytes([bmp_data[14], bmp_data[15], bmp_data[16], bmp_data[17]]);
    let palette_offset = 14 + dib_size as usize;

    let bmp_width = i32::from_le_bytes([bmp_data[18], bmp_data[19], bmp_data[20], bmp_data[21]]).abs() as u32;
    let mut bmp_height = i32::from_le_bytes([bmp_data[22], bmp_data[23], bmp_data[24], bmp_data[25]]);
    let bpp = u16::from_le_bytes([bmp_data[28], bmp_data[29]]);

    let top_down = bmp_height < 0;
    if top_down {
        bmp_height = -bmp_height;
    }
    let bmp_height = bmp_height as u32;

    if bmp_width == 0 || bmp_height == 0 {
        return;
    }

    // Công thức tính alignment padding chuẩn (chuẩn hóa mỗi hàng chia hết cho 4 byte)
    let row_size = ((bmp_width * bpp as u32 + 31) / 32) * 4;

    for ty in 0..target_h {
        let bmp_y = (ty * bmp_height) / target_h;
        let row_idx = if top_down { bmp_y } else { bmp_height - 1 - bmp_y };
        let row_offset = pixel_offset + (row_idx * row_size) as usize;

        if row_offset >= bmp_data.len() {
            break;
        }

        for tx in 0..target_w {
            let bmp_x = (tx * bmp_width) / target_w;
            
            if let Some((r, g, b)) = get_bmp_pixel(bmp_data, row_offset, bmp_x, bpp, palette_offset) {
                // Xóa nền Magenta hoặc Trắng
                let is_magenta = r > 240 && g < 15 && b > 240;
                let is_white_bg = r > 220 && g > 220 && b > 220;

                if is_magenta || is_white_bg {
                    continue;
                }

                surface.put_pixel(dest_x + tx, dest_y + ty, Color::rgb(r, g, b));
            }
        }
    }
}

pub fn draw_bmp_to_surface(surface: &mut Surface, bmp_data: &[u8], offset_x: u32, offset_y: u32) {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return;
    }

    let pixel_offset = u32::from_le_bytes([bmp_data[10], bmp_data[11], bmp_data[12], bmp_data[13]]) as usize;
    let dib_size = u32::from_le_bytes([bmp_data[14], bmp_data[15], bmp_data[16], bmp_data[17]]);
    let palette_offset = 14 + dib_size as usize;

    let bmp_width = i32::from_le_bytes([bmp_data[18], bmp_data[19], bmp_data[20], bmp_data[21]]).abs() as u32;
    let mut bmp_height = i32::from_le_bytes([bmp_data[22], bmp_data[23], bmp_data[24], bmp_data[25]]);
    let bpp = u16::from_le_bytes([bmp_data[28], bmp_data[29]]);

    let top_down = bmp_height < 0;
    if top_down {
        bmp_height = -bmp_height;
    }
    let bmp_height = bmp_height as u32;

    if bmp_width == 0 || bmp_height == 0 {
        return;
    }

    let row_size = ((bmp_width * bpp as u32 + 31) / 32) * 4;

    for row in 0..bmp_height {
        let row_idx = if top_down { row } else { bmp_height - 1 - row };
        let row_offset = pixel_offset + (row_idx * row_size) as usize;

        if row_offset >= bmp_data.len() {
            break;
        }

        for col in 0..bmp_width {
            if let Some((r, g, b)) = get_bmp_pixel(bmp_data, row_offset, col, bpp, palette_offset) {
                let is_white = r > 240 && g > 240 && b > 240;
                let is_magenta = r == 255 && g == 0 && b == 255;
                if is_white || is_magenta {
                    continue;
                }

                surface.put_pixel(offset_x + col, offset_y + row, Color::rgb(r, g, b));
            }
        }
    }
}

pub fn draw_bmp_fullscreen(display: &mut GraphicsOutput, bmp_data: &[u8]) {
    draw_bmp_with_offset(display, bmp_data, 0, 0, true);
}

pub fn draw_bmp_with_offset(display: &mut GraphicsOutput, bmp_data: &[u8], offset_x: u32, offset_y: u32, fullscreen: bool) {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return;
    }

    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes([bmp_data[offset], bmp_data[offset + 1], bmp_data[offset + 2], bmp_data[offset + 3]])
    };
    let read_i32 = |offset: usize| -> i32 {
        i32::from_le_bytes([bmp_data[offset], bmp_data[offset + 1], bmp_data[offset + 2], bmp_data[offset + 3]])
    };
    let read_u16 = |offset: usize| -> u16 {
        u16::from_le_bytes([bmp_data[offset], bmp_data[offset + 1]])
    };

    let pixel_offset = read_u32(10) as usize;
    let dib_size = read_u32(14);
    let palette_offset = 14 + dib_size as usize;

    let bmp_width = read_i32(18).abs() as u32;
    let mut bmp_height = read_i32(22);
    let bpp = read_u16(28);

    let top_down = bmp_height < 0;
    if top_down {
        bmp_height = -bmp_height;
    }
    let bmp_height = bmp_height as u32;

    if bmp_width == 0 || bmp_height == 0 {
        return;
    }

    let row_size = ((bmp_width * bpp as u32 + 31) / 32) * 4;

    let fb_w = display.width() as u32;
    let fb_h = display.height() as u32;
    let vram = display.raw_addr() as *mut u8;
    let pitch = display.pitch() as usize;

    // Ghi trực tiếp vào VRAM (Không cần lo kích thước BACKBUFFER max_width/height nữa)
    for fb_y in 0..fb_h {
        let bmp_y = if fullscreen {
            (fb_y * bmp_height) / fb_h
        } else {
            if fb_y < offset_y || fb_y >= offset_y + bmp_height {
                continue;
            }
            fb_y - offset_y
        };
        
        let row_idx = if top_down { bmp_y } else { bmp_height - 1 - bmp_y };
        let row_offset = pixel_offset + (row_idx * row_size) as usize;

        if row_offset >= bmp_data.len() {
            break;
        }

        for fb_x in 0..fb_w {
            let bmp_x = if fullscreen {
                (fb_x * bmp_width) / fb_w
            } else {
                if fb_x < offset_x || fb_x >= offset_x + bmp_width {
                    continue;
                }
                fb_x - offset_x
            };
            
            if let Some((r, g, b)) = get_bmp_pixel(bmp_data, row_offset, bmp_x, bpp, palette_offset) {
                let color = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                unsafe {
                    // Ánh xạ địa chỉ chính xác trên Framebuffer
                    let dst = vram.add((fb_y as usize) * pitch + (fb_x as usize) * 4) as *mut u32;
                    *dst = color;
                }
            }
        }
    }
}

pub fn draw_bmp_at(display: &mut GraphicsOutput, bmp_data: &[u8], x: u32, y: u32) {
    draw_bmp_with_offset(display, bmp_data, x, y, false);
}