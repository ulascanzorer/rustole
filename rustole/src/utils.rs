#![allow(dead_code)]
use std::fs;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread;

use nix::errno::Errno;
use nix::pty::{forkpty, ForkptyResult};
use winit::event_loop::EventLoopProxy;

use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::unistd::read;

use serde::Deserialize;

use crate::performer::Performer;

#[derive(Clone, Debug)]
pub struct SomethingInFd {
    pub buffer: Vec<u8>,
    pub number_of_elements_in_buffer: usize,
}

// The Config struct, used to read from a config file.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub font_name: String,
    pub font_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font_name: String::from("fonts/DejaVuSansMono.ttf"),
            font_size: 32.0,
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let config_str = fs::read_to_string(path).unwrap_or_else(|_| String::new());

        toml::from_str(&config_str).unwrap_or_else(|_| Config::default())
    }
}

// The StateConfig struct, uses the Config to store State-specific config data.
#[derive(Debug, Deserialize)]
pub struct StateConfig {
    pub font_size: f32,
    pub font: Vec<u8>,
}

impl StateConfig {
    pub fn new() -> Self {
        // Get the config from ~/.config/rustole/rustole.toml and fill in the relevant variables.

        let config_path = expand_tilde("~/.config/rustole/rustole.toml");
        let config = Config::from_file(Path::new(&config_path));

        let font = fs::read(config.font_name).unwrap();

        StateConfig {
            font_size: config.font_size,
            font,
        }
    }
}

pub fn spawn_pty_with_shell(default_shell: String) -> OwnedFd {
    unsafe {
        match forkpty(None, None) {
            Ok(fork_pty_res) => match fork_pty_res {
                ForkptyResult::Parent { child: _, master } => master,
                ForkptyResult::Child => {
                    let _ = Command::new(&default_shell).exec();
                    panic!("exec() failed!");
                }
            },
            Err(e) => panic!("Failed to fork {:?}", e),
        }
    }
}

pub fn monitor_fd(fd: OwnedFd, proxy: EventLoopProxy<SomethingInFd>) {
    thread::spawn(move || {
        let raw_fd = fd.as_raw_fd();
        let mut buffer = vec![0; 4096];

        let poll_fd = PollFd::new(fd.as_fd(), PollFlags::POLLIN);

        let mut pollfds = [poll_fd];

        loop {
            let _nready = poll(&mut pollfds, PollTimeout::NONE).unwrap(); // Wait for the file descriptor without wasting CPU cycles using the Linux poll syscall.

            match read(raw_fd, &mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    // This is for being able to print escape sequences properly as well.
                    let escaped: String = buffer[..n]
                        .iter()
                        .flat_map(|&b| std::ascii::escape_default(b))
                        .map(|c| c as char)
                        .collect();
                    println!("{escaped}");

                    match proxy.send_event(SomethingInFd {
                        buffer: buffer[..n].to_vec(),
                        number_of_elements_in_buffer: n,
                    }) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("There has been an error while sending the event: {}", e)
                        }
                    }
                }
                Err(e) => {
                    match e {
                        Errno::EIO => std::process::exit(0), // TODO: Implement graceful quiting here.
                        anything_else => println!(
                            "There has been an error with the following error code: {}",
                            anything_else
                        ),
                    }
                }
            }
        }
    });
}

pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~") {
        let mut resulting_path = std::env::var("HOME").unwrap();
        resulting_path.push_str(&path[1..]); // Remove the ~ and join the rest of the path
        resulting_path
    } else {
        String::from(path) // No tilde, return path as-is
    }
}

pub fn move_cursor_right(performer_mut: &mut Performer) {
    let cursor_section = performer_mut.cursor_section.as_mut().unwrap();
    let char_width = performer_mut.char_width;

    cursor_section.screen_position.0 += char_width;
}

pub fn move_cursor_left(performer_mut: &mut Performer) {
    let cursor_section = performer_mut.cursor_section.as_mut().unwrap();
    let char_width = performer_mut.char_width;

    cursor_section.screen_position.0 -= char_width;
}
