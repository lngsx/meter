use jiff::Zoned;

use miette::IntoDiagnostic;
use spinoff::Spinner;

use super::dtos::{MessagesUsageReport, UsageDataBucket};

const API_VERSION: &str = "2023-06-01";
const BUCKET_WIDTH: &str = "1h";
const USAGE_REPORT_ENDPOINT: &str =
    "https://api.anthropic.com/v1/organizations/usage_report/messages";
const GAP_TIME_BETWEEN_FETCH_IN_SEC: u64 = 5;

pub fn fetch(
    key: &str,
    starting_at: &Zoned,
    ending_at: Option<&Zoned>,
    spinner_container: &mut Option<Spinner>,
) -> miette::Result<Vec<UsageDataBucket>> {
    // RFC 3339, this API expects this format.
    let starting_at_timestamp = starting_at.timestamp().to_string();
    let ending_at_timestamp = ending_at.map(|time| time.timestamp().to_string());

    // This one has to be started with true so the first round can proceed, then what comes out
    // from the respond will automatically dictate this value until it reaches the false.
    let mut has_more: bool = true;

    // Manual counter.
    let mut page_number = 1;

    // Start empty.
    let mut next_page: Option<String> = None;
    let mut usages: Vec<UsageDataBucket> = vec![];

    while has_more {
        // First things first, give users something to look at.
        if let Some(spinner) = spinner_container.as_mut() {
            spinner.update_text(progress_text(page_number))
        }

        if page_number > 1 {
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
        page_number += 1;
    }

    Ok(usages)
}

fn inner_fetch(
    key: &str,
    starting_at_timestamp: &str,
    ending_at_timestamp: Option<&str>,
    next_page: Option<&str>,
) -> miette::Result<MessagesUsageReport> {
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
        .call()
        .into_diagnostic()?
        .body_mut()
        .read_json::<MessagesUsageReport>()
        .into_diagnostic()?;

    Ok(body)
}

/// Keep ourselves safe. We can wait.
fn wait() {
    let duration = std::time::Duration::from_secs(GAP_TIME_BETWEEN_FETCH_IN_SEC);

    std::thread::sleep(duration);
}

/// Returns a progress message with dots indicating the current page number.
///
/// User should see a proper visual feedback becasue fetching will take quite some time
/// since I've put a big gap between fetches as a safety measure.
fn progress_text(page_number: usize) -> String {
    format!("Retrieving{}", ".".repeat(page_number))
}

// Junkyard.

// No more of this, but I prefer to keep it until I am certain this design is okay.
// pub fn fetch_raw(key: &str, starting_at: &Zoned) -> Result<String, Box<dyn Error>> {
//     // let starting_at = start_of_day(zoned_now)?;
//
//     let starting_at_timestamp = starting_at.timestamp().to_string();
//
//     let request = ureq::get(USAGE_REPORT_ENDPOINT)
//         .header("anthropic-version", API_VERSION)
//         .header("X-Api-Key", key)
//         // ranging, sizing.
//         .query("starting_at", starting_at_timestamp)
//         .query("bucket_width", BUCKET_WIDTH)
//         // grouping.
//         .query("group_by[]", "model")
//         .query("group_by[]", "context_window")
//         .query("group_by[]", "workspace_id")
//         .query("group_by[]", "api_key_id");
//
//     let response = request.call()?.body_mut().read_to_string()?;
//
//     Ok(response)
// }
