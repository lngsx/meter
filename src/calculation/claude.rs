use itertools::Itertools;
use miette::{Context, IntoDiagnostic};
use serde::Serialize;
use std::collections::HashMap;

use crate::config::pricing_table::{PRICING, PricingTable};
use crate::error::Error;
use crate::io::claude_client::{UsageDataBucket, UsageResult};

pub fn calculate_total_cost(usages: Vec<UsageDataBucket>) -> miette::Result<f64> {
    flatten_usage_buckets(usages)
        .iter()
        .try_fold(0.0, |summed_result, result_entry| {
            let pricing = find_price(result_entry)?;

            // Collect every input tokens.
            // No ephemeral input cache thing because I don't know what it is - -'
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
            let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

            let total_cost = input_cost + output_cost;

            Ok(total_cost + summed_result)
        })
}

pub fn sum_total_tokens(usages: Vec<UsageDataBucket>) -> u64 {
    flatten_usage_buckets(usages)
        .iter()
        .fold(0, |acc, result_entry| {
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            total_input_tokens + total_output_tokens + acc
        })
}

pub fn tokens_by_model_as_csv(usages: Vec<UsageDataBucket>) -> miette::Result<String> {
    let usage_results = flatten_usage_buckets(usages);
    let keyed_results = into_key_pairs(usage_results)?;

    let grouped_tokens = keyed_results
        .into_iter()
        .into_grouping_map()
        .fold(0, |acc, _key, result_entry| {
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            total_input_tokens + total_output_tokens + acc
        });

    Ok(grouped_to_csv(grouped_tokens)?)
}

pub fn costs_by_model_as_csv(
    usages: Vec<UsageDataBucket>,
    unformatted: bool,
) -> miette::Result<String> {
    let usage_results = flatten_usage_buckets(usages);
    let keyed_results = into_key_pairs(usage_results)?;

    let grouped_costs = keyed_results.into_iter().into_grouping_map().fold(
        0.0,
        |summed_result, group_key, result_entry| {
            // Since keyed_results has validated the price existence in the table,
            // we can safely unwrap the pricing entry.
            let pricing = PRICING
                .iter()
                .find(|&table_entry| table_entry.base_model_name == group_key)
                .unwrap();

            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
            let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

            let total_cost = input_cost + output_cost;

            total_cost + summed_result
        },
    );

    let formatted_costs: HashMap<String, String> = grouped_costs
        .into_iter()
        .map(|(name, total)| {
            // Format the total to 2 decimal places (for example, 1.2345 to 1.23)
            let formatted_total = format!("{:.2}", total);

            // When formatting is enabled, include the cost in the name.
            // example: "model-name-123 ($1.23)"
            //
            // This works well for piping to tools like uplot:
            // - The display string is in the left cell.
            // - The numeric value is in the right cell for sorting, for example, | sort --xx |
            //   since dollar-prefixed numbers can't be sorted programmartically.
            let display_string = match unformatted {
                true => name,
                false => format!("{} (${})", name, formatted_total),
            };

            // Left cell, right cell.
            (display_string, formatted_total)
        })
        .collect();

    grouped_to_csv(formatted_costs)
}

// private

fn calculate_cost(tokens: u64, price_per_million: f64) -> f64 {
    // Learning note: it must be converted here because the token is stored as an integer
    // and the multiplier is a float.
    let tokens_in_millions = tokens as f64 / 1_000_000.0;

    tokens_in_millions * price_per_million
}

/// Looks up pricing by matching model names from the report api.
/// It reads the full model name that gets reported (e.g., "claude-sonnet-4-5-datexyz"),
/// and matches it to our simpler one ("claude-sonnet-4-5").
/// Panics on missing entries to force me to add them to the table.
fn find_price(result_entry: &UsageResult) -> miette::Result<&PricingTable> {
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

#[derive(Serialize)]
struct GroupByModel<T> {
    model_summary_row: String,
    content: T,
}

/// Converts a hashmap of grouped data into a CSV-formatted string.
fn grouped_to_csv<T: Serialize>(grouped_hashmap: HashMap<String, T>) -> miette::Result<String> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false) // I don't want a header.
        .from_writer(vec![]);

    for (key, value) in grouped_hashmap {
        let row = GroupByModel {
            model_summary_row: key,
            content: value,
        };

        writer
            .serialize(row)
            .into_diagnostic()
            .wrap_err("Failed to serialize grouped data row to CSV format")?;
    }

    let data = writer
        .into_inner()
        .into_diagnostic()
        .wrap_err("Failed to get writer data.")?;

    let csv_string = String::from_utf8(data)
        .into_diagnostic()
        .wrap_err("Invalid utf-8")?;

    Ok(csv_string)
}

/// Flattens a collection of usage buckets into a single list of results.
fn flatten_usage_buckets(usages: Vec<UsageDataBucket>) -> Vec<UsageResult> {
    usages
        .into_iter()
        .flat_map(|bucket| bucket.results) // pluck it.
        .collect()
}

type GroupedUsage = Vec<(String, UsageResult)>;

/// Extracts base model names as keys for each usage result.
///
/// Transforms results into (key, value) tuples compatible with itertools `into_grouping_map()`,
/// where keys are base model names from the pricing table.
///
/// This also helps validate that each reporting model name exists in the pricing table.
/// Returns Err immediately if any model is not found.
fn into_key_pairs(results: Vec<UsageResult>) -> miette::Result<GroupedUsage> {
    results
        .into_iter()
        .map(|entry| {
            let pricing = find_price(&entry)?;
            let key = pricing.base_model_name.to_owned();

            Ok((key, entry))
        })
        .collect()
}
