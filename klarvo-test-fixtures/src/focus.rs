use std::sync::{Arc, Mutex};

use klarvo_core::output::FocusCapture;

pub struct MockFocusCapture {
    captured: Arc<Mutex<Vec<Option<u64>>>>,
    restored: Arc<Mutex<Vec<Option<u64>>>>,
}

impl MockFocusCapture {
    pub fn new() -> Self {
        Self {
            captured: Default::default(),
            restored: Default::default(),
        }
    }

    pub fn capture_count(&self) -> usize {
        self.captured.lock().unwrap().len()
    }

    pub fn restore_count(&self) -> usize {
        self.restored.lock().unwrap().len()
    }

    pub fn last_restored(&self) -> Option<Option<u64>> {
        self.restored.lock().unwrap().last().copied()
    }
}

impl Default for MockFocusCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusCapture for MockFocusCapture {
    fn capture(&self) -> Option<u64> {
        let handle = Some(42u64);
        self.captured.lock().unwrap().push(handle);
        handle
    }

    fn restore(&self, handle: Option<u64>) {
        self.restored.lock().unwrap().push(handle);
    }
}
