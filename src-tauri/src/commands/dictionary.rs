//! Tauri commands for the custom dictionary (user word list).

use tauri::State;

use crate::dictionary::save_dictionary;
use crate::license::{is_feature_allowed, LicensedFeature};
use crate::AppState;

/// Maximum number of dictionary terms allowed for free-tier users.
///
/// Paid users (Licensed or active GracePeriod) have no limit.
pub const FREE_DICTIONARY_LIMIT: usize = 20;

/// Returns all terms in the custom dictionary.
#[tauri::command]
pub fn get_dictionary_terms(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = crate::lock!(state.inner().dictionary)?;
    Ok(guard.terms().to_vec())
}

/// Adds a term to the custom dictionary and persists the change.
///
/// Duplicate terms (case-insensitive) and empty strings are silently ignored.
///
/// Free-tier users are limited to [`FREE_DICTIONARY_LIMIT`] terms. Attempting
/// to exceed the limit returns an error that the frontend can parse to show
/// an upgrade prompt.
#[tauri::command]
pub fn add_dictionary_term(state: State<'_, AppState>, term: String) -> Result<(), String> {
    add_dictionary_term_inner(state.inner(), term)
}

/// Inner implementation of [`add_dictionary_term`] that works directly on
/// [`AppState`], making it testable without a live Tauri context.
pub(crate) fn add_dictionary_term_inner(inner: &AppState, term: String) -> Result<(), String> {
    // Check license status before acquiring the dictionary lock so we hold
    // the minimum number of locks at once.
    let is_paid = {
        let status = crate::lock!(inner.license_status)?;
        is_feature_allowed(&status, LicensedFeature::UnlimitedDictionary)
    };

    let mut dict = crate::lock!(inner.dictionary)?;

    // Enforce the free-tier limit before inserting.
    if !is_paid {
        // The limit only applies to genuinely new terms. We peek at whether the
        // term would be a duplicate (same logic as Dictionary::add_term) so we
        // don't reject users who are just re-submitting an existing word.
        let trimmed = term.trim().to_string();
        if !trimmed.is_empty() {
            let lower = trimmed.to_lowercase();
            let is_duplicate = dict.terms().iter().any(|t| t.to_lowercase() == lower);
            if !is_duplicate && dict.len() >= FREE_DICTIONARY_LIMIT {
                return Err(format!(
                    "Free dictionary limit reached ({FREE_DICTIONARY_LIMIT} terms). \
                     Upgrade to Dikta License for unlimited terms."
                ));
            }
        }
    }

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::LicenseStatus;
    use crate::test_helpers::{make_state, temp_dir};

    // Helper: fill the dictionary of `state` with exactly `n` distinct terms.
    fn fill_dictionary(state: &AppState, n: usize) {
        let mut dict = state.dictionary.lock().unwrap();
        for i in 0..n {
            dict.add_term(format!("Term{i}"));
        }
    }

    // Helper: set the in-memory license status on `state`.
    fn set_license(state: &AppState, status: LicenseStatus) {
        *state.license_status.lock().unwrap() = status;
    }

    // --- Free-tier limit ---

    #[test]
    fn test_free_user_can_add_up_to_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        // Unlicensed by default; fill to exactly FREE_DICTIONARY_LIMIT - 1.
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT - 1);

        // Adding the 20th term must succeed.
        let result = add_dictionary_term_inner(&state, "FinalTerm".to_string());
        assert!(result.is_ok(), "20th term must be accepted: {result:?}");
        assert_eq!(state.dictionary.lock().unwrap().len(), FREE_DICTIONARY_LIMIT);
    }

    #[test]
    fn test_free_user_blocked_at_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        // The 21st term must be rejected.
        let result = add_dictionary_term_inner(&state, "OverLimit".to_string());
        assert!(result.is_err(), "21st term must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.contains("Free dictionary limit reached"),
            "Error must mention the limit, got: {err}"
        );
        assert!(
            err.contains("Upgrade to Dikta License"),
            "Error must mention upgrade path, got: {err}"
        );
    }

    #[test]
    fn test_free_user_duplicate_not_counted_against_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        // Re-submitting an existing term (different casing) must not hit the limit check.
        let result = add_dictionary_term_inner(&state, "TERM0".to_string()); // "Term0" exists
        assert!(
            result.is_ok(),
            "Re-submitting a duplicate must not be blocked: {result:?}"
        );
        // Dictionary size stays at the limit -- duplicate was silently ignored.
        assert_eq!(
            state.dictionary.lock().unwrap().len(),
            FREE_DICTIONARY_LIMIT
        );
    }

    #[test]
    fn test_free_user_empty_term_not_counted_against_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        // Empty / whitespace-only strings must be silently ignored without hitting the limit.
        let result = add_dictionary_term_inner(&state, "   ".to_string());
        assert!(
            result.is_ok(),
            "Whitespace-only term must not be rejected at limit: {result:?}"
        );
    }

    // --- Paid user: no limit ---

    #[test]
    fn test_paid_user_can_exceed_free_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        set_license(&state, LicenseStatus::Licensed);
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        // Paid user must be able to add beyond the free limit.
        let result = add_dictionary_term_inner(&state, "BeyondLimit".to_string());
        assert!(
            result.is_ok(),
            "Paid user must not be blocked at limit: {result:?}"
        );
        assert_eq!(
            state.dictionary.lock().unwrap().len(),
            FREE_DICTIONARY_LIMIT + 1
        );
    }

    #[test]
    fn test_grace_period_user_can_exceed_free_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        // Active grace period: expires far in the future.
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + 86_400; // 24 hours from now
        set_license(&state, LicenseStatus::GracePeriod { until: future });
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        let result = add_dictionary_term_inner(&state, "GraceTerm".to_string());
        assert!(
            result.is_ok(),
            "User in active grace period must not be blocked: {result:?}"
        );
    }

    #[test]
    fn test_expired_grace_period_user_is_blocked_at_limit() {
        let dir = temp_dir();
        let state = make_state(&dir);
        // Grace period expired (until timestamp is 1 -- way in the past).
        set_license(&state, LicenseStatus::GracePeriod { until: 1 });
        fill_dictionary(&state, FREE_DICTIONARY_LIMIT);

        let result = add_dictionary_term_inner(&state, "OverLimit".to_string());
        assert!(
            result.is_err(),
            "User with expired grace period must be blocked: {result:?}"
        );
    }

    // --- Constant value ---

    #[test]
    fn test_free_dictionary_limit_is_20() {
        assert_eq!(FREE_DICTIONARY_LIMIT, 20);
    }
}
