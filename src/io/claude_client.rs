use std::env;
use std::error::Error;

use jiff::Zoned;

use crate::types::MessagesUsageReport;

// Not this time.
// use ureq::Error;

const API_VERSION: &str = "2023-06-01";
const BUCKET_WIDTH: &str = "1h";
const USAGE_REPORT_ENDPOINT: &str =
    "https://api.anthropic.com/v1/organizations/usage_report/messages";

pub fn fetch() -> Result<MessagesUsageReport, Box<dyn Error>> {
    let request = ureq::get(USAGE_REPORT_ENDPOINT)
        .header("anthropic-version", API_VERSION)
        .header("X-Api-Key", admin_key()?)
        // ranging, sizing.
        .query("starting_at", starting_at()?)
        // .query("starting_at", "2025-12-11T17:00:00Z")
        .query("bucket_width", BUCKET_WIDTH)
        // grouping.
        .query("group_by[]", "model")
        .query("group_by[]", "context_window")
        .query("group_by[]", "workspace_id")
        .query("group_by[]", "api_key_id");

    let report = request
        .call()?
        .body_mut()
        .read_json::<MessagesUsageReport>()?;

    Ok(report)
}

// private

// Compose timestring.
fn starting_at() -> Result<String, jiff::Error> {
    let start_of_day = Zoned::now().start_of_day()?;

    // rcf 3339
    let timestamp = start_of_day.timestamp().to_string();

    // Just for dev. I will later remove it.
    // println!("timestamp: {}", timestamp);

    Ok(timestamp)
}

// Retriving environment variable.
fn admin_key() -> Result<String, env::VarError> {
    let key = env::var("ANTHROPIC_ADMIN_API_KEY")?;

    Ok(key)
}
