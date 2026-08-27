// src/bmp.rs

use crate::vbe::Mb2TagFramebuffer;

// Cấp phát vùng nhớ tĩnh Backbuffer (~3MB RAM) tại section .bss
const MAX_WIDTH: usize = 1024;
const MAX_HEIGHT: usize = 768;
static mut BACKBUFFER: [u32; MAX_WIDTH * MAX_HEIGHT] = [0; MAX_WIDTH * MAX_HEIGHT];

pub fn draw_bmp_fullscreen(fb: &Mb2TagFramebuffer, bmp_data: &[u8]) {
    if bmp_data.len() < 54 || bmp_data[0] != b'B' || bmp_data[1] != b'M' {
        return;
    }

    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes([bmp_data[offset], bmp_data[offset+1], bmp_data[offset+2], bmp_data[offset+3]])
    };
    let read_i32 = |offset: usize| -> i32 {
        i32::from_le_bytes([bmp_data[offset], bmp_data[offset+1], bmp_data[offset+2], bmp_data[offset+3]])
    };
    let read_u16 = |offset: usize| -> u16 {
        u16::from_le_bytes([bmp_data[offset], bmp_data[offset+1]])
    };

    let pixel_offset = read_u32(10) as usize;
    let bmp_width = read_i32(18) as u32;
    let mut bmp_height = read_i32(22);
    let bpp = read_u16(28);

    let top_down = bmp_height < 0;
    if top_down { bmp_height = -bmp_height; }
    let bmp_height = bmp_height as u32;

    if bpp != 24 && bpp != 32 { return; }

    let bytes_per_pixel = (bpp / 8) as u32;
    
    // === FIX: Tính row_size an toàn, tránh overflow ===
    let row_size = match (bmp_width as u64).checked_mul(bytes_per_pixel as u64) {
        Some(size) => {
            // Align to 4 bytes
            let aligned = ((size + 3) / 4) * 4;
            aligned as usize
        },
        None => return,
    };

    let fb_w = fb.framebuffer_width as usize;
    let fb_h = fb.framebuffer_height as usize;
    let pitch = fb.framebuffer_pitch as usize;

    // Giới hạn an toàn tránh tràn RAM
    if fb_w > MAX_WIDTH || fb_h > MAX_HEIGHT {
        return; 
    }

    // ============================================================
    // BƯỚC 1: DỰNG HÌNH OFFLINE TRÊN RAM (BACKBUFFER)
    // Hoàn toàn không đụng tới VRAM ở bước này để tránh thắt cổ chai
    // ============================================================
    for fb_y in 0..fb_h {
        let bmp_y = (fb_y as u32 * bmp_height) / fb_h as u32;
        let row_idx = if top_down { bmp_y } else { bmp_height - 1 - bmp_y };
        
        // === FIX: Tính offset an toàn ===
        let row_offset = match (row_idx as u64).checked_mul(row_size as u64) {
            Some(offset) => pixel_offset + offset as usize,
            None => break,
        };

        if row_offset + (bmp_width * bytes_per_pixel) as usize > bmp_data.len() {
            break;
        }

        for fb_x in 0..fb_w {
            let bmp_x = (fb_x as u32 * bmp_width) / fb_w as u32;
            
            // === FIX: Tính pixel offset an toàn ===
            let p = match (bmp_x as u64).checked_mul(bytes_per_pixel as u64) {
                Some(offset) => row_offset + offset as usize,
                None => break,
            };

            if p + 2 >= bmp_data.len() {
                break;
            }

            let b = bmp_data[p] as u32;
            let g = bmp_data[p+1] as u32;
            let r = bmp_data[p+2] as u32;

            let color = (r << 16) | (g << 8) | b;
            
            unsafe {
                BACKBUFFER[fb_y * MAX_WIDTH + fb_x] = color;
            }
        }
    }

    // ============================================================
    // BƯỚC 2: BLITTING (BẮN DỮ LIỆU TỪ RAM SANG VRAM)
    // CPU chỉ việc đọc khối dữ liệu có sẵn và đẩy thẳng ra màn hình
    // ============================================================
    let vram = fb.framebuffer_addr as *mut u8;
    for fb_y in 0..fb_h {
        unsafe {
            let src_row = BACKBUFFER.as_ptr().add(fb_y * MAX_WIDTH);
            let dst_row = vram.add(fb_y * pitch) as *mut u32;
            // rep movsb: Lệnh assembly copy dữ liệu nhanh nhất của x86
            core::ptr::copy_nonoverlapping(src_row, dst_row, fb_w);
        }
    }
}