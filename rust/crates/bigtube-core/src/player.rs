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
        "bestvideo[height<={max_height}]+bestaudio/best[height<={max_height}][vcodec!=none][acodec!=none]/bestvideo+bestaudio/best"
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamUrls {
    pub video: String,
    pub audio: Option<String>,
}

fn parse_stream_urls(stdout: &str) -> Option<StreamUrls> {
    let mut lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    lines.next().map(|video| StreamUrls {
        video: video.to_string(),
        audio: lines.next().map(str::to_string),
    })
}

/// Returns a directly-playable URL for `url`. For a local file path or on any
/// failure, returns the input unchanged so the caller can still try to play it.
pub fn extract_stream_urls(url: &str) -> StreamUrls {
    if Path::new(url).exists() {
        return StreamUrls {
            video: url.to_string(),
            audio: None,
        };
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
            Err(_) => {
                return StreamUrls {
                    video: url.to_string(),
                    audio: None,
                }
            }
        }
    };

    // Prefer separate video/audio formats so preview resolutions above the
    // source's muxed formats (commonly 360p) remain available. Fall back to a
    // muxed format under the requested cap, then the best formats available.
    let mut args = vec![
        "--extractor-args".to_string(),
        "youtube:player_client=android,web".to_string(),
        "-f".to_string(),
        preview_format_selector(preview_resolution),
        "-g".to_string(),
    ];
    args.extend(common);
    args.push(url.to_string());
    let streams = match run_with_timeout(
        &binary,
        &args,
        &env,
        Duration::from_secs(timeouts::STREAM_EXTRACTION),
    ) {
        Ok((0, stdout, _)) => parse_stream_urls(&stdout),
        _ => None,
    };
    streams.unwrap_or_else(|| StreamUrls {
        video: url.to_string(),
        audio: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_selector_prefers_separate_streams_with_muxed_fallback() {
        assert_eq!(
            preview_format_selector(720),
            "bestvideo[height<=720]+bestaudio/best[height<=720][vcodec!=none][acodec!=none]/bestvideo+bestaudio/best"
        );
    }

    #[test]
    fn preview_selector_uses_safe_default_for_invalid_resolution() {
        assert_eq!(
            preview_format_selector(2160),
            preview_format_selector(DEFAULT_PREVIEW_RESOLUTION)
        );
    }

    #[test]
    fn parses_separate_video_and_audio_stream_urls() {
        assert_eq!(
            parse_stream_urls("https://cdn/video\nhttps://cdn/audio\n"),
            Some(StreamUrls {
                video: "https://cdn/video".to_string(),
                audio: Some("https://cdn/audio".to_string()),
            })
        );
    }

    #[test]
    fn parses_muxed_stream_url() {
        assert_eq!(
            parse_stream_urls("\nhttps://cdn/muxed\n"),
            Some(StreamUrls {
                video: "https://cdn/muxed".to_string(),
                audio: None,
            })
        );
    }
}
