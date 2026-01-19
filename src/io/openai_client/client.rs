use jiff::Zoned;

use super::dtos::{OpenAiUsageBucket, OpenAiUsagePage};
use crate::app::App;
use crate::error::Error::OpenAiRateLimitExceeded;
use crate::prelude::*;

const BUCKET_WIDTH: &str = "1h";
const ENDPOINT: &str = "https://api.openai.com/v1/organization/usage/completions";
const GAP_TIME_BETWEEN_FETCH_IN_SEC: u64 = 5;
// For dev test.
// const ENDPOINT: &str = "https://httpbin.org/status/429";

pub fn fetch(
    ctx: &App,
    starting_at: &Zoned,
    ending_at: Option<&Zoned>,
) -> AppResult<Vec<OpenAiUsageBucket>> {
    let key = ctx.cli.try_get_anthropic_key()?.to_owned();

    // Epoc seconds, this API expects this format.
    let starting_at = starting_at.timestamp().as_second();
    let ending_at = ending_at.map(|time| time.timestamp().as_second());

    let mut has_more: bool = true;
    let mut page_number = 1;

    // Start empty.
    let mut next_page: Option<String> = None;
    let mut usages: Vec<OpenAiUsageBucket> = vec![];

    while has_more {
        ctx.display.update_spin_message(progress_text(page_number));

        if page_number > 1 {
            wait();
        }

        let body = inner_fetch(
            &key,
            &starting_at.to_string(),
            ending_at.map(|it| it.to_string()).as_deref(),
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
    starting_at: &str,
    ending_at: Option<&str>,
    next_page: Option<&str>,
) -> AppResult<OpenAiUsagePage> {
    let request = ureq::get(ENDPOINT)
        .header("Authorization", &format!("Bearer {}", key))
        // ranging, sizing.
        .query("start_time", starting_at)
        .query("bucket_width", BUCKET_WIDTH)
        // grouping.
        .query("group_by[]", "model");

    // optional page.
    let request = match next_page {
        Some(page_token) => request.query("page", page_token),
        None => request,
    };

    // optional ending_at.
    let request = match ending_at {
        Some(timestamp) => request.query("end_time", timestamp),
        None => request,
    };

    let mut response = match request.call() {
        Ok(res) => res,
        Err(ureq::Error::StatusCode(429)) => bail!(OpenAiRateLimitExceeded),
        Err(e) => bail!(e),
    };

    let body = response
        .body_mut()
        .read_json::<OpenAiUsagePage>()
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
