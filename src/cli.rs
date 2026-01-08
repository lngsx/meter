use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::error::Error;

impl Cli {
    /// Convenience constructor to avoid redundant `Parser` imports in main.
    pub fn new() -> Self {
        Cli::parse()
    }

    // A poor man's solution for a credentials store.
    // I will later come back to it to improve if I add more providers.
    // Just platforming it now so I can understand the big picture easily in the future.
    pub fn try_get_anthropic_key(&self) -> miette::Result<&String> {
        let key = self
            .anthropic_admin_api_key
            .as_ref()
            .ok_or(Error::AnthropicKeyNotFound)?;

        Ok(key)
    }

    /// An another poor man's solution to the compact date range string parser.
    /// Return a number of day user put in.
    /// Only support day unit for now.
    pub fn try_parse_since(&self) -> miette::Result<u64> {
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
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    //
    // Global args start here..
    //

    //
    /// Skip animations
    #[arg(long, default_value_t = false, global = true)]
    #[serde(skip)] // This is cosmetic.
    pub no_animate: bool,

    /// No format.
    #[arg(long, default_value_t = false, global = true)]
    pub unformatted: bool,

    /// Time to live in minutes for the session/cache.
    /// Thinking about renaming it to debouncing window or something.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i64).range(0..), global = true)]
    #[serde(skip)] // ttl is just for querying the cache, so keep it away from the cache key.
    pub ttl_minutes: i64,

    /// Only support day for now, for example '2d'.
    #[arg(long, default_value = "0d", global = true)]
    pub since: String,

    #[arg(
        long,
        env = "ANTHROPIC_ADMIN_API_KEY",
        hide_env_values = true,
        global = true
    )]
    pub anthropic_admin_api_key: Option<String>, // This is the way to make it optional.

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
    pub provider: Vec<Provider>,
}

#[derive(Subcommand, Debug, Serialize)]
pub enum Commands {
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
pub struct SumArgs {
    /// What to measure.
    #[arg(long, default_value = "cost")]
    pub metric: Metric,

    /// Optional. How to group results.
    #[arg(long)]
    pub group_by: Option<Grouping>,
}

#[derive(Serialize, ValueEnum, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Metric {
    #[default]
    Cost,
    Tokens,
}

#[derive(Serialize, ValueEnum, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Grouping {
    #[default]
    Model,
    // Provider, // No, for now.
}

#[derive(Clone, Debug, Serialize, ValueEnum, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Anthropic,
}
