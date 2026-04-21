use async_trait::async_trait;

/// Async fire-and-forget error-push path (ADR-0009 Hybrid-C).
///
/// Core defines this trait; Shell implements it (Epic 3: `shells/windows/`)
/// using `tauri::AppHandle::emit` under the hood. The `emit_error` call is
/// advisory — implementations MUST NOT block the caller or return errors to it.
/// Implementations SHOULD log internally via `tracing` on failure.
///
/// **Why fire-and-forget?** Error events are diagnostic signals, not
/// transactional operations. A failed push (e.g. no Tauri window open) must
/// not abort the pipeline run that triggered it (ADR-0009, Resolved-Q4).
///
/// `key` MUST be a valid i18n key (`klarvo_core::i18n::is_key`). `ts_ms` is
/// session-relative monotone milliseconds from the caller's clock.
#[async_trait]
pub trait ErrorEmitter: Send + Sync + 'static {
    async fn emit_error(&self, key: &str, ts_ms: u64);
}
