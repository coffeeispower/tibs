use clay_layout::render_commands::{Custom, RenderCommand};
use skia_safe::{Canvas, Image, Paint, Path, Rect, ClipOp};
#[derive(Debug, Clone)]
pub enum CustomElements {
    Avatar { image: Image },
}

impl CustomElements {
    pub fn render(
        command: &RenderCommand<'_, Image, Self>,
        custom: &Custom<'_, Self>,
        canvas: &Canvas,
    ) {
        match custom.data {
            // Assuming the custom element is stored in a field named `element`
            CustomElements::Avatar { image } => {
                // Use the command's bounding box for position and dimensions.
                let bb = command.bounding_box;
                let center_x = bb.x + bb.width / 2.0;
                let center_y = bb.y + bb.height / 2.0;
                let radius = bb.width.min(bb.height) / 2.0;

                // Create a circular clipping path.
                let mut path = Path::new();
                path.add_circle((center_x, center_y), radius, None);

                // Save the canvas state, clip to the circle, and then draw the image.
                canvas.save();
                canvas.clip_path(&path, ClipOp::Intersect, true);

                // Prepare a rectangle to draw the image.
                let dest_rect = Rect::from_xywh(bb.x, bb.y, bb.width, bb.height);
                let paint = Paint::default();
                canvas.draw_image_rect(image, None, dest_rect, &paint);

                // Restore the canvas state.
                canvas.restore();
            }
        }
    }
}
