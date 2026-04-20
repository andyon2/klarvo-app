use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use klarvo_core::error::AppError;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::{CleanupInput, CleanupStyle};

/// Behaviour mode for [`MockCleanupStyle`].
pub enum MockCleanupMode {
    /// Return `input.raw` unchanged.
    Identity,
    /// Return `input.raw.to_uppercase()`.
    UpperCase,
    /// Always return the injected error (input is still captured).
    ErrorInject(AppError),
}

/// Test fixture implementing [`CleanupStyle`] with configurable behaviour and
/// input capture for post-call assertions.
///
/// Use [`assert_cleanup_input`] to verify the captured input after a `apply`/`process` call.
pub struct MockCleanupStyle {
    mode: MockCleanupMode,
    last_input: Arc<Mutex<Option<CleanupInput>>>,
    call_count: Arc<AtomicUsize>,
}

impl MockCleanupStyle {
    /// Returns `input.raw` unchanged on every call.
    pub fn identity() -> Self {
        Self::new(MockCleanupMode::Identity)
    }

    /// Returns `input.raw.to_uppercase()` on every call.
    pub fn uppercase() -> Self {
        Self::new(MockCleanupMode::UpperCase)
    }

    /// Returns `err` on every call (input is still captured for assertion).
    pub fn error_inject(err: AppError) -> Self {
        Self::new(MockCleanupMode::ErrorInject(err))
    }

    fn new(mode: MockCleanupMode) -> Self {
        Self {
            mode,
            last_input: Arc::new(Mutex::new(None)),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Total number of times `apply`/`process` has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Clone of the last [`CleanupInput`] passed to `apply`/`process`, or `None` if not called.
    pub fn last_input(&self) -> Option<CleanupInput> {
        self.last_input.lock().unwrap().clone()
    }
}

#[async_trait]
impl PipelineStage for MockCleanupStyle {
    type Input = CleanupInput;
    type Output = String;

    async fn process(&self, input: CleanupInput) -> Result<String, AppError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        *self.last_input.lock().unwrap() = Some(input.clone());
        match &self.mode {
            MockCleanupMode::Identity => Ok(input.raw.clone()),
            MockCleanupMode::UpperCase => Ok(input.raw.to_uppercase()),
            MockCleanupMode::ErrorInject(err) => Err(err.clone()),
        }
    }

    fn stage_type(&self) -> &'static str {
        "cleanup"
    }
}

#[async_trait]
impl CleanupStyle for MockCleanupStyle {}

// Object-safety compile-test: Box<dyn CleanupStyle> must compile.
#[allow(dead_code)]
fn _obj_safe_cleanup(_x: Box<dyn CleanupStyle>) {}

/// Assert that the last input captured by `mock` equals `expected`.
pub fn assert_cleanup_input(mock: &MockCleanupStyle, expected: &CleanupInput) {
    let last = mock.last_input();
    assert!(last.is_some(), "MockCleanupStyle: no input captured (was process/apply called?)");
    assert_eq!(last.unwrap(), *expected);
}

#[cfg(test)]
mod tests {
    use super::*;
    use klarvo_core::error::AppErrorKind;
    use klarvo_core::traits::CleanupInput;

    fn input(raw: &str) -> CleanupInput {
        CleanupInput::from_raw(raw.to_string())
    }

    #[tokio::test]
    async fn identity_mode_returns_raw_unchanged() {
        let mock = MockCleanupStyle::identity();
        let result = mock.apply(input("hello world")).await.unwrap();
        assert_eq!(result, "hello world");
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn uppercase_mode_uppercases_raw() {
        let mock = MockCleanupStyle::uppercase();
        let result = mock.apply(input("hello world")).await.unwrap();
        assert_eq!(result, "HELLO WORLD");
    }

    #[tokio::test]
    async fn error_inject_mode_propagates_error() {
        let err = AppError {
            kind: AppErrorKind::Internal,
            message: "injected".to_string(),
            user_message: None,
            retryable: false,
        };
        let mock = MockCleanupStyle::error_inject(err);
        assert!(mock.apply(input("anything")).await.is_err());
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn last_input_captured_on_success() {
        let mock = MockCleanupStyle::identity();
        let expected = input("captured text");
        mock.apply(expected.clone()).await.unwrap();
        assert_cleanup_input(&mock, &expected);
    }

    #[tokio::test]
    async fn last_input_captured_on_error() {
        let err = AppError {
            kind: AppErrorKind::Internal,
            message: "x".to_string(),
            user_message: None,
            retryable: false,
        };
        let mock = MockCleanupStyle::error_inject(err);
        let expected = input("even on error");
        let _ = mock.apply(expected.clone()).await;
        assert_cleanup_input(&mock, &expected);
    }
}
