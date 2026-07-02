//! Resolve a playable stream URL via yt-dlp, mirroring
//! `PlayerController._extract_stream_url`. Local files pass through unchanged.

use std::path::Path;
use std::time::Duration;

use crate::config;
use crate::process::run_with_timeout;
use crate::validators::timeouts;

/// Returns a directly-playable URL for `url`. For a local file path or on any
/// failure, returns the input unchanged so the caller can still try to play it.
pub fn extract_stream_url(url: &str) -> String {
    if Path::new(url).exists() {
        return url.to_string();
    }

    let (binary, env, common) = {
        let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
        match cfg.get_yt_dlp_path() {
            Ok(b) => (b, cfg.get_env_with_bin_path(), cfg.get_yt_dlp_common_args()),
            Err(_) => return url.to_string(),
        }
    };

    // The in-app player is a PREVIEW, fixed at a rock-solid 360p: format 18
    // (muxed 360p MP4) is a plain progressive stream — no HLS/adaptive, no
    // rebuffering or quality flicker. Higher YouTube renditions are only served
    // as segmented HLS (MPEG-TS, which stutters via GStreamer) or as DASH whose
    // URLs are token/UA-bound and 403 outside yt-dlp — so HD is a download job,
    // not a preview. playbin also needs a SINGLE muxed URI (it can't merge
    // separate video+audio), which 360p progressive provides.
    let mut args = vec![
        "--extractor-args".to_string(),
        "youtube:player_client=android,web".to_string(),
        "-f".to_string(),
        "18/best[height<=360][vcodec!=none][acodec!=none][protocol^=http]/best[vcodec!=none][acodec!=none]/best".to_string(),
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
