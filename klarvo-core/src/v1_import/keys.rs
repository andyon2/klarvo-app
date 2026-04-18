//! API-key extraction from v1 `config.json`.
//!
//! Keys are held as `SecretString` so they redact in `Debug`/`Display` and
//! cannot be accidentally serialized. See ADR-0004 §2.

use secrecy::SecretString;

/// All five v1 API-key fields, each optional and redacted.
///
/// Empty-string values in v1 (the default when no key is set) are
/// normalized to `None`.
#[derive(Debug, Default)]
pub struct V1ApiKeys {
    pub groq: Option<SecretString>,
    pub deepseek: Option<SecretString>,
    pub openai: Option<SecretString>,
    pub anthropic: Option<SecretString>,
    pub openrouter: Option<SecretString>,
}

impl V1ApiKeys {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.groq.is_none()
            && self.deepseek.is_none()
            && self.openai.is_none()
            && self.anthropic.is_none()
            && self.openrouter.is_none()
    }
}

/// Remove the five v1 api-key fields from the given JSON object and return
/// them as `V1ApiKeys`. Any non-string or empty-string value is discarded.
///
/// Mutates `obj` — after this call, the object no longer contains
/// `groqApiKey`, `deepseekApiKey`, `openaiApiKey`, `anthropicApiKey`,
/// `openrouterApiKey`.
pub(super) fn extract_from_object(obj: &mut serde_json::Map<String, serde_json::Value>) -> V1ApiKeys {
    V1ApiKeys {
        groq: take_nonempty_string(obj, "groqApiKey"),
        deepseek: take_nonempty_string(obj, "deepseekApiKey"),
        openai: take_nonempty_string(obj, "openaiApiKey"),
        anthropic: take_nonempty_string(obj, "anthropicApiKey"),
        openrouter: take_nonempty_string(obj, "openrouterApiKey"),
    }
}

fn take_nonempty_string(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<SecretString> {
    match obj.remove(key)? {
        serde_json::Value::String(s) if !s.is_empty() => Some(SecretString::from(s)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use serde_json::json;

    #[test]
    fn extracts_all_five_keys_and_strips_from_object() {
        let mut value = json!({
            "groqApiKey": "gsk_abc",
            "deepseekApiKey": "sk_def",
            "openaiApiKey": "sk-ghi",
            "anthropicApiKey": "sk-ant-jkl",
            "openrouterApiKey": "or_mno",
            "language": "de",
        });
        let obj = value.as_object_mut().unwrap();
        let keys = extract_from_object(obj);

        assert_eq!(keys.groq.as_ref().unwrap().expose_secret(), "gsk_abc");
        assert_eq!(keys.deepseek.as_ref().unwrap().expose_secret(), "sk_def");
        assert_eq!(keys.openai.as_ref().unwrap().expose_secret(), "sk-ghi");
        assert_eq!(
            keys.anthropic.as_ref().unwrap().expose_secret(),
            "sk-ant-jkl"
        );
        assert_eq!(keys.openrouter.as_ref().unwrap().expose_secret(), "or_mno");

        // Non-key fields remain.
        assert_eq!(obj.get("language").and_then(|v| v.as_str()), Some("de"));
        // Key fields removed.
        assert!(!obj.contains_key("groqApiKey"));
        assert!(!obj.contains_key("deepseekApiKey"));
        assert!(!obj.contains_key("openaiApiKey"));
        assert!(!obj.contains_key("anthropicApiKey"));
        assert!(!obj.contains_key("openrouterApiKey"));
    }

    #[test]
    fn empty_string_values_are_normalized_to_none() {
        let mut value = json!({
            "groqApiKey": "",
            "deepseekApiKey": "sk_real",
        });
        let keys = extract_from_object(value.as_object_mut().unwrap());
        assert!(keys.groq.is_none());
        assert_eq!(keys.deepseek.as_ref().unwrap().expose_secret(), "sk_real");
    }

    #[test]
    fn missing_fields_yield_none() {
        let mut value = json!({});
        let keys = extract_from_object(value.as_object_mut().unwrap());
        assert!(keys.is_empty());
    }

    #[test]
    fn non_string_values_are_ignored() {
        let mut value = json!({
            "groqApiKey": 42,
            "deepseekApiKey": null,
            "openaiApiKey": "real",
        });
        let keys = extract_from_object(value.as_object_mut().unwrap());
        assert!(keys.groq.is_none());
        assert!(keys.deepseek.is_none());
        assert_eq!(keys.openai.as_ref().unwrap().expose_secret(), "real");
    }

    #[test]
    fn debug_impl_redacts_secret() {
        let keys = V1ApiKeys {
            groq: Some(SecretString::from("gsk_supersecret".to_string())),
            ..V1ApiKeys::empty()
        };
        let dbg = format!("{keys:?}");
        assert!(!dbg.contains("gsk_supersecret"));
    }
}
