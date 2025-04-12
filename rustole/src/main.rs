#[path = "ctx.rs"]
mod ctx;

#[path = "utils.rs"]
mod utils;

#[path = "performer.rs"]
mod performer;

use ctx::Ctx;

use glyph_brush::ab_glyph::FontRef;
use utils::StateConfig;
use vte::Parser;

use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::time::{Duration, Instant};
use nix::unistd::write;

use wgpu_text::glyph_brush::{
    BuiltInLineBreaker, Layout, Section, Text
};
use wgpu_text::{BrushBuilder, TextBrush};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event::{KeyEvent, MouseScrollDelta};
use winit::event_loop::{self, ActiveEventLoop, ControlFlow };
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;


// The State struct, which holds the state of the application and acts as the application handler for all the events that can happen to our window that we want to react to.

struct State<'a> {
    performer: Option<performer::Performer<'a>>,
    parser: Parser,
    
    text_string: &'a mut String,
    cursor_string: &'a mut String,

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
    
        
        let performer_mut = self.performer.as_mut().unwrap();

        let font_slice = performer_mut.font.as_slice();

        let brush: Option<TextBrush<FontRef<'a>>> = Some(BrushBuilder::using_font_bytes(font_slice).unwrap().build(
            device,
            config.width,
            config.height,
            config.format,
        ));


        let section_0 = Some(
            Section::default()
                .add_text(
                    Text::new(self.text_string)
                        .with_scale(performer_mut.font_size)
                        .with_color([0.6, 0.6, 0.5, 1.0])
                )
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(
                    Layout::default()
                        .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
                )
                .with_screen_position((performer_mut.text_offset_from_left, config.height as f32 * performer_mut.text_offset_from_top_as_percentage))
                .to_owned(),
        );

        // Push the initial cursor to the cursor_string.

        self.cursor_string.push_str("█");

        let section_1 = Some(
            Section::default()
                .add_text(
                    Text::new(self.cursor_string)
                        .with_scale(performer_mut.font_size)
                        .with_color([0.6, 0.6, 0.5, 0.5]),
                )
                .with_bounds((config.width as f32 * 0.95, config.height as f32))
                .with_layout(
                    Layout::default()
                        .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
                )
                .with_screen_position((performer_mut.text_offset_from_left, config.height as f32 * performer_mut.text_offset_from_top_as_percentage / 2.))
                .to_owned(),
        );

        let window = Some(window);

        performer_mut.brush = brush;
        performer_mut.window = window;
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

                let performer_mut = self.performer.as_mut().unwrap();

                performer_mut.section_0.as_mut().unwrap().bounds = (config.width as f32 * 0.95, config.height as _);
                performer_mut.section_0.as_mut().unwrap().screen_position.1 = config.height as f32 * performer_mut.text_offset_from_top_as_percentage;

                performer_mut.section_1.as_mut().unwrap().bounds = (config.width as f32 * 0.95, config.height as _);
                performer_mut.section_1.as_mut().unwrap().screen_position.1 = config.height as f32 * performer_mut.text_offset_from_top_as_percentage;

                performer_mut.brush.as_mut().unwrap().resize_view(config.width as f32, config.height as f32, queue);
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
                let performer_mut = self.performer.as_mut().unwrap();
                match logical_key {
                    Key::Named(k) => match k {
                        NamedKey::Escape => event_loop.exit(),
                        NamedKey::Delete => {
                            // Remove the written text.
                            performer_mut.section_0.as_mut().unwrap().text[0].text.clear();

                            // Reset the cursor.
                            
                            performer_mut.section_1.as_mut().unwrap().text[0].text.clear();
                            performer_mut.section_1.as_mut().unwrap().text[0].text.push_str("█");
                        }
                        NamedKey::Enter => {
                            let text = &mut performer_mut.section_0.as_mut().unwrap().text[0].text;
                            let cursor_text = &mut performer_mut.section_1.as_mut().unwrap().text[0].text;

                            // NOTE: Define more native terminal commands like "exit" here, if necessary.

                            match text.as_ref() {
                                "exit" => event_loop.exit(),
                                "clear" => {
                                    *text = String::from("");
                                },
                                _ => ()
                            }

                            println!("{text}");

                            // Insert a "\n" (newline) to the text section.

                            text.push_str("\n");

                            // Send the user input part of the text to the shell and show some output. TODO: Make it so that it only sends the user input.

                            match write(performer_mut.pty_fd, text.as_bytes()) {
                                Ok(_n) => (),
                                Err(_e) => (),
                            }

                            // Also apply newline logic to the cursor.

                            if let Some((last_char_idx, _)) = cursor_text.char_indices().rev().nth(0) {
                                cursor_text.insert(last_char_idx, '\n');
                            }
                        }
                        NamedKey::Backspace => {
                            let text = &mut performer_mut.section_0.as_mut().unwrap().text[0].text;
                            let cursor_text = &mut performer_mut.section_1.as_mut().unwrap().text[0].text;

                            if !text.is_empty() {
                                text.pop();
                            }

                            // Move the cursor backward.

                            utils::move_cursor_left(cursor_text,1);
                        }
                        NamedKey::Space => {
                            let text = &mut performer_mut.section_0.as_mut().unwrap().text[0].text;
                            let cursor_text = &mut performer_mut.section_1.as_mut().unwrap().text[0].text;

                            text.push_str(" ");

                            utils::move_cursor_right(cursor_text, 1);
                        }

                        NamedKey::ArrowLeft => {
                            // Move the cursor backward.
                            utils::move_cursor_left(&mut performer_mut.section_1.as_mut().unwrap().text[0].text, 1);
                        }

                        NamedKey::ArrowRight => {
                            // Don't move the cursor further forward, if we are right at the end of the written text.

                            if performer_mut.section_1.as_ref().unwrap().text[0].text.len() > performer_mut.section_0.as_ref().unwrap().text[0].text.len() + 2 {
                                return;
                            }

                            // Move the cursor forward.
                            utils::move_cursor_right(&mut performer_mut.section_1.as_mut().unwrap().text[0].text, 1);
                        }
                        _ => ()
                    },

                    Key::Character(char) => {
                        let c = char.as_str();

                        let text = &mut performer_mut.section_0.as_mut().unwrap().text[0].text;
                        
                        text.push_str(c);

                        /* performer_mut.section_0.as_mut().unwrap().text.insert(
                            performer_mut.section_1.as_ref().unwrap().text.len() - 1,
                            OwnedText::new(c.to_string())
                                .with_scale(performer_mut.font_size)
                                .with_color(performer_mut.font_color),
                        ); */

                        // Move the cursor forward.
                        utils::move_cursor_right(&mut performer_mut.section_1.as_mut().unwrap().text[0].text, 1);
                    },
                    
                    _ => (),
                }}

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
        // This part is only here to show fps, maybe to debug performance issues.

        let performer_mut = self.performer.as_mut().unwrap();

        if self.target_framerate <= self.delta_time.elapsed() {
            performer_mut.window.clone().unwrap().request_redraw();
            self.delta_time = Instant::now();
            self.fps += 1;
            if self.fps_update_time.elapsed().as_millis() > 1000 {
                performer_mut.window.as_mut().unwrap().set_title(&format!(
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

        let buffer_string = String::from_utf8(buffer.clone()).unwrap();

        println!("This is what I have received from the shell: {}", buffer_string);

        self.parser.advance(self.performer.as_mut().unwrap(), &buffer[..number_of_elements_in_buffer]);

        if let Some(window) = self.performer.as_ref().unwrap().window.as_ref() {
            window.request_redraw();
        }
    }
}


impl<'a> State<'a> {
    fn new(fd: &'a OwnedFd, state_config: &'a StateConfig, content_text: &'a mut String, cursor_text: &'a mut String) -> Self {
        let font_color = [0.9, 0.5, 0.5, 1.0];

        // Create the parser.

        let parser = Parser::new();

        // Create the state.

        State {
            performer: Some(performer::Performer {
                window: None,
                font: &state_config.font,
                brush: None,
                font_size: state_config.font_size,
                font_color: font_color,
                section_0: None,
                text_offset_from_left: 20.,
                text_offset_from_top_as_percentage: 0.02,
                section_1: None,
                pty_fd: &fd,
            }),
            parser: parser,

            text_string: content_text,
            cursor_string: cursor_text,

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

fn main() {
    // Initialize the logger.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "error");
    }

    env_logger::init();

    // Create the event loop.
    let event_loop = event_loop::EventLoop::<utils::SomethingInFd>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create a proxy for the event loop, so that we can check the pty endpoint and send events to the event loop from a separate thread.
    let event_loop_proxy = event_loop.create_proxy();

    // Setup pty.
    let default_shell = std::env::var("SHELL")
        .expect("Could not find default shell from $SHELL.");

    let stdout_fd = utils::spawn_pty_with_shell(default_shell);

    utils::monitor_fd(stdout_fd.try_clone().unwrap(), event_loop_proxy);

    // Get the config.

    let state_config = utils::StateConfig::new();

    // Create Strings to store the content text and the cursor text of the State.

    let mut content_text = String::new();
    let mut cursor_text = String::new();

    let mut state = State::new(&stdout_fd, &state_config, &mut content_text, &mut cursor_text);

    let _ = event_loop.run_app(&mut state);
}