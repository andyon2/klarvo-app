//! `OutputTarget` is the Terminal-Sink abstraction for the dictation pipeline.
//! Shell-Layer-Integration-Patterns (Startup-Probes, Config-Binding) sind Epic-3-Shell-Adapter-Scope.

pub mod keys;
pub mod paste;

pub use crate::traits::output::OutputTarget;
pub use paste::PasteBackend;
