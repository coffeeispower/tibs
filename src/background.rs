use std::{
    collections::HashMap,
    f32::consts::PI,
    ops::{Add, Div, Mul},
    time::{Instant, SystemTime},
};

use assets_manager::AssetCache;
use rand::Rng;
use skia_safe::{Canvas, Paint, Rect, RuntimeEffect};

use crate::{
    all,
    animation::{colors, easing, Animation, BasicAnimation},
    seq,
    skia_shader_asset::SkiaShaderAsset,
};

pub struct Background<'a> {
    light_shader_asset: &'a assets_manager::Handle<SkiaShaderAsset>,
    animations_state: HashMap<String, f32>,
    elapsed_time: f32,
    pub time_offset: f32,
    fade_in_animation: Box<dyn Animation>, // Animação para o fade-in das cores
}
impl<'a> Background<'a> {
    pub fn new(assets: &'a AssetCache) -> Self {
        let load = |s: &str| assets.load::<SkiaShaderAsset>(s).unwrap();
        fn rd() -> f32 {
            rand::rng().random_range(0.4..7.0)
        }
        Self {
            light_shader_asset: load("shaders.background.meshgradient"),
            animations_state: HashMap::new(),
            fade_in_animation: Box::new(all!(
                BasicAnimation::new("color_0", rd(), easing::ease_out_quad),
                BasicAnimation::new("color_1", rd(), easing::ease_out_quad),
                BasicAnimation::new("color_2", rd(), easing::ease_out_quad),
                BasicAnimation::new("color_3", rd(), easing::ease_out_quad),
                BasicAnimation::new("color_4", rd(), easing::ease_out_quad)
            )),
            elapsed_time: 0.0,
            time_offset: 0.0,
        }
    }
    pub fn update(&mut self, delta: f32) {
        // Incrementa o tempo com base na velocidade
        self.elapsed_time += delta;

        // Atualiza o estado das animações
        self.animations_state
            .extend(self.fade_in_animation.update(delta));
    }
    pub fn render(&self, canvas: &Canvas) {
        let light_shader = self.light_shader_asset.read();
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
        let target_colors = [
            (1.00, 0.74, 0.63), // Laranja pastel
            (0.87, 0.07, 0.27), // Vermelho vivo
            (0.98, 0.91, 0.63), // Amarelo suave
            (0.63, 0.82, 0.80), // Azul esverdeado
            colors::rgb_to_norm("#FF8966"),
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
            std::slice::from_raw_parts::<u8>(
                (uniforms as *const T) as *const u8,
                size_of_val(uniforms),
            )
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
