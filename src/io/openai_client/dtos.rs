#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// API Reference: https://platform.openai.com/docs/api-reference/usage/completions

/// Response from the OpenAI Usage API (completions), paged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAiUsagePage {
    /// The object type, always "page".
    pub object: String,

    /// A list of usage data buckets.
    pub data: Vec<OpenAiUsageBucket>,

    /// Indicates if there are more results available.
    pub has_more: bool,

    /// Token to provide in the subsequent request to retrieve the next page of data.
    pub next_page: Option<String>,
}

/// The response page body, partitioned by time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAiUsageBucket {
    /// The object type, always "bucket".
    pub object: String,

    /// Unix timestamp (seconds) for the start of the bucket.
    pub start_time: u64,

    /// Unix timestamp (seconds) for the end of the bucket.
    pub end_time: u64,

    /// List of usage items for this time bucket.
    pub results: Vec<OpenAiUsageEntry>,
}

/// Represents a single usage aggregation result.
///
/// Fields corresponding to grouping parameters (like `model`, `api_key_id`, etc.)
/// will be `None` if that specific grouping was not requested.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OpenAiUsageEntry {
    /// The object type, always "organization.usage.completions.result".
    pub object: String,

    /// The aggregated number of text input tokens used, including cached tokens.
    pub input_tokens: u64,

    /// The aggregated number of text output tokens used.
    pub output_tokens: u64,

    /// The aggregated number of text input tokens that were cached.
    #[serde(default)]
    pub input_cached_tokens: u64,

    /// The aggregated number of audio input tokens used.
    #[serde(default)]
    pub input_audio_tokens: u64,

    /// The aggregated number of audio output tokens used.
    #[serde(default)]
    pub output_audio_tokens: u64,

    /// The count of requests made to the model.
    #[serde(default)]
    pub num_model_requests: u64,

    /// ID of the API key used. Null if not grouping by API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,

    /// ID of the User used. Null if not grouping by user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Model name used. Null if not grouping by model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// ID of the Project used. Null if not grouping by project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Service tier used (e.g., "scale", "default"). Null if not grouping by service tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Whether the usage was from the Batch API. Null if not grouping by batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<bool>,
}
