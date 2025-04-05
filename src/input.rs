use std::collections::{HashMap, HashSet};

use xkbcommon::xkb;
use input::event::keyboard::KeyState as LibInputKeyState;

/// Supported mouse buttons.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u32), // Supports additional buttons.
}

/// Trait to check user input states from keyboard and mouse.
///
/// Implementors of this trait should define how the input state is determined.
pub trait Input {
    /// Returns `true` if the given key was just pressed (i.e., this frame).
    fn is_key_pressed(&self, key_code: xkbcommon::xkb::Keysym) -> bool;
    
    /// Returns `true` if the given key was just released (i.e., this frame).
    fn is_key_released(&self, key_code: xkbcommon::xkb::Keysym) -> bool;
    
    /// Returns `true` if the given key is currently pressed.
    fn is_key_down(&self, key_code: xkbcommon::xkb::Keysym) -> bool;

    /// Returns `true` if the given key is currently not pressed.
    fn is_key_up(&self, key_code: xkbcommon::xkb::Keysym) -> bool;


    /// Returns `true` if the specified mouse button is currently pressed.
    fn is_mouse_button_pressed(&self, button: MouseButton) -> bool;

    /// Returns `true` if the specified mouse button was just pressed.
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;

    /// Returns `true` if the specified mouse button was just released.
    fn is_mouse_button_up(&self, button: MouseButton) -> bool;

    /// Returns the current mouse position as (x, y) coordinates.
    fn mouse_position(&self) -> (f32, f32);

    // Gets how much the mouse wheel was scrolled in this frame
    fn mouse_wheel(&self) -> (f32, f32);

    fn get_pressed_keys(&self) -> HashSet<xkbcommon::xkb::Keysym>;
    fn get_released_keys(&self) -> HashSet<xkbcommon::xkb::Keysym>;

    // Polls all mouse and keyboard events and updates the internal state
    fn poll_events(&mut self);

    // Whether the user pressed the X button on the window
    fn should_close(&self) -> bool {
        false
    }
}




#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum KeyStatus {
    Pressed,
    Released,
    Down,
    Up,
}

#[derive(Default)]
pub(crate) struct KeyboardState {
    keys: HashMap<xkb::Keysym, KeyStatus>,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self::default()
    }

    // Prepares a new frame by updating transient states.
    // Keys that were Pressed become Down and keys that were Released become Up.
    // Keys in the Up state are removed from the map.
    pub fn new_frame(&mut self) {
        for (_, state) in &mut self.keys {
            *state = match *state {
                KeyStatus::Pressed => KeyStatus::Down,
                KeyStatus::Released => KeyStatus::Up,
                other => other,
            };
        }
    }

    // Process a keyboard event to update the state.
    pub fn process_keyboard_event(&mut self, key: xkb::Keysym, key_state: LibInputKeyState) {
        match key_state {
            LibInputKeyState::Pressed => {
                // Only set to Pressed if the key isn't already down.
                if let Some(existing) = self.keys.get(&key) {
                    if *existing == KeyStatus::Down || *existing == KeyStatus::Pressed {
                        return;
                    }
                }
                self.keys.insert(key, KeyStatus::Pressed);
            }
            LibInputKeyState::Released => {
                // Mark the key as Released if it was previously down.
                if let Some(existing) = self.keys.get(&key) {
                    if *existing == KeyStatus::Down || *existing == KeyStatus::Pressed {
                        self.keys.insert(key, KeyStatus::Released);
                    }
                }
            }
        }
    }

    // Returns true if the key is currently held down.
    pub fn is_key_down(&self, key: xkb::Keysym) -> bool {
        matches!(self.keys.get(&key), Some(KeyStatus::Pressed | KeyStatus::Down))
    }

    // Returns true if the key was pressed during this frame.
    pub fn was_key_pressed(&self, key: xkb::Keysym) -> bool {
        matches!(self.keys.get(&key), Some(KeyStatus::Pressed))
    }

    // Returns true if the key was released during this frame.
    pub fn was_key_released(&self, key: xkb::Keysym) -> bool {
        matches!(self.keys.get(&key), Some(KeyStatus::Released))
    }


    pub fn get_pressed_keys(&self) -> HashSet<xkb::Keysym> {
        self.keys
            .iter()
            .filter(|(_, &state)| state == KeyStatus::Pressed)
            .map(|(&key, _)| key)
            .collect()
    }

    pub fn get_released_keys(&self) -> HashSet<xkb::Keysym> {
        self.keys
            .iter()
            .filter(|(_, &state)| state == KeyStatus::Released)
            .map(|(&key, _)| key)
            .collect()
    }
}