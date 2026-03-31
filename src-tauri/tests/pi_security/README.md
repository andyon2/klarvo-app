# Prompt Injection Security Tests

Automated PI security test suite based on the [Arcanum PI Taxonomy](https://github.com/Arcanum-Sec/arc_pi_taxonomy).

## Quick Start

```bash
# Output sanitization tests (no API key needed):
cargo test --test pi_security output

# Full Tier-1 audit with report:
GROQ_API_KEY=... cargo test --test pi_security tier1_all -- --ignored --nocapture

# Full Tier-2 audit:
GROQ_API_KEY=... cargo test --test pi_security tier2_all -- --ignored --nocapture

# Different provider:
PI_PROVIDER=deepseek DEEPSEEK_API_KEY=... cargo test --test pi_security -- --ignored --nocapture
```

## Architecture

```
payloads.json    — Test cases: Arcanum taxonomy → injection surface → payload → detection
harness.rs       — Adapters per injection surface (calls CleanupProvider directly)
judge.rs         — Heuristic detectors: prompt leak, behavior deviation, extraneous content, unsafe output
registry.rs      — JSON loader
report.rs        — Terminal summary with pass/fail/inconclusive
tests_tier1.rs   — 12 high-priority tests
tests_tier2.rs   — 15 evasion-wrapped variants
tests_output.rs  — 5 output sanitization checks (offline)
```

## Injection Surfaces Tested

| Surface | Adapter | Risk |
|---------|---------|------|
| custom_prompt | `inject_via_custom_prompt()` | Self-injection (user controls Settings) |
| raw_text | `inject_via_raw_text()` | Self-injection (user speaks) |
| dictionary_terms | `inject_via_dictionary()` | Self-injection (user edits dict) |
| voice_command | `inject_via_voice_command()` | Self-injection (user speaks command) |
| selected_text | `inject_via_selected_text()` | **External** (document/email/website content) |
| LLM output | Output sanitization tests | **External** (LLM response) |

## Adding Test Cases

1. Add entry to `payloads.json`:
```json
{
  "id": "JB-099",
  "name": "descriptive_name",
  "tier": 1,
  "taxonomy": { "intent": "jailbreak", "technique": "framing", "evasion": null },
  "injection_point": "custom_prompt",
  "payload": "the actual injection payload",
  "input_text": "benign text to clean up",
  "detection": {
    "strategy": "extraneous_content",
    "forbidden_in_output": ["MARKER_STRING"]
  }
}
```

2. Add test function (optional — aggregated tests auto-discover new entries)

## Reuse for Other Apps

Copy everything except `harness.rs`. Write new adapter functions that call the target app's LLM API.
