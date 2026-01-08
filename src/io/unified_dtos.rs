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

/// Represents a single usage aggregation result.
///
/// Fields corresponding to grouping parameters (like `model`, `api_key_id`, etc.)
/// will be `None` if that specific grouping was not requested.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UnifiedUsageEntry {
    /// The number of uncached input tokens processed.
    pub uncached_input_tokens: u64,

    /// The number of input tokens read from the cache.
    pub cache_read_input_tokens: u64,

    // Breakdown of tokens used for cache creation.
    // pub cache_creation: CacheCreationUsage,
    /// The number of output tokens generated.
    pub output_tokens: u64,

    // ID of the API key used. Null if not grouping by API key.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub api_key_id: Option<String>,

    // Model name used. Null if not grouping by model.
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    // ID of the Workspace used. Null if not grouping by workspace.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub workspace_id: Option<String>,

    // Service tier used (e.g., "standard", "batch"). Null if not grouping by service tier.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub service_tier: Option<String>,

    // Context window size used (e.g., "0-200k", "200k-1M"). Null if not grouping by context window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<String>,
}
