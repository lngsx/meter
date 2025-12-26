use std::error::Error;

use jiff::Zoned;

use super::dtos::{MessagesUsageReport, UsageDataBucket};

const API_VERSION: &str = "2023-06-01";
const BUCKET_WIDTH: &str = "1h";
const USAGE_REPORT_ENDPOINT: &str =
    "https://api.anthropic.com/v1/organizations/usage_report/messages";
const GAP_TIME_BETWEEN_FETCH_IN_SEC: u64 = 5;

/// The time value must be ready to use before it goes into this function.
pub fn fetch(
    key: &str,
    starting_at: &Zoned,
    ending_at: Option<&Zoned>,
) -> Result<Vec<UsageDataBucket>, Box<dyn Error>> {
    // RFC 3339, this API expects this format.
    let starting_at_timestamp = starting_at.timestamp().to_string();
    let ending_at_timestamp = ending_at.map(|time| time.timestamp().to_string());

    // This one has to be started with true so the first round can proceed, then what comes out
    // from the respond will automatically dictate this value until it reaches the false.
    let mut has_more: bool = true;

    let mut is_first_round: bool = true;

    // Start empty.
    let mut next_page: Option<String> = None;
    let mut usages: Vec<UsageDataBucket> = vec![];

    while has_more {
        if !is_first_round {
            wait();
        }

        let body = inner_fetch(
            key,
            &starting_at_timestamp,
            ending_at_timestamp.as_deref(),
            next_page.as_deref(),
        )?;
        let plucked_data = body.data;

        // Then save it.
        usages.extend(plucked_data);

        // Now prepare it for the next round.
        has_more = body.has_more;
        next_page = body.next_page;
        is_first_round = false;
    }

    Ok(usages)
}

fn inner_fetch(
    key: &str,
    starting_at_timestamp: &str,
    ending_at_timestamp: Option<&str>,
    next_page: Option<&str>,
) -> Result<MessagesUsageReport, Box<dyn Error>> {
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

    // optional page.
    let request = match next_page {
        Some(page_token) => request.query("page", page_token),
        None => request,
    };

    // optional ending_at.
    let request = match ending_at_timestamp {
        Some(timestamp) => request.query("ending_at", timestamp),
        None => request,
    };

    let body = request
        .call()?
        .body_mut()
        .read_json::<MessagesUsageReport>()?;

    Ok(body)
}

/// Fetch then does nothing to the response, returns it right away.
pub fn fetch_raw(key: &str, starting_at: &Zoned) -> Result<String, Box<dyn Error>> {
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

/// Keep ourselves safe. We can wait.
fn wait() {
    let duration = std::time::Duration::from_secs(GAP_TIME_BETWEEN_FETCH_IN_SEC);

    std::thread::sleep(duration);
}
