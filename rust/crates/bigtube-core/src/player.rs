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
        "bestvideo[height<={max_height}][protocol^=https]+bestaudio[protocol^=https]/best[height<={max_height}][protocol^=https][vcodec!=none][acodec!=none]/bestvideo[height<={max_height}]+bestaudio/best[height<={max_height}][vcodec!=none][acodec!=none]"
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamUrls {
    pub video: StreamSource,
    pub audio: Option<StreamSource>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamSource {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

fn parse_stream_source(format: &serde_json::Value) -> Option<StreamSource> {
    let url = format.get("url")?.as_str()?.to_string();
    let headers = format
        .get("http_headers")
        .and_then(serde_json::Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .as_str()
                        .map(|value| (name.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(StreamSource { url, headers })
}

fn parse_stream_urls(stdout: &str) -> Option<StreamUrls> {
    let info: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let formats = info
        .get("requested_formats")
        .and_then(serde_json::Value::as_array);

    if let Some(formats) = formats {
        let video = formats.iter().find(|format| {
            format
                .get("vcodec")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|codec| codec != "none")
        });
        let audio = formats.iter().find(|format| {
            format.get("vcodec").and_then(serde_json::Value::as_str) == Some("none")
                && format
                    .get("acodec")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|codec| codec != "none")
        });
        if let Some(video) = video.and_then(parse_stream_source) {
            return Some(StreamUrls {
                video,
                audio: audio.and_then(parse_stream_source),
            });
        }
    }

    parse_stream_source(&info).map(|video| StreamUrls { video, audio: None })
}

/// Resolves stream URLs and their HTTP headers. For a local path or extraction
/// failure, returns the input unchanged so the caller can still try to play it.
pub fn extract_stream_urls(url: &str) -> StreamUrls {
    if Path::new(url).exists() {
        return StreamUrls {
            video: StreamSource {
                url: url.to_string(),
                headers: Vec::new(),
            },
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
                    video: StreamSource {
                        url: url.to_string(),
                        headers: Vec::new(),
                    },
                    audio: None,
                }
            }
        }
    };

    // Prefer separate video/audio formats so preview resolutions above the
    // source's muxed formats (commonly 360p) remain available. Keep every
    // fallback within the requested height cap.
    let mut args = vec![
        "--no-warnings".to_string(),
        "--no-playlist".to_string(),
        "-f".to_string(),
        preview_format_selector(preview_resolution),
        "-j".to_string(),
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
        video: StreamSource {
            url: url.to_string(),
            headers: Vec::new(),
        },
        audio: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_selector_prefers_separate_streams_and_caps_all_fallbacks() {
        assert_eq!(
            preview_format_selector(720),
            "bestvideo[height<=720][protocol^=https]+bestaudio[protocol^=https]/best[height<=720][protocol^=https][vcodec!=none][acodec!=none]/bestvideo[height<=720]+bestaudio/best[height<=720][vcodec!=none][acodec!=none]"
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
            parse_stream_urls(
                r#"{"requested_formats":[{"url":"https://cdn/video","vcodec":"avc1","acodec":"none","http_headers":{"User-Agent":"test-agent"}},{"url":"https://cdn/audio","vcodec":"none","acodec":"opus","http_headers":{"Referer":"https://example.test/"}}]}"#
            ),
            Some(StreamUrls {
                video: StreamSource {
                    url: "https://cdn/video".to_string(),
                    headers: vec![("User-Agent".into(), "test-agent".into())],
                },
                audio: Some(StreamSource {
                    url: "https://cdn/audio".to_string(),
                    headers: vec![("Referer".into(), "https://example.test/".into())],
                }),
            })
        );
    }

    #[test]
    fn parses_muxed_stream_url() {
        assert_eq!(
            parse_stream_urls(
                r#"{"url":"https://cdn/muxed","vcodec":"avc1","acodec":"mp4a","http_headers":{"User-Agent":"test-agent"}}"#
            ),
            Some(StreamUrls {
                video: StreamSource {
                    url: "https://cdn/muxed".to_string(),
                    headers: vec![("User-Agent".into(), "test-agent".into())],
                },
                audio: None,
            })
        );
    }
}
