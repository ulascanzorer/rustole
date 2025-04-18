use std::{os::fd::OwnedFd, sync::Arc};

use glyph_brush::{ab_glyph::FontRef, OwnedSection};
use vte::{Params, Perform};
use wgpu_text::TextBrush;
use winit::window::Window;

#[path = "utils.rs"]
mod utils;

pub struct Performer<'a> {
    pub window: Option<Arc<Window>>,
    pub font: &'a Vec<u8>,
    pub brush: Option<TextBrush<FontRef<'a>>>,
    pub char_width: f32,
    pub font_size: f32,
    pub font_color: [f32; 4],
    pub text_section: Option<OwnedSection>,    // Our text section.
    pub text_offset_from_left: f32,
    pub text_offset_from_top_as_percentage: f32,
    pub cursor_section: Option<OwnedSection>,    // Our cursor section (the unicode character "█").
    pub pty_fd: &'a OwnedFd,    // We will write to this file descriptor, what we write here will be read by the shell on the other side.
}

impl<'a> Perform for Performer<'a> {
    fn print(&mut self, c: char) {
        let text = &mut self.text_section.as_mut().unwrap().text[0].text;

        text.push(c);

        utils::move_cursor_right(self);
    }

    fn execute(&mut self, _byte: u8) {
        let text = &mut self.text_section.as_mut().unwrap().text[0].text;

        // If it is a line feed character, render a new line.
        if _byte == 10 {
            text.push('\n');

            // Also update the cursor.
            self.cursor_section.as_mut().unwrap().screen_position.0 = self.text_offset_from_left;
            self.cursor_section.as_mut().unwrap().screen_position.1 += self.font_size;
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char,) {
        match action {
            'm' => {
                for param in params.iter() {
                    match param {
                        [0] => {
                            self.font_color = [1., 1., 1., 1.];  // Make font color white (this is the reset option).
                        }
                        [1] => {
                            ();
                        }
                        [30] => {
                            self.font_color = [0., 0., 0., 1.];  // Make font color black.
                        }
                        [31] => {
                            self.font_color = [1., 0., 0., 1.];  // Make font color red.
                        }
                        [32] => {
                            self.font_color = [0., 1., 0., 1.];  // Make font color green.
                        }
                        [33] => {
                            self.font_color = [1., 1., 0., 1.];  // Make font color yellow.
                        }
                        [34] => {
                            self.font_color = [0., 0., 1., 1.];  // Make font color blue.
                        }
                        [35] => {
                            self.font_color = [1., 0., 1., 1.];  // Make font color magenta.
                        }
                        [36] => {
                            self.font_color = [0., 1., 1., 1.];  // Make font color cyan.
                        }
                        [37] => {
                            self.font_color = [1., 1., 1., 1.];  // Make font color white.
                        }
                        [39] => {
                            self.font_color = [1., 1., 1., 1.];  // Make font color white (this is the default option).
                        }
                        _ => ()
                    }
                }
            }
            _ => ()
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        ();
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        ();
    }

    fn unhook(&mut self) {
        ();
    }

    fn put(&mut self, _byte: u8) {
        ();
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        ();
    }

    fn terminated(&self) -> bool {
        false
    }
}