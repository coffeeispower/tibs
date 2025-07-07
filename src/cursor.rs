use std::{collections::HashMap, ffi::CString};

use cairo::{Format, ImageSurface};
use hyprcursor::{CursorStyleInfo, HyprCursorManager};
use libhyprcursor_sys::hyprcursor_cursor_image_data_free;
use rustamarine::Rustamarine;
use skia_safe::{self, images, Image, ImageInfo, Paint, Point, Rect, SamplingOptions};
struct CursorVariation {
	image: Image,
	hotspot: (i32, i32),
}
pub struct Cursor {
	cursors: HashMap<String, CursorVariation>,
	cursor_size: u32,
	style_info: CursorStyleInfo,
	cursor_manager: HyprCursorManager,
}

impl Cursor {
	pub fn new(cursor_size: impl Into<Option<u32>>) -> Self {
		let cursor_size = cursor_size.into().unwrap_or(24);
		log::debug!("Initializing Cursor with size {}.", cursor_size);
		let manager = HyprCursorManager::new(Some(c""));
		let style_info = manager.new_style_info(cursor_size);
		manager.load_theme_style(&style_info);
		log::debug!("Loaded theme style for cursor.");
		Self {
			cursors: HashMap::new(),
			cursor_size,
			style_info,
			cursor_manager: manager,
		}
	}

	fn load_cursor(&mut self, cursor_name: &str) {
		log::debug!("Attempting to load cursor: {}", cursor_name);
		let image = if self.cursor_manager.is_theme_valid() {
			log::debug!("Cursor manager theme is valid.");
			let c_cursor_name = CString::new(cursor_name).unwrap();
			let data = self
				.cursor_manager
				.get_cursor_image_data(&c_cursor_name, &self.style_info);
			if data.is_empty() {
				log::debug!("No image data found for cursor: {}", cursor_name);
				return;
			}
			log::debug!("Received image data, extracting Cairo surface.");

			// Get a cairo surface from the first image data entry.
			let surface = data[0].surface();

			// Get the width and height of the surface
			let width = self.cursor_size;
			let height = self.cursor_size;

			println!(
				"[DEBUG] Creating new Cairo ImageSurface with dimensions: {}x{}",
				width, height
			);
			// Create a new Cairo ImageSurface to render into memory
			let mut image_surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32)
				.expect("Failed to create Cairo ImageSurface");
			{
				log::debug!("Creating Cairo context to draw the cursor.");
				// Create a Cairo context and draw the original surface onto the new one
				let context = cairo::Context::new(&image_surface).unwrap();
				context.set_source_surface(&surface, 0.0, 0.0).unwrap();
				context.paint().expect("Failed to paint onto ImageSurface");
				log::debug!("Finished drawing the cursor onto Cairo ImageSurface.");
			}
			log::debug!("Accessing raw pixel data from the Cairo ImageSurface.");
			// Access the raw pixel data from the ImageSurface
			let img_data = image_surface
				.data()
				.expect("Failed to get ImageSurface data")
				.to_vec();
			let width = image_surface.width();
			let height = image_surface.height();

			log::debug!("Creating Skia Image from the raw pixel data.");
			// Create a Skia image directly from the raw pixel data.
			let image_info = ImageInfo::new(
				(width, height),
				skia_safe::ColorType::BGRA8888,
				skia_safe::AlphaType::Unpremul,
				None,
			);
			let row_bytes = (width * 4) as usize;
			let image =
				images::raster_from_data(&image_info, skia_safe::Data::new_copy(&img_data), row_bytes)
					.expect("Failed to create Skia Image from raster data");

			log::debug!(
				"Successfully created Skia Image for cursor: {}",
				cursor_name
			);

			log::debug!("Freeing hyprcursor image data.");
			let hotspot = (data[0].hotspot_x(), data[0].hotspot_y());
			// Free the hyprcursor image data.
			unsafe { hyprcursor_cursor_image_data_free(data.as_mut_ptr().cast(), data.len() as _) }
			Some(CursorVariation { image, hotspot })
		} else {
			log::debug!(
				"Cursor manager theme is not valid. Skipping cursor load for: {}",
				cursor_name
			);
			None
		};
		if let Some(image) = image {
			self.cursors.insert(cursor_name.to_owned(), image);
			log::debug!("Cursor '{}' loaded and stored.", cursor_name);
		} else {
			log::debug!("Cursor '{}' failed to load.", cursor_name);
		}
	}

	pub fn get_or_load_cursor(&mut self, cursor_name: &str) -> Option<&CursorVariation> {
		if !self.cursors.contains_key(cursor_name) {
			log::debug!("Cursor '{}' not found in cache, loading now.", cursor_name);
			self.load_cursor(cursor_name);
		}
		self.cursors.get(cursor_name)
	}

	pub fn render(&mut self, skia_canvas: &skia_safe::Canvas, rmar: &Rustamarine, cursor_name: &str) {
		let (mx, my) = (rmar.get_mouse_x() as f32, rmar.get_mouse_y() as f32);
		if let Some(CursorVariation {
			image,
			hotspot: (hx, hy),
		}) = self.get_or_load_cursor(cursor_name)
		{
			let pos = Point::new(mx - *hx as f32, my - *hy as f32);
			let dest_rect = Rect::from_xywh(pos.x, pos.y, image.width() as f32, image.height() as f32);
			skia_canvas.draw_image_rect_with_sampling_options(
				image,
				None,
				dest_rect,
				SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::Linear),
				&Paint::default().set_anti_alias(true),
			);
		} else {
			let pos = Point::new(mx, my);
			log::debug!("Fallback rendering for cursor '{}'.", cursor_name);
			// Fallback: draw a circle.
			let cursor_radius = if rmar.is_mouse_button_down(0) {
				5.0
			} else {
				10.0
			};
			let cursor_color = skia_safe::Color4f::new(1.0, 0.0, 0.0, 1.0);
			let paint = Paint::new(cursor_color, None);
			skia_canvas.draw_circle(pos, cursor_radius, &paint);
		}
	}
}
