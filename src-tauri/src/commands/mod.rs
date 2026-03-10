//! Tauri command modules -- one module per feature area.
//!
//! All public functions here are annotated with `#[tauri::command]` and must
//! be registered in the `invoke_handler` in `lib.rs`.

pub mod dictionary;
pub mod history;
pub mod license;
pub mod misc;
pub mod recording;
pub mod settings;
pub mod whisper;
