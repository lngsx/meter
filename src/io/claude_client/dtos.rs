#![allow(dead_code)] // To silence the compiler warnings.

use serde::{Deserialize, Serialize};

// These were all llm generated from this:
// https://platform.claude.com/docs/en/api/admin/usage_report/retrieve_messages
// I was too lazy to do it by hand. 🙂
//
// Note that the type of tokens has to be integer and be later converted during calculation
// for all the good stuff; for example, it suggests intentionality, discreteness, and countability.

/// Response for the Get Messages Usage Report endpoint.
///
/// API Reference: https://docs.anthropic.com/en/api/admin/usage-report/retrieve-messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagesUsageReport {
    /// A list of usage data buckets.
    pub data: Vec<UsageDataBucket>,

    /// Indicates if there are more results available.
    pub has_more: bool,

    /// Token to provide in as page in the subsequent request to retrieve the next page of data.
    pub next_page: String,
}

/// Represents a specific time bucket of usage data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageDataBucket {
    /// Start of the time bucket (inclusive) in RFC 3339 format.
    pub starting_at: String,

    /// End of the time bucket (exclusive) in RFC 3339 format.
    pub ending_at: String,

    /// List of usage items for this time bucket.
    pub results: Vec<UsageResult>,
}

/// Represents a single usage aggregation result.
///
/// Fields corresponding to grouping parameters (like `model`, `api_key_id`, etc.)
/// will be `None` if that specific grouping was not requested.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UsageResult {
    /// The number of uncached input tokens processed.
    pub uncached_input_tokens: u64,

    /// The number of input tokens read from the cache.
    pub cache_read_input_tokens: u64,

    /// Breakdown of tokens used for cache creation.
    pub cache_creation: CacheCreationUsage,

    /// The number of output tokens generated.
    pub output_tokens: u64,

    /// ID of the API key used. Null if not grouping by API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,

    /// Model name used. Null if not grouping by model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// ID of the Workspace used. Null if not grouping by workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

    /// Service tier used (e.g., "standard", "batch"). Null if not grouping by service tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Context window size used (e.g., "0-200k", "200k-1M"). Null if not grouping by context window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<String>,
}

/// Detailed breakdown of cache creation tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CacheCreationUsage {
    /// The number of input tokens used to create a 1-hour cache entry.
    pub ephemeral_1h_input_tokens: u64,

    /// The number of input tokens used to create a 5-minute cache entry.
    pub ephemeral_5m_input_tokens: u64,
}
