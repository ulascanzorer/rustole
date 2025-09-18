#[path = "ctx.rs"]
mod ctx;

#[path = "utils.rs"]
pub mod utils;

#[path = "performer.rs"]
mod performer;

#[path = "screen.rs"]
mod screen;

use ctx::Ctx;

use glyph_brush::ab_glyph::{Font, FontRef, ScaleFont};
use glyph_brush::OwnedSection;
use vte::Parser;

use nix::unistd::write;
use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wgpu_text::glyph_brush::{BuiltInLineBreaker, Layout, Section, Text};
use wgpu_text::{BrushBuilder, TextBrush};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event::{KeyEvent, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use crate::state::screen::Screen;

// The State struct, which holds the state of the application and acts as the application handler for
// all the events that can happen to our window that we want to react to.
pub struct State<'a> {
    performer: Option<performer::Performer<'a>>,
    parser: Parser,
    text_string: &'a mut String,

    target_framerate: Duration,
    delta_time: Instant,
    fps_update_time: Instant,
    fps: i32,

    ctx: Option<Ctx>, // wgpu context.
}

impl<'a> ApplicationHandler<utils::SomethingInFd> for State<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Rustole"))
                .unwrap(),
        );

        self.ctx = Some(Ctx::new(window.clone()));

        let ctx = self.ctx.as_ref().unwrap();
        let device = &ctx.device;
        let config = &ctx.config;

        let performer_mut = self.performer.as_mut().unwrap();

        let font_slice = performer_mut.font.as_slice();

        // Save the character width of the given font with the given scale in the performer.
        let font_ref = FontRef::try_from_slice(font_slice).unwrap();
        let scaled_font = font_ref.as_scaled(performer_mut.font_size);
        let char_width = scaled_font.h_advance(font_ref.glyph_id(' '));
        println!("Scaled font pixel width: {}", scaled_font.scale.x);

        performer_mut.char_width = char_width;

        let brush: Option<TextBrush<FontRef<'a>>> =
            Some(BrushBuilder::using_font_bytes(font_slice).unwrap().build(
                device,
                config.width,
                config.height,
                config.format,
            ));

        let text_section = Some(
            Section::default()
                .add_text(
                    Text::new(self.text_string)
                        .with_scale(performer_mut.font_size)
                        .with_color([0.6, 0.6, 0.5, 1.0]),
                )
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker))
                .with_screen_position((
                    performer_mut.text_offset_from_left,
                    performer_mut.text_offset_from_top_as_percentage,
                ))
                .to_owned(),
        );

        // Push the initial cursor to the cursor_string.

        let cursor_section = Some(
            Section::default()
                .add_text(
                    Text::new("█")
                        .with_scale(performer_mut.font_size)
                        .with_color([0.6, 0.6, 0.5, 0.5]),
                )
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker))
                .with_screen_position((
                    performer_mut.text_offset_from_left,
                    performer_mut.text_offset_from_top_as_percentage / 2.,
                ))
                .to_owned(),
        );

        let window = Some(window);

        performer_mut.brush = brush;
        performer_mut.window = window;
        performer_mut.text_section = text_section;
        performer_mut.cursor_section = cursor_section;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                let ctx = self.ctx.as_mut().unwrap();
                let queue = &ctx.queue;
                let device = &ctx.device;
                let config = &mut ctx.config;
                let surface = &ctx.surface;

                config.width = new_size.width.max(1); // This is the real pixel width of the surface.
                config.height = new_size.height.max(1); // This is the real pixel height of the surface.
                println!("Current config width: {}", config.width);
                println!("Current config height: {}", config.height);

                surface.configure(device, config);

                let performer_mut = self.performer.as_mut().unwrap();

                performer_mut.text_section.as_mut().unwrap().bounds =
                    (config.width as f32 * 0.95, config.height as _);
                performer_mut
                    .text_section
                    .as_mut()
                    .unwrap()
                    .screen_position
                    .1 = config.height as f32 * performer_mut.text_offset_from_top_as_percentage;

                performer_mut.cursor_section.as_mut().unwrap().bounds =
                    (config.width as f32 * 0.95, config.height as _);
                performer_mut
                    .cursor_section
                    .as_mut()
                    .unwrap()
                    .screen_position
                    .1 = config.height as f32 * performer_mut.text_offset_from_top_as_percentage;

                performer_mut.brush.as_mut().unwrap().resize_view(
                    config.width as f32,
                    config.height as f32,
                    queue,
                );
            }

            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                let performer_mut = self.performer.as_mut().unwrap();
                match logical_key {
                    Key::Named(k) => match k {
                        NamedKey::Escape => event_loop.exit(),
                        NamedKey::Delete => {
                            // Clear the displayed text.
                            performer_mut.text_section.as_mut().unwrap().text[0]
                                .text
                                .clear();

                            // Reset the cursor.
                            let cursor_section = performer_mut.cursor_section.as_mut().unwrap();
                            let text_section = performer_mut.text_section.as_mut().unwrap();

                            cursor_section.screen_position.0 = performer_mut.text_offset_from_left;
                            cursor_section.screen_position.1 = text_section.screen_position.1;
                            performer_mut.cursor_index = 0;
                        }
                        NamedKey::Enter => {
                            // Send the carriage return character to the master pty.
                            match write(performer_mut.pty_fd, b"\r") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }
                        NamedKey::Backspace => {
                            // Send the backspace character to the master pty.
                            match write(performer_mut.pty_fd, b"\x7f") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }
                        NamedKey::Space => {
                            // Send the space character to the master pty.
                            match write(performer_mut.pty_fd, b" ") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }

                        NamedKey::ArrowLeft => {
                            // Send the arrow left escape sequence to the master pty.
                            match write(performer_mut.pty_fd, b"\x1b[D") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }

                        NamedKey::ArrowRight => {
                            // Send the arrow right escape sequence to the master pty.
                            match write(performer_mut.pty_fd, b"\x1b[C") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }
                        NamedKey::ArrowUp => {
                            // Send the arrow up escape sequence to the master pty.
                            match write(performer_mut.pty_fd, b"\x1b[A") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }
                        NamedKey::ArrowDown => {
                            // Send the arrow down escape sequence to the master pty.
                            match write(performer_mut.pty_fd, b"\x1b[B") {
                                Ok(_) => (),
                                Err(e) => println!(
                                    "There has been an error writing to the master pty: {}",
                                    e
                                ),
                            }
                        }
                        _ => (),
                    },

                    Key::Character(char) => {
                        let c = char.as_str();

                        // Send the input character to the master pty.
                        match write(performer_mut.pty_fd, c.as_bytes()) {
                            Ok(_) => (),
                            Err(e) => {
                                println!("There has been an error writing to the master pty: {}", e)
                            }
                        }
                    }

                    _ => (),
                }
            }

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                let performer_mut = self.performer.as_mut().unwrap();

                // Increase/decrease font size.
                let mut size = performer_mut.font_size;
                if y > 0.0 {
                    size += (size / 4.0).max(2.0)
                } else {
                    size *= 4.0 / 5.0
                };
                performer_mut.font_size = (size.clamp(3.0, 25000.0) * 2.0).round() / 2.0;

                performer_mut.text_section.as_mut().unwrap().text[0].scale =
                    performer_mut.font_size.into();
                performer_mut.cursor_section.as_mut().unwrap().text[0].scale =
                    performer_mut.font_size.into();
            }

            WindowEvent::RedrawRequested => {
                let performer = self.performer.as_mut().unwrap();

                let brush = performer.brush.as_mut().unwrap();
                let ctx = self.ctx.as_ref().unwrap();
                let queue = &ctx.queue;
                let device = &ctx.device;
                let config = &ctx.config;
                let surface = &ctx.surface;
                let text_section = performer.text_section.as_ref().unwrap();
                let cursor_section = performer.cursor_section.as_ref().unwrap();

                // NOTE: Section order in the brush queue should be [text_section, cursor_section], once cursor_section is implemented as the cursor, so that it stays on top of the text section.
                let mut screen_section_refs: Vec<&OwnedSection> = performer.screen.lines.iter().collect();
                //screen_section_refs.push(text_section);
                screen_section_refs.push(cursor_section);
                match brush.queue(device, queue, screen_section_refs) {
                    Ok(_) => (),
                    Err(err) => panic!("{err}"),
                }

                // NOTE: This part is a little weird, probably because of the linux nvidia 550 driver.

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(device, config);
                        return ();
                    } // {
                      //    surface.configure(device, config);
                      //    surface.get_current_texture().expect("Failed to acquire next surface texture!")
                      //}
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.2,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    brush.draw(&mut render_pass);
                }

                queue.submit([encoder.finish()]);
                frame.present();
            }

            _ => (),
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        // This part is only here to show fps, maybe to debug performance issues.

        let performer_mut = self.performer.as_mut().unwrap();

        if self.target_framerate <= self.delta_time.elapsed() {
            performer_mut.window.clone().unwrap().request_redraw();
            self.delta_time = Instant::now();
            self.fps += 1;
            if self.fps_update_time.elapsed().as_millis() > 1000 {
                performer_mut
                    .window
                    .as_mut()
                    .unwrap()
                    .set_title(&format!("wgpu-text: 'simple' example, FPS: {}", self.fps));
                self.fps = 0;
                self.fps_update_time = Instant::now();
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        println!("Exiting!");
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: utils::SomethingInFd) {
        let buffer = event.buffer;
        let number_of_elements_in_buffer = event.number_of_elements_in_buffer;

        self.parser.advance(
            self.performer.as_mut().unwrap(),
            &buffer[..number_of_elements_in_buffer],
        );

        if let Some(window) = self.performer.as_ref().unwrap().window.as_ref() {
            window.request_redraw();
        }
    }
}

impl<'a> State<'a> {
    pub fn new(
        fd: &'a OwnedFd,
        state_config: &'a utils::StateConfig,
        content_text: &'a mut String,
    ) -> Self {
        let font_color = [0.9, 0.5, 0.5, 1.0];

        // Create the parser.
        let parser = Parser::new();

        // Create the state.
        State {
            performer: Some(performer::Performer {
                window: None,
                font: &state_config.font,
                brush: None,
                char_width: 0.0,
                cursor_index: 0,
                font_size: state_config.font_size,
                font_color: font_color,
                text_section: None,
                text_offset_from_left: 20.,
                text_offset_from_top_as_percentage: 0.02,
                cursor_section: None,
                screen: Screen::new(10, 100, state_config.font_size, 1920, 1080, 20., 0.02),
                pty_fd: &fd,
            }),
            parser: parser,

            text_string: content_text,

            // FPS and window updating:
            // change '60.0' if you want different FPS cap
            target_framerate: Duration::from_secs_f64(1.0 / 60.0),
            delta_time: Instant::now(),
            fps_update_time: Instant::now(),
            fps: 0,

            ctx: None,
        }
    }
}
