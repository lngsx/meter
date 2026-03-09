use crate::cli::Provider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedBucketByTime {
    /// Inclusive - include this very exact moment, in jiff Zoned timestamp.
    pub start: i64,

    /// Exclusive - not include, in jiff Zoned timestamp.
    pub end: i64,

    /// List of usage items for this time bucket. The real work is inside it.
    pub results: Vec<UnifiedUsageEntry>,

    /// The service provider for the usage data in this bucket.
    pub provider: Provider,
}

/// Breakdown of tokens used for cache creation, split by TTL tier.
///
/// Note: Because we have to resend our accumulated prompts every time for
/// consecutive messages, they let us pay a higher upfront to cache those 
/// accumulated inputs. This ultimately means we pay less for the next message
/// in the conversation chain.
///
/// Note 2: I'm not sure if this format will work with the openai dto.
/// Let's leave it as a future problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CacheCreationUsage {
    /// Tokens written into a 1-hour cache entry (2x base input price).
    pub ephemeral_1h_input_tokens: u64,

    /// Tokens written into a 5-minute cache entry (1.25x base input price).
    pub ephemeral_5m_input_tokens: u64,
}

/// Represents a single usage aggregation result.
///
/// Fields corresponding to grouping parameters (like `model`, `api_key_id`, etc.)
/// will be `None` if that specific grouping was not requested.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UnifiedUsageEntry {
    /// The number of new, uncached input tokens we sent to them.
    pub uncached_input_tokens: u64,

    /// Breakdown of tokens used for cache creation, split by TTL tier.
    pub cache_creation: CacheCreationUsage,

    /// The number of input tokens retrieved and reused from the cache.
    /// We have to pay them this.
    pub cache_read_input_tokens: u64,

    /// The number of output tokens generated.
    pub output_tokens: u64,

    // ID of the API key used. Null if not grouping by API key.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub api_key_id: Option<String>,
    /// Model name used. Null if not grouping by model.
    pub model: Option<String>,

    // ID of the Workspace used. Null if not grouping by workspace.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub workspace_id: Option<String>,

    // Service tier used (e.g., "standard", "batch"). Null if not grouping by service tier.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub service_tier: Option<String>,

    /// Context window size used (e.g., "0-200k", "200k-1M"). Null if not grouping by context window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<String>,
}

/// A collapsed version of UnifiedUsageEntry. It aggregates everything inside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UnifiedUsageEntryCollapsed {
    /// The number of uncached input tokens processed.
    pub uncached_input_tokens: u64,

    /// The number of tokens written into the 1-hour cache (2x base input price).
    pub cache_write_1h_input_tokens: u64,

    /// The number of tokens written into the 5-minute cache (1.25x base input price).
    pub cache_write_5m_input_tokens: u64,

    /// The number of input tokens read from the cache.
    pub cache_read_input_tokens: u64,

    /// The number of output tokens generated.
    pub output_tokens: u64,

    /// Model name used.
    pub model: String,

    /// Context window size used (e.g., "0-200k", "200k-1M").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<String>,
}
