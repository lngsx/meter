mod claude_client;
mod model;

use std::error::Error;

use spinoff::{Color, Spinner, spinners};

use crate::model::usage_report::MessagesUsageReport;
use claude_client::fetch;

/// Cost multiplier in dollars per million tokens.
#[derive(Debug)]
struct CostMultiplier {
    input_cost: f64,
    output_cost: f64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut spinner = Spinner::new(spinners::Dots, "Retrieving...", Color::Blue);

    let body: MessagesUsageReport = fetch()?;

    let summed = body.data
        .iter()
        .flat_map(|bucket| &bucket.results) // pluck it.
        .fold(0.0, |summed_result, result| {
            let context_window = &result.context_window;

            // Trim it off so that it can use the lookup table easily.
            // From claude-sonnet-4-5-datexyz to claude-sonnet-4-5
            let trimmed_model_name = &result.model
                .as_ref()
                .and_then(|full_model_name| {
                    ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-sonnet-4"]
                        .iter()
                        .find(|&&base_model| full_model_name.starts_with(base_model))
                        .copied()
                });

            let multiplier: CostMultiplier = match(trimmed_model_name.as_deref(), context_window.as_deref()) {
                (Some("claude-haiku-4-5"), Some("0-200k")) => { 
                    CostMultiplier {
                        input_cost: 1.0,  // $1 per million input tokens
                        output_cost: 5.0, // $5 per million output tokens
                    }
                },

                (Some("claude-sonnet-4-5"), Some("0-200k")) => { 
                    CostMultiplier {
                        input_cost: 3.0,   // $3 per million input tokens
                        output_cost: 15.0, // $15 per million output tokens
                    }
                },

                (Some("claude-sonnet-4-5"), Some("200k-1M")) => { 
                    CostMultiplier {
                        input_cost: 6.0,   // $6 per million input tokens
                        output_cost: 22.5, // $22.5 per million output tokens
                    }
                },

                (Some("claude-sonnet-4"), Some("0-200k")) => { 
                    CostMultiplier {
                        input_cost: 3.0,   // $3 per million input tokens
                        output_cost: 15.0, // $15 per million output tokens
                    }
                },

                (Some("claude-sonnet-4"), Some("200k-1M")) => { 
                    CostMultiplier {
                        input_cost: 6.0,   // $6 per million input tokens
                        output_cost: 22.5, // $22.5 per million output tokens
                    }
                },

                _ => panic!("🙏 Sorry! Pricing configuration is missing: \n{:?}\n{:?}.\nPlease inform the author to update the pricing table.", trimmed_model_name, context_window),
            };

            // Dev thing.
            // println!("{:?}", multiplier);

            // Collect every input tokens.
            // No ephemeral input cache thing as I don't know what it is - -'
            let total_input_tokens = result.uncached_input_tokens + result.cache_read_input_tokens;

            let total_output_tokens = result.output_tokens;

            // Calculate costs (convert tokens to millions).
            // Then apply multiplier to each.
            // let input_cost = (total_input_tokens as f64 / 1_000_000.0) * multiplier.input_cost;
            // let output_cost = (total_output_tokens as f64 / 1_000_000.0) * multiplier.output_cost;

            let input_cost = calculate_cost(total_input_tokens, multiplier.input_cost);
            let output_cost = calculate_cost(total_output_tokens, multiplier.output_cost);

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

// Learning note: it must be converted here.
fn calculate_cost(tokens: u64, price_per_million: f64) -> f64 {
    let tokens_in_millions = tokens as f64 / 1_000_000.0;

    tokens_in_millions * price_per_million
}
