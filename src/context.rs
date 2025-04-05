use crate::{gles_context::GlesContext, input::Input};

pub mod drm;
#[cfg(feature = "glfw")]
pub mod glfw;

pub trait TibsContext: GlesContext + Input {
    fn as_input_mut(&mut self) -> &mut dyn Input;
    fn as_gles_context_mut(&mut self) -> &mut dyn GlesContext;
    fn as_gles_context(&self) -> &dyn GlesContext;
    fn as_input(&self) -> &dyn Input;
}
impl<T: GlesContext + Input> TibsContext for T {
    fn as_gles_context(&self) -> &dyn GlesContext {
        self
    }

    fn as_input(&self) -> &dyn Input {
        self
    }
    fn as_gles_context_mut(&mut self) -> &mut dyn GlesContext {
        self
    }

    fn as_input_mut(&mut self) -> &mut dyn Input {
        self
    }
}



pub fn select_and_init_context() -> Box<dyn TibsContext> {
    let display_is_defined = std::env::var("DISPLAY").is_ok();
    if display_is_defined {
        #[cfg(not(feature = "glfw"))]
        println!("[WARN] GLFW feature is not enabled, ignoring DISPLAY variable");
        #[cfg(feature = "glfw")]
        return Box::new(glfw::GlfwContext::new("Tiago's Incredible Boot Screen"));
    }
    Box::new(drm::DrmContext::new())
}
