// use clap::Parser;

use crate::cli::Cli;
use crate::display::Display;

pub struct App {
    pub cli: Cli,
    pub display: Display,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        let no_animate_flag = cli.no_animate.to_owned();

        App {
            cli: cli,
            display: Display::new(no_animate_flag),
        }
    }

    /// Automatically detect both environment and user input to start a spinner, accordingly.
    // Try to stick to the original implementation for now.
    pub fn maybe_start_spin(&mut self) {
        let no_animate = self.cli.no_animate;
        let new_spinner_container = self
            .display
            .spinner
            .create_spinner_unless_no_terminal_or(no_animate);

        self.display.spinner = new_spinner_container;
    }

    pub fn stop_spin_with_message(&mut self, message: &str) {
        self.display.spinner.stop_with_message(message);
    }

    pub fn update_spin_message(&mut self, message: String) {
        self.display.spinner.update_text(message);
    }
}
