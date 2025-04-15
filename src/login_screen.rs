use std::collections::HashMap;

use assets_manager::AssetCache;
use clay_layout::fit;
use skia_safe::Image;
use uzers::os::unix::UserExt;
use crate::{custom_elements::CustomElements, skia_image_asset::SkiaImageAsset};
use crate::TibsClayScope;
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
    login_icon: Image,
    avatars: HashMap<u32, CustomElements>
}

fn is_user_uid(uid: u32) -> bool {
    return uid >= 1000 && uid < 65534 ;
}

fn load_avatar(u: &User) -> Option<CustomElements> {
    let face_file_path = u.home_dir().join(".face");
    let face_image_data = skia_safe::Data::from_filename(face_file_path)?;
    let face_image = skia_safe::Image::from_encoded(face_image_data)?;
    return Some(CustomElements::Avatar { image: face_image });
}

impl LoginScreen {
    pub fn new(assets: &AssetCache) -> Self {
        let SkiaImageAsset(login_icon) = assets.load_owned("icons.login").expect("Failed to load icons.login");

        let user_list = unsafe { all_users() }.filter(|u| is_user_uid(u.uid()) && !u.shell().ends_with("nologin")).collect::<Vec<User>>();
        
        let selected_user = 1000;

        Self {
            avatars: user_list.iter().filter_map(|u| Some((u.uid(), load_avatar(u)?))).collect(),
            user_list,
            selected_user,
            login_icon,
        }
    }

    pub fn render<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        c.with(
            Declaration::new()
                .background_color((0x0F, 0x14, 0x19).into())
                .layout()
                .width(grow!())
                .height(grow!())
                .end(),
            |c| {
                self.render_user_list(c);
                self.render_selected_user(c);
            },
        );
    }

    fn render_user_list<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        c.with(
            Declaration::new()
                .floating()
                .attach_to(FloatingAttachToElement::Parent)
                .attach_points(FloatingAttachPointType::LeftBottom, FloatingAttachPointType::LeftBottom)
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
    )
    where
        'clay: 'render,
    {
        let user_name = user.name().to_str().unwrap();
        let id = c.id(&format!("user_item-{user_name}"));
        // If the user is selected, apply a highlight background color.
        let mut decl = Declaration::new();
        decl.layout()
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
            println!("Hover");
            decl.background_color((0x2E/2, 0x3E/2, 0x4E/2).into());
        } else if is_selected {
            decl.background_color((0x2E, 0x3E, 0x4E).into());
        }

        c.with(&decl, |c| {
            // User avatar as a circle
            let mut avatar_declaration = Declaration::new();
            avatar_declaration.layout()
                    .width(fixed!(50.0))
                    .height(fixed!(50.0))
                    .end();
            if let Some(avatar) = self.avatars.get(&user.uid()){
                avatar_declaration.custom_element(avatar);
            }
            c.with(
                &avatar_declaration,
                |_| {},
            );
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

    fn render_selected_user<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        // Retrieve the selected user info
        if let Some(selected) = self.user_list.iter().find(|u| u.uid() == self.selected_user) {
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
                            avatar_declaration.layout()
                                .width(fixed!(128.0))
                                .height(fixed!(128.0))
                                .end()
                                .background_color((0xAA, 0xAA, 0xAA).into())
                                .corner_radius()
                                .all(99999.0)
                                .end();

                            if let Some(avatar) = self.avatars.get(&selected.uid()){
                                avatar_declaration.custom_element(avatar);
                            }
                            // Selected user avatar
                            c.with(
                                &avatar_declaration,
                                |_| {},
                            );
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
                                    // Textbox for future input
                                    Self::render_textbox(c);
                                    // "Login" button
                                    Self::render_login_button(c, &self.login_icon);
                                },
                            );
                        },
                    );
                },
            );
        }
    }

    fn render_textbox<'clay, 'render>(c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        c.with(
            Declaration::new()
                .layout()
                .width(fixed!(300.0))
                .height(fixed!(50.0))
                .end()
                .background_color((0x0E, 0x1A, 0x26).into())
                .corner_radius()
                .all(10.0)
                .end(),
            |_| {},
        );
    }

    fn render_login_button<'clay, 'render>(
        c: &mut TibsClayScope<'clay, 'render>,
        login_icon: &'render Image,
    )
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
                .background_color((0x0E, 0x1A, 0x26).into())
                .corner_radius()
                .all(10.0)
                .end(),
            |c| {
                let dimensions = login_icon.dimensions();
                c.with(
                    Declaration::new()
                        .image()
                        .data(login_icon)
                        .source_dimensions((dimensions.width as f32, dimensions.height as f32).into())
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
