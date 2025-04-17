use clay_layout::{
    fixed, grow,
    layout::{Alignment, Padding},
    text::TextConfig,
    Declaration,
};

use crate::{input::Input, skia_clay::create_measure_text_function, TibsClayScope, FONTS};

pub struct Textbox {
    buffer: String,
    cursor: usize,
    focused: bool,
    censored_buffer: String,
    id: String,
    pub hide_input: bool,
}

impl Textbox {
    pub fn new(id: impl Into<String>, hide_input: bool) -> Self {
        Self {
            buffer: String::new(),
            cursor: 0,
            focused: true,
            censored_buffer: String::new(),
            id: id.into(),
            hide_input,
        }
    }
    fn chars_count(s: &str) -> usize {
        s.chars().count()
    }
    fn char_index_to_byte_index(s: &str, char_index: usize) -> usize {
        let chars_count = Self::chars_count(s);
        if chars_count == 0 {
            return 0;
        }
        if char_index >= chars_count {
            return s.len();
        }
        s.char_indices()
            .skip(char_index)
            .next()
            .map(|(i, _)| i)
            .unwrap_or_else(|| Self::char_index_to_byte_index(s, chars_count - 1))
    }
    fn scroll_cursor_into_view<'clay, 'render>(&self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        let measure_text = create_measure_text_function(&FONTS);
        let text_config = Self::text_config();
        let buffer_to_render = if self.hide_input {
            &self.censored_buffer
        } else {
            &self.buffer
        };
        let cursor_byte_index = Self::char_index_to_byte_index(buffer_to_render, self.cursor);
        let Some(textbox_scroll) = c.scroll_container_data(c.id(&self.id)) else {
            return;
        };
        let x_position_cursor =
            measure_text(&buffer_to_render[..cursor_byte_index], &text_config).width;
        let view_width = textbox_scroll.scrollContainerDimensions.width;
        let view_start = -unsafe { *textbox_scroll.scrollPosition }.x;
        let view_end = view_start + view_width;

        if view_start > x_position_cursor {
            unsafe { (*textbox_scroll.scrollPosition).x = -(x_position_cursor);}
        }
        if view_end - 24. < x_position_cursor {
            unsafe { (*textbox_scroll.scrollPosition).x = -(x_position_cursor - view_width + 24.)};
        }
    }
    fn handle_mouse_clicks<'clay, 'render>(
        &mut self,
        input: &dyn Input,
        c: &mut TibsClayScope<'clay, 'render>,
    ) where
        'clay: 'render,
    {
        let id = c.id(&self.id);
        if !input.is_mouse_button_pressed(crate::input::MouseButton::Left) || !c.pointer_over(id) {
            return;
        }
    
        let Some(bounding_box) = c.bounding_box(id) else {
            return;
        };
    
        let Some(textbox_scroll) = c.scroll_container_data(id) else {
            return;
        };
    
        let buffer_to_render = if self.hide_input {
            &self.censored_buffer
        } else {
            &self.buffer
        };
    
        let text_config = Self::text_config();
        let measure_text = create_measure_text_function(&FONTS);
    
        let click_x = input.mouse_position().0;
        let relative_x = click_x - bounding_box.x - unsafe { (*textbox_scroll.scrollPosition).x } - 15.;
    
        let mut best_index = 0;
        let mut closest_distance = f32::MAX;
    
        for (i, _) in buffer_to_render.char_indices() {
            let width = measure_text(&buffer_to_render[..i], &text_config).width;
            let distance = (width - relative_x).abs();
    
            if distance < closest_distance {
                best_index = i;
                closest_distance = distance;
            }
        }
    
        let end_width = measure_text(buffer_to_render, &text_config).width;
        if relative_x > end_width {
            self.cursor = Self::chars_count(buffer_to_render);
        } else {
            self.cursor = buffer_to_render[..best_index].chars().count();
        }
    
        self.scroll_cursor_into_view(c);
    }
    
    fn text_config() -> TextConfig {
        let mut config = TextConfig::new();
        config
            .color((0xFF, 0xFF, 0xFF).into())
            .font_size(14)
            .alignment(clay_layout::text::TextAlignment::Left);
        return config;
    }
    pub fn update<'clay, 'render>(
        &mut self,
        input: &dyn Input,
        c: &mut TibsClayScope<'clay, 'render>,
    ) where
        'clay: 'render,
    {
        if !self.focused {
            return;
        }
        self.handle_mouse_clicks(input, c);
        for key in input.get_pressed_keys() {
            let chars_count = Self::chars_count(&self.buffer);
            match key {
                xkbcommon::xkb::Keysym::BackSpace => {
                    if self.cursor > 0 {
                        let cursor_byte_index =
                            Self::char_index_to_byte_index(&self.buffer, self.cursor - 1);
                        self.cursor -= 1;
                        self.buffer.remove(cursor_byte_index);
                    }
                    self.scroll_cursor_into_view(c);
                }
                xkbcommon::xkb::Keysym::Left => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    self.scroll_cursor_into_view(c);
                }
                xkbcommon::xkb::Keysym::Right => {
                    if self.cursor < chars_count {
                        self.cursor += 1;
                    }
                    self.scroll_cursor_into_view(c);
                }
                xkbcommon::xkb::Keysym::Delete => {
                    let cursor_byte_index =
                        Self::char_index_to_byte_index(&self.buffer, self.cursor);
                    if self.buffer.len() > cursor_byte_index && !self.buffer.is_empty() {
                        self.buffer.remove(cursor_byte_index);
                    }
                    self.scroll_cursor_into_view(c);
                }
                xkbcommon::xkb::Keysym::Home => {
                    self.cursor = 0;
                    self.scroll_cursor_into_view(c);
                }
                xkbcommon::xkb::Keysym::End => {
                    self.cursor = self.buffer.chars().count();
                    self.scroll_cursor_into_view(c);
                }
                key => {
                    if let Some(c) = key.key_char() {
                        let cursor_byte_index =
                            Self::char_index_to_byte_index(&self.buffer, self.cursor);
                        self.buffer.insert(cursor_byte_index, c);
                        self.cursor += 1;
                    }
                    self.scroll_cursor_into_view(c);
                }
            }
        }
        let buffer_chars_count = Self::chars_count(&self.buffer);
        let censored_chars_count = Self::chars_count(&self.censored_buffer);
        if buffer_chars_count != censored_chars_count {
            self.censored_buffer = "•".repeat(buffer_chars_count);
        }
    }

    pub fn render<'clay, 'render>(&'render self, c: &mut TibsClayScope<'clay, 'render>)
    where
        'clay: 'render,
    {
        c.with(
            Declaration::new()
                .layout()
                .width(fixed!(300.0))
                .height(fixed!(50.0))
                .padding(Padding::all(15))
                .child_alignment(Alignment::new(
                    clay_layout::layout::LayoutAlignmentX::Left,
                    clay_layout::layout::LayoutAlignmentY::Center,
                ))
                .end()
                .scroll(true, false)
                .id(c.id(&self.id))
                .background_color((0x0E, 0x1A, 0x26).into())
                .corner_radius()
                .all(10.0)
                .end(),
            |c| {
                let buffer_to_render = if self.hide_input {
                    &self.censored_buffer
                } else {
                    &self.buffer
                };
                let cursor_byte_index =
                    Self::char_index_to_byte_index(buffer_to_render, self.cursor);
                c.text(
                    &buffer_to_render[..cursor_byte_index],
                    Self::text_config().end(),
                );
                if self.focused {
                    c.with(
                        Declaration::new()
                            .layout()
                            .width(fixed!(1.))
                            .height(grow!())
                            .end()
                            .background_color((0xff, 0xff, 0xff).into()),
                        |_| {},
                    );
                }
                c.text(
                    &buffer_to_render[cursor_byte_index..],
                    Self::text_config().end(),
                );
            },
        );
    }

    pub fn set_focused(&mut self, value: bool) {
        self.focused = value;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn text(&self) -> &str {
        &self.buffer
    }
}
