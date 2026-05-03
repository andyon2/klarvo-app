/// Captures and restores the OS foreground window around a dictation session.
///
/// Platform-agnostic trait — Windows impl lives in the shell, test/null impl
/// in `klarvo-test-fixtures` / `klarvo-core` (NullFocusCapture).
pub trait FocusCapture: Send + Sync + 'static {
    /// Capture the current foreground window handle as an opaque u64.
    /// Returns `None` if no window has focus, feature is unsupported, or handle is 0.
    fn capture(&self) -> Option<u64>;

    /// Restore focus to a previously captured handle. No-op if handle is None.
    /// Best-effort: silently ignores OS failures (target window may no longer exist).
    fn restore(&self, handle: Option<u64>);
}

/// No-op implementation for tests and non-Windows platforms.
pub struct NullFocusCapture;

impl FocusCapture for NullFocusCapture {
    fn capture(&self) -> Option<u64> {
        None
    }

    fn restore(&self, _handle: Option<u64>) {}
}
