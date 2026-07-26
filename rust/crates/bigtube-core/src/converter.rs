//! Media conversion via ffmpeg. Ported from `core/converter.py`.
//!
//! Progress is reported through a callback `(progress, speed, eta)`; the UI
//! marshals it to the main thread (Python used `GLib.idle_add`). Cancellation is
//! cooperative via a shared `AtomicBool`.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use once_cell::sync::Lazy;
use wait_timeout::ChildExt;

use crate::config;
use crate::errors::BigTubeError;
use crate::process::{new_process_group, run_with_timeout, terminate_group};
use crate::util::{lock, which};
use crate::Result;

const FFPROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// Process environment snapshot for spawning ffprobe — captured once instead of
/// re-collecting `std::env::vars()` on every probe call.
static PROBE_ENV: Lazy<HashMap<String, String>> = Lazy::new(|| std::env::vars().collect());

/// `(progress 0..1, speed, eta_seconds)` — speed/eta may be `None`.
pub type ConvertProgressFn = Arc<dyn Fn(f64, Option<f64>, Option<f64>) + Send + Sync>;

/// True if both ffmpeg and ffprobe are on `$PATH` (`check_ffmpeg`).
pub fn check_ffmpeg() -> bool {
    which("ffmpeg").is_some() && which("ffprobe").is_some()
}

/// Media duration in seconds via ffprobe (`get_media_duration`); 0.0 on failure.
pub fn get_media_duration(input_path: &str) -> f64 {
    let args = [
        "-v".to_string(),
        "error".to_string(),
        "-show_entries".to_string(),
        "format=duration".to_string(),
        "-of".to_string(),
        "default=noprint_wrappers=1:nokey=1".to_string(),
        input_path.to_string(),
    ];
    match run_with_timeout("ffprobe", &args, &PROBE_ENV, FFPROBE_TIMEOUT) {
        Ok((0, stdout, _)) => {
            let s = stdout.trim();
            if s.is_empty() || s == "N/A" {
                0.0
            } else {
                s.parse().unwrap_or(0.0)
            }
        }
        _ => 0.0,
    }
}

/// Real codecs + on-disk size of a finished media file (`probe_media_summary`).
/// Codecs come from ffprobe (empty if ffprobe is missing or the stream is
/// absent); `size_bytes` is always the real file size from the filesystem.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaSummary {
    pub vcodec: String,
    pub acodec: String,
    pub size_bytes: u64,
    // Video resolution (0 when not a video / unknown).
    pub width: u32,
    pub height: u32,
    // Audio sample rate in Hz (0 when unknown).
    pub sample_rate: u32,
}

/// Parse ffprobe `-of json -show_entries stream=codec_type,codec_name` output
/// into `(vcodec, acodec)`. Pure/testable.
pub fn parse_ffprobe_streams(json: &str) -> (String, String) {
    let m = parse_ffprobe_meta(json);
    (m.vcodec, m.acodec)
}

/// True when a "video" stream is really an embedded still image (cover art),
/// not a playable video track. Checks ffprobe's `disposition.attached_pic`
/// flag first, then falls back to the codec name (still-image codecs).
fn is_cover_art(stream: &serde_json::Value, codec_name: &str) -> bool {
    let attached = stream
        .get("disposition")
        .and_then(|d| d.get("attached_pic"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        == 1;
    attached || matches!(codec_name, "mjpeg" | "png" | "bmp" | "gif" | "webp")
}

/// Parse ffprobe stream JSON into codecs + resolution + sample rate. Pure.
pub fn parse_ffprobe_meta(json: &str) -> MediaSummary {
    let mut m = MediaSummary::default();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(streams) = val.get("streams").and_then(|s| s.as_array()) {
            for s in streams {
                let kind = s.get("codec_type").and_then(|x| x.as_str()).unwrap_or("");
                let name = s
                    .get("codec_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let num = |k: &str| -> u32 {
                    s.get(k)
                        .and_then(|x| {
                            x.as_u64()
                                .or_else(|| x.as_str().and_then(|t| t.parse().ok()))
                        })
                        .unwrap_or(0) as u32
                };
                match kind {
                    // A "video" stream that is actually embedded cover art (album
                    // thumbnail) must NOT make an audio file look like a video.
                    // ffprobe flags it with disposition.attached_pic=1; image
                    // codecs (mjpeg/png/…) are the same thing as a fallback.
                    "video" if is_cover_art(s, &name) => {}
                    "video" if m.vcodec.is_empty() => {
                        m.vcodec = name;
                        m.width = num("width");
                        m.height = num("height");
                    }
                    "audio" if m.acodec.is_empty() => {
                        m.acodec = name;
                        m.sample_rate = num("sample_rate");
                    }
                    _ => {}
                }
            }
        }
    }
    m
}

/// Inspect a finished file: real on-disk size (always) + video/audio codec names
/// via ffprobe (best-effort). Used to show "H.264 · AAC · 57.9 MiB" on a row once
/// a download/conversion completes.
pub fn probe_media_summary(path: &str) -> MediaSummary {
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut summary = MediaSummary {
        size_bytes,
        ..Default::default()
    };
    if which("ffprobe").is_none() {
        return summary;
    }
    let args = [
        "-v".to_string(),
        "error".to_string(),
        "-show_entries".to_string(),
        "stream=codec_type,codec_name,width,height,sample_rate:stream_disposition=attached_pic"
            .to_string(),
        "-of".to_string(),
        "json".to_string(),
        path.to_string(),
    ];
    if let Ok((0, stdout, _)) = run_with_timeout("ffprobe", &args, &PROBE_ENV, FFPROBE_TIMEOUT) {
        let m = parse_ffprobe_meta(&stdout);
        summary.vcodec = m.vcodec;
        summary.acodec = m.acodec;
        summary.width = m.width;
        summary.height = m.height;
        summary.sample_rate = m.sample_rate;
    }
    summary
}

/// Prefix of the hidden temp file a conversion writes to before the final
/// rename. Keeping the real extension last lets ffmpeg infer the muxer, and
/// the distinctive name makes leftovers from a crash safe to sweep on startup
/// (`cleanup_conv_temps`) without ever touching user files.
const CONV_TMP_PREFIX: &str = ".bigtube-conv.";

/// The temp path a conversion to `output` writes to (same directory, so the
/// final rename is atomic).
fn conv_temp_path(output: &Path) -> std::path::PathBuf {
    let name = output
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    output.with_file_name(format!("{CONV_TMP_PREFIX}{name}"))
}

/// Remove conversion temp files left in `dir` by a crash or hard kill (a clean
/// cancel/failure already deletes its own temp). Returns how many were removed.
pub fn cleanup_conv_temps(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(CONV_TMP_PREFIX)
            && entry.file_type().map(|t| t.is_file()).unwrap_or(false)
            && std::fs::remove_file(entry.path()).is_ok()
        {
            tracing::info!("removed stale conversion temp: {:?}", entry.path());
            removed += 1;
        }
    }
    removed
}

/// True when the two paths refer to the same existing file (canonical compare,
/// falling back to a literal compare when either does not exist).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Resolve the output directory and path. With `overwrite`, returns the natural
/// `{base}.{ext}` path (replacing any existing file — including the input
/// itself for a same-format conversion, which the GUI confirms via its
/// "File already exists" dialog); otherwise appends " (n)" to avoid colliding
/// with an existing file. Overwriting the source is safe ONLY because ffmpeg
/// writes to a hidden temp that replaces the final name after success
/// (`conv_temp_path`) — the original is never touched on failure or cancel.
fn resolve_output_path(input_path: &str, output_format: &str, overwrite: bool) -> String {
    let input = Path::new(input_path);
    let cfg = config::global().read().unwrap_or_else(|e| e.into_inner());
    let use_source = cfg.get_bool("use_source_folder");

    let dir = if use_source {
        input.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        let conv = cfg.get_string("converter_path");
        let conv_path = Path::new(&conv);
        // Fallback to source dir if unset or parent doesn't exist.
        if conv.is_empty() || !conv_path.parent().map(Path::exists).unwrap_or(false) {
            input.parent().map(Path::to_path_buf).unwrap_or_default()
        } else {
            let _ = std::fs::create_dir_all(conv_path);
            conv_path.to_path_buf()
        }
    };
    drop(cfg);

    dedupe_output(&dir, input, output_format, overwrite)
        .to_string_lossy()
        .into_owned()
}

/// Pick `{base}.{ext}` inside `dir`, appending " (n)" when needed (see
/// `resolve_output_path` for the rules). Pure given the filesystem — testable.
fn dedupe_output(
    dir: &Path,
    input: &Path,
    output_format: &str,
    overwrite: bool,
) -> std::path::PathBuf {
    let base = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let mut output = dir.join(format!("{base}.{output_format}"));
    if !overwrite {
        let mut counter = 1;
        while output.exists() {
            output = dir.join(format!("{base} ({counter}).{output_format}"));
            counter += 1;
        }
    }
    output
}

/// The natural output path a conversion would write to (before any " (n)"
/// de-duplication). The GUI checks this to decide whether to prompt about
/// overwriting an existing file.
pub fn planned_output_path(input_path: &str, output_format: &str) -> String {
    resolve_output_path(input_path, output_format, true)
}

/// One sidecar subtitle next to the conversion input.
#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleTrack {
    pub path: String,
    /// Language code parsed from the filename suffix (`video.pt-BR.srt` →
    /// `pt-BR`); `None` for a bare `video.srt`.
    pub lang: Option<String>,
}

impl SubtitleTrack {
    /// Short label for the UI: the language code, or the file extension for an
    /// unlabelled track ("SRT").
    pub fn label(&self) -> String {
        match &self.lang {
            Some(l) => l.clone(),
            None => Path::new(&self.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("sub")
                .to_uppercase(),
        }
    }
}

/// Map a filename language code to the ISO 639-2 code ffmpeg expects in stream
/// `language` metadata. Unknown codes are passed through — players still show
/// them, just without the canonical three-letter form.
fn iso639_2(code: &str) -> String {
    let base = code.split(['-', '_']).next().unwrap_or(code).to_lowercase();
    match base.as_str() {
        "en" => "eng",
        "pt" => "por",
        "es" => "spa",
        "fr" => "fra",
        "de" => "deu",
        "it" => "ita",
        "nl" => "nld",
        "pl" => "pol",
        "ru" => "rus",
        "ja" => "jpn",
        "ko" => "kor",
        "zh" => "zho",
        "ar" => "ara",
        "hi" => "hin",
        "tr" => "tur",
        "cs" => "ces",
        "sk" => "slk",
        "hu" => "hun",
        "ro" => "ron",
        "sv" => "swe",
        other => other,
    }
    .to_string()
}

/// Build the ffmpeg argument list (pure, testable). Each entry in `subs` is a
/// sidecar subtitle mapped as its own stream, tagged with its language.
fn build_ffmpeg_args(
    input_path: &str,
    output_path: &str,
    output_format: &str,
    subs: &[SubtitleTrack],
    add_metadata: bool,
) -> Vec<String> {
    let mut cmd = vec!["-i".to_string(), input_path.to_string()];
    for sub in subs {
        cmd.push("-i".into());
        cmd.push(sub.path.clone());
    }
    cmd.push("-y".into());
    if !subs.is_empty() {
        cmd.extend(["-map".into(), "0:v?".into(), "-map".into(), "0:a?".into()]);
        for i in 1..=subs.len() {
            cmd.push("-map".into());
            cmd.push(format!("{i}:s?"));
        }
        if output_format.to_lowercase() == "mp4" {
            cmd.extend(["-c:s".into(), "mov_text".into()]);
        } else {
            cmd.extend(["-c:s".into(), "copy".into()]);
        }
        for (n, sub) in subs.iter().enumerate() {
            if let Some(lang) = &sub.lang {
                cmd.push(format!("-metadata:s:s:{n}"));
                cmd.push(format!("language={}", iso639_2(lang)));
            }
        }
    }
    if add_metadata {
        cmd.extend(["-map_metadata".into(), "0".into()]);
    }
    cmd.extend(["-progress".into(), "pipe:1".into(), "-nostats".into()]);
    cmd.push(output_path.to_string());
    cmd
}

/// Find every sidecar subtitle (.srt/.vtt/.ass) next to the input: the bare
/// `stem.ext` form plus language-suffixed `stem.LANG.ext` variants
/// (`video.pt-BR.srt`). Sorted with the bare track first, then by language.
pub fn find_subtitles(input_path: &str) -> Vec<SubtitleTrack> {
    let input = Path::new(input_path);
    let (Some(stem), Some(dir)) = (input.file_stem().and_then(|s| s.to_str()), input.parent())
    else {
        return Vec::new();
    };
    let mut tracks = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let name_os = entry.file_name();
        let Some(name) = name_os.to_str() else {
            continue;
        };
        let Some(ext) = name.rsplit('.').next() else {
            continue;
        };
        if !matches!(ext.to_lowercase().as_str(), "srt" | "vtt" | "ass") {
            continue;
        }
        let base = &name[..name.len() - ext.len() - 1];
        let lang = if base == stem {
            None
        } else if let Some(rest) = base.strip_prefix(stem).and_then(|r| r.strip_prefix('.')) {
            // A language-ish suffix: short, alphanumeric with - or _ ("pt-BR").
            if !rest.is_empty()
                && rest.len() <= 10
                && rest
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                Some(rest.to_string())
            } else {
                continue; // e.g. a different video sharing the prefix
            }
        } else {
            continue;
        };
        tracks.push(SubtitleTrack {
            path: entry.path().to_string_lossy().into_owned(),
            lang,
        });
    }
    // Deterministic order: the bare track first, then by language code.
    tracks.sort_by(|a, b| a.lang.cmp(&b.lang));
    tracks
}

/// Convert `input_path` to `output_format` (`convert_media`). Returns the output
/// path. Blocking; run off the UI thread. `subtitles` are the sidecar tracks to
/// embed (the GUI collects the user's selection via [`find_subtitles`]).
#[allow(clippy::too_many_arguments)]
pub fn convert_media(
    input_path: &str,
    output_format: &str,
    progress: Option<&ConvertProgressFn>,
    add_metadata: bool,
    subtitles: &[SubtitleTrack],
    cancel: Option<&Arc<AtomicBool>>,
    overwrite: bool,
) -> Result<String> {
    if !Path::new(input_path).exists() {
        return Err(BigTubeError::Config(format!(
            "Input file not found: {input_path}"
        )));
    }

    let output_path = resolve_output_path(input_path, output_format, overwrite);
    // ffmpeg writes to a hidden temp; only a successful conversion renames it
    // to the final name. A cancel, failure, quit, or crash therefore never
    // leaves a partial file that looks like a finished conversion.
    let tmp_path = conv_temp_path(Path::new(&output_path));
    let tmp_str = tmp_path.to_string_lossy().into_owned();
    let duration = get_media_duration(input_path);
    let args = build_ffmpeg_args(input_path, &tmp_str, output_format, subtitles, add_metadata);

    tracing::info!("Starting conversion: {input_path} -> {output_path}");

    let mut cmd = Command::new("ffmpeg");
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    new_process_group(&mut cmd);
    let mut child = cmd.spawn()?;
    let pid = child.id();

    let (tx, rx) = mpsc::channel::<String>();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(out)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    // ffmpeg writes its stats/warnings to stderr; if nobody reads it the pipe
    // buffer fills, ffmpeg blocks on write and never exits — deadlocking the
    // wait below. Drain it on its own thread, keeping the last lines for a
    // useful error message on failure. Keep the handle so we can join it after
    // the child exits: otherwise the failure path could read the tail before
    // the drain thread has pushed ffmpeg's final (most diagnostic) lines.
    let stderr_tail: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let stderr_handle = child.stderr.take().map(|err| {
        let tail = stderr_tail.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(err)
                .lines()
                .map_while(std::result::Result::ok)
            {
                let mut t = lock(&tail);
                if t.len() == 20 {
                    t.pop_front();
                }
                t.push_back(line);
            }
        })
    });

    let cancelled = || cancel.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false);
    let mut us: f64 = 0.0;
    let mut user_cancelled = false;

    loop {
        if cancelled() {
            terminate_group(pid, Duration::from_secs(2));
            user_cancelled = true;
            break;
        }
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => parse_progress_line(&line, duration, &mut us, progress),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Bound the final reap so a wedged ffmpeg can never hang the worker thread.
    let status = match child.wait_timeout(Duration::from_secs(10)) {
        Ok(Some(s)) => Some(s),
        _ => {
            terminate_group(pid, Duration::from_secs(2));
            child.wait().ok()
        }
    };

    // The child is now dead, so its stderr write-end is closed and the drain
    // thread will finish; join it so the tail below is complete.
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }

    // Belt-and-suspenders: the cleanup below must never touch the source file
    // (the temp name can't collide with the input, but deleting a user's
    // original is unrecoverable — keep the guard local too).
    let cleanup_output = || {
        if !same_file(&tmp_path, Path::new(input_path)) {
            let _ = std::fs::remove_file(&tmp_path);
        }
    };

    if user_cancelled || cancelled() {
        cleanup_output();
        return Err(BigTubeError::Config("Conversion cancelled by user".into()));
    }
    match status.and_then(|s| s.code()) {
        Some(0) => {
            // Success: move the temp into place (same dir → atomic). Replaces
            // an existing file only in the overwrite case, matching `-y`.
            if let Err(e) = std::fs::rename(&tmp_path, &output_path) {
                cleanup_output();
                return Err(BigTubeError::Config(format!(
                    "Conversion finished but the output could not be moved into place: {e}"
                )));
            }
            if let Some(cb) = progress {
                cb(1.0, Some(0.0), Some(0.0));
            }
            Ok(output_path)
        }
        other => {
            terminate_group(pid, Duration::from_secs(2));
            cleanup_output();
            let tail = lock(&stderr_tail)
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            tracing::warn!("ffmpeg failed (code {other:?}):\n{tail}");
            Err(BigTubeError::Config(format!(
                "Conversion failed with code {other:?}"
            )))
        }
    }
}

fn parse_progress_line(
    line: &str,
    duration: f64,
    us: &mut f64,
    progress: Option<&ConvertProgressFn>,
) {
    if let Some(rest) = line.split_once("out_time_us=") {
        if let Ok(v) = rest.1.trim().parse::<f64>() {
            *us = v;
            if duration > 0.0 {
                let p = (*us / (duration * 1_000_000.0)).min(0.99);
                if let Some(cb) = progress {
                    cb(p, None, None);
                }
            }
        }
    } else if let Some(rest) = line.split_once("speed=") {
        let s = rest.1.trim().trim_end_matches('x');
        let speed = if s.is_empty() || s == "N/A" {
            0.0
        } else {
            s.parse().unwrap_or(0.0)
        };
        if speed > 0.0 && duration > 0.0 && *us > 0.0 {
            let frac = *us / (duration * 1_000_000.0);
            let remaining = duration * (1.0 - frac);
            let eta = remaining / speed;
            if let Some(cb) = progress {
                cb(frac.min(0.99), Some(speed), Some(eta));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conv_temp_lives_next_to_output_and_sweep_only_removes_temps() {
        let out = Path::new("/dir/video (1).mp4");
        let tmp = conv_temp_path(out);
        assert_eq!(tmp, Path::new("/dir/.bigtube-conv.video (1).mp4"));

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".bigtube-conv.a.mp4"), b"partial").unwrap();
        std::fs::write(dir.path().join(".bigtube-conv.b.webm"), b"partial").unwrap();
        std::fs::write(dir.path().join("a.mp4"), b"user file").unwrap();
        std::fs::write(dir.path().join(".hidden"), b"user file").unwrap();
        assert_eq!(cleanup_conv_temps(dir.path()), 2);
        assert!(dir.path().join("a.mp4").exists());
        assert!(dir.path().join(".hidden").exists());
        assert!(!dir.path().join(".bigtube-conv.a.mp4").exists());
        // Nonexistent dir: no panic, nothing removed.
        assert_eq!(cleanup_conv_temps(&dir.path().join("nope")), 0);
    }

    #[test]
    fn same_format_overwrite_replaces_keep_both_dedupes() {
        // Same-format conversion into the source folder: the natural path IS
        // the input. With overwrite (user confirmed "Overwrite" in the GUI's
        // exists-dialog) it resolves to the input — safe because ffmpeg writes
        // to the hidden temp and only a successful rename replaces the final
        // name. With "Keep Both" (no overwrite) it dedupes to " (1)".
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("video.mp4");
        std::fs::write(&input, b"x").unwrap();

        assert_eq!(dedupe_output(dir.path(), &input, "mp4", true), input);
        let kept = dedupe_output(dir.path(), &input, "mp4", false);
        assert!(
            kept.to_string_lossy().ends_with(" (1).mp4"),
            "got: {}",
            kept.display()
        );

        // Different format still overwrites its own previous output normally.
        let prev = dir.path().join("video.webm");
        std::fs::write(&prev, b"x").unwrap();
        assert_eq!(dedupe_output(dir.path(), &input, "webm", true), prev);

        // The temp the conversion actually writes to is never the input, so
        // the failure/cancel cleanup can't touch the original.
        assert!(!same_file(&conv_temp_path(&input), &input));
    }

    #[test]
    fn parse_ffprobe_streams_picks_first_video_and_audio() {
        let json = r#"{"streams":[
            {"codec_type":"video","codec_name":"h264"},
            {"codec_type":"audio","codec_name":"aac"},
            {"codec_type":"audio","codec_name":"opus"}
        ]}"#;
        assert_eq!(
            parse_ffprobe_streams(json),
            ("h264".to_string(), "aac".to_string())
        );
        // Audio-only file -> empty video codec.
        let audio = r#"{"streams":[{"codec_type":"audio","codec_name":"mp3"}]}"#;
        assert_eq!(
            parse_ffprobe_streams(audio),
            (String::new(), "mp3".to_string())
        );
        // Garbage -> empty, no panic.
        assert_eq!(
            parse_ffprobe_streams("not json"),
            (String::new(), String::new())
        );
    }

    #[test]
    fn embedded_cover_art_does_not_count_as_video() {
        // mp3 with an embedded album cover: ffprobe lists a mjpeg/png video
        // stream flagged attached_pic. It must be ignored so the file reads as
        // pure audio (otherwise the status shows "PNG"/"Video").
        let json = r#"{"streams":[
            {"codec_type":"video","codec_name":"png","disposition":{"attached_pic":1}},
            {"codec_type":"audio","codec_name":"mp3"}
        ]}"#;
        let (v, a) = parse_ffprobe_streams(json);
        assert_eq!(v, "");
        assert_eq!(a, "mp3");
        // Even without the disposition flag, a still-image codec is cover art.
        let json2 = r#"{"streams":[
            {"codec_type":"video","codec_name":"mjpeg"},
            {"codec_type":"audio","codec_name":"flac"}
        ]}"#;
        assert_eq!(parse_ffprobe_streams(json2), (String::new(), "flac".into()));
        // A genuine video track is still detected.
        let real = r#"{"streams":[{"codec_type":"video","codec_name":"h264"}]}"#;
        assert_eq!(parse_ffprobe_meta(real).vcodec, "h264");
    }

    #[test]
    fn ffmpeg_args_with_subtitles_mp4_use_mov_text() {
        let subs = [SubtitleTrack {
            path: "/in.srt".into(),
            lang: None,
        }];
        let args = build_ffmpeg_args("/in.mkv", "/out.mp4", "mp4", &subs, true);
        assert!(args
            .windows(2)
            .any(|w| w[0] == "-c:s" && w[1] == "mov_text"));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "-map_metadata" && w[1] == "0"));
        assert!(args.contains(&"-progress".to_string()));
        assert_eq!(args.last().unwrap(), "/out.mp4");
    }

    #[test]
    fn ffmpeg_args_map_every_subtitle_with_language() {
        let subs = [
            SubtitleTrack {
                path: "/v.srt".into(),
                lang: None,
            },
            SubtitleTrack {
                path: "/v.en.srt".into(),
                lang: Some("en".into()),
            },
            SubtitleTrack {
                path: "/v.pt-BR.vtt".into(),
                lang: Some("pt-BR".into()),
            },
        ];
        let args = build_ffmpeg_args("/v.mkv", "/out.mkv", "mkv", &subs, false);
        // Three subtitle inputs, each mapped as its own stream.
        assert_eq!(args.iter().filter(|a| *a == "-i").count(), 4); // video + 3 subs
        for m in ["1:s?", "2:s?", "3:s?"] {
            assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == m));
        }
        // Language metadata per labelled track (ISO 639-2), none for the bare one.
        assert!(args
            .windows(2)
            .any(|w| w[0] == "-metadata:s:s:1" && w[1] == "language=eng"));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "-metadata:s:s:2" && w[1] == "language=por"));
        assert!(!args.iter().any(|a| a == "-metadata:s:s:0"));
    }

    #[test]
    fn find_subtitles_collects_and_labels_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("video.mp4");
        std::fs::write(&input, b"x").unwrap();
        std::fs::write(dir.path().join("video.srt"), b"s").unwrap();
        std::fs::write(dir.path().join("video.en.srt"), b"s").unwrap();
        std::fs::write(dir.path().join("video.pt-BR.vtt"), b"s").unwrap();
        // Must NOT match: another video sharing the prefix, and its subtitle.
        std::fs::write(dir.path().join("video. the sequel.mp4"), b"x").unwrap();
        std::fs::write(dir.path().join("video. the sequel.srt"), b"s").unwrap();

        let tracks = find_subtitles(&input.to_string_lossy());
        let labels: Vec<String> = tracks.iter().map(SubtitleTrack::label).collect();
        assert_eq!(labels, ["SRT", "en", "pt-BR"]);
        assert!(tracks
            .iter()
            .all(|t| std::path::Path::new(&t.path).exists()));
    }

    #[test]
    fn ffmpeg_args_non_mp4_subtitles_copy() {
        let subs = [SubtitleTrack {
            path: "/in.srt".into(),
            lang: None,
        }];
        let args = build_ffmpeg_args("/in.mp4", "/out.mkv", "mkv", &subs, false);
        assert!(args.windows(2).any(|w| w[0] == "-c:s" && w[1] == "copy"));
        assert!(!args.contains(&"-map_metadata".to_string()));
    }

    #[test]
    fn progress_parsing_emits_fraction() {
        let captured = Arc::new(std::sync::Mutex::new(Vec::<f64>::new()));
        let c2 = captured.clone();
        let cb: ConvertProgressFn = Arc::new(move |p, _s, _e| c2.lock().unwrap().push(p));
        let mut us = 0.0;
        parse_progress_line("out_time_us=5000000", 10.0, &mut us, Some(&cb));
        assert!((captured.lock().unwrap()[0] - 0.5).abs() < 1e-6);
    }
}
