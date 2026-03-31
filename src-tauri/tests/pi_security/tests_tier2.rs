//! Tier 2 prompt injection tests — evasion-wrapped variants.
//!
//! Run with:
//!   GROQ_API_KEY=... cargo test --test pi_security tier2 -- --ignored --nocapture

use crate::harness::{self, Provider};
use crate::judge;
use crate::registry;
use crate::report::{self, TestResult};

const DELAY_MS: u64 = 1200;

async fn delay() {
    tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
}

// ---------------------------------------------------------------------------
// Individual Tier-2 tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn tier2_sp005_base64_evasion() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-005");
    let input = entry.input_text.as_deref().unwrap();
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-005] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "SP-005 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb007_alt_language_evasion() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-007");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-007] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-007 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb008_json_wrapper_evasion() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-008");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-008] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-008 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb009_cognitive_overload_dictionary() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-009");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_dictionary(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-009] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-009 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb010_contradiction() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-010");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-010] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-010 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_sp006_hex_evasion() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-006");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-006] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "SP-006 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb011_russian_doll() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-011");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-011] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-011 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb012_variable_expansion() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-012");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-012] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-012 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_sp007_reformat_leak() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-007");
    delay().await;
    let result = harness::inject_via_raw_text(&*p, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-007] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "SP-007 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb013_case_changing() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-013");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-013] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-013 FAILED: {verdict}");
}

#[tokio::test]
#[ignore]
async fn tier2_jb014_link_injection_voice_command() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-014");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_voice_command(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-014] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    assert!(verdict.is_pass(), "JB-014 FAILED: {verdict}");
}

// ---------------------------------------------------------------------------
// Aggregated Tier-2 run
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn tier2_all_with_report() {
    let provider = Provider::from_env();
    let p = provider.make();
    println!("\n[tier2] Running all Tier-2 tests against: {}\n", provider.name());

    let entries = registry::load_tier(2);
    let mut results: Vec<TestResult> = Vec::new();

    for entry in &entries {
        if entry.injection_point == "output_sanitization" {
            continue;
        }

        delay().await;

        let api_result = match entry.injection_point.as_str() {
            "custom_prompt" => {
                let input = entry.input_text.as_deref().unwrap();
                harness::inject_via_custom_prompt(&*p, input, &entry.payload).await
            }
            "raw_text" => harness::inject_via_raw_text(&*p, &entry.payload).await,
            "dictionary_terms" => {
                let input = entry.input_text.as_deref().unwrap();
                harness::inject_via_dictionary(&*p, input, &entry.payload).await
            }
            "voice_command" => {
                let input = entry.input_text.as_deref().unwrap();
                harness::inject_via_voice_command(&*p, input, &entry.payload).await
            }
            "selected_text" => {
                let cmd = entry.input_text.as_deref().unwrap();
                harness::inject_via_selected_text(&*p, &entry.payload, cmd).await
            }
            other => {
                println!("[{id}] Skipping unknown: {other}", id = entry.id);
                continue;
            }
        };

        let (verdict, output_preview) = match api_result {
            Ok(r) => {
                let v = judge::check(&r.text, &entry.detection);
                println!("[{}] {} — {}", entry.id, entry.name, v);
                (v, r.text)
            }
            Err(e) => {
                let v = judge::Verdict::Inconclusive(format!("API error: {e}"));
                println!("[{}] {} — {}", entry.id, entry.name, v);
                (v, String::new())
            }
        };

        results.push(TestResult {
            id: entry.id.clone(),
            name: entry.name.clone(),
            injection_point: entry.injection_point.clone(),
            intent: entry.taxonomy.intent.clone(),
            technique: entry.taxonomy.technique.clone(),
            evasion: entry.taxonomy.evasion.clone(),
            verdict,
            output_preview,
        });
    }

    report::print_summary(&results);
}
