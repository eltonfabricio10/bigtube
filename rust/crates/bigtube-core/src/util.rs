//! Small shared helpers.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// Lock a `Mutex`, recovering from poisoning instead of panicking.
///
/// The stores guarded by these mutexes hold plain data (a `Vec`/`Option`), so a
/// poisoned lock — which can only happen if some other thread panicked while
/// holding the guard — leaves the data in a usable state. Recovering keeps a
/// single panic from cascading into every later access (relevant mostly in
/// debug/tests, since release builds use `panic = "abort"`).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Seconds since the Unix epoch as a float, matching Python's `time.time()`.
pub fn now_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// `shutil.which`: find an executable on `$PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}
