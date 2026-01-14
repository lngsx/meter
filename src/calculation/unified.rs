use itertools::Itertools;
use std::collections::HashMap;

use crate::cli::Provider;
use crate::config::pricing_table::{PRICING, PricingTable};
use crate::error::Error;
use crate::io::unified_dtos::{UnifiedBucketByTime, UnifiedUsageEntry, UnifiedUsageEntryCollapsed};
use crate::prelude::*;

use super::usage_report::UsageReport;

// 1. [Primitives]: Group by provider. (The Base)
// 2. [Tokens Group]: Primitives -> Collapse Tokens -> HashMap
// 3. [Tokens Sum]:   Tokens Group -> Fold            -> u64
// 4. [Cost Group]:   Primitives -> Collapse Cost   -> HashMap
// 5. [Cost Sum]:     Cost Group   -> Fold            -> u64
fn _example(buckets: Vec<UnifiedBucketByTime>) -> AppResult<()> {
    let primitive = make_primitives(buckets)?;

    let tokens_group = collapse_tokens(primitive.clone());
    let tokens_sum = fold(collapse_tokens(primitive.clone()));

    let cost_group = collapse_cost(primitive.clone());
    let cost_sum = fold(collapse_cost(primitive.clone()));

    let _cost_sum_ready = UsageReport::from(cost_sum);
    let _cost_group_ready = UsageReport::from(cost_group);
    let _tokens_sum_ready = UsageReport::from(tokens_sum);
    let _tokens_group_ready = UsageReport::from(tokens_group);

    todo!()
}

/// Looks up pricing by matching model names from the report api.
/// It reads the full model name that gets reported (e.g., "claude-sonnet-4-5-datexyz"),
/// and matches it to our simpler one ("claude-sonnet-4-5").
/// Panics on missing entries to force me to add them to the table.
fn find_price(result_entry: &UnifiedUsageEntry) -> AppResult<&PricingTable> {
    let context_window = &result_entry.context_window;

    // Find the pricing data from the lookup table.
    let pricing_data = PRICING.iter().find(|table_entry| {
        result_entry.model.as_ref().is_some_and(|full_model_name| {
            // This will match "claude-sonnet-4-5" from the full name "claude-sonnet-4-5-datexyz"
            full_model_name.starts_with(table_entry.base_model_name)
        })
    });

    let pricing_entry = pricing_data.ok_or_else(|| {
        let reported_model_name = result_entry.model.as_deref().unwrap_or("Unknown");
        let reported_context_window = context_window.as_deref().unwrap_or("Unknown");

        Error::PricingNotFound {
            model: reported_model_name.to_owned(),
            context_window: reported_context_window.to_owned(),
        }
    })?;

    Ok(pricing_entry)
}

/// It's a model name + usage entry.
/// Example: ('the-model-name-4-5', content)
type BaseModelUsageEntryPair = (String, UnifiedUsageEntry);

/// Extracts base model names as keys for each usage result.
///
/// Transforms results into (key, value) tuples compatible with itertools `into_grouping_map()`,
/// where keys are base model names from the pricing table.
///
/// This also helps validate that each reporting model name exists in the pricing table.
/// Returns Err immediately if any model is not found.
fn try_into_base_model_pairs(
    results: Vec<UnifiedUsageEntry>,
) -> AppResult<Vec<BaseModelUsageEntryPair>> {
    results
        .into_iter()
        .map(|entry| {
            let pricing = find_price(&entry)?;
            let key = pricing.base_model_name.to_owned();
            Ok((key, entry))
        })
        .collect()
}

/// Collapses time-bucketed usage data into a nested map indexed by Provider and Model.
///
/// This aggregates token counts (uncached, cached, and output) across all time buckets,
/// grouping the final results first by the cloud provider and then by the specific model name.
///
/// Terminology:
/// - collapse -> Fold columns, from the right, horizontally.
/// - fold -> fold rows, from bottom, vertically.
pub fn make_primitives(
    buckets: Vec<UnifiedBucketByTime>,
) -> AppResult<HashMap<Provider, HashMap<String, UnifiedUsageEntryCollapsed>>> {
    let usage_by_provider = collapse_by_providers(buckets).into_iter().try_fold(
        HashMap::new(),
        |mut providers_map, (provider, entries)| -> AppResult<_> {
            let usage_entries_by_model = try_into_base_model_pairs(entries)?;

            let collapsed_usage_by_model =
                usage_entries_by_model.into_iter().into_grouping_map().fold(
                    UnifiedUsageEntryCollapsed::default(),
                    |mut collapsed, _model_name, entry| {
                        collapsed.uncached_input_tokens += entry.uncached_input_tokens;
                        collapsed.cache_read_input_tokens += entry.cache_read_input_tokens;
                        collapsed.output_tokens += entry.output_tokens;
                        collapsed.model = entry.model.unwrap_or("Unknown".to_owned());

                        collapsed
                    },
                );

            providers_map.insert(provider, collapsed_usage_by_model);

            Ok(providers_map)
        },
    )?;

    Ok(usage_by_provider)
}

/// Collapses usage tokens by provider.
///
/// Takes nested provider and usage entry data and aggregates them
/// into a simple provider-to-total-usage mapping.
///
/// # Arguments
///
/// * `primitive` - A map of providers to their usage entries
///
/// # Returns
///
/// A map of each provider to its total aggregated token usage
pub fn collapse_tokens(
    primitive: HashMap<Provider, HashMap<String, UnifiedUsageEntryCollapsed>>,
) -> HashMap<String, u64> {
    let tokens_by_provider_then_model = primitive.into_iter().fold(
        HashMap::new(),
        |mut providers_map, (provider, usage_entry_collapsed_by_model)| {
            let tokens_count_by_model = usage_entry_collapsed_by_model.into_iter().fold(
                HashMap::new(),
                |mut models_map, (base_model_name, entry)| -> HashMap<String, u64> {
                    let tokens_counts = entry.uncached_input_tokens
                        + entry.cache_read_input_tokens
                        + entry.output_tokens;

                    models_map.insert(base_model_name, tokens_counts);

                    models_map
                },
            );

            providers_map.insert(provider, tokens_count_by_model);

            providers_map
        },
    );

    tokens_by_provider_then_model
        .into_values()
        .flatten()
        .into_grouping_map()
        .sum()
}

/// Sums the pre-calculated products of each row into a single value.
pub fn fold<T>(some_map: HashMap<String, T>) -> T
where
    T: std::iter::Sum,
{
    some_map.into_values().sum()
}

/// Collapses nested usage data into a flat map of model names and their calculated costs.
///
/// Costs are computed using the global `PRICING` table. If the same model appears across
/// multiple providers, their costs are summed together.
pub fn collapse_cost(
    primitive: HashMap<Provider, HashMap<String, UnifiedUsageEntryCollapsed>>,
) -> HashMap<String, f64> {
    let costs_iter = primitive
        .into_iter()
        .flat_map(|(_provider, usage_by_model)| {
            usage_by_model.into_iter().map(|(base_model_name, entry)| {
                // Safety: Since `keyed_results` has validated the price existence in the table,
                // we can safely unwrap the pricing entry.
                let pricing = PRICING
                    .iter()
                    .find(|table_entry| table_entry.base_model_name == base_model_name)
                    .unwrap();

                let total_input_tokens =
                    entry.uncached_input_tokens + entry.cache_read_input_tokens;
                let total_output_tokens = entry.output_tokens;

                let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
                let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

                (base_model_name, input_cost + output_cost)
            })
        });

    // This ensures field uniqueness, if there are duplicates, sum their values.
    // In case the same model name comes from multiple providers.
    // Example: { model-a: 2, model-a: 3 } -> { model-a: 5 }
    costs_iter.into_grouping_map().sum()
}

/// Vector of usage entry, groupped by provider.
fn collapse_by_providers(
    buckets: Vec<UnifiedBucketByTime>,
) -> HashMap<Provider, Vec<UnifiedUsageEntry>> {
    buckets
        .into_iter()
        .map(
            |UnifiedBucketByTime {
                 provider, results, ..
             }| { (provider, results) },
        )
        .into_group_map()
        .into_iter()
        .map(|(key, value)| -> (Provider, Vec<UnifiedUsageEntry>) {
            let joined_vectors = value.into_iter().flatten().collect();

            (key, joined_vectors)
        })
        .collect()
}

fn calculate_cost(tokens: u64, price_per_million: f64) -> f64 {
    let tokens_in_millions = tokens as f64 / 1_000_000.0;

    tokens_in_millions * price_per_million
}

// impl Sum for UnifiedUsageEntry {
//     fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
//         // Start with Default (all zeros/Nones) and fold (accumulate) the results
//         iter.fold(Self::default(), |acc, x| {
//             Self {
//                 // Sum the numeric fields
//                 uncached_input_tokens: acc.uncached_input_tokens + x.uncached_input_tokens,
//                 cache_read_input_tokens: acc.cache_read_input_tokens + x.cache_read_input_tokens,
//                 output_tokens: acc.output_tokens + x.output_tokens,
//
//                 // Logic for Option<String>:
//                 // If 'acc' has a model, keep it. Otherwise, take 'x's model.
//                 // This preserves the grouping key if it exists.
//                 model: acc.model.or(x.model),
//                 context_window: acc.context_window.or(x.context_window),
//             }
//         })
//     }
// }
