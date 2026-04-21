use std::sync::atomic::{AtomicU64, Ordering};

/// Session-relative monotone millisecond clock for deterministic tests.
///
/// Generates `ts_ms` values (session-relative monotone ms per NFR3 /
/// `memory/project_event_ts_ms_convention`) without touching the system clock.
/// `advance(ms)` is the only way to move time forward — tests remain
/// deterministic and fast regardless of wall-clock speed.
///
/// Clock-Trait-Seam is deferred to Story 1A.3: once `klarvo-core` defines a
/// `Clock` trait, 1A.3 adds `impl Clock for FakeClock` additively. This
/// struct is standalone in 1A.1 per Divergenz D1.
pub struct FakeClock {
    inner: AtomicU64,
}

impl FakeClock {
    pub fn new() -> Self {
        Self { inner: AtomicU64::new(0) }
    }

    /// Advance the clock by `ms` milliseconds.
    pub fn advance(&self, ms: u64) {
        self.inner.fetch_add(ms, Ordering::Relaxed);
    }

    /// Return current session-relative ms (monotone, starts at 0).
    pub fn now_ms(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves Story 1A.1 D1 deferral: additive `Clock`-trait impl once the trait
/// landed in `klarvo-core::time` (Story 1A.3).
impl klarvo_core::time::Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_zero() {
        let clock = FakeClock::new();
        assert_eq!(clock.now_ms(), 0);
    }

    #[test]
    fn advance_monotone_increments() {
        let clock = FakeClock::new();
        clock.advance(100);
        assert_eq!(clock.now_ms(), 100);
        clock.advance(50);
        assert_eq!(clock.now_ms(), 150);
    }

    #[test]
    fn now_ms_returns_current() {
        let clock = FakeClock::new();
        clock.advance(42);
        assert_eq!(clock.now_ms(), 42);
    }
}
