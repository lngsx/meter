extern crate dirs;

mod calculation;
mod config;
mod io;
mod types;

use std::error::Error;

use jiff::Zoned;
use spinoff::{Color, Spinner, spinners};

use types::MessagesUsageReport;

fn main() -> Result<(), Box<dyn Error>> {
    // Do this because when it hits the cache, the spinner is not needed, and the spinner api
    // itself doesn't provide a way to create an empty instance, so I have to use this trick.
    // By declaring an empty option beforehand, I can assign the spinner to it as needed.
    let mut spinner_container: Option<Spinner> = None;

    // Use this to make an api call, it has to be aligned with my time.
    let zoned_now = Zoned::now();

    // System time is a naked utc time.
    // So, use this to work with the system, cache retrival.
    let system_now = &zoned_now.in_tz("UTC")?.timestamp();

    // I am going to make this an input argument in the future.
    let ttl_minutes: i64 = 1;

    let cache_dir = dirs::cache_dir()
        .expect("Could not find a cache directory.")
        .join("meter")
        .join("claude");

    // I will improve this later. I have some ideas about it.
    let cache_file_path = &cache_dir.join("cache");

    let output_message: String =
        match io::cache::try_retrieve_cache(cache_file_path, &ttl_minutes, system_now) {
            // Cache hit. The content is ready to use.
            Ok(Some(string)) => string,

            // No cache, expired, or it doesn't exist, so it's okay to refresh.
            Ok(None) => {
                spinner_container = create_spinner();

                let body: MessagesUsageReport = io::claude_client::fetch(&zoned_now)?;

                let summed = calculation::claude::sum(body);

                format(summed)
            }

            // This program must not be run without a cache.
            //
            // It's meant to be used inside a tmux plugin, which may be invoked repeatedly
            // based on its refresh rate (I don't know the exact number, but I am sure it must be
            // very frequent), in multiple instances.
            //
            // The result of the command has to be memoized, and when that very command is asked
            // again, we return the result immediately from the filesystem.
            // That was the initial design. I really have no idea what it would be in the
            // real implementation. Let's hope it works!
            //
            // So, we can't let it silently break inside.
            // That's why I have to make this explicit.
            Err(e) => {
                return Err(e);
            }
        };

    // A simple way to check the output validity, for now.
    if !output_message.is_empty() {
        io::cache::try_write_cache(cache_file_path, &output_message, &ttl_minutes, system_now)?;
    }

    // Print the result.
    match spinner_container.as_mut() {
        Some(s) => s.stop_with_message(&output_message),
        None => println!("{}", output_message),
    }

    Ok(())
}

// private

fn format(calculated_number: f64) -> String {
    format!("${:.2?}", calculated_number)
}

fn create_spinner() -> Option<Spinner> {
    Some(Spinner::new(spinners::Dots, "Retrieving...", Color::Blue))
}
