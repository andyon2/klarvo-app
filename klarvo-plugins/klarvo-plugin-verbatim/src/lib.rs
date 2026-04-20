//! `Verbatim` is the literal identity-passthrough [`CleanupStyle`] implementation — no trim,
//! no normalization, no dictionary application. Phase-1 default CleanupStyle per
//! `docs/rebuild-discussion.md` Polished-Mode-Deferral.
//!
//! The companion reference-plugin `klarvo-plugin-groq` (Story 1B.4) demonstrates an
//! external-API-dependent trait impl (HTTPS client, KeyStore-dependent) —
//! `klarvo-plugin-verbatim` covers the opposite pole of the plugin-complexity-spectrum:
//! zero-dependency, zero-network, pure-transform.
//!
//! Polished-Mode CleanupStyle (with dictionary application and output-language transformation)
//! is deferred to Phase 2 per `memory/feedback_polished_designschwaeche` — `Verbatim`
//! remains the Phase-1 default.

mod provider;

pub use provider::{Verbatim, ID, register};
