use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use klarvo_core::error::AppError;
use klarvo_core::history::{HistoryBackend, HistoryEntry, NewHistoryEntry};

pub struct MockHistoryBackend {
    entries: Arc<Mutex<Vec<HistoryEntry>>>,
    next_id: Arc<Mutex<i64>>,
}

/// Lock a `Mutex` and recover transparently from poison — a panicking earlier test
/// must not cascade-poison every subsequent test in the same process.
fn lock_or_recover<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl MockHistoryBackend {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn entry_count(&self) -> usize {
        lock_or_recover(&self.entries).len()
    }

    pub fn all_entries(&self) -> Vec<HistoryEntry> {
        lock_or_recover(&self.entries).clone()
    }
}

impl Default for MockHistoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HistoryBackend for MockHistoryBackend {
    async fn append(&self, entry: &NewHistoryEntry) -> Result<i64, AppError> {
        let mut id_guard = lock_or_recover(&self.next_id);
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let hist_entry = HistoryEntry {
            id,
            text: entry.text.clone(),
            raw_text: entry.raw_text.clone(),
            style: entry.style.clone(),
            language: entry.language.clone(),
            app_name: entry.app_name.clone(),
            created_at: entry.created_at.clone(),
            uuid: entry.uuid.clone(),
            device_id: entry.device_id.clone(),
            plugin_id: entry.plugin_id.clone(),
            manifest_version: entry.manifest_version.clone(),
            output_language: entry.output_language.clone(),
        };

        lock_or_recover(&self.entries).push(hist_entry);
        Ok(id)
    }

    async fn list(&self, limit: u32) -> Result<Vec<HistoryEntry>, AppError> {
        // Sort newest-first by id, then truncate — robust against out-of-order id insertion.
        let mut sorted: Vec<HistoryEntry> = lock_or_recover(&self.entries).clone();
        sorted.sort_by(|a, b| b.id.cmp(&a.id));
        sorted.truncate(limit as usize);
        Ok(sorted)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        lock_or_recover(&self.entries).retain(|e| e.id != id);
        Ok(())
    }

    async fn clear(&self) -> Result<(), AppError> {
        lock_or_recover(&self.entries).clear();
        Ok(())
    }

    async fn count(&self) -> Result<u32, AppError> {
        Ok(lock_or_recover(&self.entries).len() as u32)
    }
}
