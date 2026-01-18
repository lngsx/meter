use crate::prelude::*;
use std::collections::HashMap;

/// Represents usage data in different formats for reporting.
///
/// Can hold either raw token counts, monetary costs, or nested
/// groupings of usage data (e.g., per-model breakdowns).
#[derive(Serialize)]
pub enum UsageReport {
    /// Total token count.
    Token(u64),
    /// Total cost in dollars.
    Money(f64),
    /// Nested usage data, typically grouped by model name.
    Map(HashMap<String, UsageReport>),
    /// Raw JSON dump for the raw command.
    Raw(String),
}

impl UsageReport {
    /// Renders the report into a string based on its variant.
    /// - Maps become CSV data.
    /// - Numeric values (Money/Token) become formatted strings.
    pub fn render(&self, no_format: bool, with_symbol: Option<bool>) -> AppResult<String> {
        match self {
            // Numeric reports: Format as currency or raw numbers.
            UsageReport::Token(number) => Ok(Self::render_token(number)),
            UsageReport::Money(number) => Ok(Self::render_money(number, no_format, with_symbol)),

            // Map reports: Serialize to CSV.
            UsageReport::Map(_hp) => self.format_csv(no_format),

            _ => unreachable!(
                "UsageReport::render: Unhandled variant. Did you add a new report type?"
            ),
        }
    }

    /// Internal helper: Serializes map data into a valid CSV string.
    fn format_csv(&self, no_format: bool) -> AppResult<String> {
        match self {
            UsageReport::Map(hp) => {
                /// Temporary struct to define the CSV column layout for render groupping.
                /// There are 2 columns: entity name and formatted value.
                #[derive(Serialize)]
                struct CsvRow {
                    /// Left column, for an entity name.
                    display_name: String,
                    /// Right column, for formatted value.
                    content: String,
                }

                let mut writer = csv::WriterBuilder::new()
                    .has_headers(false) // I don't want a header.
                    .from_writer(vec![]);

                for (key, value) in hp {
                    let (display_name, content) = match value {
                        // This is a special case when rendering money inside csv.
                        //
                        // When formatting is enabled, include the cost in the name.
                        // example: "model-name-123 ($1.23)"
                        //
                        // This works well for piping to tools like uplot:
                        // - The display string is in the left cell.
                        // - The numeric value is in the right cell for sorting, for example, | sort --xx |
                        //   since dollar-prefixed numbers can't be sorted programmartically.
                        UsageReport::Money(_) => {
                            // Make render's with_symbol shadow the no_format flip.
                            // It's basically this: if no format → no dollar sign.
                            // Note: Could make this configurable via CLI flag.
                            let cost_with_symbol = value.render(no_format, Some(!no_format))?;
                            let cost_without_symbol = value.render(no_format, Some(false))?;

                            // We need this -> model-name-123 ($1.23).
                            let display_column = format!("{} ({})", key, cost_with_symbol);

                            // We don't need the right column to have a dollar sign as it breaks the
                            // program like uplot.
                            (display_column, cost_without_symbol)
                        }
                        _ => (key.clone(), value.render(no_format, None)?), // Just passing them along.
                    };

                    let row = CsvRow {
                        display_name,
                        content,
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

            _ => unreachable!("Logic error: format_csv called on a non-Map variant."),
        }
    }

    // Internal helper: Formats numeric variants, optionally removing the unit.

    fn render_token(value: &u64) -> String {
        value.to_string()
    }

    /// Render money.
    /// with_symbol is optional to maintain backward compatibility; default is true.
    /// Note: I will later replace this with something like rusty-money.
    fn render_money(value: &f64, no_format: bool, with_symbol: Option<bool>) -> String {
        // Should this returns .amount() in the future?
        if no_format {
            // example: 1.23456
            return value.to_string();
        }

        // This should be enough for now.
        let symbol = if with_symbol.unwrap_or(true) { "$" } else { "" };

        // example: $1.23 or 1.23, depending on optional with_symbol.
        format!("{}{:.2}", symbol, value)
    }
}

/// Converts a cost value into a Money report.
impl From<f64> for UsageReport {
    fn from(value: f64) -> Self {
        UsageReport::Money(value)
    }
}

/// Converts a token count into a Token report.
impl From<u64> for UsageReport {
    fn from(value: u64) -> Self {
        UsageReport::Token(value)
    }
}

/// Converts a map of costs (e.g., per-model) into a nested report.
///
/// Example: `{ "model-a": 1.23, "model-b": 4.56 }`
impl From<HashMap<String, f64>> for UsageReport {
    fn from(map: HashMap<String, f64>) -> Self {
        let converted = map
            .into_iter()
            .map(|(k, v)| (k, UsageReport::Money(v)))
            .collect();
        UsageReport::Map(converted)
    }
}

/// Converts a map of token counts (e.g., per-model) into a nested report.
///
/// Example: `{ "model-a": 1000, "model-b": 2000 }`
impl From<HashMap<String, u64>> for UsageReport {
    fn from(map: HashMap<String, u64>) -> Self {
        let converted = map
            .into_iter()
            .map(|(k, v)| (k, UsageReport::Token(v)))
            .collect();
        UsageReport::Map(converted)
    }
}
