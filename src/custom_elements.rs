use std::time::UNIX_EPOCH;

use clay_layout::{
	math::BoundingBox,
	render_commands::{Custom, RenderCommand},
};
use skia_safe::{Canvas, Color, Image, Paint, PaintCap, PaintStyle, Rect};
#[derive(Debug, Clone)]
pub enum CustomElements {
	Spinner,
}

impl CustomElements {
	pub fn render(
		command: &RenderCommand<'_, Image, Self>,
		custom: &Custom<'_, Self>,
		canvas: &Canvas,
	) {
		match custom.data {
			CustomElements::Spinner => {
				let time = std::time::SystemTime::now()
					.duration_since(UNIX_EPOCH)
					.unwrap()
					.as_secs_f64();
				let duration_secs = 1.0;
				let rotation = ((time % duration_secs) / duration_secs) * 360.0;
				let BoundingBox {
					x,
					y,
					width,
					height,
				} = command.bounding_box;

				let oval = Rect::new(x, y, x + width, y + height);

				// Paint do arco
				let mut paint = Paint::default();
				paint.set_anti_alias(true);
				paint.set_color(Color::from_argb(255, 255, 255, 255)); // azul claro
				paint.set_stroke_width(4.0);
				paint.set_style(PaintStyle::Stroke);
				paint.set_stroke_cap(PaintCap::Round);

				// Desenhar arco com rotação animada
				let start_angle = rotation as f32;
				let sweep_angle = 270.0;
				let use_center = false;

				canvas.draw_arc(oval, start_angle, sweep_angle, use_center, &paint);
			}
		}
	}
}
