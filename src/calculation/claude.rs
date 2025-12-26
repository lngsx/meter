use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;

use crate::config::pricing_table::{PRICING, PricingTable};
use crate::io::claude_client::{MessagesUsageReport, UsageResult, UsageDataBucket};

pub fn calculate_total_cost(usages: Vec<UsageDataBucket>) -> f64 {
    usages
        .iter()
        .flat_map(|bucket| &bucket.results) // pluck it.
        .fold(0.0, |summed_result, result_entry| {
            let pricing = find_price(result_entry);

            // Collect every input tokens.
            // No ephemeral input cache thing because I don't know what it is - -'
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
            let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

            let total_cost = input_cost + output_cost;

            total_cost + summed_result
        })
}

pub fn sum_total_tokens(usages: Vec<UsageDataBucket>) -> u64 {
    usages
        .iter()
        .flat_map(|bucket| &bucket.results) // pluck it.
        .fold(0, |acc, result_entry| {
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            total_input_tokens + total_output_tokens + acc
        })
}

pub fn tokens_by_model_as_csv(usages: Vec<UsageDataBucket>) -> String {
    let grouped_tokens = usages
        .into_iter()
        .flat_map(|bucket| bucket.results)
        .into_grouping_map_by(|result_entry| find_price(result_entry).base_model_name.to_owned())
        .fold(0, |acc, _key, result_entry| {
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            total_input_tokens + total_output_tokens + acc
        });

    grouped_to_csv(grouped_tokens)
}

pub fn costs_by_model_as_csv(usages: Vec<UsageDataBucket>, unformatted: bool) -> String {
    let grouped_costs = usages
        .into_iter()
        .flat_map(|bucket| bucket.results)
        .into_grouping_map_by(|result_entry| find_price(result_entry).base_model_name.to_owned())
        .fold(0.0, |summed_result, group_key, result_entry| {
            let pricing = PRICING
                .iter()
                .find(|&table_entry| table_entry.base_model_name == group_key)
                .expect("No model found in the pricing table.");

            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
            let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

            let total_cost = input_cost + output_cost;

            total_cost + summed_result
        });

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
fn find_price(result_entry: &UsageResult) -> &PricingTable {
    let context_window = &result_entry.context_window;

    // Find the pricing data from the lookup table.
    let pricing_data = PRICING.iter().find(|table_entry| {
        result_entry.model.as_ref().is_some_and(|full_model_name| {
            // This will match "claude-sonnet-4-5" from the full name "claude-sonnet-4-5-datexyz"
            full_model_name.starts_with(table_entry.base_model_name)
        })
    });

    // I am too lazy to add every models into the table.
    pricing_data.unwrap_or_else(|| {
        panic!(
            "🙏 Sorry! Pricing configuration is missing: \n{:?}\n{:?}.\n\
            Please inform the author to update the pricing table.",
            result_entry.model, context_window
        );
    })
}

#[derive(Serialize)]
struct GroupByModel<T> {
    model_summary_row: String,
    content: T,
}

/// Converts a hashmap of grouped data into a CSV-formatted string.
fn grouped_to_csv<T: Serialize>(grouped_hashmap: HashMap<String, T>) -> String {
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
            .expect("Something went wrong in the csv serialization, go investigate this.");
    }

    let data = writer.into_inner().expect("Failed to get writer data.");

    String::from_utf8(data).expect("Invalid utf-8")
}
