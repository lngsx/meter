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
    pub fn render(&self, no_format: bool) -> AppResult<String> {
        match self {
            // Numeric reports: Format as currency or raw numbers.
            UsageReport::Token(number) => Ok(Self::render_token(number)),
            UsageReport::Money(number) => Ok(Self::render_money(number, no_format)),

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
                /// Temporary struct to define the CSV column layout.
                #[derive(Serialize)]
                struct CsvRow {
                    model_summary_row: String,
                    content: String,
                }

                let mut writer = csv::WriterBuilder::new()
                    .has_headers(false) // I don't want a header.
                    .from_writer(vec![]);

                for (key, value) in hp {
                    let row = CsvRow {
                        model_summary_row: key.clone(),
                        content: value.render(no_format)?, // Reuse the renderer.
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

    fn render_money(value: &f64, no_format: bool) -> String {
        if no_format {
            return value.to_string();
        }

        format!("${:.2}", value)
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
