use std::{
	ffi::{c_void, CString},
	marker::PhantomData,
	mem::ManuallyDrop,
	str::FromStr,
};

mod sys;
#[cfg(feature = "opengl_loader")]
pub use sys::opengl;
mod input;
pub use input::keys;
pub mod screen;
pub struct Rustamarine {
	inner: *mut sys::Rustamarine,
}

impl Rustamarine {
	pub fn new() -> Self {
		Self {
			inner: unsafe { sys::rmarInitialize() },
		}
	}
	pub fn poll_events(&mut self) {
		unsafe { sys::rmarPollEvents(self.inner) };
	}

	pub fn get_opengl_proc_address(&self, name: &str) -> *const c_void {
		let name_cstring = CString::from_str(name).unwrap();
		unsafe { sys::rmarGetProcAddress(self.inner, name_cstring.as_ptr()) }
	}

	pub fn is_drm(&self) -> bool {
		unsafe { sys::rmarIsDRM(self.inner) }
	}

	pub fn go_to_tty(&mut self, tty: u16) {
		unsafe { sys::rmarGoToTTY(self.inner, tty) }
	}
}

impl Drop for Rustamarine {
	fn drop(&mut self) {
		unsafe {
			sys::rmarTearDown(self.inner);
		}
	}
}

pub struct RustamarineRef<'r> {
	pub(crate) inner: ManuallyDrop<Rustamarine>,
	pub(crate) _e: PhantomData<&'r ()>,
}

impl std::ops::Deref for RustamarineRef<'_> {
	type Target = Rustamarine;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl std::ops::DerefMut for RustamarineRef<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
