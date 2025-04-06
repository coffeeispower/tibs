use assets_manager::AssetCache;
use skia_safe::Image;
use crate::{custom_elements::CustomElements, skia_image_asset::SkiaImageAsset};
use crate::TibsClayScope;

use clay_layout::{
    elements::{FloatingAttachPointType, FloatingAttachToElement},
    fixed, grow,
    layout::{Alignment, LayoutAlignmentX as LX, LayoutAlignmentY as LY, LayoutDirection, Padding, Sizing},
    text::TextConfig,
    Declaration,
};

pub struct LoginScreen {
    user_list: Vec<(CustomElements, String)>,
    selected_user: (CustomElements, String),
    login_icon: Image,
}

impl LoginScreen {
    pub fn new(assets: &AssetCache) -> Self {
        let SkiaImageAsset(avatar1) = assets.load_owned("avatar.user1").expect("Failed to load avatar.user1");
        let SkiaImageAsset(avatar2) = assets.load_owned("avatar.user2").expect("Failed to load avatar.user2");
        let SkiaImageAsset(login_icon) = assets.load_owned("icons.login").expect("Failed to load icons.login");

        let user_list = vec![
            (CustomElements::Avatar { image: avatar1.clone() }, String::from("Alice")),
            (CustomElements::Avatar { image: avatar2.clone() }, String::from("Bob")),
        ];

        let selected_user = user_list[0].clone();
        Self {
            user_list,
            selected_user,
            login_icon,
        }
    }

    pub fn render<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        c.with(Declaration::new().background_color((0x0F, 0x14, 0x19).into()).layout().width(grow!()).height(grow!()).end(), |c| {
            self.render_user_list(c);
            self.render_selected_user(c);
        });
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
                .end(),
            |c| {
                for (avatar, name) in &self.user_list {
                    Self::render_user_item(c, avatar, name);
                }
            },
        );
    }

    fn render_user_item<'clay, 'render>(c: &mut TibsClayScope<'clay, 'render>, avatar: &'render CustomElements, name: &str)
    where
        'clay: 'render,
    {
        c.with(
            Declaration::new()
                .layout()
                .direction(LayoutDirection::LeftToRight)
                .padding(Padding::all(5))
                .child_gap(20)
                .child_alignment(Alignment::new(LX::Left, LY::Center))
                .end(),
            |c| {
                // User avatar as a circle
                c.with(
                    Declaration::new()
                        .layout()
                        .width(fixed!(50.0))
                        .height(fixed!(50.0))
                        .end()
                        .custom_element(avatar),
                    |_| {},
                );
                // Username text
                c.text(
                    name,
                    TextConfig::new()
                        .color((0xFF, 0xFF, 0xFF).into())
                        .font_size(20)
                        .alignment(clay_layout::text::TextAlignment::Left)
                        .end(),
                );
            },
        );
    }

    fn render_selected_user<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
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
                        // Selected user avatar
                        c.with(
                            Declaration::new()
                                .layout()
                                .width(fixed!(128.0))
                                .height(fixed!(128.0))
                                .end()
                                .background_color((0xAA, 0xAA, 0xAA).into())
                                .corner_radius()
                                .all(99999.0)
                                .end()
                                .custom_element(&self.selected_user.0),
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
                        // Selected user name text
                        c.text(
                            &self.selected_user.1,
                            TextConfig::new()
                                .color((0xFF, 0xFF, 0xFF).into())
                                .font_size(32)
                                .alignment(clay_layout::text::TextAlignment::Center)
                                .end(),
                        );
                        c.with(Declaration::new().layout().child_alignment(Alignment::new(LX::Center, LY::Center)).end()
                        .layout().padding(Padding::new(0, 0, 56, 0)).child_gap(14).end(), |c| {

                            // Textbox for future input
                            Self::render_textbox(c);
                            // "Login" button
                            self.render_login_button(c);
                        });
                    },
                );
            },
        );
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

    fn render_login_button<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
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
                let dimensions = self.login_icon.dimensions();
                c.with(
                    Declaration::new()
                        .image()
                        .data(&self.login_icon)
                        .source_dimensions((dimensions.width as f32, dimensions.height as f32).into())
                        .end()
                        .layout()
                        .width(fixed!(24.))
                        .height(fixed!(24.))
                        .end(),
                    |_| {},
                );
            },
        );
    }
}