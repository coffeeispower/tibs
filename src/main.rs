#![allow(unsafe_op_in_unsafe_fn)]

use background::Background;
use clay_layout::{
	fixed, grow,
	Declaration,
};
use rustamarine::screen::Screen;
pub mod skia_clay;
pub mod background;
pub mod custom_elements;
pub mod fps_counter;
pub mod gl;
pub mod gl_errors;
#[macro_use]
pub mod animation;
pub mod cursor;
pub mod loading_screen;
pub mod login;
pub mod progress_watcher;
pub mod session_manager;
pub mod skia;
pub mod skia_image_asset;
pub mod skia_shader_asset;
pub mod textbox;
pub mod tty;
pub type TibsClayScope<'clay, 'render> =
	SkiaClayScope<'clay, 'render, custom_elements::CustomElements>;

use crate::{
	animation::{easing::ease_in_out_circ, Animation, BasicAnimation},
	cursor::Cursor,
	custom_elements::CustomElements,
	loading_screen::LoadingScreen,
	login::LoginManager,
	session_manager::SessionManager, skia_clay::{create_measure_text_function, SkiaClayScope},
};
use assets_manager::AssetCache;
use skia::{create_skia_surface, init_skia};
use skia_safe::{gpu::DirectContext, FontMgr, FontStyle, Surface, Typeface};
use std::{
	rc::Rc,
	sync::{LazyLock, Mutex},
};

static UBUNTU_FONT: LazyLock<Typeface> = LazyLock::new(|| {
	FontMgr::new()
		.match_family_style("UbuntuSans NF", FontStyle::normal())
		.unwrap()
});
pub static FONTS: LazyLock<Vec<&Typeface>> = LazyLock::new(|| vec![&UBUNTU_FONT]);
struct SkiaContext {
	skia_context: DirectContext,
	skia_surface: Surface,
}

impl SkiaContext {
	pub fn new_from_screen(screen: &mut Screen) -> color_eyre::Result<Self> {
		let (skctx, sksrf) = init_skia(screen)?;
		Ok(Self {
			skia_context: skctx,
			skia_surface: sksrf,
		})
	}
}

struct AppState {
	boot_progress: progress_watcher::ProgressWatcher,
	fps_counter: fps_counter::FPSCounter,
	last_time: std::time::Instant,
	clay: clay_layout::Clay,
	context: Option<SkiaContext>,
	assets: Rc<AssetCache>,
	loading_screen: LoadingScreen,
	login_screen: login_screen::LoginScreen,
	cursor: Cursor,
	skip_animation: bool,
	screen_slide_animation: BasicAnimation,
	show_login_screen: bool,
	screen_slide_animation_progress: f32,
	devtools: bool,
	background: Background,
	should_exit: bool,
	login_manager: LoginManager,
	session_manager: SessionManager,
}

fn update_app_state(state: &mut AppState, rmar: &rustamarine::Rustamarine, screen: &mut Screen) {
	ensure_skia_context(state, screen);

	// Clamp mouse position to screen boundaries
	let screen_width = screen.get_width() as i32;
	let screen_height = screen.get_height() as i32;
	let mouse_x = rmar.get_mouse_x();
	let mouse_y = rmar.get_mouse_y();

	rmar.set_mouse_x(mouse_x.max(0).min(screen_width));
	rmar.set_mouse_y(mouse_y.max(0).min(screen_height));

	let progress = state.boot_progress.poll_progress();

	let current_time = std::time::Instant::now();

	// Handle escape key to exit
	if rmar.is_key_down(rustamarine::keys::KEY_Escape)
		&& std::env::var("TIBS_DEV_MODE") == Ok("1".to_string())
	{
		state.should_exit = true;
		return;
	}

	// Toggle devtools with Caps Lock
	if rmar.is_key_pressed(rustamarine::keys::KEY_Caps_Lock)
		&& std::env::var("TIBS_DEV_MODE") == Ok("1".to_string())
	{
		state.devtools = !state.devtools;
		state.clay.set_debug_mode(state.devtools);
	}

	// Calculate delta time
	let delta = current_time.duration_since(state.last_time).as_secs_f32();
	state.last_time = current_time;

	// Get mouse position
	let mouse_position = (rmar.get_mouse_x() as f32, rmar.get_mouse_y() as f32);

	// Update animation
	if !state.skip_animation {
		if let Some((_, p)) = state
			.screen_slide_animation
			.update(if state.show_login_screen {
				delta
			} else {
				-delta
			})
			.get(0)
		{
			state.screen_slide_animation_progress = *p;
			state.background.time_offset = state.screen_slide_animation_progress * 5.0;
		}
	}
	state
		.login_screen
		.update(&mut state.clay, rmar, &mut state.login_manager);
	state.loading_screen.update(&progress, delta);
	// Update background
	state.background.update(delta, &state.login_manager, &state.login_screen);

	// Update clay pointer state
	state.clay.pointer_state(
		(mouse_position.0, mouse_position.1).into(),
		rmar.is_mouse_button_down(0),
	);

	// Update scroll containers
	state.clay.update_scroll_containers(
		true,
		(
			rmar.get_mouse_scroll_x() as f32,
			rmar.get_mouse_scroll_y() as f32,
		)
			.into(),
		delta,
	);
	// Hot reload assets
	state.assets.hot_reload();
}
fn ensure_skia_context(state: &mut AppState, screen: &mut Screen) {
	let (screen_width, screen_height) = (screen.get_width() as u32, screen.get_height() as u32);

	if state.context.is_none() {
		let c = SkiaContext::new_from_screen(screen).unwrap();
		state
			.clay
			.set_layout_dimensions((screen_width as f32, screen_height as f32).into());

		screen
			.get_rustamarine()
			.set_mouse_x(screen_width as i32 / 2);
		screen
			.get_rustamarine()
			.set_mouse_y(screen_height as i32 / 2);
		state.context = Some(c);
	} else if let Some(ctx) = &mut state.context {
		if ctx.skia_surface.width() != screen_width as _
			|| ctx.skia_surface.height() != screen_height as _
		{
			ctx.skia_surface =
				create_skia_surface(&mut ctx.skia_context, screen_width, screen_height).unwrap();
			state
				.clay
				.set_layout_dimensions((screen_width as f32, screen_height as f32).into());
			screen
				.get_rustamarine()
				.set_mouse_x(screen_width as i32 / 2);
			screen
				.get_rustamarine()
				.set_mouse_y(screen_height as i32 / 2);
		}
	}
}
fn render_app(state: &mut AppState, screen: &mut Screen) {
	screen.use_screen();
	let rmar = screen.get_rustamarine();
	let progress = state.boot_progress.poll_progress();
	let (_screen_width, screen_height) = (screen.get_width() as u32, screen.get_height() as u32);

	// Access context
	let SkiaContext {
		skia_context,
		skia_surface,
	} = state.context.as_mut().unwrap();

	let camera_y = state.screen_slide_animation_progress * screen_height as f32;

	gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

	let canvas = skia_surface.canvas();

	state.background.render(canvas);

	{
		let mut c = state.clay.begin::<_, CustomElements>();

		c.with(
			Declaration::new()
				.layout()
				.direction(clay_layout::layout::LayoutDirection::TopToBottom)
				.width(grow!())
				.height(grow!())
				.end()
				.clip(true, true, (0.0, -camera_y).into()),
			|c| {
				let continue_anyway_button_id = c.id("loading_continue_anyway_button");
				let continue_anyway_button_clicked =
					c.pointer_over(continue_anyway_button_id) && rmar.is_mouse_button_released(0);
				if continue_anyway_button_clicked
					|| (state.loading_screen.get_animation_progress("progress") >= 0.99
						&& !progress.has_failed_services())
				{
					state.show_login_screen = true;
				}

				c.with(
					Declaration::new()
						.layout()
						.width(grow!())
						.height(fixed!(screen_height as f32))
						.end(),
					|c| {
						state.loading_screen.render(progress, c);
					},
				);
				c.with(
					Declaration::new()
						.layout()
						.width(grow!())
						.height(fixed!(screen_height as f32))
						.end(),
					|c| {
						state.login_screen.render(c, &state.login_manager);
					},
				);
			},
		);
		skia_clay::clay_skia_render(canvas, c.end(), CustomElements::render, &FONTS);
	}

	if progress.finished {
		state.cursor.render(canvas, &rmar, "default");
	}

	skia_context.flush(None);
	// Update FPS counter
	if let Some(fps) = state.fps_counter.tick() {
		println!("FPS: {:.2}", fps);
	}
	screen.swap_buffers();
}

fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;
	env_logger::init();
	let mut rmar = rustamarine::Rustamarine::new();
	gl::load_with(|n| rmar.get_opengl_proc_address(n));

	// Create assets
	let assets = Rc::new(AssetCache::new(
		std::env::var("TIBS_ASSETS_FOLDER").unwrap_or("assets".into()),
	)?);

	// Initialize boot progress and check if we can skip animation
	let mut boot_progress = progress_watcher::ProgressWatcher::new();
	let skip_animation = {
		let progress = boot_progress.poll_progress();
		progress.finished && !progress.has_failed_services()
	};

	// Create clay layout
	let mut clay = clay_layout::Clay::new((0.0, 0.0).into());
	clay.set_measure_text_function(create_measure_text_function(&FONTS));

	// Create app state
	let app_state = Mutex::new(AppState {
		boot_progress,
		fps_counter: fps_counter::FPSCounter::new(),
		last_time: std::time::Instant::now(),
		clay,
		context: None,
		loading_screen: LoadingScreen::new(&assets),
		login_screen: login_screen::LoginScreen::new(&assets),
		cursor: Cursor::new(None),
		skip_animation,
		screen_slide_animation: BasicAnimation::new("screen_slide", 1.5, ease_in_out_circ),
		show_login_screen: skip_animation,
		screen_slide_animation_progress: skip_animation as u8 as f32,
		devtools: false,
		background: Background::new(Rc::clone(&assets)),
		assets,
		should_exit: false,
		login_manager: LoginManager::new(),
		session_manager: SessionManager::new(),
	});
	let start_instant = std::time::Instant::now();
	let mut first_render = false;
	while !app_state.lock().unwrap().should_exit {
		let mut screens = rmar.screens();
		let Some(mut screen) = screens.first_mut() else {
			rmar.poll_events();
			continue;
		};

		update_app_state(
			&mut app_state.lock().unwrap(),
			&screen.get_rustamarine(),
			&mut screen,
		);
		// Set render callback
		screen.set_on_render(|mut screen| {
			if !first_render {
				first_render = true;
				let elapsed = std::time::Instant::now().duration_since(start_instant);
				println!("First render took: {:?}", elapsed);
			}
			// Update state outside of render callback
			render_app(&mut app_state.lock().unwrap(), &mut screen);
		});

		rmar.poll_events();
	}
	Ok(())
}
