use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use klarvo_core::error::AppError;
use klarvo_core::history::{HistoryBackend, HistoryEntry, NewHistoryEntry};

pub struct MockHistoryBackend {
    entries: Arc<Mutex<Vec<HistoryEntry>>>,
    next_id: Arc<Mutex<i64>>,
}

impl MockHistoryBackend {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn all_entries(&self) -> Vec<HistoryEntry> {
        self.entries.lock().unwrap().clone()
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
        let mut id_guard = self.next_id.lock().unwrap();
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

        self.entries.lock().unwrap().push(hist_entry);
        Ok(id)
    }

    async fn list(&self, limit: u32) -> Result<Vec<HistoryEntry>, AppError> {
        let entries = self.entries.lock().unwrap();
        let mut result: Vec<HistoryEntry> = entries.iter().cloned().rev().take(limit as usize).collect();
        result.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(result)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        self.entries.lock().unwrap().retain(|e| e.id != id);
        Ok(())
    }

    async fn clear(&self) -> Result<(), AppError> {
        self.entries.lock().unwrap().clear();
        Ok(())
    }

    async fn count(&self) -> Result<u32, AppError> {
        Ok(self.entries.lock().unwrap().len() as u32)
    }
}
