//! Reads v1 `config.json` and splits it into settings + API-keys.
//!
//! Settings are held as a raw `serde_json::Value` object because v1 has
//! ~80 fields with deprecated tombstones and platform-conditional defaults
//! — typing them in Phase 0 would pre-pin the v2 settings schema. The
//! Phase-1 writer maps v1 field names to v2 settings via a rename table.
//! See `docs/migration/v1-to-v2.md`.

use std::path::Path;

use super::{V1ApiKeys, V1ImportWarning, keys};

const CONFIG_FILE: &str = "config.json";

/// v1 settings — the raw object after api-key fields have been removed.
#[derive(Debug, Clone)]
pub struct V1Settings {
    /// Raw settings object with `groqApiKey` / `deepseekApiKey` / `openaiApiKey`
    /// / `anthropicApiKey` / `openrouterApiKey` stripped.
    pub raw: serde_json::Map<String, serde_json::Value>,
    /// Always `true` when constructed by `load` — the five key fields have been
    /// moved to `V1ApiKeys`. A `false` value would indicate the struct was
    /// constructed outside the loader and keys might still be present; any
    /// writer that accepts `V1Settings` should assert `keys_stripped`.
    pub keys_stripped: bool,
}

/// Load `<appdata>/config.json`. Returns `(settings, api_keys)`.
///
/// - Missing file → `(None, V1ApiKeys::empty())` + `FileMissing` warning.
/// - Invalid JSON or non-object root → `(None, V1ApiKeys::empty())` + `ParseError` warning.
pub fn load(appdata: &Path, warnings: &mut Vec<V1ImportWarning>) -> (Option<V1Settings>, V1ApiKeys) {
    let path = appdata.join(CONFIG_FILE);
    if !path.exists() {
        warnings.push(V1ImportWarning::FileMissing { file: CONFIG_FILE });
        return (None, V1ApiKeys::empty());
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: CONFIG_FILE,
                detail: e.to_string(),
            });
            return (None, V1ApiKeys::empty());
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: CONFIG_FILE,
                detail: e.to_string(),
            });
            return (None, V1ApiKeys::empty());
        }
    };

    let mut obj = match value {
        serde_json::Value::Object(o) => o,
        other => {
            warnings.push(V1ImportWarning::ParseError {
                file: CONFIG_FILE,
                detail: format!("root is not an object (found {})", json_kind(&other)),
            });
            return (None, V1ApiKeys::empty());
        }
    };

    let api_keys = keys::extract_from_object(&mut obj);
    let settings = V1Settings {
        raw: obj,
        keys_stripped: true,
    };
    (Some(settings), api_keys)
}

fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_import::test_util::tempdir;
    use secrecy::ExposeSecret;

    #[test]
    fn missing_file_yields_file_missing_warning() {
        let tmp = tempdir();
        let mut warnings = Vec::new();
        let (s, k) = load(tmp.path(), &mut warnings);
        assert!(s.is_none());
        assert!(k.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            V1ImportWarning::FileMissing { file: CONFIG_FILE }
        ));
    }

    #[test]
    fn valid_file_splits_keys_and_settings() {
        let tmp = tempdir();
        std::fs::write(
            tmp.path().join(CONFIG_FILE),
            r#"{
                "groqApiKey": "gsk_xx",
                "deepseekApiKey": "",
                "language": "de",
                "cleanupStyle": "polished",
                "hotkeySlots": [{"hotkey":"ctrl+d","mode":"hold","insertAndSend":false}]
            }"#,
        )
        .unwrap();

        let mut warnings = Vec::new();
        let (settings, keys) = load(tmp.path(), &mut warnings);
        assert!(warnings.is_empty());

        let keys = keys;
        assert_eq!(keys.groq.as_ref().unwrap().expose_secret(), "gsk_xx");
        assert!(keys.deepseek.is_none(), "empty-string should normalize to None");

        let s = settings.unwrap();
        assert!(s.keys_stripped);
        assert!(!s.raw.contains_key("groqApiKey"));
        assert!(!s.raw.contains_key("deepseekApiKey"));
        assert_eq!(s.raw.get("language").and_then(|v| v.as_str()), Some("de"));
        assert_eq!(
            s.raw.get("cleanupStyle").and_then(|v| v.as_str()),
            Some("polished")
        );
        assert!(s.raw.get("hotkeySlots").and_then(|v| v.as_array()).is_some());
    }

    #[test]
    fn invalid_json_yields_parse_error() {
        let tmp = tempdir();
        std::fs::write(tmp.path().join(CONFIG_FILE), b"{ not valid json").unwrap();
        let mut warnings = Vec::new();
        let (s, k) = load(tmp.path(), &mut warnings);
        assert!(s.is_none());
        assert!(k.is_empty());
        assert!(matches!(
            &warnings[0],
            V1ImportWarning::ParseError { file: CONFIG_FILE, .. }
        ));
    }

    #[test]
    fn non_object_root_yields_parse_error() {
        let tmp = tempdir();
        std::fs::write(tmp.path().join(CONFIG_FILE), b"[1, 2, 3]").unwrap();
        let mut warnings = Vec::new();
        let (s, k) = load(tmp.path(), &mut warnings);
        assert!(s.is_none());
        assert!(k.is_empty());
        match &warnings[0] {
            V1ImportWarning::ParseError { detail, .. } => assert!(detail.contains("array")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }
}
