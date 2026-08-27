// src/graphics/mod.rs
pub mod surface;
pub mod font;
pub mod window;
pub mod gfx;

// Thêm allow để bỏ warning
#[allow(unused_imports)]
pub use surface::Surface;
#[allow(unused_imports)]
pub use font::Font;
#[allow(unused_imports)]
pub use window::Window;

use crate::gop::Color;

pub fn init_graphics(fb_addr: *mut u32, width: u32, height: u32, pitch: u32) -> Surface {
    let mut screen = Surface::new(fb_addr, width, height, pitch, 32);
    screen.fill(Color::rgb(20, 20, 25));
    screen
}

pub fn draw_desktop(surface: &mut Surface) {
    for y in 0..surface.height {
        let intensity = 20 + (y * 30 / surface.height) as u8;
        let color = Color::rgb(intensity, intensity, intensity + 10);
        for x in 0..surface.width {
            surface.put_pixel(x, y, color);
        }
    }
}

pub fn draw_progress_bar(surface: &mut Surface, x: u32, y: u32, width: u32, progress: f32) {
    let bar_height = 20;
    let border_color = Color::rgb(80, 80, 85);
    let bg_color = Color::rgb(30, 30, 35);
    let fill_color = Color::rgb(0, 120, 215);
    
    surface.fill_rect(x, y, width, bar_height, bg_color);
    surface.draw_rect(x, y, width, bar_height, border_color);
    
    let fill_width = (width as f32 * progress.clamp(0.0, 1.0)) as u32;
    if fill_width > 2 {
        surface.fill_rect(x + 2, y + 2, fill_width - 4, bar_height - 4, fill_color);
    }
}