use std::error::Error;

use jiff::Zoned;

use super::dtos::MessagesUsageReport;

// Not this time.
// use ureq::Error;

const API_VERSION: &str = "2023-06-01";
const BUCKET_WIDTH: &str = "1h";
const USAGE_REPORT_ENDPOINT: &str =
    "https://api.anthropic.com/v1/organizations/usage_report/messages";

/// The time value must be ready to use before it goes into this function.
pub fn fetch(starting_at: &Zoned, key: &str) -> Result<MessagesUsageReport, Box<dyn Error>> {
    // let starting_at = start_of_day(jiff_zoned_time)?;

    // RFC 3339, this API expects this format.
    let starting_at_timestamp = starting_at.timestamp().to_string();

    let request = ureq::get(USAGE_REPORT_ENDPOINT)
        .header("anthropic-version", API_VERSION)
        .header("X-Api-Key", key)
        // ranging, sizing.
        .query("starting_at", starting_at_timestamp)
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

/// Fetch then does nothing to the response, returns it right away.
pub fn fetch_raw(starting_at: &Zoned, key: &str) -> Result<String, Box<dyn Error>> {
    // let starting_at = start_of_day(zoned_now)?;

    let starting_at_timestamp = starting_at.timestamp().to_string();

    let request = ureq::get(USAGE_REPORT_ENDPOINT)
        .header("anthropic-version", API_VERSION)
        .header("X-Api-Key", key)
        // ranging, sizing.
        .query("starting_at", starting_at_timestamp)
        .query("bucket_width", BUCKET_WIDTH)
        // grouping.
        .query("group_by[]", "model")
        .query("group_by[]", "context_window")
        .query("group_by[]", "workspace_id")
        .query("group_by[]", "api_key_id");

    let response = request.call()?.body_mut().read_to_string()?;

    Ok(response)
}

// Junkyard

// Compose timestring.
// fn start_of_day(zoned_now: &Zoned) -> Result<String, jiff::Error> {
//     // let start_of_day = Zoned::now().start_of_day()?;
//     let start_of_day = zoned_now.start_of_day()?;
//
//     // rcf 3339
//     let timestamp = start_of_day.timestamp().to_string();
//
//     Ok(timestamp)
// }

// Will later remove this.
// It's now handle with clap. Thank you for your service.
// Retriving environment variable.
// fn admin_key() -> Result<String, env::VarError> {
//     let key = env::var("ANTHROPIC_ADMIN_API_KEY")?;
//
//     Ok(key)
// }
