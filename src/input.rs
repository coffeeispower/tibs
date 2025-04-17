use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

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

    fn is_mouse_button_pressed(&self, button: MouseButton) -> bool;
    fn is_mouse_button_released(&self, button: MouseButton) -> bool;
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;
    fn is_mouse_button_up(&self, button: MouseButton) -> bool;

    /// Returns the current mouse position as (x, y) coordinates.
    fn mouse_position(&self) -> (f32, f32);

    /// Gets how much the mouse wheel was scrolled in this frame.
    fn mouse_wheel(&self) -> (f32, f32);

    fn get_pressed_keys(&self) -> HashSet<xkbcommon::xkb::Keysym>;
    fn get_released_keys(&self) -> HashSet<xkbcommon::xkb::Keysym>;

    /// Polls all mouse and keyboard events and updates the internal state.
    fn poll_events(&mut self);

    /// Whether the user pressed the X button on the window.
    fn should_close(&self) -> bool {
        false
    }

    /// Returns whether the input context currently has focus.
    fn has_focus(&self) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum KeyStatus {
    Pressed,
    Down,
    Released,
    Up,
}

#[derive(Debug)]
struct KeyInfo {
    state: KeyStatus,
    next_repeat: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct KeyboardState {
    keys: HashMap<xkb::Keysym, KeyInfo>,
    repeat_delay: Duration,
    repeat_interval: Duration,
    repeat_keys: Vec<xkb::Keysym>,
}

impl KeyboardState {
    /// Cria um novo KeyboardState com valores padrão para repeat.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            repeat_delay: Duration::from_millis(250),
            repeat_interval: Duration::from_millis(20),
            repeat_keys: Vec::new(),
        }
    }

    /// Atualiza o estado de todas as teclas e agenda eventos de repeat.
    pub fn new_frame(&mut self) {
        let now = Instant::now();
        self.repeat_keys.clear();

        for (&key, info) in &mut self.keys {
            match info.state {
                KeyStatus::Pressed => {
                    info.state = KeyStatus::Down;
                    info.next_repeat = Some(now + self.repeat_delay);
                }
                KeyStatus::Down => {
                    if let Some(next_time) = info.next_repeat {
                        if now >= next_time {
                            // Agrega evento de repeat
                            self.repeat_keys.push(key);
                            // Agenda próximo repeat
                            info.next_repeat = Some(now + self.repeat_interval);
                        }
                    }
                }
                KeyStatus::Released => {
                    info.state = KeyStatus::Up;
                    info.next_repeat = None;
                }
                KeyStatus::Up => {}
            }
        }

        // Remove teclas em Up
        self.keys.retain(|_, info| info.state != KeyStatus::Up);
    }

    /// Processa eventos de teclado, marcando Pressed ou Released.
    pub fn process_keyboard_event(&mut self, key: xkb::Keysym, key_state: LibInputKeyState) {
        match key_state {
            LibInputKeyState::Pressed => {
                let already = self.keys.get(&key)
                    .map_or(false, |info| matches!(info.state, KeyStatus::Pressed | KeyStatus::Down));
                if !already {
                    self.keys.insert(
                        key,
                        KeyInfo { state: KeyStatus::Pressed, next_repeat: None },
                    );
                }
            }
            LibInputKeyState::Released => {
                if let Some(info) = self.keys.get_mut(&key) {
                    if matches!(info.state, KeyStatus::Pressed | KeyStatus::Down) {
                        info.state = KeyStatus::Released;
                    }
                }
            }
        }
    }

    /// Retorna true se a tecla está pressionada ou mantida.
    pub fn is_key_down(&self, key: xkb::Keysym) -> bool {
        self.keys.get(&key)
            .map_or(false, |info| matches!(info.state, KeyStatus::Pressed | KeyStatus::Down))
    }

    /// Retorna true se a tecla foi pressionada neste frame.
    pub fn was_key_pressed(&self, key: xkb::Keysym) -> bool {
        self.keys.get(&key)
            .map_or(false, |info| info.state == KeyStatus::Pressed) || self.should_repeat_key(key)
    }

    /// Retorna true se a tecla foi libertada neste frame.
    pub fn was_key_released(&self, key: xkb::Keysym) -> bool {
        self.keys.get(&key)
            .map_or(false, |info| info.state == KeyStatus::Released)
    }

    /// Retorna true se a tecla gerou um evento de repeat neste frame.
    pub fn should_repeat_key(&self, key: xkb::Keysym) -> bool {
        self.repeat_keys.contains(&key)
    }

    /// Retorna todas as teclas que foram pressionadas neste frame.
    pub fn get_pressed_keys(&self) -> HashSet<xkb::Keysym> {
        self.keys.iter()
            .filter_map(|(&k, info)| if info.state == KeyStatus::Pressed || self.should_repeat_key(k) { Some(k) } else { None })
            .collect()
    }

    /// Retorna todas as teclas que foram libertadas neste frame.
    pub fn get_released_keys(&self) -> HashSet<xkb::Keysym> {
        self.keys.iter()
            .filter_map(|(&k, info)| if info.state == KeyStatus::Released { Some(k) } else { None })
            .collect()
    }
}
