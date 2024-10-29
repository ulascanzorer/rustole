// The bridge definition of our QObject.
#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        // An alias to the QString type.
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        // Define a signal for when we update the output buffer.

        #[qsignal]
        #[cxx_name = "outputBufferUpdated"]
        fn output_buffer_updated(self: Pin<&mut TerminalTextObject>);
    }

    unsafe extern "RustQt" {
        // The QObject definition
        // We tell CXX-Qt that we want a QObject class with the name TerminalTextObject
        // based on the Rust struct MyObjectRust.
        #[qobject]
        #[qml_element]
        #[qproperty(QString, input_buffer)]
        #[qproperty(QString, output_buffer)]
        #[qproperty(i32, input_start_index)]
        type TerminalTextObject = super::TerminalTextObjectRust;
    }

    impl cxx_qt::Threading for TerminalTextObject{}

    unsafe extern "RustQt" {
        // Declare the invokable methods we want to expose on the QObject
        #[qinvokable]
        #[cxx_name = "processCommand"]
        fn process_command(self: Pin<&mut TerminalTextObject>);

        #[qinvokable]
        #[cxx_name = "appendInputBuffer"]
        fn append_input_buffer(self: Pin<&mut TerminalTextObject>, thing_to_append: QString);
    }
}


// Pure Rust part of this qobject.
use core::pin::Pin;
use std::process::Command;
use cxx_qt_lib::QString;
use cxx_qt::{CxxQtType, Threading};

// The Rust struct for the QObject.
#[derive(Default)]
pub struct TerminalTextObjectRust {
    input_buffer: QString,
    output_buffer: QString,
    input_start_index: i32,
}

impl qobject::TerminalTextObject {
    pub fn append_input_buffer(self: Pin<&mut Self>, thing_to_append: QString) {
        let previous_input_buffer = (*self.input_buffer()).clone();
        self.set_input_buffer(previous_input_buffer + thing_to_append);
    }

    pub fn process_command(mut self: Pin<&mut Self>) {
        // TODO: Implement the actual command processing logic.

        let qt_thread = self.qt_thread();

        let qt_input_buffer_len = self.as_mut().input_buffer().len();

        let input_buffer = String::from(self.as_mut().input_buffer());

        let old_input_start_index = *self.as_mut().input_start_index();

        self.as_mut().set_input_start_index(old_input_start_index + (qt_input_buffer_len as i32));

        std::thread::spawn(move || {
            // Command processing logic.

            let command = input_buffer;

            let output = Command::new("sh")
                            .arg("-c")
                            .arg(command)
                            .output()   // TODO: Change this so that we can have long running processes which contantly output stuff even when they are not done.
                            .expect("Failed to execute process.");

            let real_output = String::from_utf8(output.stdout).unwrap();

            qt_thread.queue(move |mut qobject_terminal| {
                // Update the qt object accordingly.

                qobject_terminal.as_mut().set_input_buffer(QString::from(""));
                let qstring_output = QString::from(&real_output);
                let old_input_start_index = *qobject_terminal.as_mut().input_start_index();
                qobject_terminal.as_mut().set_input_start_index(old_input_start_index + (qstring_output.len() as i32));
                qobject_terminal.as_mut().set_output_buffer(qstring_output);
                qobject_terminal.output_buffer_updated();   // Signal to the qml, so that the new output buffer is printed in GUI.
            }).unwrap();
        });
    }
}