# Golden-Vector Seeds — Story 7.3 (STT consolidation)

**Status:** Seeded in Story 7.3 inline tests. To be consolidated by Story 7.7 into the
cross-platform parity net fixture harness.

**Governing:** ADR-0017, AC3 / AC5 / AC6 / AC8

---

## H14 Whole-Word Match (AC3)

| Input | Expected | Fixture test |
|-------|----------|--------------|
| "Standard" | Pass (not blocked) | `test_h14_standard_not_blocked_single_word_ard` |
| "Milliarde" | Pass | `test_h14_milliarde_not_blocked` |
| "Hardware" | Pass | `test_h14_hardware_not_blocked` |
| "ZDF" | Blocked | `test_h14_zdf_still_blocked_whole_word` |
| "WDR" | Blocked | `test_h14_wdr_still_blocked` |
| "ARD" | Blocked | `test_h14_ard_standalone_still_blocked` |
| "Untertitelung des ZDF" | Blocked (multi-word substring) | `test_h14_multi_word_entry_substring_match_preserved` |

**Source:** `src-tauri/src/stt/hallucination.rs`, test module `tests`

---

## Stockphrase Ghost Family + Trailing Ghost (AC5)

| Input | Expected | Fixture test |
|-------|----------|--------------|
| "Groß- und Kleinschreibung, Satzzeichen und Interpunktion" | Blocked | `test_ac5_stockphrase_grosz_und_kleinschreibung_blocked` |
| "Klinge" | Blocked | `test_ac5_stockphrase_klinge_blocked` |
| "Klingel" | Blocked | `test_ac5_stockphrase_klinge_blocked` |
| "Groß- und Klinge" | Blocked | `test_ac5_grosz_und_klinge_short_clip_blocked` |
| "…<9-word-real-dictation> Groß- und Kleinschreibung" | Blocked (no word-count gate for stockphrases) | `test_ac5_trailing_ghost_on_long_clip_blocked` |
| "…<9-word-real-dictation> Klingel" | Blocked | `test_ac5_trailing_ghost_klingel_on_long_clip_blocked` |
| "[Musik]" | Blocked | `test_ac5_musik_descriptor_blocked` |

**Thresholds / Design decision:** `STOCKPHRASE_BLOCKLIST` entries are checked BEFORE the
word-count gate. This is intentional — these entries are distinctive enough to have no
false-positive on genuine dictation.

**Source:** `src-tauri/src/stt/hallucination.rs`, `STOCKPHRASE_BLOCKLIST` + `is_hallucination`

---

## Confidence-Drop thresholds — verbose_json (AC6, AC8)

**Named verifiability downgrade (AC8):** confidence-drop is NOT human-reproducible.
Human gate is deliberately downgraded to "fixture-verified". These thresholds must be
validated against real Groq `verbose_json` responses when available.

| Field | Threshold | Action |
|-------|-----------|--------|
| `no_speech_prob` | > 0.6 → DROP | High silence probability |
| `compression_ratio` | < 0.1 → DROP | Near-empty output |
| `avg_logprob` | < -1.0 → DROP | Very low token confidence |
| missing field | — → KEEP (fail-open) | Unknown = preserve segment |

**Boundary cases:**
- `no_speech_prob == 0.6` → KEEP (strict `>`, not `>=`)
- `avg_logprob == -1.0` → KEEP (strict `<`, not `<=`)

**Fixture tests (golden-vector):**
- `test_ac6_segment_drop_high_no_speech_prob`
- `test_ac6_segment_drop_low_compression_ratio`
- `test_ac6_segment_drop_low_avg_logprob`
- `test_ac6_segment_keep_good_confidence`
- `test_ac6_segment_missing_fields_fail_open`
- `test_ac6_extract_verbose_text_drops_low_confidence`
- `test_ac6_extract_verbose_text_both_shapes_tolerated`
- `test_ac6_no_speech_prob_boundary_exactly_06_is_kept`
- `test_ac6_avg_logprob_boundary_exactly_minus1_is_kept`

**Source:** `src-tauri/src/stt/mod.rs`, `TranscriptionSegment::should_drop` + `extract_verbose_text`

---

## Prompt Assembly (H3 / Recall #5 / L3 — AC1)

- German language + dictionary + no custom → `build_stt_prompt("terms", "de")` → contains
  "Deutsch" + dict terms. Verified by existing `test_build_stt_prompt_german_with_terms`.
- Custom hint overrides default language hint → `build_stt_prompt_with_hint(None, "de", Some("custom"))`.
- Temperature = 0.0 (deterministic) → single source in `WhisperStt.temperature` (L3 parity).

---

## Silence Filter Boundary Parity (AC4)

| Scenario | Input | Expected |
|----------|-------|---------|
| Exactly MIN_RECORDING_MS | duration=500, min=500 | Pass |
| One ms below MIN | duration=499, min=500 | TooShort |
| Exactly SILENCE_THRESHOLD | rms=0.005, threshold=0.005 | Pass |
| Below threshold | rms=0.004, threshold=0.005 | Silent |
| Malformed WAV (rms=None) | rms=None | Pass (skip RMS check) |

**Source:** `src-tauri/src/stt/groq_jni.rs` test module + `src-tauri/src/pipeline.rs::silence_skip`

---

## Weg A Runtime (R-001 Proof Gate)

- Throwaway `current_thread` Tokio runtime can be built: `test_r001_throwaway_tokio_runtime_can_be_built`
- Two sequential runtimes do not conflict: `test_r001_two_sequential_runtimes_do_not_conflict`

**Source:** `src-tauri/src/stt/groq_jni.rs` test module

---

## 7.7 Consolidation Notes

When Story 7.7 builds the full parity net:
1. Move these fixture tables into a TOML/JSON fixture format consumed by both Rust and Kotlin tests.
2. Add Android-JNI round-trip tests (device/emulator) for the `nativeTranscribe` path.
3. Add wiremock-based request-shape tests asserting `response_format=verbose_json` in the form.
4. Pin the confidence-drop thresholds against real recorded Groq `verbose_json` responses.
