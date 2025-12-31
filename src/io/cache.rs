use jiff::{Timestamp, ToSpan};
use miette::IntoDiagnostic;

use std::fs;

pub fn try_retrieve_cache(
    cache_file_path: &std::path::Path,
    ttl: &i64,
    system_now: &Timestamp,
) -> miette::Result<Option<String>> {
    if !cache_file_path.try_exists().into_diagnostic()? {
        return Ok(None);
    }

    if is_cache_expired(cache_file_path, system_now, ttl)? {
        return Ok(None);
    }

    let content = Some(fs::read_to_string(cache_file_path).into_diagnostic()?);

    Ok(content)
}

pub fn try_write_cache(
    cache_file_path: &std::path::Path,
    body_string: &str,
    ttl: &i64,
    system_now: &Timestamp,
) -> miette::Result<()> {
    // Ensure the directory exists.
    if let Some(parent) = cache_file_path.parent() {
        fs::create_dir_all(parent).into_diagnostic()?;
    }

    let is_cache_alive = cache_file_path.try_exists().into_diagnostic()?
        && !is_cache_expired(cache_file_path, system_now, ttl)?;

    // Do not touch it if the cache is still alive.
    // Because we rely on the mtime, if we touch it, the countdown will change.
    // Without this check, every cache hit would reset the ttl.
    if is_cache_alive {
        return Ok(());
    }

    fs::write(cache_file_path, body_string).into_diagnostic()?;

    Ok(())
}

fn is_cache_expired(
    cache_file_path: &std::path::Path,
    system_now: &Timestamp,
    ttl: &i64,
) -> miette::Result<bool> {
    let metadata = fs::metadata(cache_file_path).into_diagnostic()?;
    let file_mtime = metadata.modified().into_diagnostic()?;
    let expiration_time = Timestamp::try_from(file_mtime).into_diagnostic()? + ttl.minutes();

    Ok(expiration_time <= *system_now)
}
