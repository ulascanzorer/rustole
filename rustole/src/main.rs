#[path = "ctx.rs"]
mod ctx;

#[path = "utils.rs"]
mod utils;

use ctx::Ctx;

use glyph_brush::ab_glyph::FontRef;
use glyph_brush::OwnedSection;

use unicode_width::UnicodeWidthChar;

use utils::spawn_pty_with_shell;
use vte::{Params, Parser, Perform};

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;

use wgpu_text::glyph_brush::{
    BuiltInLineBreaker, Layout, OwnedText, Section, Text
};
use wgpu_text::{BrushBuilder, TextBrush};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event::{KeyEvent, MouseScrollDelta};
use winit::event_loop::{self, ActiveEventLoop, ControlFlow };
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use serde::Deserialize;

// The Config struct, used to read from a config file and use the values from there at startup.

#[derive(Debug, Deserialize)]
struct Config {
    font_name: String,
    font_size: f32,
}

fn default_font() -> String {
    String::from("fonts/DejaVuSansMono.ttf")
}

fn default_font_size() -> f32 {
    32.
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font_name: default_font(),
            font_size: default_font_size()
        }
    }
}

impl Config{
    fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let config_str = fs::read_to_string(path)
            .unwrap_or_else(|_| String::new());

        println!("Config string: {config_str}");

        toml::from_str(&config_str).unwrap_or_else(|_| Config::default())
    }
}

// The State struct, which holds the state of the application and acts as the application handler for all the events that can happen to our window that we want to react to.

struct State<'a> {
    performer: Option<Performer<'a>>,
    parser: &'a mut Parser,

    target_framerate: Duration,
    delta_time: Instant,
    fps_update_time: Instant,
    fps: i32,

    // wgpu
    ctx: Option<Ctx>
}

impl<'a> ApplicationHandler<utils::SomethingInFd> for State<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_title("wgpu-text simple example")
                ).unwrap()
        );

        self.ctx = Some(Ctx::new(window.clone()));

        let ctx = self.ctx.as_ref().unwrap();
        let device = &ctx.device;
        let config = &ctx.config;

        let brush = Some(BrushBuilder::using_font_bytes(self.performer.as_ref().unwrap().font).unwrap().build(
            device,
            config.width,
            config.height,
            config.format,
        ));

        let section_0 = Some(
            Section::default()
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(
                    Layout::default()
                        .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
                )
                .with_screen_position((self.performer.as_ref().unwrap().text_offset_from_left, config.height as f32 * self.performer.as_ref().unwrap().text_offset_from_top_as_percentage))
                .to_owned(),
        );

        let section_1 = Some(
            Section::default()
                .add_text(
                    Text::new("█")
                        .with_scale(self.performer.as_ref().unwrap().font_size)
                        .with_color([0.6, 0.6, 0.5, 0.5]),
                )
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(
                    Layout::default()
                        .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
                )
                .with_screen_position((self.performer.as_ref().unwrap().text_offset_from_left, config.height as f32 * self.performer.as_ref().unwrap().text_offset_from_top_as_percentage / 2.))
                .to_owned(),
        );

        let window = Some(window);

        let performer_mut = self.performer.as_mut().unwrap();

        performer_mut.window = window;
        performer_mut.brush = brush;
        performer_mut.section_0 = section_0;
        performer_mut.section_1 = section_1;

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

                config.width = new_size.width.max(1);
                config.height = new_size.height.max(1);
                surface.configure(device, config);

                self.performer.as_mut().unwrap().section_0.as_mut().unwrap().bounds = (config.width as f32 * 0.95, config.height as _);
                self.performer.as_mut().unwrap().section_0.as_mut().unwrap().screen_position.1 = config.height as f32 * self.performer.as_ref().unwrap().text_offset_from_top_as_percentage;

                self.performer.as_mut().unwrap().section_1.as_mut().unwrap().bounds = (config.width as f32 * 0.95, config.height as _);
                self.performer.as_mut().unwrap().section_1.as_mut().unwrap().screen_position.1 = config.height as f32 * self.performer.as_ref().unwrap().text_offset_from_top_as_percentage;

                self.performer.as_mut().unwrap().brush.as_mut().unwrap().resize_view(config.width as f32, config.height as f32, queue);
            }

            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key,
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => {  
                let performer = self.performer.as_mut().unwrap();
                match logical_key {
                    Key::Named(k) => match k {
                        NamedKey::Escape => event_loop.exit(),
                        NamedKey::Delete => {
                            // Remove the written text.
                            performer.section_0.as_mut().unwrap().text.clear();

                            // Reset the cursor.
                            performer.section_1.as_mut().unwrap().text.clear();
                            performer.section_1.as_mut().unwrap().text.push(
                                OwnedText::new("█")
                                    .with_scale(performer.font_size)
                                    .with_color([0.6, 0.6, 0.5, 0.5]),
                            );
                        }
                        NamedKey::Enter => {
                            let written_text: String = performer.section_0.as_mut().unwrap().text
                                .iter()
                                .map(|element| element.text.clone())
                                .collect();

                            // NOTE: Define more native terminal commands like "exit" here, if necessary.

                            match written_text.as_ref() {
                                "exit" => event_loop.exit(),
                                "clear" => {
                                    performer.section_0.as_mut().unwrap().text.clear();
                                },
                                _ => ()
                            }

                            println!("{written_text}");

                            // Remove the written text.
                            performer.section_0.as_mut().unwrap().text.clear();

                            // Reset the cursor.
                            performer.section_1.as_mut().unwrap().text.clear();
                            performer.section_1.as_mut().unwrap().text.push(
                                OwnedText::new("█")
                                    .with_scale(performer.font_size)
                                    .with_color([0.6, 0.6, 0.5, 0.5]),
                            );
                        }
                        NamedKey::Backspace => {
                            let section_0 = performer.section_0.as_mut().unwrap();
                            let section_1 = performer.section_1.as_mut().unwrap();

                            if !section_0.text.is_empty() && section_1.text.len() >= 2 {
                                let mut end_text = section_0.text.remove(section_1.text.len() - 2);
                                end_text.text.pop();
                                if !end_text.text.is_empty() {
                                    section_0.text.push(end_text.clone());
                                }
                            }

                            // Move the cursor backward.

                            if section_1.text.len() > 1 {
                                section_1.text.pop();
                                if let Some(last) = section_1.text.last_mut() {
                                    *last = OwnedText::new("█")
                                                .with_scale(performer.font_size)
                                                .with_color([0.6, 0.6, 0.5, 0.5]);
                                }
                            }
                        }
                        NamedKey::Space => {
                            performer.section_0.as_mut().unwrap().text.insert(
                                performer.section_1.as_ref().unwrap().text.len() - 1,
                                OwnedText::new(" ")
                                    .with_scale(performer.font_size)
                            );

                            // Move the cursor forward.

                            // NOTE: Here, we add an example character with 0 opacity as "space", because using an actual space character can cause problems
                            // in line breaks, which leads to the cursor falling behind at each new line :).

                            if let Some(last) = performer.section_1.as_mut().unwrap().text.last_mut() {
                                *last = OwnedText::new("0")
                                            .with_scale(performer.font_size)
                                            .with_color([0.9, 0.5, 0.5, 0.0]);
                            }

                            performer.section_1.as_mut().unwrap().text.push(
                                OwnedText::new("█")
                                    .with_scale(performer.font_size)
                                    .with_color([0.6, 0.6, 0.5, 0.5])
                            );
                        }

                        NamedKey::ArrowLeft => {
                            let section_1 = performer.section_1.as_mut().unwrap();

                            // Move the cursor backward.

                            if section_1.text.len() > 1 {
                                section_1.text.pop();
                                if let Some(last) = section_1.text.last_mut() {
                                    *last = OwnedText::new("█")
                                                .with_scale(performer.font_size)
                                                .with_color([0.6, 0.6, 0.5, 0.5]);
                                }
                            }
                        }

                        NamedKey::ArrowRight => {
                            // Don't move the cursor further forward, if we are right at the end of the written text.

                            if performer.section_1.as_ref().unwrap().text.len() >= performer.section_0.as_ref().unwrap().text.len() + 1 {
                                return;
                            }

                            // Move the cursor forward.

                            if let Some(last) = performer.section_1.as_mut().unwrap().text.last_mut() {
                                *last = OwnedText::new("0")
                                            .with_scale(performer.font_size)
                                            .with_color([0.9, 0.5, 0.5, 0.0]);
                            }

                            performer.section_1.as_mut().unwrap().text.push(
                                OwnedText::new("█")
                                    .with_scale(performer.font_size)
                                    .with_color([0.6, 0.6, 0.5, 0.5])
                            );
                        }
                        _ => ()
                    },

                    Key::Character(char) => {
                        let c = char.as_str();
                        if c != "\u{7f}" && c != "\u{8}" {
                            performer.section_0.as_mut().unwrap().text.insert(
                                performer.section_1.as_ref().unwrap().text.len() - 1,
                                OwnedText::new(c.to_string())
                                    .with_scale(performer.font_size)
                                    .with_color(*performer.font_color),
                            );

                            // Move the cursor forward.

                            if let Some(last) = performer.section_1.as_mut().unwrap().text.last_mut() {
                                *last = OwnedText::new("0")
                                            .with_scale(performer.font_size)
                                            .with_color([0.9, 0.5, 0.5, 0.0]);
                            }

                            performer.section_1.as_mut().unwrap().text.push(
                                OwnedText::new("█")
                                    .with_scale(performer.font_size)
                                    .with_color([0.6, 0.6, 0.5, 0.5])
                            );
                        }
                    },
                    
                    _ => (),
                }}

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                let performer = self.performer.as_mut().unwrap();

                // Increase/decrease font size.
                let mut size = performer.font_size;
                if y > 0.0 {
                    size += (size / 4.0).max(2.0)
                } else {
                    size *= 4.0 / 5.0
                };
                performer.font_size = (size.clamp(3.0, 25000.0) * 2.0).round() / 2.0;
            }

            WindowEvent::RedrawRequested => {
                let performer = self.performer.as_mut().unwrap();

                let brush = performer.brush.as_mut().unwrap();
                let ctx = self.ctx.as_ref().unwrap();
                let queue = &ctx.queue;
                let device = &ctx.device;
                let config = &ctx.config;
                let surface = &ctx.surface;
                let section_0 = performer.section_0.as_ref().unwrap();
                let section_1 = performer.section_1.as_ref().unwrap();

                // NOTE: Section order in the brush queue should be [section_0, section_1], once section_1 is implemented as the cursor, so that it stays on top of the text section.
                match brush.queue(device, queue, [section_0, section_1]) {
                    Ok(_) => (),
                    Err(err) => panic!("{err}")
                }

                // TODO: This part is a little weird, probably because of the linux nvidia 550 driver.
            
                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(device, config);
                        return ();
                    },
                    // {
                    //    surface.configure(device, config);
                    //    surface.get_current_texture().expect("Failed to acquire next surface texture!")
                    //}
                };

                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

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
        let performer = self.performer.as_mut().unwrap();

        if self.target_framerate <= self.delta_time.elapsed() {
            performer.window.clone().unwrap().request_redraw();
            self.delta_time = Instant::now();
            self.fps += 1;
            if self.fps_update_time.elapsed().as_millis() > 1000 {
                performer.window.as_mut().unwrap().set_title(&format!(
                    "wgpu-text: 'simple' example, FPS: {}",
                    self.fps
                ));
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

        self.parser.advance(self.performer.as_mut().unwrap(), &buffer[..number_of_elements_in_buffer]);

        if let Some(window) = self.performer.as_ref().unwrap().window.as_ref() {
            window.request_redraw();
        }
    }
}


struct Performer<'a> {
    window: Option<Arc<Window>>,
    font: &'a [u8],
    brush: Option<TextBrush<FontRef<'a>>>,
    font_size: f32,
    font_color: &'a mut [f32; 4],
    section_0: Option<OwnedSection>,    // Our text section.
    text_offset_from_left: f32,
    text_offset_from_top_as_percentage: f32,
    section_1: Option<OwnedSection>,    // Our cursor section (the unicode character "█").
}

impl<'a> Perform for Performer<'a> {
    fn print(&mut self, c: char) {
        self.section_0.as_mut().unwrap().text.push(
            OwnedText::new(c)
                .with_scale(self.font_size)
                .with_color(*self.font_color)
        );

        let width = UnicodeWidthChar::width(c).unwrap_or(0);

        utils::move_cursor_right(&mut self.section_1, &self.font_size, width);
    }

    fn execute(&mut self, _byte: u8) {
        // unimplemented!();
        ();
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char,) {
        match action {
            'm' => {
                for param in params.iter() {
                    match param {
                        [0] => {
                            *self.font_color = [1., 1., 1., 1.];  // Make font color white (this is the reset option).
                        }
                        [1] => {
                            ();
                        }
                        [30] => {
                            *self.font_color = [0., 0., 0., 1.];  // Make font color black.
                        }
                        [31] => {
                            *self.font_color = [1., 0., 0., 1.];  // Make font color red.
                        }
                        [32] => {
                            *self.font_color = [0., 1., 0., 1.];  // Make font color green.
                        }
                        [33] => {
                            *self.font_color = [1., 1., 0., 1.];  // Make font color yellow.
                        }
                        [34] => {
                            *self.font_color = [0., 0., 1., 1.];  // Make font color blue.
                        }
                        [35] => {
                            *self.font_color = [1., 0., 1., 1.];  // Make font color magenta.
                        }
                        [36] => {
                            *self.font_color = [0., 1., 1., 1.];  // Make font color cyan.
                        }
                        [37] => {
                            *self.font_color = [1., 1., 1., 1.];  // Make font color white.
                        }
                        [39] => {
                            *self.font_color = [1., 1., 1., 1.];  // Make font color white (this is the default option).
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

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "error");
    }

    env_logger::init();

    let event_loop = event_loop::EventLoop::<utils::SomethingInFd>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create a proxy for the event loop, so that we can check the pty endpoint and send events to the event loop from a separate thread.
    let event_loop_proxy = event_loop.create_proxy();

    // Setup pty.

    let default_shell = std::env::var("SHELL")
        .expect("Could not find default shell from $SHELL.");

    let stdout_fd = spawn_pty_with_shell(default_shell);

    utils::monitor_fd(stdout_fd, event_loop_proxy);

    // Get the config from ~/.config/rustole/rustole.toml
    
    let config_path = utils::expand_tilde("~/.config/rustole/rustole.toml");
    let config = Config::from_file(Path::new(&config_path));

    let font_vector = fs::read(config.font_name).unwrap();
    let font = font_vector.as_slice();

    let mut font_color = [0.9, 0.5, 0.5, 1.0];

    // Create the parser.

    let mut parser = Parser::new();

    // Create the state.

    let mut state = State {
        performer: Some(Performer {
            window: None,
            font: font,
            brush: None,
            font_size: config.font_size,
            font_color: &mut font_color,
            section_0: None,
            text_offset_from_left: 20.,
            text_offset_from_top_as_percentage: 0.02,
            section_1: None,
        }),
        parser: &mut parser,

        // FPS and window updating:
        // change '60.0' if you want different FPS cap
        target_framerate: Duration::from_secs_f64(1.0 / 60.0),
        delta_time: Instant::now(),
        fps_update_time: Instant::now(),
        fps: 0,

        ctx: None,
    };

    let _ = event_loop.run_app(&mut state);
}