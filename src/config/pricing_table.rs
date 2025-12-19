#![allow(dead_code)] // To silence the compiler warnings.

#[derive(Debug)]
pub struct PricingTable {
    pub base_model_name: &'static str,
    pub context_window: &'static str,
    pub input_multiplier: f64,
    pub output_multiplier: f64,
}

pub static PRICING: &[PricingTable] = &[
    PricingTable {
        base_model_name: "claude-haiku-4-5",
        context_window: "0-200k",
        input_multiplier: 1.0,
        output_multiplier: 5.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4-5",
        context_window: "0-200k",
        input_multiplier: 3.0,
        output_multiplier: 15.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4-5",
        context_window: "200k-1M",
        input_multiplier: 6.0,
        output_multiplier: 22.5,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4",
        context_window: "0-200k",
        input_multiplier: 3.0,
        output_multiplier: 15.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4",
        context_window: "200k-1M",
        input_multiplier: 6.0,
        output_multiplier: 22.5,
    },
    PricingTable {
        base_model_name: "claude-opus-4-5",
        context_window: "0-200k", // Claude doesn't have long context pricing for this model.
        input_multiplier: 5.0,
        output_multiplier: 25.0,
    },
];
