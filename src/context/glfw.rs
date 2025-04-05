use crate::gl;
use crate::input::KeyboardState;
use glfw::{Context, Glfw, GlfwReceiver, PWindow, WindowEvent, WindowHint};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use super::GlesContext;

pub struct GlfwContext {
    glfw: RefCell<Glfw>,
    window: Rc<RefCell<PWindow>>,
    keyboard_state: KeyboardState,
    mouse_wheel_delta: (f64, f64),
    events: GlfwReceiver<(f64, WindowEvent)>
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
                    1,
                    1,
                    title,
                    m.map_or(glfw::WindowMode::Windowed, |m| {
                        glfw::WindowMode::FullScreen(m)
                    }),
                )
            })
            .expect("Failed to create GLFW window.");

        window.make_current();
        window.set_key_polling(true);
        window.set_cursor_mode(glfw::CursorMode::Hidden);
        let mut context = GlfwContext {
            glfw: RefCell::new(glfw),
            window: Rc::new(RefCell::new(window)),
            events,
            keyboard_state: KeyboardState::new(),
            mouse_wheel_delta: (0.0, 0.0),
        };
        gl::load_with(|symbol| context.get_proc_address(symbol));
        context
    }
    pub fn glfw_window(&self) -> Rc<RefCell<PWindow>> {
        Rc::clone(&self.window)
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