#![allow(unsafe_op_in_unsafe_fn)]

use animation::easing::ease_in_out_cubic;
use background::Background;
use clay_layout::{fixed, grow, Declaration};
use skia_clay::SkiaClayScope;
use xkbcommon::xkb::Keysym;

pub mod background;
pub mod custom_elements;
pub mod fps_counter;
pub mod gl;
pub mod gl_errors;
pub mod login_screen;
#[macro_use]
pub mod animation;
pub mod context;
pub mod cursor;
pub mod gles_context;
pub mod input;
pub mod loading_screen;
pub mod progress_watcher;
pub mod skia;
pub mod skia_clay;
pub mod skia_image_asset;
pub mod skia_shader_asset;
pub mod textbox;
pub type TibsClayScope<'clay, 'render> =
    SkiaClayScope<'clay, 'render, custom_elements::CustomElements>;

use crate::{
    animation::{easing::ease_in_out_circ, Animation, BasicAnimation},
    cursor::Cursor,
    custom_elements::CustomElements,
    loading_screen::LoadingScreen,
    skia_clay::{clay_skia_render, create_measure_text_function},
};
use assets_manager::AssetCache;
use context::select_and_init_context;
use skia::{create_skia_surface, init_skia};
use skia_safe::{FontMgr, FontStyle, Typeface};
use std::{mem::ManuallyDrop, sync::LazyLock};

static UBUNTU_FONT: LazyLock<Typeface> = LazyLock::new(|| {
    FontMgr::new()
        .match_family_style("UbuntuSans NF", FontStyle::normal())
        .unwrap()
});
pub static FONTS: LazyLock<Vec<&Typeface>> = LazyLock::new(|| vec![&UBUNTU_FONT]);

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init();
    let mut boot_progress = progress_watcher::ProgressWatcher::new();
    let mut context = select_and_init_context();
    let mut fps_counter = fps_counter::FPSCounter::new();

    let mut last_time = std::time::Instant::now();
    let (mut skia_context, mut skia_surface) = init_skia(context.as_gles_context_mut())?;
    let (screen_width, screen_height) = context.size();
    let mut clay = ManuallyDrop::new(clay_layout::Clay::new(
        (screen_width as f32, screen_height as f32).into(),
    ));
    clay.set_measure_text_function(create_measure_text_function(&FONTS));
    let assets = AssetCache::new(std::env::var("TIBS_ASSETS_FOLDER").unwrap_or("assets".into()))?;
    let mut loading_screen = LoadingScreen::new(&assets);
    let mut login_screen = login_screen::LoginScreen::new(&assets);
    let mut cursor = Cursor::new(None);
    let skip_animation = {
        let progress = boot_progress.poll_progress();
        progress.finished && !progress.has_failed_services()
    };
    let mut screen_slide_animation = BasicAnimation::new("screen_slide", 1.5, ease_in_out_circ);
    let mut show_login_screen = skip_animation;
    let mut screen_slide_animation_progress: f32 = skip_animation as u8 as f32;
    let mut devtools = false;
    let mut background = Background::new(&assets);
    while !context.should_close() {
        context.poll_events();

        if !context.has_focus() {
            continue;
        }
        if context.is_key_pressed(Keysym::Caps_Lock) {
            devtools = !devtools;
            clay.set_debug_mode(devtools);
        }
        let (screen_width, screen_height) = context.size();
        let current_time = std::time::Instant::now();
        let delta = current_time.duration_since(last_time).as_secs_f32();
        last_time = current_time;
        let camera_y = screen_slide_animation_progress * screen_height as f32;
        let mouse_position = context.mouse_position();

        let progress = boot_progress.poll_progress();
        if !skip_animation {
            if let Some((_, p)) = screen_slide_animation
                .update(if show_login_screen { delta } else { -delta })
                .get(0)
            {
                screen_slide_animation_progress = *p;
                background.time_offset = screen_slide_animation_progress * 5.0;
            }
        }
        background.update(delta);

        clay.pointer_state(
            (mouse_position.0, mouse_position.1).into(),
            context.is_mouse_button_down(input::MouseButton::Left),
        );
        clay.update_scroll_containers(true, context.mouse_wheel().into(), delta);

        if let Some(fps) = fps_counter.tick() {
            println!("FPS: {:.2}", fps);
        }
        assets.hot_reload();

        // Render
        gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        let canvas = skia_surface.canvas();

        background.render(canvas);

        {
            let mut c = clay.begin::<_, CustomElements>();
            let root_container_scroll = c.scroll_container_data(c.id("root"));

            if let Some(root_container_scroll) = root_container_scroll {
                unsafe { (*root_container_scroll.scrollPosition).y = -camera_y }
            }
            c.with(
                Declaration::new()
                    .layout()
                    .direction(clay_layout::layout::LayoutDirection::TopToBottom)
                    .width(grow!())
                    .height(grow!())
                    .end()
                    .scroll(true, true)
                    .id(c.id("root")),
                |c| {
                    let continue_anyway_button_id = c.id("loading_continue_anyway_button");
                    let continue_anyway_button_clicked = c.pointer_over(continue_anyway_button_id)
                        && context.is_mouse_button_released(input::MouseButton::Left);
                    if continue_anyway_button_clicked
                        || (loading_screen.get_animation_progress("progress") >= 0.99
                            && !progress.has_failed_services())
                    {
                        show_login_screen = true;
                    }

                    c.with(
                        Declaration::new()
                            .layout()
                            .width(grow!())
                            .height(fixed!(screen_height as f32))
                            .end(),
                        |c| {
                            loading_screen.render(progress, c, delta);
                        },
                    );
                    c.with(
                        Declaration::new()
                            .layout()
                            .width(grow!())
                            .height(fixed!(screen_height as f32))
                            .end(),
                        |c| {
                            login_screen.render(c, context.as_input());
                        },
                    );
                },
            );
            clay_skia_render(canvas, c.end(), CustomElements::render, &FONTS);
        }

        if progress.finished {
            cursor.render(canvas, context.as_input(), "default");
        }

        skia_context.flush(None);

        // Recover a dead context if needed.
        if !context.swap_buffers() {
            context = select_and_init_context();
            (skia_context, skia_surface) = init_skia(context.as_gles_context_mut())?;
        }

        if skia_surface.width() != screen_width as _ || skia_surface.height() != screen_height as _
        {
            skia_surface = create_skia_surface(&mut skia_context, screen_width, screen_height)?;
            clay.set_layout_dimensions((screen_width as f32, screen_height as f32).into());
        }
    }

    Ok(())
}
