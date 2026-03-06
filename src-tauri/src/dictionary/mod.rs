//! Custom dictionary module.
//!
//! Manages a user-maintained word list (technical terms, names, brand names)
//! that should be transcribed and preserved correctly throughout the pipeline.
//!
//! Persistence: JSON file at `{app_data_dir}/dictionary.json`.
//! Chosen over SQLite for MVP simplicity -- a list of strings doesn't need
//! a relational schema, and JSON is human-editable.
//!
//! ## How terms are used in the pipeline
//!
//! 1. **Groq Whisper `prompt` parameter** (`terms_as_prompt`): fed to the
//!    STT API as a context hint. Whisper uses this to improve transcription
//!    accuracy for rare words (max 224 tokens).
//!
//! 2. **DeepSeek system prompt** (`terms_as_list`): injected into the LLM
//!    cleanup prompt so the model preserves the exact spelling of technical
//!    terms even when fixing grammar.

use std::path::Path;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Dictionary struct
// ---------------------------------------------------------------------------

/// A sorted, deduplicated list of user-defined terms.
///
/// Invariants maintained by the mutation methods:
/// - No duplicate terms (case-insensitive check on insert, exact removal).
/// - Terms are stored in insertion order (no automatic sorting) so the user
///   sees them in the order they added them.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Dictionary {
    terms: Vec<String>,
}

impl Dictionary {
    /// Creates an empty dictionary.
    pub fn new() -> Self {
        Dictionary::default()
    }

    /// Adds a term if it is not already present (case-insensitive duplicate check).
    ///
    /// Leading/trailing whitespace is trimmed. Empty strings are silently ignored.
    pub fn add_term(&mut self, term: String) {
        let trimmed = term.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        // Case-insensitive duplicate check.
        let lower = trimmed.to_lowercase();
        let already_exists = self.terms.iter().any(|t| t.to_lowercase() == lower);
        if !already_exists {
            self.terms.push(trimmed);
        }
    }

    /// Removes the first term that matches exactly (case-sensitive).
    ///
    /// Does nothing if the term is not present.
    pub fn remove_term(&mut self, term: &str) {
        self.terms.retain(|t| t != term);
    }

    /// Returns a reference to the full list of terms.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    /// Returns the terms as a comma-separated string suitable for the Groq
    /// Whisper `prompt` parameter.
    ///
    /// Example output: `"Kubernetes, TypeScript, Dikta"`
    ///
    /// Returns an empty string if the dictionary is empty.
    pub fn terms_as_prompt(&self) -> String {
        self.terms.join(", ")
    }

    /// Returns the terms as a comma-separated string suitable for injection
    /// into the DeepSeek system prompt.
    ///
    /// Identical to `terms_as_prompt` for now; kept as a separate method so
    /// the formatting can diverge if needed (e.g. bullet-point list for LLM).
    pub fn terms_as_list(&self) -> String {
        self.terms.join(", ")
    }

    /// Returns `true` if the dictionary contains no terms.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Returns the number of terms in the dictionary.
    pub fn len(&self) -> usize {
        self.terms.len()
    }
}

// ---------------------------------------------------------------------------
// File name
// ---------------------------------------------------------------------------

const DICTIONARY_FILE: &str = "dictionary.json";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads the dictionary from `{app_data_dir}/dictionary.json`.
///
/// Returns an empty `Dictionary` if the file does not exist or cannot be
/// parsed. Never panics.
pub fn load_dictionary(app_data_dir: &Path) -> Dictionary {
    let path = app_data_dir.join(DICTIONARY_FILE);

    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<Dictionary>(&contents) {
            Ok(dict) => dict,
            Err(e) => {
                log::warn!("[dictionary] Failed to parse dictionary.json ({e}), using empty dictionary");
                Dictionary::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("[dictionary] dictionary.json not found, starting with empty dictionary");
            Dictionary::default()
        }
        Err(e) => {
            log::warn!("[dictionary] Failed to read dictionary.json ({e}), using empty dictionary");
            Dictionary::default()
        }
    }
}

/// Saves the dictionary to `{app_data_dir}/dictionary.json`.
///
/// Creates the directory if it does not exist.
///
/// # Errors
/// Returns an error if the directory cannot be created, the file cannot be
/// written, or serialization fails.
pub fn save_dictionary(app_data_dir: &Path, dictionary: &Dictionary) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;

    let path = app_data_dir.join(DICTIONARY_FILE);
    let contents = serde_json::to_string_pretty(dictionary)?;

    std::fs::write(&path, contents)?;

    log::debug!(
        "[dictionary] Saved {} terms to {}",
        dictionary.len(),
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    // --- Unit tests for Dictionary methods ---

    #[test]
    fn test_new_dictionary_is_empty() {
        let dict = Dictionary::new();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_add_term_stores_term() {
        let mut dict = Dictionary::new();
        dict.add_term("Kubernetes".to_string());
        assert_eq!(dict.terms(), &["Kubernetes"]);
    }

    #[test]
    fn test_add_term_deduplicates_case_insensitive() {
        let mut dict = Dictionary::new();
        dict.add_term("Kubernetes".to_string());
        dict.add_term("kubernetes".to_string()); // duplicate, different case
        dict.add_term("KUBERNETES".to_string()); // another duplicate
        assert_eq!(dict.len(), 1, "duplicates should be ignored");
        assert_eq!(dict.terms()[0], "Kubernetes"); // original casing preserved
    }

    #[test]
    fn test_add_term_trims_whitespace() {
        let mut dict = Dictionary::new();
        dict.add_term("  TypeScript  ".to_string());
        assert_eq!(dict.terms(), &["TypeScript"]);
    }

    #[test]
    fn test_add_term_ignores_empty_string() {
        let mut dict = Dictionary::new();
        dict.add_term(String::new());
        dict.add_term("   ".to_string()); // whitespace only
        assert!(dict.is_empty());
    }

    #[test]
    fn test_remove_term_removes_exact_match() {
        let mut dict = Dictionary::new();
        dict.add_term("Dikta".to_string());
        dict.add_term("TypeScript".to_string());
        dict.remove_term("Dikta");
        assert_eq!(dict.terms(), &["TypeScript"]);
    }

    #[test]
    fn test_remove_term_noop_if_not_found() {
        let mut dict = Dictionary::new();
        dict.add_term("Dikta".to_string());
        dict.remove_term("NonExistent");
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_remove_term_is_case_sensitive() {
        let mut dict = Dictionary::new();
        dict.add_term("Dikta".to_string());
        dict.remove_term("dikta"); // different case -- should NOT match
        assert_eq!(dict.len(), 1, "remove_term is case-sensitive");
    }

    #[test]
    fn test_terms_as_prompt_empty_dictionary() {
        let dict = Dictionary::new();
        assert_eq!(dict.terms_as_prompt(), "");
    }

    #[test]
    fn test_terms_as_prompt_single_term() {
        let mut dict = Dictionary::new();
        dict.add_term("Kubernetes".to_string());
        assert_eq!(dict.terms_as_prompt(), "Kubernetes");
    }

    #[test]
    fn test_terms_as_prompt_multiple_terms() {
        let mut dict = Dictionary::new();
        dict.add_term("Kubernetes".to_string());
        dict.add_term("TypeScript".to_string());
        dict.add_term("Dikta".to_string());
        assert_eq!(dict.terms_as_prompt(), "Kubernetes, TypeScript, Dikta");
    }

    #[test]
    fn test_terms_as_list_equals_terms_as_prompt() {
        let mut dict = Dictionary::new();
        dict.add_term("Rust".to_string());
        dict.add_term("Tauri".to_string());
        // Both methods produce identical output (may diverge in future).
        assert_eq!(dict.terms_as_list(), dict.terms_as_prompt());
    }

    // --- Persistence tests ---

    #[test]
    fn test_load_missing_file_returns_empty() {
        let dir = temp_dir();
        let dict = load_dictionary(dir.path());
        assert!(dict.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = temp_dir();

        let mut original = Dictionary::new();
        original.add_term("Kubernetes".to_string());
        original.add_term("TypeScript".to_string());
        original.add_term("Dikta".to_string());

        save_dictionary(dir.path(), &original).expect("save should succeed");

        let loaded = load_dictionary(dir.path());
        assert_eq!(loaded, original);
    }

    #[test]
    fn test_save_creates_directory() {
        let dir = temp_dir();
        let nested = dir.path().join("nested").join("data");

        save_dictionary(&nested, &Dictionary::new()).expect("save into nested dir should succeed");
        assert!(nested.join("dictionary.json").exists());
    }

    #[test]
    fn test_load_corrupt_file_returns_empty() {
        let dir = temp_dir();
        fs::write(dir.path().join("dictionary.json"), b"{{invalid json}}").unwrap();

        let dict = load_dictionary(dir.path());
        assert!(dict.is_empty(), "corrupt file should yield empty dictionary");
    }

    #[test]
    fn test_save_empty_dictionary() {
        let dir = temp_dir();
        save_dictionary(dir.path(), &Dictionary::new()).unwrap();

        let loaded = load_dictionary(dir.path());
        assert!(loaded.is_empty());
    }
}
