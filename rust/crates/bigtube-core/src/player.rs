//! Resolve a playable stream URL via yt-dlp, mirroring
//! `PlayerController._extract_stream_url`. Local files pass through unchanged.

use std::path::Path;
use std::time::Duration;

use crate::config;
use crate::process::run_with_timeout;
use crate::validators::timeouts;

const PREVIEW_FORMAT_SELECTOR: &str =
    "18/best[height<=360][vcodec!=none][acodec!=none][protocol^=http]/best[vcodec!=none][acodec!=none]/best";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamUrls {
    pub video: StreamSource,
    pub audio: Option<StreamSource>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamSource {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamError {
    AccessBlocked,
    ExtractionFailed(String),
}

/// Resolves a single muxed preview stream at the previously stable quality.
/// Local paths pass through unchanged; failures are returned rather than
/// attempting to play the original YouTube watch page as if it were media.
pub fn extract_stream_urls(url: &str) -> Result<StreamUrls, StreamError> {
    let unchanged = || StreamUrls {
        video: StreamSource {
            url: url.to_string(),
            headers: Vec::new(),
        },
        audio: None,
    };

    if Path::new(url).exists() {
        return Ok(unchanged());
    }

    let (binary, env, common) = {
        let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
        (
            cfg.get_yt_dlp_path(),
            cfg.get_env_with_bin_path(),
            cfg.get_yt_dlp_common_args(),
        )
    };
    let binary = binary.map_err(|error| {
        tracing::warn!("preview extraction unavailable: {error}");
        StreamError::ExtractionFailed("yt-dlp is unavailable".into())
    })?;

    let mut args = vec![
        "--extractor-args".to_string(),
        "youtube:player_client=android,web".to_string(),
        "--no-playlist".to_string(),
        "-f".to_string(),
        PREVIEW_FORMAT_SELECTOR.to_string(),
        "-g".to_string(),
    ];
    args.extend(common);
    args.push(url.to_string());

    let (code, stdout, stderr) = match run_with_timeout(
        &binary,
        &args,
        &env,
        Duration::from_secs(timeouts::STREAM_EXTRACTION),
    ) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!("preview extraction failed: {error}");
            return Err(StreamError::ExtractionFailed(
                "yt-dlp timed out or could not start".into(),
            ));
        }
    };

    if is_access_blocked(&stderr) {
        tracing::warn!("preview extraction blocked: {}", redact_urls(&stderr));
        return Err(StreamError::AccessBlocked);
    }

    if code != 0 {
        let safe_stderr = redact_urls(&stderr);
        tracing::warn!(
            "preview extraction failed (exit code {code}): {}",
            safe_stderr
        );
        return Err(StreamError::ExtractionFailed(safe_stderr));
    }

    let stream_url = stdout.lines().map(str::trim).find(|line| !line.is_empty());
    match stream_url {
        Some(url) if url.starts_with("https://") || url.starts_with("http://") => Ok(StreamUrls {
            video: StreamSource {
                url: url.to_string(),
                headers: Vec::new(),
            },
            audio: None,
        }),
        Some(_) => {
            tracing::warn!("preview extraction returned a non-HTTP stream URL");
            Err(StreamError::ExtractionFailed(
                "yt-dlp returned an invalid stream URL".into(),
            ))
        }
        None => {
            tracing::warn!("preview extraction succeeded without returning a stream URL");
            Err(StreamError::ExtractionFailed(
                "yt-dlp returned no stream URL".into(),
            ))
        }
    }
}

fn is_access_blocked(stderr: &str) -> bool {
    let stderr = stderr.to_lowercase();
    [
        "login_required",
        "login required",
        "sign in to confirm",
        "confirm you're not a bot",
        "confirm you are not a bot",
        "not a bot",
        "http error 401",
        "http error 403",
        "403 forbidden",
        "http error 429",
        "too many requests",
        "sign in to view",
        "members-only",
        "members only",
        "age-restricted",
        "age restricted",
        "private video",
        "video is unavailable",
        "video unavailable",
        "not available in your country",
        "geo-restricted",
        "geo restricted",
    ]
    .iter()
    .any(|pattern| stderr.contains(pattern))
}

fn redact_urls(text: &str) -> String {
    let mut redacted = String::with_capacity(text.len());
    for (index, part) in text.split_inclusive(char::is_whitespace).enumerate() {
        if index > 0 && redacted.len() >= 1200 {
            break;
        }
        let trimmed = part.trim_end_matches(char::is_whitespace);
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            redacted.push_str("[URL redacted]");
        } else {
            redacted.push_str(trimmed);
        }
        redacted.push_str(&part[trimmed.len()..]);
    }
    while !redacted.is_char_boundary(redacted.len().min(1200)) {
        redacted.pop();
    }
    redacted.truncate(redacted.len().min(1200));
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_selector_uses_the_stable_muxed_360p_format() {
        assert_eq!(
            PREVIEW_FORMAT_SELECTOR,
            "18/best[height<=360][vcodec!=none][acodec!=none][protocol^=http]/best[vcodec!=none][acodec!=none]/best"
        );
    }

    #[test]
    fn local_file_is_returned_as_a_single_stream() {
        let stream = extract_stream_urls(env!("CARGO_MANIFEST_DIR"));
        let stream = stream.unwrap();
        assert!(stream.video.url.ends_with("bigtube-core"));
        assert!(stream.video.headers.is_empty());
        assert!(stream.audio.is_none());
    }

    #[test]
    fn access_block_markers_include_login_and_rate_limits() {
        assert!(is_access_blocked("playability status: LOGIN_REQUIRED"));
        assert!(is_access_blocked(
            "ERROR: [youtube] This video is unavailable"
        ));
        assert!(is_access_blocked("HTTP Error 403: Forbidden"));
        assert!(is_access_blocked("HTTP Error 429: Too Many Requests"));
        assert!(!is_access_blocked(
            "Unable to download webpage: connection reset"
        ));
    }

    #[test]
    fn redacts_stream_urls_from_extractor_diagnostics() {
        assert_eq!(
            redact_urls("ERROR: https://example.test/video?token=secret failed"),
            "ERROR: [URL redacted] failed"
        );
    }
}
