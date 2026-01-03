extern crate dirs;

mod calculation;
mod config;
mod error;
mod io;

use clap::{Parser, Subcommand, ValueEnum};
use jiff::{Span, Zoned};
use miette::{IntoDiagnostic, WrapErr, miette};
use serde::Serialize;
use spinoff::{Color, Spinner, spinners};
use std::hash::Hasher;
use std::io::IsTerminal;
use twox_hash::XxHash64;

use error::Error;
use io::claude_client::UsageDataBucket;

pub struct App {
    cli: Cli,
    spinner_container: SpinnerContainer,
}

impl App {
    fn new() -> Self {
        App {
            cli: Cli::parse(),
            spinner_container: SpinnerContainer::new(),
        }
    }

    /// Automatically detect both environment and user input to start a spinner, accordingly.
    // Try to stick to the original implementation for now.
    fn maybe_start_spin(&mut self) {
        let no_animate = self.cli.no_animate;
        let new_spinner_container = self
            .spinner_container
            .create_spinner_unless_no_terminal_or(no_animate);

        self.spinner_container = new_spinner_container;
    }

    fn stop_spin_with_message(&mut self, message: &str) {
        self.spinner_container.stop_with_message(message);
    }

    fn update_spin_message(&mut self, message: String) {
        self.spinner_container.update_text(message);
    }
}

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
    fn create_spinner_unless_no_terminal_or(&mut self, no_animate: bool) -> Self {
        if no_animate || !std::io::stdout().is_terminal() {
            return SpinnerContainer { instance: None };
        }

        SpinnerContainer {
            instance: Some(Spinner::new(spinners::Dots, "Retrieving", Color::Blue)),
        }
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

fn main() -> miette::Result<()> {
    let mut app = App::new();
    // let cli = Cli::parse();
    let args_signature = create_args_signature(&app.cli)?;
    let cache_file_path = create_cache_file_path(&args_signature)?;

    // let mut spinner_container = SpinnerContainer::new();

    // Use this to make an api call, it has to be aligned with my time.
    let zoned_now = Zoned::now();

    // System time is a naked utc time.
    // So, we have to convert it back to the utc.
    let system_now = &zoned_now.in_tz("UTC").into_diagnostic()?.timestamp();

    let ttl_minutes: i64 = app.cli.ttl_minutes;

    let output_message: String =
        match io::cache::try_retrieve_cache(&cache_file_path, &ttl_minutes, system_now) {
            // Cache hit. The content is ready to use.
            Ok(Some(cached_string)) => cached_string,

            // Cache failed to load somehow.
            //
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
                return Err(e).wrap_err(
                    "Cache failed to load. Aborting to avoid the API ban (you're welcome).",
                );
            }

            // No cache, expired, or doesn't exist, so it's okay to refresh.
            // The actual application logic happens here.
            Ok(None) => {
                // app.spinner_container = app
                //     .spinner_container
                //     .create_spinner_unless_no_terminal_or(app.cli.no_animate);

                app.maybe_start_spin();

                let days_ago = app.cli.try_parse_since()? as i64;
                let report_start = calculate_start_date(&zoned_now, days_ago)?;

                // Everyone uses the same usages.
                let usages: Vec<UsageDataBucket> = io::claude_client::fetch(
                    &mut app,
                    // app.cli.try_get_anthropic_key()?,
                    &report_start,
                    None,
                    // &mut app.spinner_container,
                )?;

                match &app.cli.command {
                    // meter raw.
                    Commands::Raw => {
                        if app.cli.unformatted {
                            serde_json::to_string(&usages).into_diagnostic()?
                        } else {
                            serde_json::to_string_pretty(&usages).into_diagnostic()?
                        }
                    }

                    // meter sum.
                    Commands::Sum(args) => match args {
                        SumArgs {
                            metric: Metric::Cost,
                            group_by: None,
                            ..
                        } => {
                            let summed = calculation::claude::calculate_total_cost(usages)?;

                            format(summed, app.cli.unformatted)
                        }

                        SumArgs {
                            metric: Metric::Cost,
                            group_by: Some(Grouping::Model),
                        } => {
                            calculation::claude::costs_by_model_as_csv(usages, app.cli.unformatted)?
                        }

                        SumArgs {
                            metric: Metric::Tokens,
                            group_by: Some(Grouping::Model),
                        } => calculation::claude::tokens_by_model_as_csv(usages)?,

                        SumArgs {
                            metric: Metric::Tokens,
                            group_by: None,
                        } => calculation::claude::sum_total_tokens(usages).to_string(),
                    },
                }
            }
        };

    // A simple way to check the output validity, for now.
    if !output_message.is_empty() {
        io::cache::try_write_cache(&cache_file_path, &output_message, &ttl_minutes, system_now)?;
    }

    app.stop_spin_with_message(&output_message);

    Ok(())
}

// private

/// Formats a number as currency or plain text.
///
/// Formats it as USD with 2 decimal places (e.g., "$123.45").
fn format(calculated_number: f64, no_format: bool) -> String {
    if no_format {
        return calculated_number.to_string();
    }

    format!("${:.2}", calculated_number)
}

/// Hashes serialized arguments into a cache filename.
///
/// Uses XxHash64 to produce a fast, deterministic hash of the input,
/// then formats it as hexadecimal.
///
/// Returns a short string like "7a2f4c91b0e3".
fn generate_cache_filename(serialized_args: &str) -> String {
    let mut hasher = XxHash64::default();

    hasher.write(serialized_args.as_bytes());
    let hashed = hasher.finish();

    // Returns something like "7a2f4c91b0e3"
    format!("{:x}", hashed)
}

/// Generates a cache key from CLI arguments.
///
/// Serializes the provided CLI args to JSON and produces a cache filename
/// that uniquely identifies the command.
fn create_args_signature(cli: &Cli) -> miette::Result<String> {
    let serialized = serde_json::to_string(cli)
        .into_diagnostic()
        .wrap_err("Failed to serialize command arguments; debounce failed, operation rejected.")?;

    let file_name = generate_cache_filename(&serialized);

    Ok(file_name)
}

/// Calculates the start of a day N days ago.
///
/// Subtracts `days_ago` from the given time, then returns midnight
/// of that resulting date in the same timezone.
fn calculate_start_date(zoned_now: &Zoned, days_ago: i64) -> miette::Result<Zoned> {
    let time_span = Span::new().days(days_ago);

    let target_date = zoned_now.checked_sub(time_span).into_diagnostic()?;

    let target_start_of_day = target_date
        .start_of_day()
        .into_diagnostic()
        .wrap_err("Could not resolve the start of the day (midnight) for this date/timezone")?;

    Ok(target_start_of_day)
}

/// Compose the platform-specific cache file path for a given argument signature.
///
/// The resulting path follows the pattern:
/// `{cache_dir}/meter/claude/cache_7a2f4c91b0e3`
fn create_cache_file_path(args_signature: &str) -> miette::Result<std::path::PathBuf> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| miette!("Could not find a cache directory."))?
        .join("meter")
        .join("claude");

    let file_name = format!("cache_{}", args_signature);
    let file_path = dir.join(file_name);

    Ok(file_path)
}

impl Cli {
    // A poor man's solution for a credentials store.
    // I will later come back to it to improve if I add more providers.
    // Just platforming it now so I can understand the big picture easily in the future.
    fn try_get_anthropic_key(&self) -> miette::Result<&String> {
        let key = self
            .anthropic_admin_api_key
            .as_ref()
            .ok_or(Error::AnthropicKeyNotFound)?;

        Ok(key)
    }

    /// An another poor man's solution to the compact date range string parser.
    /// Return a number of day user put in.
    /// Only support day unit for now.
    fn try_parse_since(&self) -> miette::Result<u64> {
        let since = &self.since;

        let Some(digits) = since.strip_suffix('d') else {
            let error = Error::UnsupportedTimeUnit(since.to_owned());

            return Err(error.into());
        };

        let numbers = digits
            .parse::<u64>()
            .map_err(|_| Error::InvalidDuration(digits.to_owned()))?;

        Ok(numbers)
    }
}

// Structs

#[derive(Parser, Serialize, Debug)]
#[command(name = "meter", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    //
    // Global args start here..
    //

    //
    /// Skip animations
    #[arg(long, default_value_t = false, global = true)]
    #[serde(skip)] // This is cosmetic.
    no_animate: bool,

    /// No format.
    #[arg(long, default_value_t = false, global = true)]
    unformatted: bool,

    /// Time to live in minutes for the session/cache.
    /// Thinking about renaming it to debouncing window or something.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i64).range(0..), global = true)]
    #[serde(skip)] // ttl is just for querying the cache, so keep it away from the cache key.
    ttl_minutes: i64,

    /// Only support day for now, for example '2d'.
    #[arg(long, default_value = "0d", global = true)]
    since: String,

    #[arg(
        long,
        env = "ANTHROPIC_ADMIN_API_KEY",
        hide_env_values = true,
        global = true
    )]
    anthropic_admin_api_key: Option<String>, // This is the way to make it optional.

    // #[serde(skip)]
    // Decided to include this key in the command signature itself to ensure integrity
    // if the user has multiple keys on the same machine.
    /// Provider to use. Currently only supports 'anthropic'.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "anthropic",
        global = true
    )]
    provider: Vec<Provider>,
}

#[derive(Subcommand, Debug, Serialize)]
enum Commands {
    /// Calculate aggregated usage (cost, tokens).
    Sum(SumArgs),

    /// Retrieve the raw, unaggregated usage data as JSON.
    ///
    /// This outputs the full history of usage buckets. Useful for piping into
    /// tools like `jq` or for building custom analysis scripts.
    ///
    /// Go build something fun on top of this!
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

#[derive(Clone, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Provider {
    Anthropic,
}
