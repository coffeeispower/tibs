use std::{collections::HashMap, rc::Rc, sync::mpsc::Sender};

use assets_manager::AssetCache;
use rand::Rng;
use skia_safe::{Canvas, Paint, Rect, RuntimeEffect};

use crate::{
	all, animation::{self, colors::{self, interpolate_color_normalized}, easing, Animation, BasicAnimation}, login::{LoginManager, LoginState}, login_screen, skia_shader_asset::SkiaShaderAsset
};

pub struct Background {
	assets: Rc<AssetCache>,
	animations_state: HashMap<String, f32>,
	elapsed_time: f32,
	pub time_offset: f32,
	fade_in_animation: Box<dyn Animation>, // Animação para o fade-in das cores
	success_animation: Box<dyn Animation>,
	success_animation_target: Sender<f32>,

}
impl Background {
	pub fn new(assets: Rc<AssetCache>) -> Self {
		fn rd() -> f32 {
			rand::rng().random_range(0.4..7.0)
		}
		let (success_animation, success_animation_target) = animation::ProgressBarAnimation::new("success_animation", 5.0);
		Self {
			assets,
			animations_state: HashMap::new(),
			fade_in_animation: Box::new(all!(
				BasicAnimation::new("color_0", rd(), easing::ease_out_quad),
				BasicAnimation::new("color_1", rd(), easing::ease_out_quad),
				BasicAnimation::new("color_2", rd(), easing::ease_out_quad),
				BasicAnimation::new("color_3", rd(), easing::ease_out_quad),
				BasicAnimation::new("color_4", rd(), easing::ease_out_quad)
			)),
			success_animation: Box::new(success_animation),
			elapsed_time: 0.0,
			time_offset: 0.0,
			success_animation_target
		}
	}
	pub fn update(&mut self, delta: f32, login_manager: &LoginManager, login_screen: &login_screen::LoginScreen) {
		// Incrementa o tempo com base na velocidade
		self.elapsed_time += delta;

		// Atualiza o estado das animações
		self
			.animations_state
			.extend(self.fade_in_animation.update(delta));
		self
			.animations_state
			.extend(self.success_animation.update(delta));
		let login_state = login_manager.get_current_login_state(login_screen.username());
		match login_state {
			Some(LoginState::Authenticated(_)) => {
				self.success_animation_target.send(1.0);
			},
			_ => {
				self.success_animation_target.send(0.0);
			}
		}
	}
	pub fn render(&self, canvas: &Canvas) {
		let light_shader = self
			.assets
			.load::<SkiaShaderAsset>("shaders.background.meshgradient")
			.unwrap()
			.read();
		let screen_size = (
			unsafe { canvas.surface() }.unwrap().width() as f32,
			unsafe { canvas.surface() }.unwrap().height() as f32,
		);
		#[repr(C)]
		struct LightShaderUniforms {
			screen_size: (f32, f32),
			i_time: f32,
			colors: [(f32, f32, f32); 5],
			forces: [f32; 5],
		}
		let success_animation = self.get_animation_progress("success_animation");
		let target_colors = [
			interpolate_color_normalized(colors::rgb_to_norm("#0F1419"), colors::rgb_to_norm("#0f1a13"), success_animation),
			interpolate_color_normalized(colors::rgb_to_norm("#225282"), colors::rgb_to_norm("#218245"), success_animation),
			interpolate_color_normalized(colors::rgb_to_norm("#112f4e"), colors::rgb_to_norm("#114f28"), success_animation),
			interpolate_color_normalized(colors::rgb_to_norm("#031120"), colors::rgb_to_norm("#03210e"), success_animation),
			interpolate_color_normalized(colors::rgb_to_norm("#38628f"), colors::rgb_to_norm("#388f58"), success_animation),
		];
		// Interpolação das cores com base no progresso das animações
		let interpolated_colors: [(f32, f32, f32); 5] = target_colors
			.iter()
			.enumerate()
			.map(|(i, &color)| {
				let progress = self.get_animation_progress(&format!("color_{}", i));
				colors::interpolate_color_normalized((0.0, 0.0, 0.0), color, progress)
			})
			.collect::<Vec<_>>()
			.try_into()
			.unwrap();

		let uniforms = LightShaderUniforms {
			screen_size,
			i_time: self.elapsed_time + self.time_offset,
			colors: interpolated_colors,
			forces: [
				self.get_animation_progress("color_0"),
				self.get_animation_progress("color_1"),
				self.get_animation_progress("color_2"),
				self.get_animation_progress("color_3"),
				self.get_animation_progress("color_4"),
			],
		};

		self.render_shader(&light_shader, &uniforms, canvas);
	}
	fn render_shader<T>(&self, shader: &RuntimeEffect, uniforms: &T, canvas: &Canvas) {
		let uniforms_as_bytes = unsafe {
			std::slice::from_raw_parts::<u8>((uniforms as *const T) as *const u8, size_of_val(uniforms))
		};
		let uniforms_data = skia_safe::Data::new_copy(uniforms_as_bytes);
		let shader = shader.make_shader(uniforms_data, &[], None).unwrap();
		canvas.draw_rect(
			Rect::new(
				0.,
				0.,
				unsafe { canvas.surface() }.unwrap().width() as f32,
				unsafe { canvas.surface() }.unwrap().height() as f32,
			),
			Paint::default().set_shader(shader),
		);
	}

	pub fn get_animation_progress(&self, id: &str) -> f32 {
		self.animations_state.get(id).copied().unwrap_or(0.0)
	}
}
