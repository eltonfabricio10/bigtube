//! BigTube headless CLI — the Rust port's first runnable milestone.
//!
//! Mirrors the Python app's headless mode (`bigtube -d <url> [-o DIR]
//! [--audio-only] [--format FMT]`). The GUI (Fases 2-6) is a separate binary.

use std::io::Write;
use std::sync::Arc;

use bigtube_core::config;
use bigtube_core::downloader::{DownloadParams, VideoDownloader};
use bigtube_core::enums::VideoQuality;
use bigtube_core::progress::{Progress, ProgressFn, StatusCode};
use bigtube_core::updater;
use clap::Parser;

/// Git-derived version when available, else the Cargo version.
const VERSION: &str = match option_env!("BIGTUBE_GIT_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Parser)]
#[command(name = "bigtube", version = VERSION, about = "Universal Multimedia Downloader (Rust)")]
struct Cli {
    /// Download a URL headlessly (no GUI)
    #[arg(short = 'd', long = "download", value_name = "URL")]
    download: Option<String>,

    /// Destination folder for --download
    #[arg(short = 'o', long = "output", value_name = "DIR")]
    output: Option<String>,

    /// With --download, extract audio as MP3
    #[arg(long = "audio-only")]
    audio_only: bool,

    /// With --download, custom yt-dlp format selector
    #[arg(long = "format", value_name = "FMT")]
    format: Option<String>,

    /// With --format, the output container/extension (default: mp4). Use e.g.
    /// m4a/opus/webm when the selector picks an audio-only format, so the file
    /// isn't mislabeled as .mp4.
    #[arg(long = "ext", value_name = "EXT")]
    ext: Option<String>,

    /// Print the bundled yt-dlp version and exit
    #[arg(long = "yt-dlp-version")]
    ytdlp_version: bool,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    if cli.ytdlp_version {
        let path = config::global()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .yt_dlp_path
            .clone();
        match updater::get_local_version(&path) {
            Some(v) => println!("yt-dlp {v}"),
            None => println!("yt-dlp not installed"),
        }
        return;
    }

    let Some(url) = cli.download else {
        eprintln!("No action. Use --download <URL> (the GUI is a separate binary).");
        std::process::exit(2);
    };

    std::process::exit(run_download(
        &url,
        cli.output,
        cli.audio_only,
        cli.format,
        cli.ext,
    ));
}

fn run_download(
    url: &str,
    output: Option<String>,
    audio_only: bool,
    format: Option<String>,
    ext: Option<String>,
) -> i32 {
    // Make sure yt-dlp exists (auto-download if missing), like Python's startup.
    {
        let (yt, deno) = {
            let c = config::global().read().unwrap_or_else(|e| e.into_inner());
            (c.yt_dlp_path.clone(), c.deno_path.clone())
        };
        if !yt.exists() {
            eprintln!("yt-dlp not found; downloading...");
            updater::ensure_exists(&yt, &deno);
        }
    }

    let downloader = match VideoDownloader::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    // Title for the output filename (best-effort).
    let title = downloader
        .fetch_video_info(url)
        .map(|info| info.title)
        .unwrap_or_else(|| "video".to_string());

    let (format_id, ext) = if audio_only {
        if format.is_some() {
            eprintln!("Warning: --audio-only takes precedence; --format is ignored.");
        }
        (VideoQuality::AudioMp3.as_value().to_string(), "mp3".into())
    } else if let Some(f) = format {
        // A custom selector may resolve to audio-only media; --ext lets the
        // user label the container correctly instead of a hardcoded .mp4.
        (f, ext.unwrap_or_else(|| "mp4".into()))
    } else {
        (VideoQuality::Best.as_value().to_string(), "mp4".into())
    };

    let params = build_params(url, title, format_id, ext, output);

    let progress: ProgressFn = Arc::new(print_progress);
    if downloader.start_download(params, &progress) {
        println!();
        println!("Done.");
        0
    } else {
        println!();
        eprintln!("Download failed.");
        1
    }
}

fn build_params(
    url: &str,
    title: String,
    format_id: String,
    ext: String,
    output: Option<String>,
) -> DownloadParams {
    DownloadParams {
        url: url.to_string(),
        format_id,
        title,
        ext,
        force_overwrite: false,
        estimated_size_mb: None,
        download_dir: output,
        subfolder: None,
        subtitles: None,
    }
}

fn print_progress(p: Progress) {
    let pct = p.percent.unwrap_or_default();
    let label = status_label(p.status);
    print!("\r{label}: {pct}        ");
    let _ = std::io::stdout().flush();
}

/// Minimal English labels for CLI output (the GUI uses the gettext catalog).
fn status_label(s: StatusCode) -> &'static str {
    use StatusCode::*;
    match s {
        Starting => "Starting",
        Downloading => "Downloading",
        Processing => "Processing",
        Merging => "Merging",
        Extracting => "Extracting audio",
        Completed => "Completed",
        Cancelled => "Cancelled",
        Resuming => "Resuming",
        Scheduled => "Scheduled",
        Queued => "Queued",
        FfmpegMissingMetadata => "ffmpeg missing (metadata skipped)",
        FfmpegMissingSubtitles => "ffmpeg missing (subtitles skipped)",
        DiskSpaceError => "Not enough disk space",
        Timeout => "Timed out",
        NetworkError => "Network error",
        DrmError => "DRM protected",
        PrivateError => "Private content",
        FfmpegError => "ffmpeg error",
        BotBlocked => "Blocked by YouTube (enable cookies)",
        UnknownError => "Unknown error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_stored_only_on_download_params() {
        let params = build_params(
            "https://example.com/video",
            "Video".into(),
            "best".into(),
            "mp4".into(),
            Some("/tmp/one-shot".into()),
        );
        assert_eq!(params.download_dir.as_deref(), Some("/tmp/one-shot"));
    }
}
