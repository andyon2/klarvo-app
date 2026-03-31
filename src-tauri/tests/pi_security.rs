//! Prompt Injection Security Test Suite for Klarvo
//!
//! Based on the Arcanum PI Taxonomy (https://arcanum-sec.github.io/arc_pi_taxonomy/)
//!
//! Usage:
//!
//!   # Output sanitization tests (no API key required):
//!   cargo test --test pi_security output
//!
//!   # Golden path baseline:
//!   GROQ_API_KEY=... cargo test --test pi_security baseline -- --ignored --nocapture
//!
//!   # All Tier-1 tests (individual):
//!   GROQ_API_KEY=... cargo test --test pi_security tier1 -- --ignored --nocapture
//!
//!   # All Tier-2 tests:
//!   GROQ_API_KEY=... cargo test --test pi_security tier2 -- --ignored --nocapture
//!
//!   # Use a different provider:
//!   PI_PROVIDER=deepseek DEEPSEEK_API_KEY=... cargo test --test pi_security -- --ignored --nocapture

#[path = "pi_security/harness.rs"]
mod harness;
#[path = "pi_security/judge.rs"]
mod judge;
#[path = "pi_security/registry.rs"]
mod registry;
#[path = "pi_security/report.rs"]
mod report;
#[path = "pi_security/tests_output.rs"]
mod tests_output;
#[path = "pi_security/tests_tier1.rs"]
mod tests_tier1;
#[path = "pi_security/tests_tier2.rs"]
mod tests_tier2;
