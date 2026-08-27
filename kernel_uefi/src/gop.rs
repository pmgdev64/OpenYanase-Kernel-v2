// src/gop.rs
// Module quản lý UEFI Graphics Output Protocol (GOP) qua Multiboot2 Framebuffer Tag

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u32);

impl Color {
    pub const BLACK: Self = Self(0xFF000000);
    pub const WHITE: Self = Self(0xFFFFFFFF);
    pub const RED: Self = Self(0xFFFF0000);
    pub const GREEN: Self = Self(0xFF00FF00);
    pub const BLUE: Self = Self(0xFF0000FF);
    pub const DARK_BLUE: Self = Self(0xFF000033);

    /// Tạo màu từ thành phần R, G, B (8-bit mỗi kênh, format ARGB 32-bit)
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Trả về giá trị màu dạng u32
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Cấu trúc Header Framebuffer Tag chuẩn Multiboot2 (Tag Type 8)
#[repr(C, packed)]
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

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub addr: *mut u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32, // Đơn vị: Bytes per scanline
    pub bpp: u8,
}

pub struct GraphicsOutput {
    info: FramebufferInfo,
}

impl GraphicsOutput {
    /// Phân tích địa chỉ Multiboot2 Info để trích xuất Tag Framebuffer (Tag Type 8)
    pub unsafe fn from_multiboot(mb_info_ptr: u64) -> Option<Self> {
        if mb_info_ptr == 0 {
            return None;
        }

        let addr = mb_info_ptr as *const u32;
        let total_size = addr.read_volatile();
        let mut current = mb_info_ptr + 8;
        let end = mb_info_ptr + total_size as u64;

        while current < end {
            let tag_ptr = current as *const u32;
            let tag_type = tag_ptr.read_volatile();
            let tag_size = tag_ptr.add(1).read_volatile();

            if tag_type == 0 {
                break;
            }

            // Tag Type 8 = Multiboot2 Framebuffer
            if tag_type == 8 {
                let fb_tag = &*(current as *const Mb2TagFramebuffer);
                return Some(Self {
                    info: FramebufferInfo {
                        addr: fb_tag.framebuffer_addr as *mut u32,
                        width: fb_tag.framebuffer_width,
                        height: fb_tag.framebuffer_height,
                        pitch: fb_tag.framebuffer_pitch,
                        bpp: fb_tag.framebuffer_bpp,
                    },
                });
            }

            // Căn chỉnh 8-byte alignment cho tag kế tiếp
            current = (current + tag_size as u64 + 7) & !7;
        }

        None
    }

    /// Trả về con trỏ vùng nhớ thô VRAM
    pub fn raw_addr(&self) -> *mut u32 {
        self.info.addr
    }

    /// Trả về độ rộng màn hình (pixel)
    pub fn width(&self) -> u32 {
        self.info.width
    }

    /// Trả về chiều cao màn hình (pixel)
    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// Trả về độ dài scanline tính bằng Byte (pitch)
    pub fn pitch(&self) -> u32 {
        self.info.pitch
    }

    /// Trả về số bit trên mỗi pixel (bpp)
    pub fn bpp(&self) -> u8 {
        self.info.bpp
    }

    /// Vẽ 1 Pixel tại vị trí (x, y)
    #[inline]
    pub fn draw_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.info.width && y < self.info.height {
            let stride = self.info.pitch / 4; // Quy đổi byte pitch sang u32 stride
            let offset = (y * stride) + x;
            unsafe {
                self.info.addr.add(offset as usize).write_volatile(color.0);
            }
        }
    }

    /// Vẽ hình chữ nhật đặc
    pub fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        for dy in 0..height {
            for dx in 0..width {
                self.draw_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Xóa sạch màn hình với một màu nền
    pub fn clear(&mut self, color: Color) {
        self.draw_rect(0, 0, self.info.width, self.info.height, color);
    }
}