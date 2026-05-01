//! Reads v1 `dictionary.json` — a simple `{ "terms": [String] }` file.

use std::path::Path;

use serde::Deserialize;

use super::V1ImportWarning;

const DICTIONARY_FILE: &str = "dictionary.json";

/// v1 user-maintained term list.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct V1Dictionary {
    pub terms: Vec<String>,
}

/// Load `<appdata>/dictionary.json`. `None` on missing-file, unreadable,
/// or invalid-JSON — each case also pushes a warning.
pub fn load(appdata: &Path, warnings: &mut Vec<V1ImportWarning>) -> Option<V1Dictionary> {
    let path = appdata.join(DICTIONARY_FILE);
    if !path.exists() {
        warnings.push(V1ImportWarning::FileMissing {
            file: DICTIONARY_FILE,
        });
        return None;
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: DICTIONARY_FILE,
                detail: e.to_string(),
            });
            return None;
        }
    };

    match serde_json::from_str::<V1Dictionary>(&text) {
        Ok(d) => Some(d),
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: DICTIONARY_FILE,
                detail: e.to_string(),
            });
            None
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::v1_import::test_util::tempdir;

    #[test]
    fn missing_file_yields_file_missing_warning() {
        let tmp = tempdir();
        let mut warnings = Vec::new();
        let d = load(tmp.path(), &mut warnings);
        assert!(d.is_none());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            V1ImportWarning::FileMissing { file: DICTIONARY_FILE }
        ));
    }

    #[test]
    fn valid_file_parses_terms() {
        let tmp = tempdir();
        std::fs::write(
            tmp.path().join(DICTIONARY_FILE),
            br#"{"terms":["Kubernetes","TypeScript","Klarvo"]}"#,
        )
        .unwrap();
        let mut warnings = Vec::new();
        let d = load(tmp.path(), &mut warnings).unwrap();
        assert_eq!(warnings.len(), 0);
        assert_eq!(d.terms, vec!["Kubernetes", "TypeScript", "Klarvo"]);
    }

    #[test]
    fn invalid_json_yields_parse_error() {
        let tmp = tempdir();
        std::fs::write(tmp.path().join(DICTIONARY_FILE), b"{ not valid").unwrap();
        let mut warnings = Vec::new();
        let d = load(tmp.path(), &mut warnings);
        assert!(d.is_none());
        assert!(matches!(
            &warnings[0],
            V1ImportWarning::ParseError { file: DICTIONARY_FILE, .. }
        ));
    }

    #[test]
    fn empty_terms_array_is_valid() {
        let tmp = tempdir();
        std::fs::write(tmp.path().join(DICTIONARY_FILE), br#"{"terms":[]}"#).unwrap();
        let mut warnings = Vec::new();
        let d = load(tmp.path(), &mut warnings).unwrap();
        assert!(warnings.is_empty());
        assert!(d.terms.is_empty());
    }
}
