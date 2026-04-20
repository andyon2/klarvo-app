use std::marker::PhantomData;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::pipeline::PipelineStage;

/// Generic test fixture implementing [`PipelineStage`] with a canned output queue
/// and optional error injection.
///
/// `I` is the input type (consumed but otherwise ignored). `O` is the output type
/// drawn from an internal queue on each `process` call.
pub struct MockPipelineStage<I, O> {
    output_queue: Vec<O>,
    error: Option<AppError>,
    call_count: Arc<AtomicUsize>,
    _phantom: PhantomData<(I, O)>,
}

impl<I, O> MockPipelineStage<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
{
    /// Returns `output` on the first `process` call. Further calls exhaust the queue
    /// and return an `Internal` error.
    pub fn with_output(output: O) -> Self {
        Self {
            output_queue: vec![output],
            error: None,
            call_count: Arc::new(AtomicUsize::new(0)),
            _phantom: PhantomData,
        }
    }

    /// Returns `error` on every `process` call regardless of the queue state.
    pub fn with_error(error: AppError) -> Self {
        Self {
            output_queue: vec![],
            error: Some(error),
            call_count: Arc::new(AtomicUsize::new(0)),
            _phantom: PhantomData,
        }
    }

    /// Returns each element of `outputs` in order across successive `process` calls.
    pub fn with_queue(outputs: Vec<O>) -> Self {
        Self {
            output_queue: outputs,
            error: None,
            call_count: Arc::new(AtomicUsize::new(0)),
            _phantom: PhantomData,
        }
    }

    /// Total number of times `process` has been called (success and error alike).
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl<I, O> PipelineStage for MockPipelineStage<I, O>
where
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
{
    type Input = I;
    type Output = O;

    async fn process(&self, _input: I) -> Result<O, AppError> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        if let Some(ref err) = self.error {
            return Err(err.clone());
        }
        self.output_queue.get(idx).cloned().ok_or_else(|| AppError {
            kind: AppErrorKind::Internal,
            message: format!("MockPipelineStage queue exhausted at call {idx}"),
            user_message: None,
            retryable: false,
        })
    }

    fn stage_type(&self) -> &'static str {
        "mock"
    }
}

/// Headless stage-execution wrapper for unit-tests in downstream crates.
pub async fn harness_run_stage<S: PipelineStage>(
    stage: &S,
    input: S::Input,
) -> Result<S::Output, AppError> {
    stage.process(input).await
}

// Object-safety compile-test: Box<dyn PipelineStage<Input=(), Output=()>> must compile.
#[allow(dead_code)]
fn _obj_safe(_x: Box<dyn PipelineStage<Input = (), Output = ()>>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use klarvo_core::error::AppErrorKind;

    #[tokio::test]
    async fn canned_output_returns_correctly() {
        let stage = MockPipelineStage::<(), String>::with_output("hello".to_string());
        let result = harness_run_stage(&stage, ()).await;
        assert_eq!(result.unwrap(), "hello");
        assert_eq!(stage.call_count(), 1);
    }

    #[tokio::test]
    async fn error_injection_propagates() {
        let err = AppError {
            kind: AppErrorKind::Internal,
            message: "injected".to_string(),
            user_message: None,
            retryable: false,
        };
        let stage = MockPipelineStage::<(), String>::with_error(err);
        let result = harness_run_stage(&stage, ()).await;
        assert!(result.is_err());
        assert_eq!(stage.call_count(), 1);
    }

    #[tokio::test]
    async fn call_count_increments_across_calls() {
        let stage = MockPipelineStage::with_queue(vec!["a", "b", "c"]);
        let _ = harness_run_stage(&stage, ()).await;
        let _ = harness_run_stage(&stage, ()).await;
        assert_eq!(stage.call_count(), 2);
    }

    #[tokio::test]
    async fn harness_run_stage_propagates_queue_in_order() {
        let stage = MockPipelineStage::with_queue(vec![1u32, 2u32, 3u32]);
        assert_eq!(harness_run_stage(&stage, ()).await.unwrap(), 1);
        assert_eq!(harness_run_stage(&stage, ()).await.unwrap(), 2);
        assert_eq!(harness_run_stage(&stage, ()).await.unwrap(), 3);
    }
}
