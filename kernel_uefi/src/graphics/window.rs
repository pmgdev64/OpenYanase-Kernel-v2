// src/graphics/window.rs
use crate::graphics::surface::Surface;
use crate::graphics::font::Font;
use crate::gop::Color;

#[derive(Clone, Copy)]
pub struct Window {
    pub id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
    pub is_active: bool,
    pub is_dragging: bool,
    pub is_visible: bool,
}

impl Window {
    pub const fn new(id: u32, x: u32, y: u32, width: u32, height: u32, title: &'static str) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            title,
            is_active: false,
            is_dragging: false,
            is_visible: true,
        }
    }

    pub fn render(&self, surface: &mut Surface, font: Option<&Font>) {
        if !self.is_visible { return; }

        let title_bg = if self.is_active { Color::rgb(0, 120, 215) } else { Color::rgb(45, 45, 50) };
        let win_bg = Color::rgb(30, 30, 35);
        let border_color = Color::rgb(60, 60, 65);

        surface.fill_rect(self.x, self.y, self.width, self.height, win_bg);
        surface.fill_rect(self.x, self.y, self.width, 28, title_bg);

        let close_x = self.x + self.width - 24;
        surface.fill_rect(close_x, self.y + 4, 20, 20, Color::rgb(200, 40, 40));

        surface.draw_rect(self.x, self.y, self.width, self.height, border_color);

        if let Some(f) = font {
            f.draw_string(surface, self.title, self.x + 8, self.y + 6, Color::WHITE);
            f.draw_string(surface, "X", close_x + 6, self.y + 6, Color::WHITE);
        }
    }

    pub fn contains_point(&self, mx: u32, my: u32) -> bool {
        if !self.is_visible { return false; }
        mx >= self.x && mx < self.x + self.width && my >= self.y && my < self.y + self.height
    }

    pub fn is_titlebar_hit(&self, mx: u32, my: u32) -> bool {
        if !self.is_visible { return false; }
        let close_x = self.x + self.width - 24;
        mx >= self.x && mx < close_x && my >= self.y && my < self.y + 28
    }

    pub fn is_close_hit(&self, mx: u32, my: u32) -> bool {
        if !self.is_visible { return false; }
        let close_x = self.x + self.width - 24;
        mx >= close_x && mx < close_x + 20 && my >= self.y + 4 && my < self.y + 24
    }
}