use std::{collections::HashMap, sync::mpsc::Sender};

use assets_manager::AssetCache;
use clay_layout::{
	elements::{FloatingAttachPointType, FloatingAttachToElement},
	fixed, grow,
	layout::{
		Alignment, LayoutAlignmentX as LX, LayoutAlignmentY as LY, LayoutDirection, Padding, Sizing,
	},
	text::TextConfig,
	Declaration,
};

use crate::{
	all,
	animation::{
		self,
		colors::interpolate_color,
		easing::{ease_out_elastic, ease_out_quad, ease_out_quint},
		Animation, BasicAnimation, ParallelAnimation, ProgressBarAnimation,
	},
	progress_watcher::ProgressData,
	seq,
	skia_image_asset::SkiaImageAsset,
	TibsClayScope,
};
pub struct LoadingScreen {
	loading_animation: ParallelAnimation,
	end_progress: BasicAnimation,
	progress_bar_sender: Sender<f32>,
	animations_state: HashMap<String, f32>,
	logo: skia_safe::Image,
	alert_icon: skia_safe::Image,
	success_icon: skia_safe::Image,
}

impl LoadingScreen {
	pub fn new(assets: &AssetCache) -> Self {
		let SkiaImageAsset(success_icon) = assets
			.load_owned("icons.check")
			.expect("Failed to load check icon");
		let SkiaImageAsset(alert_icon) = assets
			.load_owned("icons.triangle-alert")
			.expect("Failed to load warning icon");
		let SkiaImageAsset(logo) = assets
			.load_owned("logo")
			.expect("Failed to load check icon");
		let (progress_bar_animation, progress_bar_sender) = ProgressBarAnimation::new("progress", 2.5);
		Self {
			loading_animation: all!(
				BasicAnimation::new("logo", 2.0, ease_out_elastic),
				progress_bar_animation
			),
			end_progress: BasicAnimation::new("end_progress", 0.25, ease_out_quad),
			progress_bar_sender,
			animations_state: HashMap::new(),
			success_icon,
			alert_icon,
			logo,
		}
	}
	pub fn update(&mut self, progress: &ProgressData, delta_time: f32) {
		self
			.progress_bar_sender
			.send(progress.get_percentage())
			.unwrap();
		self
			.animations_state
			.extend(self.loading_animation.update(delta_time));
		self
			.animations_state
			.extend(self.end_progress.update(delta_time));
	}

	pub fn render<'clay, 'render>(
		&'render self,
		progress: &'render ProgressData,
		c: &mut TibsClayScope<'clay, 'render>,
	) where
		'clay: 'render,
	{
		let end_progress_animation = self.get_animation_progress("end_progress");

		c.with(
			Declaration::new()
				.layout()
				.child_alignment(Alignment::new(LX::Center, LY::Center))
				.direction(LayoutDirection::TopToBottom)
				.width(grow!())
				.height(grow!())
				.end(),
			|c| {
				self.logo(c);
				c.with(
					Declaration::new().layout().height(fixed!(52.0)).end(),
					|_| {},
				);
				self.progress_bar(
					progress,
					end_progress_animation,
					progress.finished.then_some(&self.success_icon),
					c,
				);
				if progress.has_failed_services() && progress.finished {
					c.with(
						Declaration::new().layout().height(fixed!(22.0)).end(),
						|_| {},
					);
					self.warning(c, end_progress_animation);
				}
			},
		);
	}

	fn warning<'clay, 'render>(
		&'render self,
		c: &mut TibsClayScope<'clay, 'render>,
		end_progress_animation: f32,
	) where
		'clay: 'render,
	{
		c.with(
			Declaration::new()
				.layout()
				.child_alignment(Alignment::new(LX::Center, LY::Center))
				.direction(LayoutDirection::LeftToRight)
				.child_gap((21. * end_progress_animation) as u16)
				.end(),
			|c| {
				c.with(
					Declaration::new()
						.layout()
						.width(Sizing::Fixed(24. * end_progress_animation))
						.height(Sizing::Fixed(24. * end_progress_animation))
						.end()
						.image()
						.data(&self.alert_icon)
						.end(),
					|_| {},
				);
				c.text(
					"Some services failed to start",
					TextConfig::new()
						.color((0xFF, 0xCC, 0x00).into())
						.font_size((14. * end_progress_animation) as u16)
						.alignment(clay_layout::text::TextAlignment::Center)
						.end(),
				);
			},
		);
		c.with(
			Declaration::new()
				.layout()
				.height(Sizing::Fixed(22.0 * end_progress_animation))
				.end(),
			|_| {},
		);
		c.with(
			Declaration::new()
				.layout()
				.direction(LayoutDirection::LeftToRight)
				.child_gap((10. * end_progress_animation) as u16)
				.end(),
			|c| {
				let check_logs_button_id = c.id("check_logs_button_id");
				c.with(
					Declaration::new()
						.layout()
						.padding(Padding::new(
							(13. * end_progress_animation) as u16,
							(13. * end_progress_animation) as u16,
							(8. * end_progress_animation) as u16,
							(8. * end_progress_animation) as u16,
						))
						.end()
						.corner_radius()
						.all(8.0 * end_progress_animation)
						.end()
						.background_color(
							interpolate_color(
								(0x0C, 0x70, 0x94),
								(0, 0, 0),
								if c.pointer_over(check_logs_button_id) {
									0.1
								} else {
									0.
								},
							)
							.into(),
						)
						.id(check_logs_button_id),
					|c| {
						c.text(
							"Check Logs",
							TextConfig::new()
								.color((0xFF, 0xFF, 0xFF).into())
								.font_size((14. * end_progress_animation) as u16)
								.alignment(clay_layout::text::TextAlignment::Center)
								.end(),
						);
					},
				);
				let continue_anyway_button_id = c.id("loading_continue_anyway_button");
				c.with(
					Declaration::new()
						.layout()
						.padding(Padding::new(
							(12. * end_progress_animation) as u16,
							(12. * end_progress_animation) as u16,
							(8. * end_progress_animation) as u16,
							(8. * end_progress_animation) as u16,
						))
						.end()
						.corner_radius()
						.all(8.0 * end_progress_animation)
						.end()
						.border()
						.all_directions(1)
						.color((0x22, 0x30, 0x50).into())
						.end()
						.id(continue_anyway_button_id)
						.background_color(
							interpolate_color(
								(0x21, 0x23, 0x42),
								(0, 0, 0),
								if c.pointer_over(continue_anyway_button_id) {
									0.1
								} else {
									0.
								},
							)
							.into(),
						),
					|c| {
						c.text(
							"Continue anyway",
							TextConfig::new()
								.color((0xFF, 0xFF, 0xFF).into())
								.font_size((14. * end_progress_animation) as u16)
								.alignment(clay_layout::text::TextAlignment::Center)
								.end(),
						);
					},
				);
			},
		);
	}
	fn progress_bar<'clay, 'render>(
		&'render self,
		progress: &ProgressData,
		end_progress_animation: f32,
		leading_icon: Option<&'render skia_safe::Image>,
		c: &mut TibsClayScope<'clay, 'render>,
	) where
		'clay: 'render,
	{
		c.with(
			Declaration::new()
				.layout()
				.width(fixed!(242.0))
				.end()
				.background_color((0x18, 0x1F, 0x3F).into())
				.corner_radius()
				.all(9999.0)
				.end(),
			|c| {
				if let Some(leading_icon) = leading_icon {
					c.with(
						Declaration::new()
							.floating()
							.attach_to(FloatingAttachToElement::Parent)
							.attach_points(
								FloatingAttachPointType::RightCenter,
								FloatingAttachPointType::LeftCenter,
							)
							.offset((-24., 0.).into())
							.dimensions((12., 12.).into())
							.end()
							.image()
							.data(leading_icon)
							.end(),
						|_| {},
					);
				}
				c.with(
					Declaration::new()
						.layout()
						.width(Sizing::Percent(self.get_animation_progress("progress")))
						.height(fixed!(5.0))
						.end()
						.background_color(
							interpolate_color(
								(0xFF, 0xFF, 0xFF),
								if !progress.has_failed_services() {
									(0x4C, 0xE3, 0xA2)
								} else {
									(0xFF, 0xCC, 0x00)
								},
								end_progress_animation,
							)
							.into(),
						)
						.corner_radius()
						.all(8.0)
						.end(),
					|_| {},
				);
			},
		);
	}

	fn logo<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
	where
		'clay: 'render,
	{
		c.with(
			Declaration::new()
				.layout()
				.child_alignment(Alignment::new(LX::Center, LY::Center))
				.height(fixed!(183.0))
				.width(fixed!(183.0))
				.end(),
			|c| {
				c.with(
					Declaration::new()
						.image()
						.data(&self.logo)
						.end()
						.layout()
						.width(Sizing::Fixed(self.get_animation_progress("logo") * 183.))
						.height(Sizing::Fixed(self.get_animation_progress("logo") * 183.))
						.end(),
					// .aspect_ratio(self.logo.width() as f32 / self.logo.height() as f32),
					|_| {},
				)
			},
		);
	}
	pub fn get_animation_progress(&self, id: &str) -> f32 {
		self.animations_state.get(id).copied().unwrap_or(0.0)
	}
}
