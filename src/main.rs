extern crate dirs;

mod calculation;
mod config;
mod io;

use std::error::Error;

use clap::{Parser, Subcommand, ValueEnum};
use jiff::Zoned;
use serde::Serialize;
use spinoff::{Color, Spinner, spinners};
use std::hash::Hasher;
use std::io::IsTerminal;
use twox_hash::XxHash64;

use io::claude_client::MessagesUsageReport;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let args_signature = create_args_signature(&cli);

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
    let ttl_minutes: i64 = cli.ttl_minutes;

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
                // Improve ergonomics by auto detecting terminals.
                // This prevents the fancy spinner from flooding pipes or breaking tmux status bars.
                // This saves users from having to mandatory, constantly append `--no-animate`.
                if !cli.no_animate && std::io::stdout().is_terminal() {
                    spinner_container = create_spinner();
                }

                match cli.command {
                    // meter raw.
                    Commands::Raw => {
                        io::claude_client::fetch_raw(&zoned_now, &cli.anthropic_admin_api_key)?
                    }

                    // meter sum.
                    Commands::Sum(args) => {
                        // Everyone uses the same body.
                        let body: MessagesUsageReport =
                            io::claude_client::fetch(&zoned_now, &cli.anthropic_admin_api_key)?;

                        match args {
                            SumArgs {
                                metric: Metric::Cost,
                                group_by: None,
                                ..
                            } => {
                                let summed = calculation::claude::calculate_total_cost(body);

                                format(summed, cli.unformatted)
                            }

                            SumArgs {
                                metric: Metric::Cost,
                                group_by: Some(Grouping::Model),
                            } => calculation::claude::costs_by_model_as_csv(body),

                            SumArgs {
                                metric: Metric::Tokens,
                                group_by: Some(Grouping::Model),
                            } => calculation::claude::tokens_by_model_as_csv(body),

                            SumArgs {
                                metric: Metric::Tokens,
                                group_by: None,
                            } => calculation::claude::sum_total_tokens(body).to_string(),
                        }
                    }
                }
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

    format!("${:.2}", calculated_number)
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

fn create_args_signature(cli: &Cli) -> String {
    let serialized = serde_json::to_string(cli)
        .expect("Failed to serialize command arguments; debounce failed, operation rejected");

    generate_cache_filename(&serialized)
}

// Structs

#[derive(Clone, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Provider {
    Anthropic,
}

#[derive(Serialize, ValueEnum, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
enum Metric {
    #[default]
    Cost,
    Tokens,
}

#[derive(Serialize, ValueEnum, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
enum Grouping {
    #[default]
    Model,
    // Provider, // No, for now.
}

#[derive(Parser, Serialize, Debug)]
#[command(name = "tad", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    //
    // Global args start here..
    //

    //
    /// Skip animations
    #[arg(long, default_value_t = false)]
    #[serde(skip)] // This is cosmetic.
    no_animate: bool,

    /// No format.
    #[arg(long, default_value_t = false)]
    unformatted: bool,

    /// Time to live in minutes for the session/cache.
    /// Thinking about renaming it to debouncing window or something.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i64).range(0..))]
    #[serde(skip)] // ttl is just for querying the cache, so keep it away from the cache key.
    ttl_minutes: i64,

    #[arg(
        long,
        env = "ANTHROPIC_ADMIN_API_KEY",
        hide_env_values = true,
        required = true
    )]
    anthropic_admin_api_key: String,
    // #[serde(skip)]
    // Decided to include this key in the command signature itself to ensure integrity
    // if the user has multiple keys on the same machine.
    /// Provider to use. Currently only supports 'anthropic'.
    #[arg(long, value_delimiter = ',', default_value = "anthropic")]
    provider: Vec<Provider>,
}

#[derive(Subcommand, Debug, Serialize)]
enum Commands {
    /// meter sum
    Sum(SumArgs),

    /// meter raw
    Raw,
}

#[derive(clap::Args, Debug, Serialize)]
struct SumArgs {
    /// What to measure.
    #[arg(long, default_value = "cost")]
    metric: Metric,

    /// Optional. How to group results.
    #[arg(long)]
    group_by: Option<Grouping>,
}

