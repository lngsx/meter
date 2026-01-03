use spinoff::{Color, Spinner, spinners};
use std::io::IsTerminal;
use std::sync::Mutex;

// Display

pub struct Display {
    pub spinner: Mutex<SpinnerContainer>,
    no_animate: bool,
}

impl Display {
    pub fn new(no_animate: bool) -> Self {
        Self {
            no_animate,
            spinner: Mutex::new(SpinnerContainer::new()),
        }
    }

    /// Automatically detect both environment and user input to start a spinner, accordingly.
    // Try to stick to the original implementation for now.
    pub fn maybe_start_spin(&self) {
        let no_animate = self.no_animate;

        let mut locked_spinner = self.spinner.lock().unwrap();

        locked_spinner.create_spinner_unless_no_terminal_or(no_animate);
    }

    pub fn stop_spin_with_message(&self, message: &str) {
        self.spinner.lock().unwrap().stop_with_message(message);
    }

    pub fn update_spin_message(&self, message: String) {
        self.spinner.lock().unwrap().update_text(message);
    }
}

// End of Display

pub struct SpinnerContainer {
    instance: Option<Spinner>,
}

impl SpinnerContainer {
    // Do this because when it hits the cache, the spinner is not needed, and the spinner api
    // itself doesn't provide a way to create an empty instance, so I have to use this trick.
    // By declaring an empty option beforehand, I can assign the spinner to it as needed.
    fn new() -> Self {
        SpinnerContainer { instance: None }
    }

    fn stop_with_message(&mut self, message: &str) {
        // Note that it has to take ownership to prevent double stopping.
        match self.instance.take() {
            Some(mut s) => s.stop_with_message(message),
            None => println!("{}", message),
        }
    }

    fn update_text(&mut self, message: String) {
        if let Some(spinner) = self.instance.as_mut() {
            spinner.update_text(message)
        }
    }

    /// Attempts to create a spinner based on user preference and terminal capabilities.
    ///
    /// This improves ergonomics by auto-detecting terminals, which prevents the fancy
    /// spinner from flooding pipes or breaking tmux status bars. This saves users
    /// from having to mandatory, constantly append `--no-animate`.
    //
    // Note: Just wanted to be clear about the dependency, so I encoded it in the name.
    fn create_spinner_unless_no_terminal_or(&mut self, no_animate: bool) {
        if no_animate || !std::io::stdout().is_terminal() {
            self.instance = None;

            return;
        }

        self.instance = Some(Spinner::new(spinners::Dots, "Retrieving", Color::Blue));
    }
}

impl Drop for SpinnerContainer {
    fn drop(&mut self) {
        if let Some(s) = self.instance.as_mut() {
            // I don't know why .clear() doesn't work, and I didn't bother
            // to do it correctly, so we have to live with this.
            s.stop_with_message("");
        }
    }
}
