mod config;
mod io;
mod types;

use std::error::Error;

use spinoff::{Color, Spinner, spinners};

use config::pricing_table::PRICING;
use io::claude_client::fetch;

use types::MessagesUsageReport;

fn main() -> Result<(), Box<dyn Error>> {
    let mut spinner = Spinner::new(spinners::Dots, "Retrieving...", Color::Blue);

    let body: MessagesUsageReport = fetch()?;

    let summed = body
        .data
        .iter()
        .flat_map(|bucket| &bucket.results) // pluck it.
        .fold(0.0, |summed_result, result_entry| {
            let context_window = &result_entry.context_window;

            // Find the pricing data from the lookup table.
            let pricing_data = PRICING.iter().find(|table_entry| {
                result_entry.model.as_ref().is_some_and(|full_model_name| {
                    // This will match "claude-sonnet-4-5" from the full name "claude-sonnet-4-5-datexyz"
                    full_model_name.starts_with(table_entry.base_model_name)
                })
            });

            // I am too lazy to add every models into the table.
            // I just wanted to be explicit here.
            let pricing = pricing_data.unwrap_or_else(|| {
                panic!(
                    "🙏 Sorry! Pricing configuration is missing: \n{:?}\n{:?}.\n\
                    Please inform the author to update the pricing table.",
                    result_entry.model, context_window
                );
            });

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
        });

    let summed_message = format!("${:.2?}", summed);

    // println!("${:.2?}", summed);
    // println!("{}", &summed_message);

    spinner.stop_with_message(&summed_message);

    Ok(())
}

// private

fn calculate_cost(tokens: u64, price_per_million: f64) -> f64 {
    // Learning note: it must be converted here because the token is stored as an integer
    // and the multiplier is a float.
    let tokens_in_millions = tokens as f64 / 1_000_000.0;

    tokens_in_millions * price_per_million
}
