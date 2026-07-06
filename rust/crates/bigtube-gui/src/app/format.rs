//! Pure text-formatting helpers for the UI: media summaries and status labels.
//!
//! These are deliberately widget-free so they can be unit-tested headlessly
//! (see the tests at the bottom). Anything that touches GTK/GDK stays in the
//! parent module.

use bigtube_core::converter::MediaSummary;
use bigtube_core::progress::StatusCode;

use super::downloads::codec_pretty;
use super::widgets::human_size;
use crate::i18n::tr;

/// "Video MP4" / "Audio MP3": the media kind (by presence of a video stream)
/// plus the upper-cased file extension, derived from `path`.
fn media_kind_label(s: &MediaSummary, path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_default();
    let kind = if !s.vcodec.is_empty() {
        tr("Video")
    } else if !s.acodec.is_empty() {
        tr("Audio")
    } else {
        String::new()
    };
    match (kind.is_empty(), ext.is_empty()) {
        (false, false) => format!("{kind} {ext}"),
        (false, true) => kind,
        (true, false) => ext,
        (true, true) => String::new(),
    }
}

/// One-line probed-media summary, e.g. "Video MP4 · H.264 · 1920×1080 · AAC ·
/// 48 kHz · 57.9 MiB" (omitting unknown parts).
pub(crate) fn media_summary_text(s: &MediaSummary, path: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    // Lead with the media kind + container, e.g. "Video MP4" / "Audio MP3".
    let kind = media_kind_label(s, path);
    if !kind.is_empty() {
        parts.push(kind);
    }
    let (v, a) = (codec_pretty(&s.vcodec), codec_pretty(&s.acodec));
    if !v.is_empty() {
        parts.push(v);
        // Resolution next to the video codec.
        if s.width > 0 && s.height > 0 {
            parts.push(format!("{}×{}", s.width, s.height));
        }
    }
    if !a.is_empty() {
        parts.push(a);
        // Sample rate next to the audio codec (e.g. 48 kHz).
        if s.sample_rate > 0 {
            let khz = s.sample_rate as f64 / 1000.0;
            if (khz.fract()).abs() < 0.05 {
                parts.push(format!("{khz:.0} kHz"));
            } else {
                parts.push(format!("{khz:.1} kHz"));
            }
        }
    }
    if s.size_bytes > 0 {
        parts.push(human_size(s.size_bytes));
    }
    parts.join(" · ")
}

/// "Location: <folder>" label text for the directory holding `path` (falls back
/// to the path itself if it has no parent). Shown under a row's title instead of
/// the full file path.
pub(crate) fn location_label(path: &str) -> String {
    let dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.to_string());
    format!("{} {dir}", tr("Location:"))
}

/// Map a persisted history status string to its translated label.
pub(crate) fn history_status_label(status: &str) -> String {
    use StatusCode::*;
    let code = match status {
        "completed" => Completed,
        "cancelled" | "interrupted" | "paused" => Cancelled,
        "error" => UnknownError,
        _ => Queued,
    };
    status_label(code)
}

/// Translated label for a status code. The msgids match `locales.py`'s
/// `StringKey` values so the existing `.mo` catalogs resolve them.
pub(crate) fn status_label(s: StatusCode) -> String {
    use StatusCode::*;
    let msgid = match s {
        Starting => "Starting download...",
        Downloading => "Downloading...",
        Processing => "Processing...",
        Merging => "Merging...",
        Extracting => "Extracting Audio...",
        Completed => "Completed",
        Cancelled => "Cancelled",
        Resuming => "Resuming download...",
        Scheduled => "Scheduled",
        Queued => "Queued",
        FfmpegMissingMetadata => "FFmpeg not found. Skipping metadata.",
        FfmpegMissingSubtitles => "FFmpeg not found. Skipping subtitles.",
        DiskSpaceError => "Not enough disk space!",
        Timeout => "Timeout",
        NetworkError => "Network Error!",
        DrmError => "Content is DRM Protected!",
        PrivateError => "Video is Private!",
        FfmpegError => "FFmpeg Error - Missing or incompatible!",
        BotBlocked => "Blocked by YouTube — enable cookies in Settings",
        UnknownError => "Unknown Error!",
    };
    tr(msgid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary() -> MediaSummary {
        MediaSummary::default()
    }

    #[test]
    fn media_kind_video_with_ext() {
        let mut s = summary();
        s.vcodec = "avc1".into();
        // tr() with no catalog loaded returns the English source string.
        assert_eq!(media_kind_label(&s, "/x/clip.mp4"), "Video MP4");
    }

    #[test]
    fn media_kind_audio_and_empty() {
        let mut s = summary();
        s.acodec = "mp4a".into();
        assert_eq!(media_kind_label(&s, "/x/song.mp3"), "Audio MP3");
        // No codecs, no extension → empty.
        assert_eq!(media_kind_label(&summary(), "/x/noext"), "");
    }

    #[test]
    fn summary_joins_known_parts_only() {
        let mut s = summary();
        s.vcodec = "avc1.640028".into();
        s.width = 1920;
        s.height = 1080;
        s.acodec = "mp4a.40.2".into();
        s.sample_rate = 48000;
        let out = media_summary_text(&s, "/x/clip.mp4");
        assert!(out.starts_with("Video MP4 · H.264 · 1920×1080 · AAC · 48 kHz"));
        // A bare summary (no streams) falls back to just the container from the
        // path — no stray separators.
        assert_eq!(media_summary_text(&summary(), "/x/clip.mp4"), "MP4");
    }

    #[test]
    fn location_uses_parent_dir() {
        assert_eq!(
            location_label("/home/u/Downloads/x.mp4"),
            "Location: /home/u/Downloads"
        );
        // No parent → the path itself.
        assert_eq!(location_label("bare.mp4"), "Location: bare.mp4");
    }

    #[test]
    fn history_status_maps_known_and_unknown() {
        assert_eq!(history_status_label("completed"), "Completed");
        assert_eq!(history_status_label("paused"), "Cancelled");
        assert_eq!(history_status_label("error"), "Unknown Error!");
        // Anything unrecognised falls back to Queued.
        assert_eq!(history_status_label("weird"), "Queued");
    }
}
