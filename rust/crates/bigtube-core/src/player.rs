//! Resolve a playable stream URL via yt-dlp, mirroring
//! `PlayerController._extract_stream_url`. Local files pass through unchanged.

use std::path::Path;
use std::time::Duration;

use crate::config;
use crate::process::run_with_timeout;
use crate::validators::timeouts;

const DEFAULT_PREVIEW_RESOLUTION: i64 = 360;

fn preview_format_selector(max_height: i64) -> String {
    let max_height = match max_height {
        144 | 240 | 360 | 480 | 720 | 1080 => max_height,
        _ => DEFAULT_PREVIEW_RESOLUTION,
    };
    format!(
        "best[height<={max_height}][vcodec!=none][acodec!=none][protocol^=http]/best[height<={max_height}][vcodec!=none][acodec!=none]/best[vcodec!=none][acodec!=none]"
    )
}

/// Returns a directly-playable URL for `url`. For a local file path or on any
/// failure, returns the input unchanged so the caller can still try to play it.
pub fn extract_stream_url(url: &str) -> String {
    if Path::new(url).exists() {
        return url.to_string();
    }

    let (binary, env, common, preview_resolution) = {
        let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
        match cfg.get_yt_dlp_path() {
            Ok(b) => (
                b,
                cfg.get_env_with_bin_path(),
                cfg.get_yt_dlp_common_args(),
                cfg.get_i64("preview_resolution"),
            ),
            Err(_) => return url.to_string(),
        }
    };

    // playbin needs one muxed audio/video URL. Prefer a progressive HTTP format
    // at or below the selected preview cap. If none exists, fall back to the
    // best muxed rendition at any resolution rather than failing playback.
    let mut args = vec![
        "--extractor-args".to_string(),
        "youtube:player_client=android,web".to_string(),
        "-f".to_string(),
        preview_format_selector(preview_resolution),
        "-g".to_string(),
    ];
    args.extend(common);
    args.push(url.to_string());
    match run_with_timeout(
        &binary,
        &args,
        &env,
        Duration::from_secs(timeouts::STREAM_EXTRACTION),
    ) {
        Ok((0, stdout, _)) => stdout
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| url.to_string()),
        _ => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_selector_caps_resolution_and_prefers_progressive_muxed_formats() {
        assert_eq!(
            preview_format_selector(720),
            "best[height<=720][vcodec!=none][acodec!=none][protocol^=http]/best[height<=720][vcodec!=none][acodec!=none]/best[vcodec!=none][acodec!=none]"
        );
    }

    #[test]
    fn preview_selector_uses_safe_default_for_invalid_resolution() {
        assert_eq!(
            preview_format_selector(2160),
            preview_format_selector(DEFAULT_PREVIEW_RESOLUTION)
        );
    }
}
