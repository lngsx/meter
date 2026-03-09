#![allow(dead_code)] // To silence the compiler warnings.

#[derive(Debug)]
pub struct PricingTable {
    pub base_model_name: &'static str,
    pub context_window: &'static str,
    pub input_multiplier: f64,
    pub cache_write_5m_multiplier: f64,
    pub cache_write_1h_multiplier: f64,
    pub cache_read_multiplier: f64,
    pub output_multiplier: f64,
}

/// Pricing per million tokens (mtok), sourced from...
/// https://platform.claude.com/docs/en/about-claude/pricing
///
/// Cache write multipliers:
/// - 5-minute cache write: 1.25x base input price.
/// - 1-hour cache write:   2.0x base input price.
///
/// Cache read multipliers:
/// - Always: 0.1x base input price.
pub static PRICING: &[PricingTable] = &[
    PricingTable {
        base_model_name: "claude-haiku-4-5",
        context_window: "0-200k",
        input_multiplier: 1.0,
        cache_write_5m_multiplier: 1.25,
        cache_write_1h_multiplier: 2.0,
        cache_read_multiplier: 0.1,
        output_multiplier: 5.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4-5",
        context_window: "0-200k",
        input_multiplier: 3.0,
        cache_write_5m_multiplier: 3.75,
        cache_write_1h_multiplier: 6.0,
        cache_read_multiplier: 0.3,
        output_multiplier: 15.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4-5",
        context_window: "200k-1M",
        input_multiplier: 6.0,
        cache_write_5m_multiplier: 7.5,
        cache_write_1h_multiplier: 12.0,
        cache_read_multiplier: 0.6,
        output_multiplier: 22.5,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4",
        context_window: "0-200k",
        input_multiplier: 3.0,
        cache_write_5m_multiplier: 3.75,
        cache_write_1h_multiplier: 6.0,
        cache_read_multiplier: 0.3,
        output_multiplier: 15.0,
    },
    PricingTable {
        base_model_name: "claude-sonnet-4",
        context_window: "200k-1M",
        input_multiplier: 6.0,
        cache_write_5m_multiplier: 7.5,
        cache_write_1h_multiplier: 12.0,
        cache_read_multiplier: 0.6,
        output_multiplier: 22.5,
    },
    PricingTable {
        base_model_name: "claude-opus-4-5",
        context_window: "0-200k", // Claude doesn't have long context pricing for this model.
        input_multiplier: 5.0,
        cache_write_5m_multiplier: 6.25,
        cache_write_1h_multiplier: 10.0,
        cache_read_multiplier: 0.5,
        output_multiplier: 25.0,
    },
];
