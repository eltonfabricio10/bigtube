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

    if crate::diagnostics::is_youtube_block_error(&stderr) {
        tracing::warn!(
            "preview extraction blocked: {}",
            crate::diagnostics::summarize_error_detail(&stderr)
        );
        return Err(StreamError::AccessBlocked);
    }

    if code != 0 {
        let safe_stderr = crate::diagnostics::summarize_error_detail(&stderr);
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
        assert!(crate::diagnostics::is_youtube_block_error(
            "playability status: LOGIN_REQUIRED"
        ));
        assert!(crate::diagnostics::is_youtube_block_error(
            "ERROR: [youtube] This video is unavailable"
        ));
        assert!(crate::diagnostics::is_youtube_block_error(
            "HTTP Error 403: Forbidden"
        ));
        assert!(crate::diagnostics::is_youtube_block_error(
            "HTTP Error 429: Too Many Requests"
        ));
        assert!(!crate::diagnostics::is_youtube_block_error(
            "Unable to download webpage: connection reset"
        ));
    }

    #[test]
    fn redacts_stream_urls_from_extractor_diagnostics() {
        assert_eq!(
            crate::diagnostics::summarize_error_detail(
                "ERROR: https://example.test/video?token=secret failed"
            ),
            "ERROR: [URL redacted] failed"
        );
    }
}
