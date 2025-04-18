#![allow(dead_code)]
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;

use nix::errno::Errno;
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
                        let _ = Command::new(&default_shell)
                            .exec();
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
        let mut buffer = vec![0; 4096];
        loop {
            match read(raw_fd, &mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    println!("{}", String::from_utf8(buffer[..n].to_vec()).unwrap());

                    match proxy.send_event(SomethingInFd {
                        buffer: buffer[..n].to_vec(),
                        number_of_elements_in_buffer: n,
                    }) {
                        Ok(_) => (),
                        Err(e) => println!("There has been an error while sending the event: {}", e),
                    }
                }
                Err(e) => {
                    match e {
                        Errno::EIO => std::process::exit(0), // TODO: Implement graceful quiting here.
                        anything_else => println!("There has been an error with the following error code: {}", anything_else)
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));   // Polling rate. TODO: Change this to async or a similar approach instead of polling.
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

pub fn move_cursor_right(cursor_text: &mut String, number_of_chars: usize) {
    // NOTE: Here, we used to add an example character with 0 opacity as "space", because using an actual space character can cause problems
    // in line breaks, which leads to the cursor falling behind at each new line :).

   // TODO: Fix the space vs 0 opacity character problem with the new approach.

    cursor_text.pop();

    for _ in 0..number_of_chars {
        cursor_text.push(' ');
    }

    cursor_text.push_str("█");
}

pub fn move_cursor_left(cursor_text: &mut String, number_of_chars: usize) {
    let truncation_idx = cursor_text.char_indices().rev().nth(number_of_chars).map(|(i, _)| i).unwrap_or(0);

    cursor_text.truncate(truncation_idx);
    cursor_text.push_str("█");
}