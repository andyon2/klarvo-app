/// Context-holder for headless integration tests.
///
/// Single construction-point for test harness state, designed as an additive
/// extension anchor. Fields are added per future stories (1A.2+); none are
/// required for Story 1A.1. Existing mocks (`MockAudioSource`, `InMemoryKeyStore`,
/// etc.) remain standalone types — they are not retro-migrated per
/// `memory/feedback_premature_abstraction_guard`.
pub struct HeadlessTestEnv {
    // additive fields per 1A.2+
}

impl HeadlessTestEnv {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for HeadlessTestEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_infallible() {
        let _env = HeadlessTestEnv::new();
    }
}
