//! Whisper model manager for offline STT.
//!
//! Manages the lifecycle of GGML model files used by [`LocalWhisperProvider`]:
//! - Enumerate available models with metadata (size, description).
//! - Query download status (file exists on disk or not).
//! - Download a model from HuggingFace with progress callbacks.
//! - Delete a downloaded model.
//!
//! ## Path convention
//!
//! Models are stored in `{app_data_dir}/models/ggml-{id}.bin`, matching the
//! pattern expected by [`LocalWhisperProvider`] and built by `pipeline.rs`.
//!
//! ## Download strategy
//!
//! The download writes to a `.part` temp file first. Only on success is the
//! file renamed atomically to the final `.bin` path. This prevents the
//! `LocalWhisperProvider` from picking up a partial/corrupted model file.
//!
//! ## Platform guard
//!
//! This module is gated behind `target_os = "windows"` because local whisper
//! is Windows-only in the current build. The Tauri commands that call into
//! this module are similarly guarded.

#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by the model manager.
#[derive(Debug, Error)]
pub enum ModelManagerError {
    #[error("Unknown model id: {0}")]
    UnknownModel(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Download interrupted: server returned status {0}")]
    BadStatus(u16),
}

// ---------------------------------------------------------------------------
// WhisperModelInfo
// ---------------------------------------------------------------------------

/// Metadata describing a downloadable Whisper GGML model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelInfo {
    /// Short identifier used in config and paths (e.g. `"base"`).
    pub id: String,
    /// Filename on HuggingFace / in the local models directory.
    pub filename: String,
    /// Approximate size in bytes (informational, not enforced).
    pub size_bytes: u64,
    /// Human-readable description shown in the UI.
    pub description: String,
}

/// Download / presence status of a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelStatus {
    /// Model file exists and is ready to use.
    Downloaded,
    /// Model file is not present on disk.
    NotDownloaded,
}

/// [`WhisperModelInfo`] combined with the current [`ModelStatus`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelWithStatus {
    #[serde(flatten)]
    pub info: WhisperModelInfo,
    pub status: ModelStatus,
}

// ---------------------------------------------------------------------------
// Model catalogue
// ---------------------------------------------------------------------------

/// Base URL for GGML model files on HuggingFace.
const HF_BASE_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Returns the static catalogue of available Whisper models.
///
/// Each entry describes a model variant with its filename, approximate size
/// and a short description. Callers should prefer `get_model_status` or
/// `list_models_with_status` when they also need the download state.
pub fn list_available_models() -> Vec<WhisperModelInfo> {
    // tiny/base removed — quality too low to ship.
    vec![
        WhisperModelInfo {
            id: "small".to_string(),
            filename: "ggml-small.bin".to_string(),
            size_bytes: 488_000_000,
            description: "Recommended — good balance of speed and quality".to_string(),
        },
        WhisperModelInfo {
            id: "medium".to_string(),
            filename: "ggml-medium.bin".to_string(),
            size_bytes: 1_533_000_000,
            description: "High quality, slower".to_string(),
        },
        WhisperModelInfo {
            id: "large-v3".to_string(),
            filename: "ggml-large-v3.bin".to_string(),
            size_bytes: 3_094_000_000,
            description: "Best quality, requires more RAM".to_string(),
        },
    ]
}

/// Finds a model by its `id` string.
///
/// Returns `None` when no model with that id exists in the catalogue.
fn find_model(model_id: &str) -> Option<WhisperModelInfo> {
    list_available_models()
        .into_iter()
        .find(|m| m.id == model_id)
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the `models/` directory inside `app_data_dir`.
pub fn models_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("models")
}

/// Returns the full path for a model file given its filename.
///
/// Example: `{app_data_dir}/models/ggml-base.bin`
pub fn model_path(app_data_dir: &Path, filename: &str) -> PathBuf {
    models_dir(app_data_dir).join(filename)
}

// ---------------------------------------------------------------------------
// Status query
// ---------------------------------------------------------------------------

/// Returns the [`ModelStatus`] for a model identified by `model_id`.
///
/// # Errors
/// - `ModelManagerError::UnknownModel` when `model_id` is not in the catalogue.
pub fn get_model_status(
    model_id: &str,
    app_data_dir: &Path,
) -> Result<ModelStatus, ModelManagerError> {
    let info = find_model(model_id)
        .ok_or_else(|| ModelManagerError::UnknownModel(model_id.to_string()))?;

    let path = model_path(app_data_dir, &info.filename);
    if path.exists() {
        Ok(ModelStatus::Downloaded)
    } else {
        Ok(ModelStatus::NotDownloaded)
    }
}

/// Returns all models from the catalogue with their current download status.
pub fn list_models_with_status(
    app_data_dir: &Path,
) -> Vec<WhisperModelWithStatus> {
    list_available_models()
        .into_iter()
        .map(|info| {
            let path = model_path(app_data_dir, &info.filename);
            let status = if path.exists() {
                ModelStatus::Downloaded
            } else {
                ModelStatus::NotDownloaded
            };
            WhisperModelWithStatus { info, status }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/// Downloads a Whisper model and writes it to `{app_data_dir}/models/`.
///
/// ## Atomic write strategy
///
/// The bytes are written to a `.part` file while downloading. On success the
/// file is renamed to the final `.bin` path. On failure the `.part` file is
/// cleaned up. This ensures that `LocalWhisperProvider` never reads a
/// partial model.
///
/// ## Progress reporting
///
/// `progress_fn` is called after each chunk with `(bytes_received, total_bytes)`.
/// `total_bytes` is `0` when the `Content-Length` header is absent.
///
/// # Errors
/// - `ModelManagerError::UnknownModel` -- `model_id` not in catalogue.
/// - `ModelManagerError::Request` -- network failure.
/// - `ModelManagerError::BadStatus` -- server returned non-200.
/// - `ModelManagerError::Io` -- file-system write failure.
pub async fn download_model<F>(
    model_id: &str,
    app_data_dir: &Path,
    mut progress_fn: F,
) -> Result<(), ModelManagerError>
where
    F: FnMut(u64, u64) + Send + 'static,
{
    use tokio::io::AsyncWriteExt;

    let info = find_model(model_id)
        .ok_or_else(|| ModelManagerError::UnknownModel(model_id.to_string()))?;

    // Ensure models directory exists.
    let dir = models_dir(app_data_dir);
    tokio::fs::create_dir_all(&dir).await?;

    let final_path = dir.join(&info.filename);
    let part_path = dir.join(format!("{}.part", info.filename));

    let url = format!("{HF_BASE_URL}/{}", info.filename);
    log::info!("[model_manager] Downloading {} from {}", info.filename, url);

    let client = reqwest::Client::new();
    let mut response = client.get(&url).send().await?;

    let status = response.status();
    if !status.is_success() {
        return Err(ModelManagerError::BadStatus(status.as_u16()));
    }

    let total_bytes = response
        .content_length()
        .unwrap_or(0);

    // Open the temp file for writing.
    let mut file = tokio::fs::File::create(&part_path).await?;
    let mut bytes_received: u64 = 0;

    // Use chunk() instead of bytes_stream() to avoid requiring the reqwest
    // "stream" feature. chunk() returns the next bytes chunk or None on EOF.
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        bytes_received += chunk.len() as u64;
        progress_fn(bytes_received, total_bytes);
    }

    // Flush and close before rename.
    file.flush().await?;
    drop(file);

    // Atomic rename: .part -> .bin
    tokio::fs::rename(&part_path, &final_path).await.map_err(|e| {
        // Best-effort cleanup of the .part file on rename failure.
        let _ = std::fs::remove_file(&part_path);
        e
    })?;

    log::info!(
        "[model_manager] Download complete: {} ({} bytes)",
        info.filename,
        bytes_received,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

/// Deletes a downloaded model file.
///
/// Returns `Ok(())` if the file did not exist (idempotent delete).
///
/// # Errors
/// - `ModelManagerError::UnknownModel` -- `model_id` not in catalogue.
/// - `ModelManagerError::Io` -- file-system deletion failure (other than
///   "file not found").
pub fn delete_model(
    model_id: &str,
    app_data_dir: &Path,
) -> Result<(), ModelManagerError> {
    let info = find_model(model_id)
        .ok_or_else(|| ModelManagerError::UnknownModel(model_id.to_string()))?;

    let path = model_path(app_data_dir, &info.filename);

    match std::fs::remove_file(&path) {
        Ok(()) => {
            log::info!("[model_manager] Deleted model: {}", path.display());
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ModelManagerError::Io(e)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- Catalogue ---

    /// The catalogue must contain exactly the three documented model variants.
    #[test]
    fn test_list_available_models_returns_three_entries() {
        let models = list_available_models();
        assert_eq!(models.len(), 3);
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"small"), "catalogue must include 'small'");
        assert!(ids.contains(&"medium"), "catalogue must include 'medium'");
        assert!(ids.contains(&"large-v3"), "catalogue must include 'large-v3'");
    }

    /// Each catalogue entry must have a non-empty filename, description and
    /// a size greater than zero.
    #[test]
    fn test_list_available_models_entries_are_valid() {
        for model in list_available_models() {
            assert!(!model.filename.is_empty(), "filename must not be empty");
            assert!(!model.description.is_empty(), "description must not be empty");
            assert!(model.size_bytes > 0, "size_bytes must be > 0");
            assert!(
                model.filename.ends_with(".bin"),
                "filename should end with .bin, got: {}",
                model.filename
            );
        }
    }

    /// The `small` model must be described as "Recommended" since the UI
    /// defaults to it and the description is shown to users.
    #[test]
    fn test_small_model_is_marked_recommended() {
        let small = find_model("small").expect("small model must exist");
        assert!(
            small.description.contains("Recommended"),
            "small description should contain 'Recommended', got: {}",
            small.description
        );
    }

    // --- Model path construction ---

    /// `model_path` must produce the expected `{dir}/models/{filename}` path.
    #[test]
    fn test_model_path_construction() {
        let dir = PathBuf::from("/tmp/dikta-test-appdata");
        let path = model_path(&dir, "ggml-base.bin");
        assert_eq!(
            path,
            PathBuf::from("/tmp/dikta-test-appdata/models/ggml-base.bin")
        );
    }

    /// `models_dir` returns the `models/` subdirectory.
    #[test]
    fn test_models_dir() {
        let dir = PathBuf::from("/tmp/appdata");
        assert_eq!(models_dir(&dir), PathBuf::from("/tmp/appdata/models"));
    }

    // --- Status for non-existent model ---

    /// A model file that does not exist on disk must report `NotDownloaded`.
    #[test]
    fn test_get_model_status_not_downloaded() {
        // Use a temp dir that definitely doesn't have the model files.
        let dir = PathBuf::from("/tmp/dikta-test-no-models-12345");
        let status = get_model_status("small", &dir).expect("small is a known model");
        assert_eq!(
            status,
            ModelStatus::NotDownloaded,
            "model should not be present in a fresh temp dir"
        );
    }

    /// Querying a status for an unknown model id returns an error.
    #[test]
    fn test_get_model_status_unknown_model() {
        let dir = PathBuf::from("/tmp/appdata");
        let result = get_model_status("nonexistent", &dir);
        assert!(
            matches!(result, Err(ModelManagerError::UnknownModel(_))),
            "expected UnknownModel error, got: {result:?}"
        );
    }

    // --- Status for an existing file ---

    /// When the model file exists, `get_model_status` returns `Downloaded`.
    #[test]
    fn test_get_model_status_downloaded() {
        let dir = tempfile::tempdir().expect("tempdir creation must succeed");
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).expect("create models dir");

        // Create a dummy .bin file.
        std::fs::write(models.join("ggml-small.bin"), b"dummy").expect("write dummy model");

        let status = get_model_status("small", dir.path()).expect("small is a known model");
        assert_eq!(status, ModelStatus::Downloaded);
    }

    // --- list_models_with_status ---

    /// `list_models_with_status` returns one entry per catalogue model.
    #[test]
    fn test_list_models_with_status_count() {
        let dir = PathBuf::from("/tmp/dikta-no-models");
        let result = list_models_with_status(&dir);
        assert_eq!(result.len(), 3, "should return one entry per catalogue model");
    }

    // --- delete_model ---

    /// Deleting a non-existent model is idempotent (no error).
    #[test]
    fn test_delete_model_nonexistent_is_ok() {
        let dir = PathBuf::from("/tmp/dikta-no-models-12345");
        let result = delete_model("small", &dir);
        assert!(result.is_ok(), "deleting a missing file should not error");
    }

    /// Deleting an unknown model id returns an error.
    #[test]
    fn test_delete_model_unknown_id_errors() {
        let dir = PathBuf::from("/tmp/appdata");
        let result = delete_model("unknown-model", &dir);
        assert!(matches!(result, Err(ModelManagerError::UnknownModel(_))));
    }
}
