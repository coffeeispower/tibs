use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    os::{fd::{AsRawFd, OwnedFd}, unix::fs::OpenOptionsExt},
    path::Path,
};

use input::{
    event::{
        keyboard::KeyboardEventTrait,
        pointer::{Axis as LibInputPointerAxis, ButtonState, PointerScrollEvent},
        PointerEvent,
    },
    LibinputInterface,
};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use xkbcommon::xkb::{self, Keycode};

use crate::{gles_context::GlesContext, input::Input};

use super::DrmContext;

pub struct InputInterface;

impl LibinputInterface for InputInterface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
    }
}

impl Input for DrmContext {
    fn is_key_pressed(&self, key_code: xkb::Keysym) -> bool {
        self.keyboard_state.was_key_pressed(key_code)
    }

    fn is_key_down(&self, key_code: xkb::Keysym) -> bool {
        self.keyboard_state.is_key_down(key_code)
    }

    fn is_key_up(&self, key_code: xkb::Keysym) -> bool {
        !self.keyboard_state.is_key_down(key_code)
    }
    fn is_key_released(&self, key_code: xkbcommon::xkb::Keysym) -> bool {
        self.keyboard_state.was_key_released(key_code)
    }

    fn get_pressed_keys(&self) -> HashSet<xkb::Keysym> {
        self.keyboard_state.get_pressed_keys()
    }

    fn get_released_keys(&self) -> HashSet<xkb::Keysym> {
        self.keyboard_state.get_released_keys()
    }

    fn is_mouse_button_pressed(&self, button: crate::input::MouseButton) -> bool {
        // Check if the mouse button state changed to pressed this frame.
        *self
            .mouse_state
            .buttons_state_changes
            .get(&button)
            .unwrap_or(&false)
    }

    fn is_mouse_button_down(&self, button: crate::input::MouseButton) -> bool {
        // Check whether the mouse button is currently down.
        *self
            .mouse_state
            .buttons_state
            .get(&button)
            .unwrap_or(&false)
    }

    fn is_mouse_button_up(&self, button: crate::input::MouseButton) -> bool {
        !self.is_mouse_button_down(button)
    }

    fn mouse_position(&self) -> (f32, f32) {
        // Return the current mouse position (converted from u32 to f64).
        (
            self.mouse_state.mouse_position.0 as f32,
            self.mouse_state.mouse_position.1 as f32,
        )
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        // Return the accumulated mouse wheel delta for this frame.
        let (x, y) = self.mouse_state.mouse_wheel_delta;
        (x as f32, y as f32)
    }
    fn poll_events(&mut self) {
        let new_focus = super::TTY_FOCUS.load(std::sync::atomic::Ordering::Relaxed);
        if self.focused != new_focus {
            if new_focus {
                if unsafe {
                    libc::ioctl(self.gbm.0.as_raw_fd(), 0x2000641e, 0)
                } != 0 {
                    println!("Failed to resume rendering")
                }
            } else {
                if unsafe {
                    libc::ioctl(self.gbm.0.as_raw_fd(), 0x2000641f, 0)
                } != 0 {
                    println!("Failed to pause rendering")
                }
            }
        }
        self.focused = new_focus;

        // Reset the keyboard and mouse state
        self.mouse_state.new_frame();
        self.keyboard_state.new_frame();

        // Handle libinput events
        self.libinput.dispatch().unwrap();
        let screen_size = self.size();
        let mouse_state = &mut self.mouse_state;
        let keyboard_state = &mut self.keyboard_state;
        for event in &mut self.libinput {
            match event {
                input::Event::Keyboard(keyboard_event) => {
                    let keycode: Keycode = keyboard_event.key().into();
                    let keysyms = self.xkb_state.key_get_syms(keycode);
                    let keystate = keyboard_event.key_state();
                    for &keysym in keysyms {
                        keyboard_state.process_keyboard_event(keysym, keystate);
                    }
                }
                input::Event::Pointer(pointer_event) => {
                    mouse_state.process_pointer_event(pointer_event, screen_size)
                }

                input::Event::Touch(_) => {
                    unimplemented!("Touch screen support is not supported yet")
                }
                _ => {}
            }
        }
    }
    
    fn is_mouse_button_released(&self, button: crate::input::MouseButton) -> bool {
        self.mouse_state
            .buttons_state_changes.get(&button)
            .map(|&state| !state)
            .unwrap_or(false)
    }

    fn has_focus(&self) -> bool {
        self.focused
    }
}

#[derive(Default)]
pub struct MouseState {
    /// Absolute position of the mouse cursor
    mouse_position: (u32, u32),
    /// Delta position of the mouse cursor since the last frame
    mouse_position_delta: (i32, i32),
    /// The amount the mouse wheel was scrolled in this frame
    mouse_wheel_delta: (f64, f64),
    // The current state of the mouse buttons
    buttons_state: HashMap<crate::input::MouseButton, bool>,
    // Similar to buttons_state except it only lasts for 1 frame
    // and is used to detect if the button was pressed or released in this frame
    buttons_state_changes: HashMap<crate::input::MouseButton, bool>,
}
impl MouseState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_at_middle(screen_width: u32, screen_height: u32) -> Self {
        Self {
            mouse_position: (screen_width / 2, screen_height / 2),
            ..Self::new()
        }
    }
    pub fn new_frame(&mut self) {
        // Reset the mouse wheel delta and button state changes for the new frame
        self.mouse_wheel_delta = (0.0, 0.0);
        self.buttons_state_changes.clear();
    }

    pub fn process_pointer_event(&mut self, pointer_event: PointerEvent, screen_size: (u32, u32)) {
        match pointer_event {
            PointerEvent::Motion(motion_event) => {
                // Update mouse position and delta
                self.mouse_position_delta = (motion_event.dx() as i32, motion_event.dy() as i32);
                self.mouse_position.0 =
                    (self.mouse_position.0 as i32 + motion_event.dx() as i32).max(0).min(screen_size.0 as i32) as u32;
                self.mouse_position.1 =
                    (self.mouse_position.1 as i32 + motion_event.dy() as i32).max(0).min(screen_size.1 as i32) as u32;
            }
            PointerEvent::Button(button_event) => {
                // Update button state
                let button = match button_event.button() {
                    272 => crate::input::MouseButton::Left,
                    273 => crate::input::MouseButton::Right,
                    274 => crate::input::MouseButton::Middle,
                    b => crate::input::MouseButton::Other(b-271),
                };
                self.buttons_state
                    .insert(button, button_event.button_state() == ButtonState::Pressed);
                self.buttons_state_changes
                    .insert(button, button_event.button_state() == ButtonState::Pressed);
            }
            PointerEvent::MotionAbsolute(pointer_motion_absolute_event) => {
                // Update using absolute mouse position
                let new_position = (
                    pointer_motion_absolute_event.absolute_x_transformed(screen_size.0) as i32,
                    pointer_motion_absolute_event.absolute_y_transformed(screen_size.1) as i32,
                );
                // Get delta
                let delta_x = new_position.0 - self.mouse_position.0 as i32;
                let delta_y = new_position.1 - self.mouse_position.1 as i32;
                self.mouse_position_delta = (delta_x, delta_y);
                // Update mouse position
                self.mouse_position = (new_position.0 as u32, new_position.1 as u32);
            }
            PointerEvent::ScrollWheel(pointer_scroll_wheel_event) => {
                // Update mouse wheel delta
                self.mouse_wheel_delta.0 +=
                    pointer_scroll_wheel_event.scroll_value_v120(LibInputPointerAxis::Horizontal);
                self.mouse_wheel_delta.1 +=
                    pointer_scroll_wheel_event.scroll_value_v120(LibInputPointerAxis::Vertical);
            }
            PointerEvent::ScrollFinger(pointer_scroll_finger_event) => {
                self.mouse_wheel_delta.0 +=
                    pointer_scroll_finger_event.scroll_value(LibInputPointerAxis::Horizontal);
                self.mouse_wheel_delta.1 +=
                    pointer_scroll_finger_event.scroll_value(LibInputPointerAxis::Vertical);
            }
            PointerEvent::ScrollContinuous(pointer_scroll_continuous_event) => {
                self.mouse_wheel_delta.0 +=
                    pointer_scroll_continuous_event.scroll_value(LibInputPointerAxis::Horizontal);
                self.mouse_wheel_delta.1 +=
                    pointer_scroll_continuous_event.scroll_value(LibInputPointerAxis::Vertical);
            }
            _ => {}
        }
    }

}
