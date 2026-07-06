//! Persisted scheduled downloads, surviving restarts.
//! Ported from `core/scheduled_downloads.py`.

use std::path::PathBuf;

use serde_json::Value;

use crate::json_store::{load_json, save_json};
use crate::util::now_epoch;

pub struct ScheduledDownloadStore {
    path: PathBuf,
}

impl ScheduledDownloadStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load, keeping only JSON objects (`load`).
    pub fn load(&self) -> Vec<Value> {
        let data: Value = load_json(&self.path, Value::Array(Vec::new()));
        match data {
            Value::Array(items) => items.into_iter().filter(Value::is_object).collect(),
            _ => Vec::new(),
        }
    }

    pub fn save(&self, items: &[Value]) {
        if save_json(&self.path, &items, Some(2)) {
            tracing::debug!("Scheduled downloads saved to disk");
        }
    }

    /// Insert or replace by `id`, default `created_at`, keep sorted by time.
    pub fn upsert(&self, item: &Value) {
        let task_id = match item.get("id").and_then(Value::as_str) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => return,
        };

        let mut items: Vec<Value> = self
            .load()
            .into_iter()
            .filter(|e| e.get("id").and_then(Value::as_str) != Some(task_id.as_str()))
            .collect();

        let mut item = item.clone();
        if let Value::Object(map) = &mut item {
            map.entry("created_at")
                .or_insert_with(|| Value::from(now_epoch()));
        }
        items.push(item);
        items.sort_by(|a, b| {
            let sa = a
                .get("scheduled_time")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let sb = b
                .get("scheduled_time")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        self.save(&items);
    }

    pub fn remove(&self, task_id: &str) {
        if task_id.is_empty() {
            return;
        }
        let items: Vec<Value> = self
            .load()
            .into_iter()
            .filter(|e| e.get("id").and_then(Value::as_str) != Some(task_id))
            .collect();
        self.save(&items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn upsert_dedupes_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        let s = ScheduledDownloadStore::new(dir.path().join("sched.json"));
        s.upsert(&json!({"id": "a", "scheduled_time": 200.0}));
        s.upsert(&json!({"id": "b", "scheduled_time": 100.0}));
        s.upsert(&json!({"id": "a", "scheduled_time": 50.0})); // replace a
        let items = s.load();
        assert_eq!(items.len(), 2);
        // sorted by scheduled_time: a(50) then b(100)
        assert_eq!(items[0]["id"], json!("a"));
        assert_eq!(items[0]["scheduled_time"], json!(50.0));
        assert!(items[0].get("created_at").is_some());
    }
}
