//! Klarvo session orchestrator — Tauri-free push-to-talk state machine.
//!
//! No tauri/tauri-specta dependency — orchestrator is platform-agnostic (ADR-0012 SD-1).
//!
//! # 7-Step Push-to-Talk Cycle
//!
//! 1. **Press** → `on_press()` — start `AudioSource`, open broadcast channel
//! 2. **Audio capture** — `AudioSource` emits `AudioEvent::Samples`
//! 3. **VAD gate** — `run_capture_session` accumulates speech segments
//! 4. **Release** → `on_release()` — drop `CaptureHandle`, channel closes
//! 5. **Pipeline** — `run_pipeline(manifest, registry, audio)` → `StageData::Text`
//! 6. **Deliver** — `OutputTarget::deliver(&text)`
//! 7. **Paste** — `PasteBackend::paste()`
//!
//! Cross-ref: `memory/project_shell_session_lifecycle` (authoritative topology).

pub mod session;

pub use session::SessionOrchestrator;
