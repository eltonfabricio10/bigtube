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

/// Resolves a single muxed preview stream at the previously stable quality.
/// For a local path or extraction failure, the input is returned for playback.
pub fn extract_stream_urls(url: &str) -> StreamUrls {
    let unchanged = || StreamUrls {
        video: StreamSource {
            url: url.to_string(),
            headers: Vec::new(),
        },
        audio: None,
    };

    if Path::new(url).exists() {
        return unchanged();
    }

    let (binary, env, common) = {
        let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
        match cfg.get_yt_dlp_path() {
            Ok(binary) => (
                binary,
                cfg.get_env_with_bin_path(),
                cfg.get_yt_dlp_common_args(),
            ),
            Err(_) => return unchanged(),
        }
    };

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

    let stream_url = match run_with_timeout(
        &binary,
        &args,
        &env,
        Duration::from_secs(timeouts::STREAM_EXTRACTION),
    ) {
        Ok((0, stdout, _)) => stdout
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_string),
        _ => None,
    };

    stream_url
        .map(|url| StreamUrls {
            video: StreamSource {
                url,
                headers: Vec::new(),
            },
            audio: None,
        })
        .unwrap_or_else(unchanged)
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
        assert!(stream.video.url.ends_with("bigtube-core"));
        assert!(stream.video.headers.is_empty());
        assert!(stream.audio.is_none());
    }
}
