mod app;
mod calculation;
mod cli;
mod config;
mod display;
mod error;
mod io;
mod prelude;
mod router;

use jiff::{Span, Zoned};
use std::hash::Hasher;
use twox_hash::XxHash64;

use cli::{Cli, Provider};
use io::claude_client::BucketByTime;
use prelude::*;

use self::io::unified_dtos::UnifiedBucketByTime;

// use crate::calculation::unified::UsageReport;

fn main() -> AppResult<()> {
    let cli = Cli::new();
    let providers = cli.load_providers()?;
    let app = app::App::new(cli);
    let args_signature = create_args_signature(&app.cli)?;
    let cache_file_path = create_cache_file_path(&args_signature)?;

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
                app.display.maybe_start_spin();

                let days_ago = app.cli.try_parse_since()? as i64;
                let report_start = calculate_start_date(&zoned_now, days_ago)?;

                // Keep the higher-level logic symmetric.
                // This makes it easy to swap the closure body for thread spawning later.
                // If I can live with the messiness inside this function, I will go with it.
                let joined_results = providers
                    .iter()
                    .map(|(provider, _)| try_fetch_unified(&app, &report_start, provider))
                    .collect::<AppResult<Vec<Vec<_>>>>()?
                    .into_iter()
                    .flatten()
                    .collect();

                let report = router::does_the_thing(&app, joined_results)?;

                report.render(app.cli.unformatted, None)?
            }
        };

    // A simple way to check the output validity, for now.
    if !output_message.is_empty() {
        io::cache::try_write_cache(&cache_file_path, &output_message, &ttl_minutes, system_now)?;
    }

    app.display.stop_spin_with_message(&output_message);

    Ok(())
}

// private
fn try_fetch_unified(
    ctx: &app::App,
    report_start: &Zoned,
    provider: &Provider,
) -> AppResult<Vec<UnifiedBucketByTime>> {
    match *provider {
        Provider::Anthropic => {
            let anthropic_usages: Vec<BucketByTime> =
                crate::io::claude_client::fetch(ctx, report_start, None)?;
            let unified_usages =
                crate::calculation::transformation::unify_from_anthropic(anthropic_usages)?;

            Ok(unified_usages)
        }

        _ => unimplemented!("Do it already!"),
    }
}

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
fn create_args_signature(cli: &Cli) -> AppResult<String> {
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
fn calculate_start_date(zoned_now: &Zoned, days_ago: i64) -> AppResult<Zoned> {
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
fn create_cache_file_path(args_signature: &str) -> AppResult<std::path::PathBuf> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| miette!("Could not find a cache directory."))?
        .join("meter")
        .join("claude");

    let file_name = format!("cache_{}", args_signature);
    let file_path = dir.join(file_name);

    Ok(file_path)
}
