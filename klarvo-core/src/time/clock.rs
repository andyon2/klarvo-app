use std::time::Instant;

/// Session-relative monotone millisecond clock.
///
/// Implementations must be `Send + Sync` so the clock can be shared across
/// async tasks. `now_ms()` returns milliseconds elapsed since the session
/// started — not wall-clock UNIX time. This matches the `ts_ms` convention
/// from ADR-0001 / `memory/project_event_ts_ms_convention`.
///
/// `FakeClock` in `klarvo-test-fixtures` implements this trait for deterministic
/// test control (Story 1A.1 D1 deferral resolved in 1A.3).
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Production `Clock` backed by `std::time::Instant`.
///
/// `session_start` is captured at construction; subsequent `now_ms()` calls
/// return `session_start.elapsed().as_millis() as u64`. The first call is
/// guaranteed to return a very small value (typically 0–1 ms). Monotone
/// because `Instant` is monotone on all supported platforms.
pub struct MonotonicClock {
    session_start: Instant,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self { session_start: Instant::now() }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    fn now_ms(&self) -> u64 {
        self.session_start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotone_start_is_zero_ish() {
        let clock = MonotonicClock::new();
        assert!(
            clock.now_ms() < 100,
            "expected <100 ms at startup, got {}",
            clock.now_ms()
        );
    }

    #[test]
    fn now_ms_increases_over_time() {
        let clock = MonotonicClock::new();
        let t1 = clock.now_ms();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = clock.now_ms();
        assert!(t2 > t1, "expected t2 > t1, got t1={t1} t2={t2}");
    }
}
