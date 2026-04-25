use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use klarvo_core::{error::AppError, output::PasteBackend};

pub struct MockPasteBackend {
    calls: Arc<Mutex<Vec<()>>>,
    result: Result<(), AppError>,
}

impl MockPasteBackend {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            result: Ok(()),
        }
    }

    pub fn with_result(self, result: Result<(), AppError>) -> Self {
        Self {
            calls: self.calls,
            result,
        }
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn was_called(&self) -> bool {
        self.call_count() > 0
    }
}

impl Default for MockPasteBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PasteBackend for MockPasteBackend {
    async fn paste(&self) -> Result<(), AppError> {
        self.calls.lock().unwrap().push(());
        self.result.clone()
    }
}
