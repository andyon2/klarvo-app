//! Tauri commands for the custom dictionary (user word list).

use tauri::State;

use crate::dictionary::save_dictionary;
use crate::AppState;

/// Returns all terms in the custom dictionary.
#[tauri::command]
pub fn get_dictionary_terms(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = crate::lock!(state.inner().dictionary)?;
    Ok(guard.terms().to_vec())
}

/// Adds a term to the custom dictionary and persists the change.
///
/// Duplicate terms (case-insensitive) and empty strings are silently ignored.
#[tauri::command]
pub fn add_dictionary_term(state: State<'_, AppState>, term: String) -> Result<(), String> {
    let inner = state.inner();
    let mut dict = crate::lock!(inner.dictionary)?;
    dict.add_term(term);
    let dict_clone = dict.clone();
    drop(dict);
    save_dictionary(&inner.app_data_dir, &dict_clone)
        .map_err(|e| format!("Failed to save dictionary: {e}"))
}

/// Removes a term from the custom dictionary and persists the change.
///
/// Does nothing if the term is not present.
#[tauri::command]
pub fn remove_dictionary_term(state: State<'_, AppState>, term: String) -> Result<(), String> {
    let inner = state.inner();
    let mut dict = crate::lock!(inner.dictionary)?;
    dict.remove_term(&term);
    let dict_clone = dict.clone();
    drop(dict);
    save_dictionary(&inner.app_data_dir, &dict_clone)
        .map_err(|e| format!("Failed to save dictionary: {e}"))
}
