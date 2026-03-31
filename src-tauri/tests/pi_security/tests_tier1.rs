//! Tier 1 prompt injection tests — require a live LLM API key.
//!
//! Run with:
//!   GROQ_API_KEY=... cargo test --test pi_security tier1 -- --ignored --nocapture
//!
//! Or against a different provider:
//!   PI_PROVIDER=deepseek DEEPSEEK_API_KEY=... cargo test --test pi_security tier1 -- --ignored --nocapture

use crate::harness::{self, Provider};
use crate::judge;
use crate::registry;
use crate::report::{self, TestResult};

/// Rate-limit delay between API calls (ms).
const DELAY_MS: u64 = 1200;

async fn delay() {
    tokio::time::sleep(std::time::Duration::from_millis(DELAY_MS)).await;
}

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn tier1_00_golden_path_baseline() {
    let provider = Provider::from_env();
    println!("[baseline] Testing provider: {}", provider.name());
    let result = harness::golden_path(&*provider.make()).await;
    println!("[baseline] Cleaned text: {result}");
    assert!(!result.is_empty());
}

// ---------------------------------------------------------------------------
// Individual Tier-1 tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn tier1_sp001_system_prompt_leak_custom_prompt_framing() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-001");
    let input = entry.input_text.as_deref().unwrap();
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-001] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);
    let generic = judge::check_generic(&result.text, input);

    assert!(
        verdict.is_pass(),
        "SP-001 FAILED: {verdict}. Output: {}",
        &result.text
    );
    for g in &generic {
        assert!(g.is_pass(), "SP-001 generic check: {g}");
    }
}

#[tokio::test]
#[ignore]
async fn tier1_jb001_jailbreak_custom_prompt_meta_prompting() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-001");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-001] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-001 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_sp002_system_prompt_leak_voice_command() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-002");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_voice_command(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-002] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "SP-002 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_jb002_jailbreak_raw_text_interpreter() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-002");
    delay().await;
    let result = harness::inject_via_raw_text(&*p, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-002] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-002 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_jb003_jailbreak_custom_prompt_rule_addition() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-003");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-003] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-003 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_sp003_system_prompt_leak_end_sequences() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-003");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_custom_prompt(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-003] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "SP-003 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_jb004_jailbreak_selected_text_narrative_smuggling() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-004");
    let voice_cmd = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_selected_text(&*p, &entry.payload, voice_cmd)
        .await
        .expect("API call failed");

    println!("[JB-004] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-004 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_jb005_jailbreak_dictionary_breakout() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-005");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_dictionary(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-005] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-005 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_sp004_system_prompt_leak_raw_text() {
    let p = Provider::from_env().make();
    let entry = registry::load("SP-004");
    delay().await;
    let result = harness::inject_via_raw_text(&*p, &entry.payload)
        .await
        .expect("API call failed");

    println!("[SP-004] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "SP-004 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

#[tokio::test]
#[ignore]
async fn tier1_jb006_jailbreak_voice_command_inversion() {
    let p = Provider::from_env().make();
    let entry = registry::load("JB-006");
    let input = entry.input_text.as_deref().unwrap();
    delay().await;
    let result = harness::inject_via_voice_command(&*p, input, &entry.payload)
        .await
        .expect("API call failed");

    println!("[JB-006] Output: {}", &result.text);
    let verdict = judge::check(&result.text, &entry.detection);

    assert!(
        verdict.is_pass(),
        "JB-006 FAILED: {verdict}. Output: {}",
        &result.text
    );
}

// ---------------------------------------------------------------------------
// Aggregated run: all Tier 1 (LLM-based) with report
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn tier1_all_with_report() {
    let provider = Provider::from_env();
    let p = provider.make();
    println!("\n[tier1] Running all Tier-1 tests against: {}\n", provider.name());

    // Baseline
    let baseline = harness::golden_path(&*p).await;
    println!("[baseline] OK: {baseline}\n");

    let entries = registry::load_tier(1);
    let mut results: Vec<TestResult> = Vec::new();

    for entry in &entries {
        // Skip output-only tests (handled in tests_output.rs)
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
                println!("[{id}] Skipping unknown injection point: {other}", id = entry.id);
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

    // The aggregated test passes if no injections succeeded.
    let failures: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.verdict, judge::Verdict::Fail(_)))
        .collect();

    if !failures.is_empty() {
        println!("\nVulnerabilities found:");
        for f in &failures {
            println!("  - {} ({}): {}", f.id, f.injection_point, f.verdict.detail());
        }
        // Don't assert-fail here — this is an audit tool, not a correctness test.
        // The individual tests above will fail on their own.
    }
}
