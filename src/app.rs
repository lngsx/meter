use clap::Parser;

use crate::cli::Cli;
use crate::display::SpinnerContainer;

pub struct App {
    pub cli: Cli,
    pub spinner_container: SpinnerContainer,
}

impl App {
    pub fn new() -> Self {
        App {
            cli: Cli::parse(),
            spinner_container: SpinnerContainer::new(),
        }
    }

    /// Automatically detect both environment and user input to start a spinner, accordingly.
    // Try to stick to the original implementation for now.
    pub fn maybe_start_spin(&mut self) {
        let no_animate = self.cli.no_animate;
        let new_spinner_container = self
            .spinner_container
            .create_spinner_unless_no_terminal_or(no_animate);

        self.spinner_container = new_spinner_container;
    }

    pub fn stop_spin_with_message(&mut self, message: &str) {
        self.spinner_container.stop_with_message(message);
    }

    pub fn update_spin_message(&mut self, message: String) {
        self.spinner_container.update_text(message);
    }
}
