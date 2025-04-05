#![allow(unsafe_op_in_unsafe_fn)]

use assets_manager::AssetCache;
use context::select_and_init_context;
use skia::{create_skia_surface, init_skia};
use skia_safe::{FontMgr, FontStyle, Typeface};
use std::{mem::ManuallyDrop, sync::LazyLock};
use tibs::{
    cursor::Cursor, custom_elements::CustomElements, loading_screen::LoadingScreen, skia_clay::{clay_skia_render, create_measure_text_function}, *
};
static UBUNTU_FONT: LazyLock<Typeface> = LazyLock::new(|| {
    FontMgr::new()
        .match_family_style("UbuntuSans NF", FontStyle::normal())
        .unwrap()
});
static FONTS: LazyLock<Vec<&Typeface>> = LazyLock::new(|| vec![&UBUNTU_FONT]);
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
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
    let cursor = Cursor::new();
    while !context.should_close() {
        let (screen_width, screen_height) = context.size();
        let current_time = std::time::Instant::now();
        let delta = current_time.duration_since(last_time).as_secs_f32();
        last_time = current_time;
        context.poll_events();
        clay.pointer_state(context.mouse_position().into(), context.is_mouse_button_down(input::MouseButton::Left));
        clay.update_scroll_containers(false, context.mouse_wheel().into(), delta);
        // Render
        gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
        let mut c = clay.begin::<_, custom_elements::CustomElements>();
        let progress = boot_progress.poll_progress();
        loading_screen.render(progress, &mut c, delta);
        clay_skia_render(
            skia_surface.canvas(),
            c.end(),
            CustomElements::render,
            &FONTS,
        );
        drop(c);
        if progress.finished {
            cursor.render(skia_surface.canvas(), context.as_input());
        }
        skia_context.flush(None);

        // If failed to swap buffers, probably the context died so we recover from the error
        // By setting everything up again from scratch
        if !context.swap_buffers() {
            context = select_and_init_context();
            (skia_context, skia_surface) = init_skia(context.as_gles_context_mut())?;
        }

        if skia_surface.width() != screen_width as _ || skia_surface.height() != screen_height as _
        {
            skia_surface = create_skia_surface(&mut skia_context, screen_width, screen_height)?;
            clay.set_layout_dimensions((screen_width as f32, screen_height as f32).into());
        }

        if let Some(fps) = fps_counter.tick() {
            println!("FPS: {:.2}", fps);
        }
    }
    Ok(())
}
