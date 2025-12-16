use jiff::{Timestamp, ToSpan};

use std::error::Error;

use std::fs;

pub fn try_retrieve_cache(
    cache_file_path: &std::path::Path,
    ttl: &i64,
    system_now: &Timestamp,
) -> Result<Option<String>, Box<dyn Error>> {
    if !cache_file_path.try_exists()? {
        return Ok(None);
    }

    if is_cache_expired(cache_file_path, system_now, ttl)? {
        return Ok(None);
    }

    let content = Some(fs::read_to_string(cache_file_path)?);

    Ok(content)
}

pub fn try_write_cache(
    cache_file_path: &std::path::Path,
    body_string: &str,
    ttl: &i64,
    system_now: &Timestamp,
) -> Result<(), Box<dyn Error>> {
    // Ensure the directory exists.
    if let Some(parent) = cache_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let is_cache_alive =
        cache_file_path.try_exists()? && !is_cache_expired(cache_file_path, system_now, ttl)?;

    // Do not touch it if the cache is still alive.
    // Because we rely on the mtime, if we touch it, the countdown will change.
    // Without this check, every cache hit would reset the ttl.
    if is_cache_alive {
        return Ok(());
    }

    fs::write(cache_file_path, body_string)?;

    Ok(())
}

fn is_cache_expired(
    cache_file_path: &std::path::Path,
    system_now: &Timestamp,
    ttl: &i64,
) -> Result<bool, Box<dyn Error>> {
    let metadata = fs::metadata(cache_file_path)?;
    let file_mtime = metadata.modified()?;
    let expiration_time = Timestamp::try_from(file_mtime)? + ttl.minutes();

    Ok(expiration_time <= *system_now)
}

// This is my original implementation, it's make code read better in ruby style,
// but I removed it as I was afraid to maintain it.
// fn is_cache_alive(
//     cache_file_path: &std::path::Path,
//     system_now: &Timestamp,
//     ttl: &i64,
// ) -> Result<bool, Box<dyn Error>> {
//     is_cache_expired(cache_file_path, system_now, ttl).map(|expired| !expired)
// }
