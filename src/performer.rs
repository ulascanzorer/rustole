use std::{os::fd::OwnedFd, sync::Arc};

use glyph_brush::{ab_glyph::FontRef, OwnedSection};
use glyph_brush::{Layout, Section, Text};
use vte::{Params, Perform};
use wgpu_text::TextBrush;
use winit::window::Window;

use crate::screen::Screen;
use crate::utils;

pub struct Performer<'a> {
    pub window: Option<Arc<Window>>,
    pub font: &'a Vec<u8>,
    pub brush: Option<TextBrush<FontRef<'a>>>,
    pub char_width: f32,
    pub cursor_index: usize,

    pub font_size: f32,
    pub font_color: [f32; 4],
    pub text_offset_from_left: f32,
    pub text_offset_from_top_as_percentage: f32,
    pub cursor_section: Option<OwnedSection>, // Our cursor section (the unicode character "█").
    pub screen: Screen,
    pub pty_fd: &'a OwnedFd, // We will write to this file descriptor, what we write here will be read by the shell on the other side.
}

impl<'a> Perform for Performer<'a> {
    fn print(&mut self, c: char) {
        let screen = &mut self.screen;

        screen.glyphs[screen.row_index][screen.column_index].text[0].text = String::from(c);
        screen.glyphs[screen.row_index][screen.column_index].text[0]
            .extra
            .color = self.font_color;
        screen.column_index += 1;

        utils::move_cursor_right(self);
        self.cursor_index += 1;
    }

    fn execute(&mut self, byte: u8) {
        println!("This is execute: {byte}");
        match byte {
            b'\n' => {
                let screen = &mut self.screen;

                // Go down to the next row.
                screen.row_index += 1;

                self.cursor_index += 1;

                // Move cursor visually to the next line.
                let cursor = self.cursor_section.as_mut().unwrap();
                cursor.screen_position.0 = self.text_offset_from_left;
                cursor.screen_position.1 += self.font_size;
            }
            b'\r' => {
                // Carriage return: move to start of the line.
                self.screen.column_index = 0;
                self.cursor_index = 0;
            }
            0x08 => {
                // Backspace.
                if self.cursor_index > 0 {
                    // Move the cursor.
                    self.cursor_index -= 1;
                    utils::move_cursor_left(self);

                    // Delete the character from the screen.
                    let screen = &mut self.screen;

                    if screen.column_index > 0 {
                        screen.column_index -= 1;
                        screen.glyphs[screen.row_index][screen.column_index].text[0].text =
                            String::from("");
                    }
                }
            }
            _ => {
                // Unhandled control byte. TODO: Improve this.
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        //println!("This is the csi_dispatch: {}", action);
        match action {
            // Change font color.
            'm' => {
                for param in params.iter() {
                    match param {
                        [0] => {
                            self.font_color = [1., 1., 1., 1.]; // Make font color white (this is the reset option).
                        }
                        [1] => {}
                        [30] => {
                            self.font_color = [0., 0., 0., 1.]; // Make font color black.
                        }
                        [31] => {
                            self.font_color = [1., 0., 0., 1.]; // Make font color red.
                        }
                        [32] => {
                            self.font_color = [0., 1., 0., 1.]; // Make font color green.
                        }
                        [33] => {
                            self.font_color = [1., 1., 0., 1.]; // Make font color yellow.
                        }
                        [34] => {
                            self.font_color = [0., 0., 1., 1.]; // Make font color blue.
                        }
                        [35] => {
                            self.font_color = [1., 0., 1., 1.]; // Make font color magenta.
                        }
                        [36] => {
                            self.font_color = [0., 1., 1., 1.]; // Make font color cyan.
                        }
                        [37] => {
                            self.font_color = [1., 1., 1., 1.]; // Make font color white.
                        }
                        [39] => {
                            self.font_color = [1., 1., 1., 1.]; // Make font color white (this is the default option).
                        }
                        _ => (),
                    }
                }
            }
            // Move the cursor right.
            'C' => {}
            // Move the cursor left.
            'D' => {
                // TODO.
                println!("I am at cursor left!");
                let offset = params.iter().flatten().next().copied().unwrap_or(1);
                self.cursor_index = self.cursor_index.saturating_sub(offset as usize);

                for _ in 0..offset {
                    utils::move_cursor_left(self);
                }
            }
            // Delete a single character in the line.
            'K' => {}
            'J' => {
                for param in params.iter() {
                    match param {
                        [0] => todo!(),
                        [1] => todo!(),
                        [2] => {
                            // This means we have to clear the entire screen.
                            let screen = &mut self.screen;
                            for line in &mut screen.glyphs {
                                for glyph in line {
                                    glyph.text[0].text = String::from("");
                                }
                            }

                            self.screen.row_index = 0;
                            self.screen.column_index = 0;

                            // Reset the cursor section position.
                            let cursor_section = &mut self.cursor_section.as_mut().unwrap();
                            cursor_section.screen_position.0 = self.text_offset_from_left;
                            cursor_section.screen_position.1 = self
                                .text_offset_from_top_as_percentage
                                * self.screen.screen_height as f32;
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        println!("This is the last byte of the escape dispatch: {byte}");
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn unhook(&mut self) {}

    fn put(&mut self, _byte: u8) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn terminated(&self) -> bool {
        false
    }
}
