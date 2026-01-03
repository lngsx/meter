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
}
