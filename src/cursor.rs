use crate::input::{Input, MouseButton};

pub struct Cursor;

impl Cursor {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        skia_canvas: &skia_safe::Canvas,
        input: &dyn Input
    ) {
        let mouse_position = input.mouse_position();

        let cursor_radius = if input.is_mouse_button_down(MouseButton::Left) {
            5.0
        } else {
            10.0
        };
        
        let cursor_color = skia_safe::Color4f::new(1.0, 0.0, 0.0, 1.0);
        let paint = skia_safe::Paint::new(cursor_color, None);
        let cursor_center = skia_safe::Point::new(mouse_position.0, mouse_position.1);
        skia_canvas.draw_circle(cursor_center, cursor_radius, &paint);
    }
}