use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use bigtube_core::{config, diagnostics, paths};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

const LOG_CHECK_INTERVAL: Duration = Duration::from_secs(60);
static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());
static SAVE_LOG_TO_FILE: AtomicBool = AtomicBool::new(false);

pub fn init() {
    set_enabled(
        config::global()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get_bool("save_error_log"),
    );
    let log_path = paths::config_dir().join(diagnostics::LOG_FILE_NAME);
    remove_expired(&log_path);
    start_retention_worker(log_path.clone());

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(env_filter),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(FileLogMakeWriter { log_path })
                .with_filter(tracing_subscriber::filter::LevelFilter::WARN),
        )
        .init();
}

pub fn set_enabled(enabled: bool) {
    SAVE_LOG_TO_FILE.store(enabled, Ordering::Relaxed);
}

#[derive(Clone)]
struct FileLogMakeWriter {
    log_path: PathBuf,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileLogMakeWriter {
    type Writer = FileLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FileLogWriter {
            log_path: self.log_path.clone(),
            enabled: SAVE_LOG_TO_FILE.load(Ordering::Relaxed),
            buffer: Vec::new(),
        }
    }
}

struct FileLogWriter {
    log_path: PathBuf,
    enabled: bool,
    buffer: Vec<u8>,
}

impl Write for FileLogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.enabled {
            self.buffer.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.commit()
    }
}

impl FileLogWriter {
    fn commit(&mut self) -> io::Result<()> {
        if !self.enabled || self.buffer.is_empty() {
            return Ok(());
        }
        self.enabled = false;
        let _guard = LOG_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let raw = String::from_utf8_lossy(&self.buffer);
        let safe = diagnostics::sanitize_error_detail(&raw);
        let mut bytes = safe.into_bytes();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }

        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        remove_expired(&self.log_path);
        let mut options = std::fs::OpenOptions::new();
        options.create(true).append(true);
        let file = options.open(&self.log_path)?;
        if file.metadata()?.len().saturating_add(bytes.len() as u64) > diagnostics::LOG_MAX_SIZE {
            drop(file);
            std::fs::remove_file(&self.log_path)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        file.write_all(&bytes)?;
        file.flush()
    }
}

impl Drop for FileLogWriter {
    fn drop(&mut self) {
        if let Err(error) = self.commit() {
            eprintln!("Failed to write diagnostic log: {error}");
        }
    }
}

fn start_retention_worker(log_path: PathBuf) {
    let result = std::thread::Builder::new()
        .name("bigtube-log-retention".into())
        .spawn(move || loop {
            std::thread::sleep(LOG_CHECK_INTERVAL);
            let _guard = LOG_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            remove_expired(&log_path);
        });
    if let Err(error) = result {
        eprintln!("Failed to start diagnostic log retention worker: {error}");
    }
}

fn remove_expired(log_path: &std::path::Path) {
    if let Err(error) = diagnostics::remove_expired_log(log_path, SystemTime::now()) {
        eprintln!(
            "Failed to clean expired diagnostic log {}: {error}",
            log_path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_writer_sanitizes_and_persists_error_events() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "bigtube-log-writer-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(diagnostics::LOG_FILE_NAME);
        let mut writer = FileLogWriter {
            log_path: path.clone(),
            enabled: true,
            buffer: Vec::new(),
        };
        writer
            .write_all(b"ERROR request failed https://example.test/?token=private")
            .unwrap();
        writer.flush().unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("request failed"));
        assert!(written.contains("[URL redacted]"));
        assert!(!written.contains("private"));
        drop(writer);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn file_writer_does_not_create_a_log_when_disabled() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "bigtube-disabled-log-{}-{unique}",
            std::process::id()
        ));
        let path = dir.join(diagnostics::LOG_FILE_NAME);
        let mut writer = FileLogWriter {
            log_path: path.clone(),
            enabled: false,
            buffer: Vec::new(),
        };
        writer.write_all(b"error").unwrap();
        writer.flush().unwrap();
        assert!(!path.exists());
    }
}
