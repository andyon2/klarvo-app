/// Marker guard asserting no outbound (non-loopback) network connections in a test.
///
/// NFR6 mandates that integration tests run without real network access. Full
/// OS-level socket-interception is out-of-scope for Phase 1 (platform-specific
/// complexity, interaction with tokio's runtime). `NoNetworkGuard` documents
/// the intent as a marker-type.
///
/// **Enforcement mechanism**: tests that hold a `NoNetworkGuard` MUST route
/// all HTTP through `wiremock` loopback stubs. Wiremock returns a non-2xx
/// status (or panics) for unmatched routes, which is the actual runtime
/// enforcement. `assert_no_connect()` is a Phase-1 no-op marker call; full
/// OS-level socket-blocking is not implemented.
pub struct NoNetworkGuard;

impl NoNetworkGuard {
    pub fn new() -> Self {
        Self
    }

    /// Assert that no non-loopback network calls were made.
    ///
    /// Phase-1 implementation: no-op marker assertion. Wiremock's unmatched-
    /// route response is the runtime enforcement. Callers SHOULD pair this
    /// with `MockServer::verify()` / wiremock mount assertions for full NFR6
    /// compliance. Full OS-level socket-interception is out-of-scope per D2.
    pub fn assert_no_connect(&self) {}
}

impl Default for NoNetworkGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_infallible() {
        let _guard = NoNetworkGuard::new();
    }

    #[test]
    fn assert_no_connect_passes_when_not_triggered() {
        let guard = NoNetworkGuard::new();
        guard.assert_no_connect();
    }
}
