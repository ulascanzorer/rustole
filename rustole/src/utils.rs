use std::os::fd::{AsRawFd, OwnedFd};
use std::process::Command;
use std::thread;
use std::time::Duration;

use glyph_brush::{OwnedSection, OwnedText};
use nix::pty::{forkpty, ForkptyResult};
use winit::event_loop::EventLoopProxy;

use nix::unistd::read;

#[derive(Clone, Debug)]
pub struct SomethingInFd {
    pub buffer: Vec<u8>,
    pub number_of_elements_in_buffer: usize
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
                        Command::new(&default_shell)
                            .spawn()
                            .expect("Failed to spawn.");

                        std::thread::sleep(std::time::Duration::from_millis(2000));
                        std::process::exit(0);
                    },
                }
            }
            Err(e) => panic!("Failed to fork {:?}", e)
        }
    }
}

pub fn monitor_fd(fd: OwnedFd, proxy: EventLoopProxy<SomethingInFd>) {
    thread::spawn(move || {
        loop {
            let mut buffer = vec![0; 4096];
            match read(fd.as_raw_fd(), &mut buffer) {
                Ok(n) => {
                    let _ = proxy.send_event(SomethingInFd {
                        buffer: buffer,
                        number_of_elements_in_buffer: n,
                    });
                }
                Err(_e) => ()
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