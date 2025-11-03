mod context;
mod performer;
mod screen;
mod state;
mod utils;

use winit::event_loop::{self, ControlFlow};

fn main() {
    // Initialize the logger.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "error");
    }

    env_logger::init();

    // Easier debugging.
    std::env::set_var("RUST_BACKTRACE", "1");

    // Create the event loop.
    let event_loop = event_loop::EventLoop::<utils::SomethingInFd>::with_user_event()
        .build()
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create a proxy for the event loop, so that we can check the pty endpoint and send events to the event loop from a separate thread.
    let event_loop_proxy = event_loop.create_proxy();

    // Setup pty.
    // let _default_shell = std::env::var("SHELL").expect("Could not find default shell from $SHELL.");

    let default_shell = String::from("/usr/bin/bash"); // TODO: Remove this after implementing ANSI escape sequences properly (so we can use for example zsh with all its fancy features).

    println!("{default_shell}");

    let stdout_fd = utils::spawn_pty_with_shell(default_shell);

    utils::monitor_fd(stdout_fd.try_clone().unwrap(), event_loop_proxy);

    // Get the config.
    let state_config = utils::StateConfig::new();

    let mut state = state::State::new(&stdout_fd, &state_config);

    let _ = event_loop.run_app(&mut state);
}
