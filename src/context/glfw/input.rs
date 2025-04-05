use crate::input::Input;

use super::GlfwContext;
use crate::input::MouseButton;
use glfw::{Action, Key, MouseButton as GlfwMouseButton};
use std::collections::HashSet;
use xkbcommon::xkb::Keysym;
use input::event::keyboard::KeyState as LibInputKeyState;

fn glfw_key_to_keysym(glfw_key: Key) -> Option<Keysym> {
    Some(match glfw_key {
        Key::Space => Keysym::space,
        Key::Apostrophe => Keysym::apostrophe,
        Key::Comma => Keysym::comma,
        Key::Minus => Keysym::minus,
        Key::Period => Keysym::period,
        Key::Slash => Keysym::slash,
        Key::Num0 => Keysym::_0,
        Key::Num1 => Keysym::_1,
        Key::Num2 => Keysym::_2,
        Key::Num3 => Keysym::_3,
        Key::Num4 => Keysym::_4,
        Key::Num5 => Keysym::_5,
        Key::Num6 => Keysym::_6,
        Key::Num7 => Keysym::_7,
        Key::Num8 => Keysym::_8,
        Key::Num9 => Keysym::_9,
        Key::Semicolon => Keysym::semicolon,
        Key::Equal => Keysym::equal,
        Key::A => Keysym::a,
        Key::B => Keysym::b,
        Key::C => Keysym::c,
        Key::D => Keysym::d,
        Key::E => Keysym::e,
        Key::F => Keysym::f,
        Key::G => Keysym::g,
        Key::H => Keysym::h,
        Key::I => Keysym::i,
        Key::J => Keysym::j,
        Key::K => Keysym::k,
        Key::L => Keysym::l,
        Key::M => Keysym::m,
        Key::N => Keysym::n,
        Key::O => Keysym::o,
        Key::P => Keysym::p,
        Key::Q => Keysym::q,
        Key::R => Keysym::r,
        Key::S => Keysym::s,
        Key::T => Keysym::t,
        Key::U => Keysym::u,
        Key::V => Keysym::v,
        Key::W => Keysym::w,
        Key::X => Keysym::x,
        Key::Y => Keysym::y,
        Key::Z => Keysym::z,
        Key::LeftBracket => Keysym::leftmiddlecurlybrace,
        Key::Backslash => Keysym::backslash,
        Key::RightBracket => Keysym::rightmiddlecurlybrace,
        Key::GraveAccent => Keysym::D_grave_accent,
        Key::Escape => Keysym::Escape,
        Key::Enter => Keysym::Return,
        Key::Tab => Keysym::Tab,
        Key::Backspace => Keysym::BackSpace,
        Key::Insert => Keysym::Insert,
        Key::Delete => Keysym::Delete,
        Key::Right => Keysym::Right,
        Key::Left => Keysym::Left,
        Key::Down => Keysym::Down,
        Key::Up => Keysym::Up,
        Key::PageUp => Keysym::Page_Up,
        Key::PageDown => Keysym::Page_Down,
        Key::Home => Keysym::Home,
        Key::End => Keysym::End,
        Key::CapsLock => Keysym::Caps_Lock,
        Key::ScrollLock => Keysym::Scroll_Lock,
        Key::NumLock => Keysym::Num_Lock,
        Key::PrintScreen => Keysym::Sys_Req,
        Key::Pause => Keysym::Pause,
        Key::F1 => Keysym::F1,
        Key::F2 => Keysym::F2,
        Key::F3 => Keysym::F3,
        Key::F4 => Keysym::F4,
        Key::F5 => Keysym::F5,
        Key::F6 => Keysym::F6,
        Key::F7 => Keysym::F7,
        Key::F8 => Keysym::F8,
        Key::F9 => Keysym::F9,
        Key::F10 => Keysym::F10,
        Key::F11 => Keysym::F11,
        Key::F12 => Keysym::F12,
        Key::F13 => Keysym::F13,
        Key::F14 => Keysym::F14,
        Key::F15 => Keysym::F15,
        Key::F16 => Keysym::F16,
        Key::F17 => Keysym::F17,
        Key::F18 => Keysym::F18,
        Key::F19 => Keysym::F19,
        Key::F20 => Keysym::F20,
        Key::F21 => Keysym::F21,
        Key::F22 => Keysym::F22,
        Key::F23 => Keysym::F23,
        Key::F24 => Keysym::F24,
        Key::F25 => Keysym::F25,
        Key::Kp0 => Keysym::KP_0,
        Key::Kp1 => Keysym::KP_1,
        Key::Kp2 => Keysym::KP_2,
        Key::Kp3 => Keysym::KP_3,
        Key::Kp4 => Keysym::KP_4,
        Key::Kp5 => Keysym::KP_5,
        Key::Kp6 => Keysym::KP_6,
        Key::Kp7 => Keysym::KP_7,
        Key::Kp8 => Keysym::KP_8,
        Key::Kp9 => Keysym::KP_9,
        Key::KpDecimal => Keysym::KP_Decimal,
        Key::KpDivide => Keysym::KP_Divide,
        Key::KpMultiply => Keysym::KP_Multiply,
        Key::KpSubtract => Keysym::KP_Subtract,
        Key::KpAdd => Keysym::KP_Add,
        Key::KpEnter => Keysym::KP_Enter,
        Key::KpEqual => Keysym::KP_Equal,
        Key::LeftShift => Keysym::Shift_L,
        Key::LeftControl => Keysym::Control_L,
        Key::LeftAlt => Keysym::Alt_L,
        Key::LeftSuper => Keysym::Super_L,
        Key::RightShift => Keysym::Shift_R,
        Key::RightControl => Keysym::Control_R,
        Key::RightAlt => Keysym::Alt_R,
        Key::RightSuper => Keysym::Super_R,
        Key::Menu => Keysym::Menu,
        _ => return None,
    })
}
fn mouse_button_to_glfw(button: MouseButton) -> Option<GlfwMouseButton> {
    match button {
        MouseButton::Left => Some(GlfwMouseButton::Left),
        MouseButton::Right => Some(GlfwMouseButton::Right),
        MouseButton::Middle => Some(GlfwMouseButton::Middle),
        MouseButton::Other(n) => GlfwMouseButton::from_i32(n as i32),
    }
}
impl Input for GlfwContext {
    fn is_key_pressed(&self, key_code: Keysym) -> bool {
        self.keyboard_state.was_key_pressed(key_code)
    }

    fn is_key_down(&self, key_code: Keysym) -> bool {
        self.keyboard_state.is_key_down(key_code)
    }

    fn is_key_up(&self, key_code: Keysym) -> bool {
        !self.is_key_down(key_code)
    }

    fn is_mouse_button_pressed(&self, button: crate::input::MouseButton) -> bool {
        if let Some(glfw_button) = mouse_button_to_glfw(button) {
            self.window.borrow().get_mouse_button(glfw_button) == Action::Press
        } else {
            false
        }
    }

    fn is_mouse_button_down(&self, button: crate::input::MouseButton) -> bool {
        if let Some(glfw_button) = mouse_button_to_glfw(button) {
            let action = self.window.borrow().get_mouse_button(glfw_button);
            action == Action::Press || action == Action::Repeat
        } else {
            false
        }
    }

    fn is_mouse_button_up(&self, button: crate::input::MouseButton) -> bool {
        !self.is_mouse_button_down(button)
    }

    fn mouse_position(&self) -> (f32, f32) {
        // Query current cursor position from GLFW
        let (x, y) = self.window.borrow().get_cursor_pos();
        (x as f32, y as f32)
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        let (x, y) = self.mouse_wheel_delta;
        (x as f32, y as f32)
    }

    fn get_pressed_keys(&self) -> HashSet<Keysym> {
        self.keyboard_state.get_pressed_keys()
    }

    fn get_released_keys(&self) -> HashSet<Keysym> {
        self.keyboard_state.get_released_keys()
    }

    fn poll_events(&mut self) {
        // Reset per-frame mouse wheel delta and key state changes.
        self.mouse_wheel_delta = (0.0, 0.0);
        self.keyboard_state.new_frame();

        // Poll GLFW for events and process them.
        self.glfw.borrow_mut().poll_events();

        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::WindowEvent::Key(key, _scancode, action, _mods) => {
                    if let Some(keysym) = glfw_key_to_keysym(key) {
                        match action {
                            Action::Press => {
                                self.keyboard_state.process_keyboard_event(keysym, LibInputKeyState::Pressed);
                            }
                            Action::Release => {
                                self.keyboard_state.process_keyboard_event(keysym, LibInputKeyState::Released);
                            }
                            _ => {}
                        }
                    }
                }
                glfw::WindowEvent::Scroll(xoffset, yoffset) => {
                    self.mouse_wheel_delta.0 += xoffset;
                    self.mouse_wheel_delta.1 += yoffset;
                }
                _ => {}
            }
        }
    }
    fn should_close(&self) -> bool {
        self.window.borrow().should_close()
    }
    
    fn is_key_released(&self, key_code: xkbcommon::xkb::Keysym) -> bool {
        self.keyboard_state.was_key_released(key_code)
    }
}
