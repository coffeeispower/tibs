use crate::gl;
use crate::input::KeyboardState;
use glfw::{Context, Glfw, GlfwReceiver, MouseButton, PWindow, WindowEvent, WindowHint};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use super::GlesContext;

pub struct GlfwContext {
    glfw: RefCell<Glfw>,
    window: Rc<RefCell<PWindow>>,
    keyboard_state: KeyboardState,
    mouse_wheel_delta: (f64, f64),
    events: GlfwReceiver<(f64, WindowEvent)>,
    mouse_state_changes: HashMap<MouseButton, bool>,
    old_mouse_state: HashMap<MouseButton, bool>,
}

impl GlfwContext {
    pub fn new(title: &str) -> Self {
        let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

        glfw.window_hint(WindowHint::ContextVersion(3, 0));
        glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(WindowHint::OpenGlForwardCompat(true));
        glfw.window_hint(WindowHint::ClientApi(glfw::ClientApiHint::OpenGlEs));
        glfw.window_hint(WindowHint::ContextCreationApi(
            glfw::ContextCreationApi::Egl,
        ));
        let (mut window, events) = glfw
            .with_primary_monitor(|glfw, m| {
                glfw.create_window(
                    1280,
                    800,
                    title,
                    glfw::WindowMode::Windowed
                )
            })
            .expect("Failed to create GLFW window.");

        window.make_current();
        window.set_key_polling(true);
        window.set_scroll_polling(true);
        window.set_cursor_mode(glfw::CursorMode::Hidden);
        let mut context = GlfwContext {
            glfw: RefCell::new(glfw),
            window: Rc::new(RefCell::new(window)),
            events,
            keyboard_state: KeyboardState::new(),
            mouse_wheel_delta: (0.0, 0.0),
            mouse_state_changes: HashMap::new(),
            old_mouse_state: HashMap::new(),
        };
        gl::load_with(|symbol| context.get_proc_address(symbol));
        context
    }
    pub fn glfw_window(&self) -> Rc<RefCell<PWindow>> {
        Rc::clone(&self.window)
    }
    fn on_mouse_button_change(&mut self, button: MouseButton, action: glfw::Action) {
        match action {
            glfw::Action::Press => {
                self.mouse_state_changes.insert(button, true);
            }
            glfw::Action::Release => {
                self.mouse_state_changes.insert(button, false);
            }
            _ => {}
        }
    }
}

impl GlesContext for GlfwContext {
    fn get_proc_address(&mut self, fn_name: &str) -> *const c_void {
        self.window.borrow_mut().get_proc_address(fn_name)
    }
    fn swap_buffers(&self) -> bool {
        self.glfw.borrow_mut().poll_events();
        let mut window = self.window.borrow_mut();
        window.swap_buffers();
        true
    }

    fn size(&self) -> (u32, u32) {
        let (width, height) = self.window.borrow().get_size();
        (width as u32, height as u32)
    }
}
pub mod input;