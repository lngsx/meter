use clap::{Parser, Subcommand, ValueEnum};
use itertools::Itertools;
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

    /// Loads API keys for service providers based on user selection.
    ///
    /// If the user explicitly chose providers, this returns an error if any are missing keys.
    /// If no providers were specified, it returns all providers that have keys available
    /// and ignores those that don't.
    pub fn load_providers(&self) -> miette::Result<Vec<ProviderKeyPair>> {
        match self.user_selected_providers() {
            // Strict mode: user specified providers, error if keys missing.
            Some(validated_user_inputs) => self
                .provider_blueprints()
                .into_iter()
                .filter(|ProviderSpec { provider, .. }| validated_user_inputs.contains(provider))
                .map(
                    |ProviderSpec {
                         provider,
                         key,
                         missing_key_error,
                     }| {
                        match key {
                            Some(key_found) => Ok((provider, key_found)),
                            None => Err(missing_key_error.into()),
                        }
                    },
                )
                .collect::<miette::Result<Vec<ProviderKeyPair>>>(),

            // Auto mode: No providers specified by user.
            // Return all providers that have API keys available.
            // Silently skip providers without keys.
            None => {
                let available_provider = self
                    .provider_blueprints()
                    .into_iter()
                    .filter_map(|ProviderSpec { provider, key, .. }| {
                        key.map(|key_found| (provider, key_found))
                    })
                    .collect();

                Ok(available_provider)
            }
        }
    }

    /// Cleaned-up version of user inputs due to potential duplicates.
    fn user_selected_providers(&self) -> Option<Vec<Provider>> {
        let providers = self.provider.as_ref()?;
        let deduplicated = providers.iter().unique().cloned().collect();

        Some(deduplicated)
    }

    /// Returns a list of supported providers and their associated API keys and errors.
    /// Configure them right here.
    /// I should move this into src/config/ in the future, maybe.
    pub fn provider_blueprints(&self) -> [ProviderSpec; 2] {
        [
            ProviderSpec {
                provider: Provider::Anthropic,
                key: self.anthropic_admin_api_key.clone(),
                missing_key_error: Error::AnthropicKeyNotFound,
            },
            ProviderSpec {
                provider: Provider::Openai,
                key: self.openai_admin_api_key.clone(),
                missing_key_error: Error::OpenaiKeyNotFound,
            },
        ]
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
    pub anthropic_admin_api_key: Option<String>,

    #[arg(
        long,
        env = "OPENAI_ADMIN_API_KEY",
        hide_env_values = true,
        global = true
    )]
    pub openai_admin_api_key: Option<String>,

    // #[serde(skip)]
    // Decided to include this key in the command signature itself to ensure integrity
    // if the user has multiple keys on the same machine.
    /// Provider to use. Currently only supports 'anthropic'.
    #[arg(
        long,
        value_delimiter = ',',
        // default_value = "anthropic", Nope.
        global = true,
    )]
    pub provider: Option<Vec<Provider>>,
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

#[derive(Clone, Debug, Serialize, ValueEnum, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Anthropic,
    Openai,
}

/// This is a blueprint for each provider to run the load_providers function.
/// The function runs this vector and checks if the key exists in the system.
/// If not, it raises the associated error.
/// This struct is not used in the main application logic.
pub struct ProviderSpec {
    pub provider: Provider,
    pub key: Option<String>,
    pub missing_key_error: Error,
}

/// a simple pair representing a validated provider and its required api key.
/// This is used to control the main logic, determining which provider to dispatch fetch to.
pub type ProviderKeyPair = (Provider, String);
