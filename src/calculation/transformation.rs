use miette::IntoDiagnostic;

use crate::cli::Provider;
use crate::io::claude_client::UsageEntry;
use crate::io::claude_client::dtos::BucketByTime;
use crate::io::unified_dtos::{UnifiedBucketByTime, UnifiedUsageEntry};

/// Converts a collection of Anthropic-specific usage buckets into a unified format.
pub fn unify_from_anthropic(
    anthropic_buckets: Vec<BucketByTime>,
) -> miette::Result<Vec<UnifiedBucketByTime>> {
    anthropic_buckets
        .into_iter()
        .map(UnifiedBucketByTime::try_from)
        .collect()
}

// Private

/// Maps a raw Anthropic usage entry to my unified version of it.
impl From<UsageEntry> for UnifiedUsageEntry {
    fn from(entry: UsageEntry) -> Self {
        UnifiedUsageEntry {
            model: entry.model,
            context_window: entry.context_window,
            cache_read_input_tokens: entry.cache_read_input_tokens,
            uncached_input_tokens: entry.uncached_input_tokens,
            output_tokens: entry.output_tokens,
        }
    }
}

/// Try to transform an Anthropic bucket into my unified bucket.
impl TryFrom<BucketByTime> for UnifiedBucketByTime {
    type Error = miette::Report;

    fn try_from(bucket: BucketByTime) -> Result<Self, Self::Error> {
        let provider = Provider::Anthropic;
        let start = bucket
            .starting_at
            .parse::<jiff::Timestamp>()
            .into_diagnostic()?
            .as_second();
        let end = bucket
            .ending_at
            .parse::<jiff::Timestamp>()
            .into_diagnostic()?
            .as_second();
        let results = bucket
            .results
            .into_iter()
            .map(UnifiedUsageEntry::from)
            .collect();

        Ok(Self {
            provider,
            start,
            end,
            results,
        })
    }
}
