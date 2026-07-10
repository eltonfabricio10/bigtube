//! User favorites — a single persisted list of tracks the user has starred from
//! search results or downloads, playable as a queue.
//!
//! Kept deliberately simple: every mutation reads the current file, edits, and
//! writes it back atomically (via `json_store`). The list is small and only ever
//! changed by explicit user actions, so there's no debouncer or in-memory cache
//! to keep in sync — and always reading disk first means a concurrent import or
//! a second window can never be clobbered.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::json_store::{load_json, save_json};
use crate::util::now_epoch;

/// One starred track. Mirrors the fields a `QueueItem`/result row needs so the
/// favorites view can play the list without re-resolving metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FavoriteItem {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub uploader: String,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub is_video: bool,
    /// True for a downloaded local file (played directly); false for a remote
    /// URL that must be resolved via yt-dlp at play time.
    #[serde(default)]
    pub is_local: bool,
    /// Unix epoch (seconds) when added — oldest first, newest appended last.
    #[serde(default)]
    pub added: i64,
}

/// Persisted favorites list, addressed by a file path (one per app).
pub struct Favorites {
    path: PathBuf,
}

impl Favorites {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The whole list, in insertion order (oldest first, newest last).
    pub fn list(&self) -> Vec<FavoriteItem> {
        load_json(&self.path, Vec::new())
    }

    /// Whether `url` is already favorited.
    pub fn contains(&self, url: &str) -> bool {
        self.list().iter().any(|f| f.url == url)
    }

    /// Add `item` (by URL) if not present. Returns true if it was added.
    pub fn add(&self, mut item: FavoriteItem) -> bool {
        if item.url.is_empty() {
            return false;
        }
        let mut list = self.list();
        if list.iter().any(|f| f.url == item.url) {
            return false;
        }
        if item.added == 0 {
            item.added = now_epoch() as i64;
        }
        // Newest appended at the end.
        list.push(item);
        save_json(&self.path, &list, Some(2));
        true
    }

    /// Remove the favorite with this URL (no-op if absent).
    pub fn remove(&self, url: &str) {
        let mut list = self.list();
        let before = list.len();
        list.retain(|f| f.url != url);
        if list.len() != before {
            save_json(&self.path, &list, Some(2));
        }
    }

    /// The set of favorited URLs — one disk read for bulk membership checks
    /// (per-item `contains` re-reads the file every call).
    pub fn url_set(&self) -> std::collections::HashSet<String> {
        self.list().into_iter().map(|f| f.url).collect()
    }

    /// Add every item not already present, in ONE read + ONE write. Returns how
    /// many were added. Per-item `add` on a large playlist is O(N²) in disk
    /// traffic (each call re-reads and rewrites the whole growing file).
    pub fn add_many(&self, items: Vec<FavoriteItem>) -> usize {
        let mut list = self.list();
        let mut seen: std::collections::HashSet<String> =
            list.iter().map(|f| f.url.clone()).collect();
        let now = now_epoch() as i64;
        let mut added = 0;
        for mut item in items {
            if item.url.is_empty() || !seen.insert(item.url.clone()) {
                continue;
            }
            if item.added == 0 {
                item.added = now;
            }
            list.push(item);
            added += 1;
        }
        if added > 0 {
            save_json(&self.path, &list, Some(2));
        }
        added
    }

    /// Remove every URL in `urls`, in ONE read + ONE write. Returns how many
    /// were removed.
    pub fn remove_many(&self, urls: &std::collections::HashSet<String>) -> usize {
        let mut list = self.list();
        let before = list.len();
        list.retain(|f| !urls.contains(&f.url));
        let removed = before - list.len();
        if removed > 0 {
            save_json(&self.path, &list, Some(2));
        }
        removed
    }

    /// Toggle membership. Returns the new state (true = now favorited).
    pub fn toggle(&self, item: FavoriteItem) -> bool {
        if self.contains(&item.url) {
            self.remove(&item.url);
            false
        } else {
            self.add(item)
        }
    }

    /// Empty the list entirely.
    pub fn clear(&self) {
        if self.path.exists() && std::fs::remove_file(&self.path).is_err() {
            save_json(&self.path, &Vec::<FavoriteItem>::new(), Some(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(url: &str) -> FavoriteItem {
        FavoriteItem {
            url: url.to_string(),
            title: format!("title {url}"),
            ..Default::default()
        }
    }

    #[test]
    fn add_dedupes_and_toggles() {
        let dir = tempfile::tempdir().unwrap();
        let f = Favorites::new(dir.path().join("favs.json"));
        assert!(f.add(item("a")));
        assert!(!f.add(item("a"))); // dup ignored
        assert!(f.add(item("b")));
        assert!(f.contains("a"));
        assert_eq!(f.list().len(), 2);
        // newest last
        assert_eq!(f.list()[0].url, "a");
        assert_eq!(f.list()[1].url, "b");

        // toggle removes, then re-adds
        assert!(!f.toggle(item("a")));
        assert!(!f.contains("a"));
        assert!(f.toggle(item("a")));
        assert!(f.contains("a"));
    }

    #[test]
    fn remove_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let f = Favorites::new(dir.path().join("favs.json"));
        f.add(item("a"));
        f.add(item("b"));
        f.remove("a");
        assert!(!f.contains("a"));
        assert!(f.contains("b"));
        f.clear();
        assert!(f.list().is_empty());
    }

    #[test]
    fn add_many_remove_many_batch_and_dedupe() {
        let dir = tempfile::tempdir().unwrap();
        let f = Favorites::new(dir.path().join("favs.json"));
        f.add(item("a"));
        // Batch add: dedupes against disk AND within the batch, keeps order.
        let n = f.add_many(vec![item("a"), item("b"), item("c"), item("b"), item("")]);
        assert_eq!(n, 2);
        let urls: Vec<_> = f.list().into_iter().map(|x| x.url).collect();
        assert_eq!(urls, ["a", "b", "c"]);
        assert!(f.list().iter().all(|x| x.added > 0));

        let n = f.remove_many(&["a".into(), "c".into(), "nope".into()].into());
        assert_eq!(n, 2);
        assert_eq!(f.url_set(), ["b".to_string()].into());
    }

    #[test]
    fn empty_url_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let f = Favorites::new(dir.path().join("favs.json"));
        assert!(!f.add(item("")));
        assert!(f.list().is_empty());
    }
}
