//! Integration tests for the v1→v2 migration path (ADR-0004).
//!
//! Happy-path uses the committed snapshot at `test-assets/v1-appdata/`
//! (accessed via `klarvo-test-fixtures`). Fail-mode tests build synthetic
//! AppData directories in tempdirs.

use std::path::{Path, PathBuf};

use klarvo_core::v1_import::{V1ImportWarning, load_from_path};
use klarvo_test_fixtures::v1_appdata;
use secrecy::ExposeSecret;

// ---- tempdir helper (avoids pulling tempfile just for fail-mode tests) ----

struct TempDir(PathBuf);
impl TempDir {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn tempdir(label: &str) -> TempDir {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("klarvo-v1-import-it-{label}-{pid}-{n}"));
    std::fs::create_dir_all(&path).expect("create tempdir");
    TempDir(path)
}

// ---- Happy path ----

#[test]
fn happy_path_loads_all_sections_from_fixture() {
    let fixture = v1_appdata::fixture_path();
    let bundle = load_from_path(&fixture);

    assert!(
        bundle.warnings.is_empty(),
        "no warnings expected on valid fixture, got: {:?}",
        bundle.warnings
    );
    assert!(bundle.is_nonempty());

    // history: 3 rows in a known order.
    let history = bundle.history.expect("history");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].id, 1);
    assert_eq!(
        history[0].text,
        "Meeting notes about the upcoming deployment and rollback plan."
    );
    assert_eq!(history[0].style, "polished");
    assert_eq!(history[0].language, "en");
    assert!(!history[0].is_note);
    assert_eq!(history[0].app_name.as_deref(), Some("Notepad"));
    assert_eq!(history[0].uuid.as_deref(), Some("fixture-0000-0001"));
    // Row 2 is a voice note.
    assert!(history[1].is_note);
    assert_eq!(history[1].style, "verbatim");
    assert_eq!(history[1].language, "de");
    assert!(history[1].app_name.is_none());
    // Row 3 has no raw_text.
    assert!(history[2].raw_text.is_none());
    assert_eq!(history[2].app_name.as_deref(), Some("VS Code"));

    // usage: 1 STT + 1 LLM.
    let usage = bundle.usage.expect("usage");
    assert_eq!(usage.len(), 2);
    let stt = usage.iter().find(|u| u.service == "groq_stt").expect("stt");
    assert_eq!(stt.audio_duration_ms, Some(4200));
    assert!(stt.prompt_tokens.is_none());
    let llm = usage
        .iter()
        .find(|u| u.service == "deepseek_cleanup")
        .expect("llm");
    assert_eq!(llm.prompt_tokens, Some(180));
    assert_eq!(llm.completion_tokens, Some(95));

    // dictionary.
    let dict = bundle.dictionary.expect("dictionary");
    assert_eq!(dict.terms.len(), 4);
    assert!(dict.terms.contains(&"Kubernetes".to_string()));
    assert!(dict.terms.contains(&"DeepSeek".to_string()));

    // settings: key fields removed, others preserved.
    let settings = bundle.settings.expect("settings");
    assert!(settings.keys_stripped);
    assert!(!settings.raw.contains_key("groqApiKey"));
    assert!(!settings.raw.contains_key("deepseekApiKey"));
    assert!(!settings.raw.contains_key("openrouterApiKey"));
    assert_eq!(
        settings.raw.get("language").and_then(|v| v.as_str()),
        Some("de")
    );
    assert_eq!(
        settings.raw.get("cleanupStyle").and_then(|v| v.as_str()),
        Some("polished")
    );
    assert!(
        settings
            .raw
            .get("hotkeySlots")
            .and_then(|v| v.as_array())
            .is_some()
    );

    // api keys: 3 set, 2 empty (normalized to None).
    let groq = bundle.api_keys.groq.as_ref().expect("groq set");
    assert!(groq.expose_secret().starts_with("gsk_test_FIXTURE_NOT_REAL_"));
    assert!(bundle.api_keys.deepseek.is_some());
    assert!(bundle.api_keys.openrouter.is_some());
    assert!(bundle.api_keys.openai.is_none());
    assert!(bundle.api_keys.anthropic.is_none());
}

// ---- Fail modes (per ADR-0004 §4) ----

#[test]
fn fail_mode_missing_appdata_emits_single_warning_and_empty_bundle() {
    let nowhere = PathBuf::from("/tmp/klarvo-v1-does-not-exist-integration");
    assert!(
        !nowhere.exists(),
        "test precondition: the path must not exist"
    );
    let bundle = load_from_path(&nowhere);

    assert!(!bundle.is_nonempty());
    assert_eq!(bundle.warnings.len(), 1);
    assert!(matches!(
        &bundle.warnings[0],
        V1ImportWarning::AppDataMissing { .. }
    ));
    assert!(bundle.history.is_none());
    assert!(bundle.usage.is_none());
    assert!(bundle.settings.is_none());
    assert!(bundle.dictionary.is_none());
    assert!(bundle.api_keys.is_empty());
}

#[test]
fn fail_mode_corrupted_db_keeps_other_sections_loadable() {
    let tmp = tempdir("corrupt-db");
    // Corrupted history.db (random bytes, not a real SQLite file).
    std::fs::write(tmp.path().join("history.db"), b"not a real sqlite file").unwrap();
    // Valid dictionary.json and config.json so we can confirm they load independently.
    std::fs::write(
        tmp.path().join("dictionary.json"),
        br#"{"terms":["alpha","beta"]}"#,
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("config.json"),
        br#"{"language":"en","groqApiKey":"gsk_x"}"#,
    )
    .unwrap();

    let bundle = load_from_path(tmp.path());

    // History/usage failed — but dictionary + settings + keys came through.
    assert!(bundle.history.is_none());
    assert!(bundle.usage.is_none());
    let dict = bundle.dictionary.expect("dictionary should still load");
    assert_eq!(dict.terms, vec!["alpha", "beta"]);
    let settings = bundle.settings.expect("settings should still load");
    assert_eq!(
        settings.raw.get("language").and_then(|v| v.as_str()),
        Some("en")
    );
    assert!(bundle.api_keys.groq.is_some());

    // Warnings: at least one ParseError pointing at history.db.
    assert!(
        bundle
            .warnings
            .iter()
            .any(|w| matches!(w, V1ImportWarning::ParseError { file: "history.db", .. })),
        "expected ParseError for history.db, got: {:?}",
        bundle.warnings
    );
}

#[test]
fn fail_mode_invalid_json_keeps_db_loadable() {
    let tmp = tempdir("bad-json");
    // Copy the real fixture db so history/usage still produce rows.
    let fixture_db = v1_appdata::fixture_path().join("history.db");
    std::fs::copy(&fixture_db, tmp.path().join("history.db")).unwrap();
    // Invalid JSON in both config and dictionary.
    std::fs::write(tmp.path().join("config.json"), b"{ not valid json here").unwrap();
    std::fs::write(tmp.path().join("dictionary.json"), b"[also broken").unwrap();

    let bundle = load_from_path(tmp.path());

    // JSON sections failed.
    assert!(bundle.settings.is_none());
    assert!(bundle.dictionary.is_none());
    assert!(bundle.api_keys.is_empty());

    // DB section still works — history + usage loaded normally.
    let history = bundle.history.expect("history should still load");
    assert_eq!(history.len(), 3);
    let usage = bundle.usage.expect("usage should still load");
    assert_eq!(usage.len(), 2);

    // Warnings: two ParseErrors, one per bad file.
    let parse_errors: Vec<&V1ImportWarning> = bundle
        .warnings
        .iter()
        .filter(|w| matches!(w, V1ImportWarning::ParseError { .. }))
        .collect();
    assert_eq!(
        parse_errors.len(),
        2,
        "expected 2 ParseErrors (config + dictionary), got: {:?}",
        bundle.warnings
    );
    assert!(
        bundle
            .warnings
            .iter()
            .any(|w| matches!(w, V1ImportWarning::ParseError { file: "config.json", .. }))
    );
    assert!(
        bundle
            .warnings
            .iter()
            .any(|w| matches!(w, V1ImportWarning::ParseError { file: "dictionary.json", .. }))
    );
}
