//! Output sanitization tests — no API key required.
//!
//! These tests verify that known-dangerous byte sequences would be detected
//! by the judge. They do NOT test the actual paste handler (which currently
//! has no sanitization), but establish the detection baseline.

use crate::judge::{self, Verdict};
use crate::registry;

#[test]
fn out001_ansi_escape_codes() {
    let entry = registry::load("OUT-001");
    let verdict = judge::check(&entry.payload, &entry.detection);
    assert!(
        matches!(verdict, Verdict::Fail(_)),
        "OUT-001: ANSI escapes should be detected as unsafe. Got: {verdict}"
    );
}

#[test]
fn out002_unicode_rtl_override() {
    let entry = registry::load("OUT-002");
    let verdict = judge::check(&entry.payload, &entry.detection);
    assert!(
        matches!(verdict, Verdict::Fail(_)),
        "OUT-002: RTL override should be detected as unsafe. Got: {verdict}"
    );
}

#[test]
fn out003_null_bytes() {
    let entry = registry::load("OUT-003");
    let verdict = judge::check(&entry.payload, &entry.detection);
    assert!(
        matches!(verdict, Verdict::Fail(_)),
        "OUT-003: Null bytes should be detected as unsafe. Got: {verdict}"
    );
}

#[test]
fn out004_unicode_bidi_attack() {
    let entry = registry::load("OUT-004");
    let verdict = judge::check(&entry.payload, &entry.detection);
    assert!(
        matches!(verdict, Verdict::Fail(_)),
        "OUT-004: Bidi control chars should be detected as unsafe. Got: {verdict}"
    );
}

#[test]
fn out005_clean_text_passes() {
    // Verify that normal text does not trigger false positives.
    let clean = "I went to the store yesterday and bought some groceries.";
    let det = registry::Detection::UnsafeOutput {
        patterns: vec!["\\x1b".to_string()],
    };
    let verdict = judge::check(clean, &det);
    assert!(
        verdict.is_pass(),
        "Clean text should pass unsafe output check. Got: {verdict}"
    );
}
