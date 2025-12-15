use crate::types::PricingTable;

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
];
