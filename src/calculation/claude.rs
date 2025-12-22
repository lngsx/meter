use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;

use crate::config::pricing_table::{PRICING, PricingTable};
use crate::io::claude_client::{MessagesUsageReport, UsageResult};

pub fn sum(body: MessagesUsageReport) -> f64 {
    body.data
        .iter()
        .flat_map(|bucket| &bucket.results) // pluck it.
        .fold(0.0, |summed_result, result_entry| {
            let pricing = find_price(result_entry);

            // Dev thing.
            // println!("{:?}", multiplier);

            // Collect every input tokens.
            // No ephemeral input cache thing because I don't know what it is - -'
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            // Calculate costs (convert tokens to millions).
            // Then apply multiplier to each.
            // let input_cost = (total_input_tokens as f64 / 1_000_000.0) * multiplier.input_cost;
            // let output_cost = (total_output_tokens as f64 / 1_000_000.0) * multiplier.output_cost;

            let input_cost = calculate_cost(total_input_tokens, pricing.input_multiplier);
            let output_cost = calculate_cost(total_output_tokens, pricing.output_multiplier);

            let total_cost = input_cost + output_cost;

            // Dev thing.
            // println!("{:?}", total_cost);

            total_cost + summed_result
        })
}

// pub fn group_by_model(body: MessagesUsageReport) -> HashMap<String, u64> {
pub fn group_by_model(body: MessagesUsageReport) -> String {
    let sum = body
        .data
        .into_iter()
        .flat_map(|bucket| bucket.results)
        .into_grouping_map_by(|result_entry| {
            // result_entry.model.clone().expect("Nope")
            // let full_model_name = result_entry.model.clone();

            let pricing_data = PRICING.iter().find(|table_entry| {
                result_entry.model.as_ref().is_some_and(|full_model_name| {
                    // This will match "claude-sonnet-4-5" from the full name "claude-sonnet-4-5-datexyz"
                    full_model_name.starts_with(table_entry.base_model_name)
                })
            });

            pricing_data
                .expect("No model found in the pricing table")
                .base_model_name
                .to_owned()
        })
        .fold(0, |acc, _key, result_entry| {
            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            total_input_tokens + total_output_tokens + acc
        });

    grouped_to_csv(sum)
}

pub fn group_by_model_and_cal(body: MessagesUsageReport) -> String {
    let sum = body
        .data
        .into_iter()
        .flat_map(|bucket| bucket.results)
        .into_grouping_map_by(|result_entry| find_price(result_entry).base_model_name.to_owned())
        .fold(0.0, |summed_result, key, result_entry| {
            let pricing = PRICING.iter().find(|&x| x.base_model_name == key);

            let total_input_tokens =
                result_entry.uncached_input_tokens + result_entry.cache_read_input_tokens;

            let total_output_tokens = result_entry.output_tokens;

            // Calculate costs (convert tokens to millions).
            // Then apply multiplier to each.
            // let input_cost = (total_input_tokens as f64 / 1_000_000.0) * multiplier.input_cost;
            // let output_cost = (total_output_tokens as f64 / 1_000_000.0) * multiplier.output_cost;

            let input_cost =
                calculate_cost(total_input_tokens, pricing.expect("2").input_multiplier);
            let output_cost =
                calculate_cost(total_output_tokens, pricing.expect("3").output_multiplier);

            let total_cost = input_cost + output_cost;

            // Dev thing.
            // println!("{:?}", total_cost);

            total_cost + summed_result
        });

    let formatted_costs: HashMap<String, String> = sum
        .into_iter()
        .map(|(name, total)| (name.to_string(), format!("${:.2}", total)))
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
    string_name: String,
    content: T,
}

/// Converts a hashmap of grouped data into a CSV-formatted string.
fn grouped_to_csv<T: Serialize>(grouped_hashmap: HashMap<String, T>) -> String {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false) // I don't want a header.
        .from_writer(vec![]);

    for (key, value) in grouped_hashmap {
        let row = GroupByModel {
            string_name: key,
            content: value,
        };

        wtr.serialize(row)
            .expect("Something went wrong in the csv serialization, go investigate this.");
    }

    let data = wtr.into_inner().expect("Failed to get writer data.");
    let csv_string = String::from_utf8(data).expect("Invalid utf-8");

    csv_string
}
