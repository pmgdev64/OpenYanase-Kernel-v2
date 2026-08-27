// src/bmp.rs
use crate::gop::GraphicsOutput;

const MAX_WIDTH: usize = 1024;
const MAX_HEIGHT: usize = 768;

// Backbuffer lưu trữ tĩnh tại vùng nhớ .bss (~3MB RAM)
static mut BACKBUFFER: [u32; MAX_WIDTH * MAX_HEIGHT] = [0; MAX_WIDTH * MAX_HEIGHT];

pub fn draw_bmp_fullscreen(display: &mut GraphicsOutput, bmp_data: &[u8]) {
    draw_bmp_with_offset(display, bmp_data, 0, 0, true);
}

/// Vẽ BMP với offset và tùy chọn scale fullscreen
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
    let bmp_width = read_i32(18) as u32;
    let mut bmp_height = read_i32(22);
    let bpp = read_u16(28);

    let top_down = bmp_height < 0;
    if top_down {
        bmp_height = -bmp_height;
    }
    let bmp_height = bmp_height as u32;

    if bpp != 24 && bpp != 32 {
        return;
    }

    let bytes_per_pixel = (bpp / 8) as u32;
    let row_size = ((bmp_width * bytes_per_pixel + 3) / 4) * 4;

    let fb_w = display.width() as usize;
    let fb_h = display.height() as usize;
    
    let offset_x = offset_x as usize;
    let offset_y = offset_y as usize;

    if fb_w > MAX_WIDTH || fb_h > MAX_HEIGHT {
        return;
    }

    // 1. Scale và render ảnh vào RAM Backbuffer
    for fb_y in 0..fb_h {
        let bmp_y = if fullscreen {
            (fb_y as u32 * bmp_height) / fb_h as u32
        } else {
            // Nếu không fullscreen, chỉ vẽ trong khoảng offset
            if fb_y < offset_y || fb_y >= offset_y + bmp_height as usize {
                continue;
            }
            (fb_y - offset_y) as u32
        };
        
        let row_idx = if top_down { bmp_y } else { bmp_height - 1 - bmp_y };
        let row_offset = pixel_offset + (row_idx * row_size) as usize;

        if row_offset + (bmp_width * bytes_per_pixel) as usize > bmp_data.len() {
            break;
        }

        for fb_x in 0..fb_w {
            let bmp_x = if fullscreen {
                (fb_x as u32 * bmp_width) / fb_w as u32
            } else {
                if fb_x < offset_x || fb_x >= offset_x + bmp_width as usize {
                    continue;
                }
                (fb_x - offset_x) as u32
            };
            
            let p = row_offset + (bmp_x * bytes_per_pixel) as usize;

            let b = bmp_data[p] as u32;
            let g = bmp_data[p + 1] as u32;
            let r = bmp_data[p + 2] as u32;

            let color = 0xFF000000 | (r << 16) | (g << 8) | b;

            unsafe {
                BACKBUFFER[fb_y * MAX_WIDTH + fb_x] = color;
            }
        }
    }

    // 2. Fast Blitting: Đẩy Backbuffer ra UEFI Framebuffer VRAM
    let vram = display.raw_addr() as *mut u8;
    let pitch = display.pitch() as usize;

    for fb_y in 0..fb_h {
        unsafe {
            let src_row = BACKBUFFER.as_ptr().add(fb_y * MAX_WIDTH);
            let dst_row = vram.add(fb_y * pitch) as *mut u32;
            core::ptr::copy_nonoverlapping(src_row, dst_row, fb_w);
        }
    }
}

/// Vẽ BMP với offset (không scale)
pub fn draw_bmp_at(display: &mut GraphicsOutput, bmp_data: &[u8], x: u32, y: u32) {
    draw_bmp_with_offset(display, bmp_data, x, y, false);
}