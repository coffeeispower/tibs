use std::collections::HashMap;

use super::{LoginManager, LoginState};
use crate::textbox::Textbox;
use crate::TibsClayScope;
use crate::{custom_elements::CustomElements, skia_image_asset::SkiaImageAsset};
use assets_manager::AssetCache;
use clay_layout::{fit, Clay_LayoutDirection_CLAY_LEFT_TO_RIGHT};
use rustamarine::keys::{KEY_KP_Enter, KEY_Return};
use rustamarine::Rustamarine;
use skia_safe::Image;
use uzers::os::unix::UserExt;
use uzers::{all_users, User};

use clay_layout::{
	elements::{FloatingAttachPointType, FloatingAttachToElement},
	fixed, grow,
	layout::{Alignment, LayoutAlignmentX as LX, LayoutAlignmentY as LY, LayoutDirection, Padding},
	text::TextConfig,
	Declaration,
};

pub struct LoginScreen {
	user_list: Vec<User>,
	selected_user: u32,
	selected_username: String,
	login_icon: Image,
	eye_icon: Image,
	eye_off_icon: Image,
	avatars: HashMap<u32, Image>,
	password_input: Textbox,
}

fn is_user_uid(uid: u32) -> bool {
	return uid >= 1000 && uid < 65534;
}

fn load_avatar(u: &User) -> Option<Image> {
	let face_file_path = u.home_dir().join(".face");
	let face_image_data = skia_safe::Data::from_filename(face_file_path)?;
	let face_image = Image::from_encoded(face_image_data)?;
	return Some(face_image);
}

impl LoginScreen {
	pub fn username(&self) -> &str {
		&self.selected_username
	}
	pub fn new(assets: &AssetCache) -> Self {
		let SkiaImageAsset(login_icon) = assets
			.load_owned("icons.login")
			.expect("Failed to load icons.login");
		let SkiaImageAsset(eye_icon) = assets
			.load_owned("icons.eye")
			.expect("Failed to load icons.eye");
		let SkiaImageAsset(eye_off_icon) = assets
			.load_owned("icons.eye-off")
			.expect("Failed to load icons.eye-off");

		let user_list = unsafe { all_users() }
			.filter(|u| is_user_uid(u.uid()) && !u.shell().ends_with("nologin"))
			.collect::<Vec<User>>();

		let selected_user = user_list[0].uid();
		let selected_username = user_list[0].name().to_str().unwrap().to_string();
		Self {
			avatars: user_list
				.iter()
				.filter_map(|u| Some((u.uid(), load_avatar(u)?)))
				.collect(),
			user_list,
			selected_user,
			selected_username,
			login_icon,
			password_input: Textbox::new("login-input", true),
			eye_icon,
			eye_off_icon,
		}
	}
	pub fn update<'clay, 'render>(
		&'render mut self,
		c: &mut clay_layout::Clay,
		rmar: &Rustamarine,
		login_manager: &mut LoginManager,
	) where
		'clay: 'render,
	{
		if let Some(selected) = self
			.user_list
			.iter()
			.find(|u| u.uid() == self.selected_user)
		{
			let n = selected.name().to_str().unwrap();
			if self.selected_username != n {
				self.selected_username = n.to_string();
			}
		}
		self.password_input.update(rmar, &mut *c);
		if c.pointer_over(c.id("show-password")) && rmar.is_mouse_button_pressed(0) {
			self.password_input.hide_input = !self.password_input.hide_input
		}
		if ((c.pointer_over(c.id("login-button")) && rmar.is_mouse_button_pressed(0)) || (self.password_input.is_focused() && (rmar.is_key_pressed(KEY_Return)||rmar.is_key_pressed(KEY_KP_Enter)))) && !self.password_input.disabled {
			login_manager.start_login(&self.selected_username, self.password_input.text(), true);
		}
		self.password_input.disabled = matches!(
			login_manager.get_current_login_state(&self.selected_username),
			Some(LoginState::Logging)
		);
	}
	pub fn render<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>,
	login_manager: &LoginManager)
	where
		'clay: 'render,
	{
		self.render_user_list(c);
		self.render_selected_user(c, login_manager);
	}

	fn render_user_list<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
	where
		'clay: 'render,
	{
		c.with(
			Declaration::new()
				.floating()
				.attach_to(FloatingAttachToElement::Parent)
				.attach_points(
					FloatingAttachPointType::LeftBottom,
					FloatingAttachPointType::LeftBottom,
				)
				.offset((55.0, -55.0).into())
				.end()
				.layout()
				.direction(LayoutDirection::TopToBottom)
				.width(fit!(250.0))
				.end(),
			|c| {
				for user in &self.user_list {
					let is_selected = user.uid() == self.selected_user;
					self.render_user_item(c, user, is_selected);
				}
			},
		);
	}

	fn render_user_item<'clay, 'render>(
		&'render self,
		c: &mut TibsClayScope<'clay, 'render>,
		user: &'render User,
		is_selected: bool,
	) where
		'clay: 'render,
	{
		let user_name = user.name().to_str().unwrap();
		let id = c.id(&format!("user_item-{user_name}"));
		// If the user is selected, apply a highlight background color.
		let mut decl = Declaration::new();
		decl
			.layout()
			.direction(LayoutDirection::LeftToRight)
			.padding(Padding::all(5))
			.child_gap(20)
			.child_alignment(Alignment::new(LX::Left, LY::Center))
			.width(grow!())
			.padding(Padding::all(10))
			.end()
			.corner_radius()
			.all(10.)
			.end()
			.id(id);

		let is_hovered = c.pointer_over(id);
		if is_hovered {
			decl.background_color((0x2E / 2, 0x3E / 2, 0x4E / 2, 0x30).into());
		} else if is_selected {
			decl.background_color((0x2E, 0x3E, 0x4E, 0x30).into());
		}

		c.with(&decl, |c| {
			// User avatar as a circle
			let mut avatar_declaration = Declaration::new();
			avatar_declaration
				.layout()
				.width(fixed!(50.0))
				.height(fixed!(50.0))
				.end()
				.corner_radius()
				.all(99999.0)
				.end();
			if let Some(avatar) = self.avatars.get(&user.uid()) {
				avatar_declaration.image().data(avatar).end();
			}
			c.with(&avatar_declaration, |_| {});
			// Display name text
			c.text(
				&user_name,
				TextConfig::new()
					.color((0xFF, 0xFF, 0xFF).into())
					.font_size(20)
					.alignment(clay_layout::text::TextAlignment::Left)
					.end(),
			);
		});
	}

	fn render_selected_user<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>, login_manager: &LoginManager)
	where
		'clay: 'render,
	{
		// Retrieve the selected user info
		if let Some(selected) = self
			.user_list
			.iter()
			.find(|u| u.uid() == self.selected_user)
		{
			c.with(
				Declaration::new()
					.layout()
					.child_alignment(Alignment::new(LX::Center, LY::Center))
					.width(grow!())
					.height(grow!())
					.end(),
				|c| {
					// Container for the selected user avatar and name
					c.with(
						Declaration::new()
							.layout()
							.child_alignment(Alignment::new(LX::Center, LY::Center))
							.padding(Padding::new(10, 10, 24, 24))
							.width(grow!(238.0))
							.direction(LayoutDirection::TopToBottom)
							.padding(Padding::all(20))
							.end(),
						|c| {
							let mut avatar_declaration = Declaration::new();
							avatar_declaration
								.layout()
								.width(fixed!(128.0))
								.height(fixed!(128.0))
								.end()
								.background_color((0xAA, 0xAA, 0xAA, 0x30).into())
								.corner_radius()
								.all(99999.0)
								.end();

							if let Some(avatar) = self.avatars.get(&selected.uid()) {
								avatar_declaration.image().data(avatar).end();
							}
							// Selected user avatar
							c.with(&avatar_declaration, |_| {});
							// Space between avatar and name
							c.with(
								Declaration::new()
									.layout()
									.width(grow!())
									.height(fixed!(20.0))
									.end(),
								|_| {},
							);
							let user_name = selected.name().to_str().unwrap();

							// Selected user name text
							c.text(
								&user_name,
								TextConfig::new()
									.color((0xFF, 0xFF, 0xFF).into())
									.font_size(32)
									.alignment(clay_layout::text::TextAlignment::Center)
									.end(),
							);

							c.with(
								Declaration::new()
									.layout()
									.child_alignment(Alignment::new(LX::Center, LY::Center))
									.end()
									.layout()
									.padding(Padding::new(0, 0, 56, 0))
									.child_gap(14)
									.end(),
								|c| {
									self.password_input.render(c);
									self.render_eye_button(c);
									self.render_login_button(c, login_manager);
								},
							);
						},
					);
				},
			);
		}
	}

	fn render_login_button<'clay, 'render>(
		&'render self,
		c: &mut TibsClayScope<'clay, 'render>,
		login_manager: &LoginManager,
) where
		'clay: 'render,
{
		let login_state = login_manager.get_current_login_state(&self.selected_username);

		let mut button_decl = Declaration::new();
		button_decl
			.id(c.id("login-button"))
			.layout()
			.child_alignment(Alignment::new(LX::Center, LY::Center))
			.width(fixed!(50.0))
			.height(fixed!(50.0))
			.end()
			.background_color((0x0E, 0x1A, 0x26, 0x30).into())
			.corner_radius()
			.all(10.0)
			.end();

		// Adiciona borda vermelha se login falhar
		if matches!(login_state, Some(LoginState::Failed)) {
			button_decl.border().color((255, 0, 0, 255).into()).all_directions(2).end();
		}

		c.with(&button_decl, |c| {
			match login_state {
				Some(LoginState::Logging) => {
					// Mostra apenas o spinner
					c.with(
						Declaration::new()
							.layout()
							.width(fixed!(18.0))
							.height(fixed!(18.0))
							.end()
							.custom_element(&CustomElements::Spinner),
						|_| {},
					)
				}
				_ => {
					// Ícone normal do botão de login
					c.with(
						Declaration::new()
							.image()
							.data(&self.login_icon)
							.end()
							.layout()
							.width(fixed!(24.0))
							.height(fixed!(24.0))
							.end(),
						|_| {},
					)
				}
			}
		});
}

	fn render_eye_button<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
	where
		'clay: 'render,
	{
		c.with(
			Declaration::new()
				.layout()
				.child_alignment(Alignment::new(LX::Center, LY::Center))
				.width(fixed!(50.0))
				.height(fixed!(50.0))
				.end()
				.background_color((0x0E, 0x1A, 0x26, 0x30).into())
				.corner_radius()
				.all(10.0)
				.end()
				.id(c.id("show-password")),
			|c| {
				let icon = if self.password_input.hide_input {
					&self.eye_icon
				} else {
					&self.eye_off_icon
				};
				c.with(
					Declaration::new()
						.image()
						.data(icon)
						.end()
						.layout()
						.width(fixed!(24.0))
						.height(fixed!(24.0))
						.end(),
					|_| {},
				);
			},
		);
	}
}
