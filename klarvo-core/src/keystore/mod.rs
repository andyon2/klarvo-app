//! Secret-lifecycle abstraction for API-key management.
//!
//! `KeyStore` is a secret-lifecycle abstraction, architecturally separate from the 4
//! Phase-1-stability Data-Flow-Traits (`PipelineStage`, `SttProvider`, `CleanupStyle`,
//! `VadProvider`). Stability guarantees for `KeyStore` are independently scoped: the
//! Trait-Signature is locked in Phase 1, but backend-impl-swap (Phase-1 Plain-SQLite
//! dev-only → Phase-4 OS-Keystore release-default per FR46) does not constitute a
//! Trait-Signature change.
//!
//! # Non-Goals for Phase 1
//!
//! (a) `list()` / `keys()` for enumerating all stored keys — deferred to Phase 2+ when
//! Settings-UI needs key-enumeration.
//! (b) `exists(key)` / `contains(key)` — Phase-1 callers use `get(key).is_ok()`.
//! (c) Batch-operations (`set_many`, `delete_many`) — deferred until usage-patterns
//! emerge in Phase 2+.

pub mod keys;
#[cfg(feature = "dev-plain-keystore")]
mod plain_sqlite;
mod trait_def;

#[cfg(feature = "dev-plain-keystore")]
pub use plain_sqlite::PlainSqliteKeyStore;
pub use trait_def::KeyStore;
