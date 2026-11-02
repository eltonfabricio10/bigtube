//! Helpers for safe, useful error details and short-lived diagnostic logs.

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use once_cell::sync::Lazy;
use regex::Regex;

pub const LOG_FILE_NAME: &str = "bigtube.log";
pub const LOG_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
pub const LOG_MAX_SIZE: u64 = 5 * 1024 * 1024;
const MAX_DETAIL_CHARS: usize = 1600;

static URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)https?://[^\s"'<>]+"#).expect("valid URL regex"));
static SECRET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)(--cookies(?:-from-browser)?(?:\s+|=)|(?:token|password|secret|signature|api[_-]?key|access[_-]?key)\s*[:=]\s*)([^\s,;]+)"#,
    )
    .expect("valid secret regex")
});
static AUTH_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(cookie|set-cookie|authorization|proxy-authorization)\s*:\s*[^\r\n]*"#)
        .expect("valid header regex")
});
static BEARER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/-]+=*").expect("valid bearer regex"));

/// Remove common credentials and signed URLs before displaying or persisting
/// diagnostic text. The surrounding error message remains intact.
pub fn sanitize_error_detail(input: &str) -> String {
    let without_headers = AUTH_HEADER_RE.replace_all(input, "$1: [REDACTED]");
    let without_urls = URL_RE.replace_all(&without_headers, "[URL redacted]");
    let without_secrets = SECRET_RE.replace_all(&without_urls, "$1[REDACTED]");
    let without_bearer = BEARER_RE.replace_all(&without_secrets, "$1[REDACTED]");
    truncate_chars(&without_bearer, MAX_DETAIL_CHARS)
}

/// Return a bounded, sanitized tail of subprocess output, where the final
/// lines are most likely to contain the actionable cause.
pub fn summarize_error_detail(input: &str) -> String {
    let lines = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .take(8)
        .collect::<Vec<_>>();
    let detail = lines.into_iter().rev().collect::<Vec<_>>().join("\n");
    sanitize_error_detail(&detail)
}

pub fn is_youtube_block_error(input: &str) -> bool {
    let input = input.to_ascii_lowercase();
    [
        "confirm you're not a bot",
        "confirm you are not a bot",
        "sign in to confirm",
        "not a bot",
        "login_required",
        "login required",
        "too many requests",
        "http error 401",
        "http error 403",
        "403 forbidden",
        "http error 429",
        "sign in to view",
        "members-only",
        "members only",
        "age-restricted",
        "age restricted",
        "video is unavailable",
        "video unavailable",
        "not available in your country",
        "geo-restricted",
        "geo restricted",
    ]
    .iter()
    .any(|pattern| input.contains(pattern))
}

pub fn is_network_error(input: &str) -> bool {
    let input = input.to_ascii_lowercase();
    [
        "unable to download",
        "connection",
        "timed out",
        "timeout",
        "network is unreachable",
        "temporary failure in name resolution",
        "could not resolve",
        "name or service not known",
        "dns",
        "ssl:",
        "tls",
        "http error 5",
        "http error 429",
    ]
    .iter()
    .any(|pattern| input.contains(pattern))
}

/// Delete a log once it has reached its retention age. Missing files are
/// already clean; other filesystem failures are returned to the caller.
pub fn remove_expired_log(path: &Path, now: SystemTime) -> io::Result<bool> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let created = metadata.created().or_else(|_| metadata.modified())?;
    remove_log_if_expired(path, created, now)
}

fn remove_log_if_expired(path: &Path, created: SystemTime, now: SystemTime) -> io::Result<bool> {
    let expired = is_expired(created, now);
    if expired {
        std::fs::remove_file(path)?;
    }
    Ok(expired)
}

fn is_expired(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .map(|age| age >= LOG_MAX_AGE)
        .unwrap_or(false)
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_redacts_credentials_and_urls() {
        let safe = sanitize_error_detail(
            "GET https://example.test/path?token=abc\nAuthorization: Bearer secret\n--cookies /home/me/cookies.txt",
        );
        assert!(safe.contains("[URL redacted]"));
        assert!(safe.contains("Authorization: [REDACTED]"));
        let cookie_arg = sanitize_error_detail("--cookies /home/me/cookies.txt");
        assert!(cookie_arg.contains("--cookies [REDACTED]"));
        assert!(!safe.contains("abc"));
        assert!(!safe.contains("secret"));
        assert!(!safe.contains("cookies.txt"));
    }

    #[test]
    fn subprocess_summary_keeps_the_actionable_tail_and_is_bounded() {
        let raw = format!("{}\nconnection reset by peer", "old line\n".repeat(12));
        let summary = summarize_error_detail(&raw);
        assert!(summary.contains("connection reset by peer"));
        assert!(summary.chars().count() <= MAX_DETAIL_CHARS);
    }

    #[test]
    fn identifies_youtube_access_blocks() {
        assert!(is_youtube_block_error(
            "ERROR: Sign in to confirm you're not a bot"
        ));
        assert!(is_youtube_block_error("HTTP Error 429: Too Many Requests"));
        assert!(!is_youtube_block_error("This is a private video"));
        assert!(!is_youtube_block_error("Connection timed out"));
        assert!(is_network_error("Connection timed out"));
        assert!(!is_network_error("Sign in to confirm you're not a bot"));
    }

    #[test]
    fn log_expiry_uses_a_24_hour_window() {
        let now = SystemTime::now();
        assert!(is_expired(now - LOG_MAX_AGE - Duration::from_secs(1), now));
        assert!(!is_expired(now - LOG_MAX_AGE + Duration::from_secs(1), now));
        assert!(!is_expired(now + Duration::from_secs(1), now));
    }

    #[test]
    fn expired_log_cleanup_removes_only_the_target_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE_NAME);
        let other = dir.path().join("config.json");
        std::fs::write(&path, "diagnostics").unwrap();
        std::fs::write(&other, "{}").unwrap();
        let now = SystemTime::now();

        assert!(
            remove_log_if_expired(&path, now - LOG_MAX_AGE - Duration::from_secs(1), now).unwrap()
        );
        assert!(!path.exists());
        assert!(other.exists());
    }
}
