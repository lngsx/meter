extern crate dirs;

mod calculation;
mod config;
mod io;

use std::error::Error;

use clap::Parser;
use jiff::Zoned;
use serde::Serialize;
use spinoff::{Color, Spinner, spinners};
use std::hash::Hasher;
use twox_hash::XxHash64;

use io::claude_client::MessagesUsageReport;

#[derive(Parser, Serialize, Debug)]
#[command(name = "tad", version)]
struct Args {
    // Skip animations
    #[arg(long, default_value_t = false)]
    no_animate: bool,

    /// Output raw JSON/Text
    #[arg(long, short, default_value_t = false)]
    raw: bool,

    /// No format.
    #[arg(long, short, default_value_t = false)]
    no_format: bool,

    /// Time to live in minutes for the session/cache.
    #[arg(long, default_value_t = 1)]
    ttl_minutes: i64,

    // Credentials
    /// Anthropic admin api key.
    /// Defaults to ANTHROPIC_ADMIN_API_KEY env var.
    #[arg(
        long,
        env = "ANTHROPIC_ADMIN_API_KEY",
        hide_env_values = true,
        required = true
    )]
    anthropic_admin_api_key: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let args_signature = create_args_signature(&args);

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
    let ttl_minutes: i64 = args.ttl_minutes;

    let cache_dir = dirs::cache_dir()
        .expect("Could not find a cache directory.")
        .join("meter")
        .join("claude");

    // I will improve this later. I have some ideas about it.
    let cache_file_name = format!("cache_{}", args_signature);
    let cache_file_path = &cache_dir.join(cache_file_name);

    let output_message: String =
        match io::cache::try_retrieve_cache(cache_file_path, &ttl_minutes, system_now) {
            // Cache hit. The content is ready to use.
            Ok(Some(string)) => string,

            // No cache, expired, or it doesn't exist, so it's okay to refresh.
            Ok(None) => {
                if !args.no_animate {
                    spinner_container = create_spinner();
                }

                let body: MessagesUsageReport =
                    io::claude_client::fetch(&zoned_now, &args.anthropic_admin_api_key)?;

                let summed = calculation::claude::sum(body);

                format(summed, args.no_format)
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

fn format(calculated_number: f64, no_format: bool) -> String {
    if no_format {
        return calculated_number.to_string();
    }

    format!("${:.2?}", calculated_number)
}

fn create_spinner() -> Option<Spinner> {
    Some(Spinner::new(spinners::Dots, "Retrieving...", Color::Blue))
}

fn generate_cache_filename(serialized_args: &str) -> String {
    let mut hasher = XxHash64::default();

    hasher.write(serialized_args.as_bytes());
    let hashed = hasher.finish();

    // Returns something like "7a2f4c91b0e3"
    format!("{:x}", hashed)
}

fn create_args_signature(args: &Args) -> String {
    let serialized = serde_json::to_string(args).unwrap();

    generate_cache_filename(&serialized)
}
