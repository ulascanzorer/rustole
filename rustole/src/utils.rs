#![allow(dead_code)]
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;

use glyph_brush::{OwnedSection, OwnedText};
use nix::pty::{forkpty, ForkptyResult};
use winit::event_loop::EventLoopProxy;

use nix::unistd::read;

use serde::Deserialize;


#[derive(Clone, Debug)]
pub struct SomethingInFd {
    pub buffer: Vec<u8>,
    pub number_of_elements_in_buffer: usize
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

impl Config{
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let config_str = fs::read_to_string(path)
            .unwrap_or_else(|_| String::new());

        toml::from_str(&config_str).unwrap_or_else(|_| Config::default())
    }
}

// The StateConfig struct, uses the Config to store State-specific config data.
#[derive(Debug, Deserialize)]
pub struct StateConfig {
    pub font_size: f32,
    pub font: Vec<u8>
}

impl StateConfig {
    pub fn new() -> Self {
        // Get the config from ~/.config/rustole/rustole.toml and fill in the relevant variables.
    
        let config_path = expand_tilde("~/.config/rustole/rustole.toml");
        let config = Config::from_file(Path::new(&config_path));

        let font = fs::read(config.font_name).unwrap();

        StateConfig { font_size: config.font_size, font }
    }
}

pub fn spawn_pty_with_shell(default_shell: String) -> OwnedFd {
    unsafe {
        match forkpty(None, None) {
            Ok(fork_pty_res) => {
                match fork_pty_res {
                    ForkptyResult::Parent { child: _, master } => {
                        master
                    }
                    ForkptyResult::Child => {
                        let _ = Command::new(&default_shell).exec();
                        panic!("exec() failed!");
                    },
                }
            }
            Err(e) => panic!("Failed to fork {:?}", e)
        }
    }
}

pub fn monitor_fd(fd: OwnedFd, proxy: EventLoopProxy<SomethingInFd>) {
    thread::spawn(move || {
        let raw_fd = fd.as_raw_fd();
        loop {
            let mut buffer = vec![0; 4096];
            match read(raw_fd, &mut buffer) {
                Ok(n) => {
                    let _ = proxy.send_event(SomethingInFd {
                        buffer: buffer,
                        number_of_elements_in_buffer: n,
                    });
                }
                Err(_e) => {
                    println!("There has been an error with the following error code: {}", _e);
                }
            }
            thread::sleep(Duration::from_millis(50));   // Polling rate.
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

pub fn move_cursor_right(section_1: &mut Option<OwnedSection>, font_size: &f32, number_of_chars: usize) {
    // Move the cursor forward.

    // NOTE: Here, we add an example character with 0 opacity as "space", because using an actual space character can cause problems
    // in line breaks, which leads to the cursor falling behind at each new line :).

    let section_1 = section_1.as_mut().unwrap();

    if let Some(last) = section_1.text.last_mut() {
        *last = OwnedText::new("0")
                    .with_scale(*font_size)
                    .with_color([0.9, 0.5, 0.5, 0.0]);
    }

    for _ in 0..number_of_chars {
        section_1.text.push(
            OwnedText::new("0")
                .with_scale(*font_size)
                .with_color([0.9, 0.5, 0.5, 0.0])
        );
    }

    if let Some(last) = section_1.text.last_mut() {
        *last = OwnedText::new("█")
                    .with_scale(*font_size)
                    .with_color([0.6, 0.6, 0.5, 0.5])
    }
}

pub fn move_cursor_left(section_1: &mut Option<OwnedSection>, font_size: &f32, number_of_chars: usize) {
    let section_1 = section_1.as_mut().unwrap();

    for _ in 0..number_of_chars {
        if section_1.text.len() > 1 {
            section_1.text.pop();
            if let Some(last) = section_1.text.last_mut() {
                *last = OwnedText::new("█")
                            .with_scale(*font_size)
                            .with_color([0.6, 0.6, 0.5, 0.5]);
            }
        }
    }
}