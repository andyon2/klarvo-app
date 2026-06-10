---
title: 'Android License Enforcement — honest status + trial + alternative-provider gate'
type: 'bugfix'
created: '2026-06-10'
status: 'done'
baseline_commit: 'fa8f180'
context:
  - '{project-root}/docs/cross-platform-drift-audit.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-b1-launch-license-gates.md'
  - '{project-root}/_bmad-output/project-context.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Android treats any non-blank `licenseKey` as licensed (`KlarvoApi.isLicensed()`, which has zero call sites), performs no HMAC validation, and never starts the 14-day trial (`firstInstallAt` is never read or written on Android). Every premium feature ships ungated and a garbage key "passes" — a CRITICAL launch-blocker (audit finding C1).

**Approach:** Reuse the Rust offline license logic over a new JNI export (single source of truth — no Kotlin HMAC reimplementation, secret stays in the `.so`). Android computes the honest `LicenseStatus`, starts the trial from an Android-owned timestamp (SharedPreferences; `config.json` stays single-writer per ADR-0015), and gates the one premium chokepoint chosen for launch — alternative (non-Groq) STT/LLM providers — by forcing the free Groq tier when not Licensed/Trial/Grace. Core Groq dictation stays free (freemium parity with desktop).

## Boundaries & Constraints

**Always:**
- License validation math lives in Rust only; Android calls it via JNI (ADR-0016 single-source — the fix must not itself become a drift source).
- `config.json` stays single-writer (Tauri/desktop). Android persists its trial timestamp to SharedPreferences, never `config.json` (ADR-0015).
- Core Groq dictation + basic cleanup stays free regardless of license (freemium parity).
- Fail-soft AND fail-safe: any JNI/native error → treat as NOT-allowed (gate premium) but never block core dictation and never panic.
- Match desktop status semantics exactly: `Licensed` / `Trial{until}` / `GracePeriod{until}` / `Unlicensed` via `status_to_string`.

**Ask First:**
- Adding any gate beyond alternative providers (local whisper, dictionary, history, …) — those are deferred to the correct-course batch.
- Any change to `config.json` write ownership.

**Never:**
- Reimplement HMAC/trial math in Kotlin.
- Block core dictation for unlicensed users (this is freemium, not pay-to-use).
- Build an Android license-entry/activation UI (separate funnel gap — documented below, out of scope).
- Add a second writer to `config.json`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Valid HMAC key | `licenseSource=hmac`, valid key | status `Licensed` → non-Groq providers used unchanged | N/A |
| Active trial | no key, effective `firstInstallAt` within 14d | status `Trial` → alternative providers allowed | N/A |
| Expired trial | no key, `firstInstallAt` backdated >14d | status `Unlicensed` → non-Groq forced to Groq + one-time notice; Groq dictation works | N/A |
| Garbage key (THE core fix) | `licenseSource=hmac`, key `"INVALID"` | HMAC fails → `Unlicensed` → gated (previously: treated as licensed) | N/A |
| LS cached | `licenseSource=lemon_squeezy`, cache within window | `Licensed`/`GracePeriod` per `compute_status_from_cache_ls` | N/A |
| Android-only first run | no `firstInstallAt` in config.json | stamp `androidFirstRunAt` in SharedPrefs → trial clock starts | N/A |
| Native error | `loadLibrary` fails / native throws | treat as `Unlicensed` (gate premium); core Groq dictation unaffected; log | fail-soft, no panic |

</frozen-after-approval>

## Code Map

- `src-tauri/src/license/mod.rs` — license logic. Has `validate_license_key`, `compute_status_from_cache`, `compute_status_from_cache_ls`, `compute_trial_status`, `status_to_string`, `is_feature_allowed`. Extract a `compute_cached_status(...)` reused by boot + JNI.
- `src-tauri/src/lib.rs:345-357` — boot status branch (hmac-cache / LS-cache / trial). Refactor to call the new `compute_cached_status`.
- `src-tauri/src/stt/jni_bridge.rs` — existing JNI export pattern (`#[no_mangle] extern "system" Java_com_klarvo_voice_*`, panic-safe, fallback returns). Home for the new license export.
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — `Config` (license fields 69-72) + `readConfig` (213-288) + `isLicensed()` (158-164) + `resolveLlmProvider` (93-147) + STT provider resolution (250-274).
- `android/kotlin-src/com/klarvo/voice/LocalWhisperInference.kt` — Kotlin `external fun` / `System.loadLibrary("klarvo_lib")` reference pattern.
- `android/kotlin-src/com/klarvo/voice/MainActivity.kt` — first-run entry; stamp `androidFirstRunAt`.

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/license/mod.rs` -- extract `pub fn compute_cached_status(license_key: &str, license_source: &str, ls_instance_id: &str, ls_last_validated_at: u64, license_validated_at: u64, first_install_at: u64) -> LicenseStatus` mirroring lib.rs:345-357 exactly; add `#[cfg(test)]` cases for every I/O Matrix scenario (incl. inversion: garbage key → `Unlicensed`, expired trial → `Unlicensed`).
- [x] `src-tauri/src/lib.rs` -- replace the inline boot branch (~345-357) with a `compute_cached_status(...)` call. No behavior change; guarded by existing trial tests (1554/1570/1592).
- [x] `src-tauri/src/license/jni.rs` (new, android-gated; registered as `pub mod jni;` in `license/mod.rs`) -- add `#[no_mangle] pub extern "system" fn Java_com_klarvo_voice_LicenseValidator_nativeComputeStatus(env, class, key, source, lsInstanceId, lsLastValidatedAt: jlong, licenseValidatedAt: jlong, firstInstallAt: jlong) -> jstring` returning `status_to_string(compute_cached_status(...))`; panic-safe, return `"unlicensed"` on any error (mirrors `stt/jni_bridge.rs` fallback style). Kept in the license module (not stt/jni_bridge) so the JNI surface lives with the logic it exposes.
- [x] `android/kotlin-src/com/klarvo/voice/LicenseValidator.kt` (new) -- object loading `klarvo_lib`; `external fun nativeComputeStatus(...): String`; `computeStatus(...)` parses `"licensed"|"trial:ts"|"grace_period:ts"|"unlicensed"` into a sealed status; `isAllowed()` = Licensed || Trial(until>now) || Grace(until>now); fail-soft to NOT-allowed on native error.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` -- add `licenseValidatedAt`+`firstInstallAt` to `Config`/`readConfig`; replace `isLicensed()` body with `LicenseValidator.isAllowed(...)`; after `readConfig`, if `!isAllowed` and the resolved STT/LLM provider != `"groq"`, override both to `"groq"` and emit a one-time non-blocking notice.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (`readConfig`, not MainActivity) -- on config read, if SharedPrefs `androidFirstRunAt` is absent, stamp = now (unix seconds); `effectiveFirstInstall = firstInstallAtJson.takeIf { it > 0 } ?: androidFirstRunAt`. Placed in `readConfig` (which has the `Context` and runs on the overlay path too) rather than `MainActivity`; the spec permitted "or KlarvoApi init". Limitation: a fresh install with no config.json returns early (no keys → nothing runs anyway), so the clock starts on the first read where config.json exists.

**Acceptance Criteria:**
- Given a config.json with a garbage non-blank `licenseKey` (source `hmac`), when the app resolves providers, then status is `Unlicensed` and a configured non-Groq provider is forced to Groq.
- Given no license and `androidFirstRunAt` within 14 days, when dictating, then status is `Trial` and alternative providers work.
- Given `firstInstallAt` backdated >14 days and no key, when dictating, then core Groq dictation still works and any non-Groq provider is downgraded to Groq with a one-time notice.
- Given the native lib fails to load, when resolving, then the app does not crash, treats the user as not-allowed (Groq forced), and core dictation still functions.
- Given a valid HMAC dev key, when resolving, then status is `Licensed` and the non-Groq provider is used unchanged.

## Spec Change Log

- **2026-06-10 (step-04 review, patches — no loopback):**
  - *Trial anchor changed from SharedPreferences to `PackageManager.firstInstallTime`.* Triggering findings: blind-hunter (HIGH) "SharedPreferences trial clock is reset by Clear-data → renewable trial" + edge-hunter (MED) "SharedPreferences stamp not set when config.json absent → trial measured from 2nd read." Known-bad state avoided: a trivially-renewable trial and an imprecise trial start. The OS install time survives Clear-data (only uninstall resets it), needs no write (still ADR-0015-safe), and is available immediately. This supersedes the parenthetical "SharedPreferences" mechanism named in the frozen Intent — same intent (Android-owned trial start, not config.json), better mechanism; flagged for human ratification at presentation. KEEP: config.json `firstInstallAt` still takes precedence when >0 (shared cross-device trial timeline).
  - *STT gate switched from denylist (`== "openai"`) to allowlist (any non-Groq/non-local).* Triggering finding: blind + edge "STT denylist lets future paid STT providers slip past unlicensed." Known-bad avoided: an asymmetric gate where a new paid STT value bypasses enforcement. KEEP: `local` (OfflineMode) is still left untouched as a separately-deferred gate.

## Design Notes

- **Single source of truth:** `compute_cached_status` is the ONE status function; boot (lib.rs) and the JNI export both call it. No Kotlin HMAC. This applies the ADR-0016 anti-drift principle to the fix itself.
- **Trial-timestamp split:** `config.json firstInstallAt` is desktop-written (ADR-0015 single-writer); Android can't write it, so the Android-only trial clock uses `PackageManager.firstInstallTime` (the OS install time — survives "Clear data", only an uninstall resets it; no write needed). A synced desktop `firstInstallAt` (>0) takes precedence so a desktop user's trial isn't reset on Android. (Superseded the original SharedPreferences plan during review — see Spec Change Log.)
- **Known limitation (OUT OF SCOPE — flagged for the checkpoint):** Android has no license-entry/activation UI, so an Android-only user cannot activate on-device; alternative providers stay locked after trial until activation on desktop (config sync). This funnel gap is a separate product story (onboarding/sync), not this fix.

## Verification

**Commands:**
- `cargo test --lib license` -- expected: `compute_cached_status` scenarios green incl. inversion (garbage key → Unlicensed, expired trial → Unlicensed).
- `cargo check` -- expected: boot refactor compiles on the default target.
- `scripts/android-build.sh` -- expected: fresh APK builds with the new JNI export + `LicenseValidator` (NDK build of `libklarvo_lib.so`); timestamp gate confirms freshness.

**Manual checks (on-device — the testability producer):**
- `adb push` three config.json variants, run `scripts/android-smoke.sh` for each:
  - (a) garbage `licenseKey` → non-Groq provider downgraded to Groq, dictation works.
  - (b) `firstInstallAt` backdated >14d, no key → trial expired, Groq-only, dictation works.
  - (c) valid HMAC dev key (`.dev-keys`, payload `andyon`) → status Licensed, non-Groq provider used.
- Freshness via `android-build.sh` timestamp gate + APK filename (no in-UI version screen).

## Suggested Review Order

**Shared status logic (start here)**

- The design heart: one offline-status function both platforms call — no drift by construction.
  [`mod.rs:406`](../../src-tauri/src/license/mod.rs#L406)

- Desktop boot now delegates to it (was an inline branch) — proves the reuse, no behavior change.
  [`lib.rs:351`](../../src-tauri/src/lib.rs#L351)

**Cross-language reuse (JNI)**

- The Android-only JNI export wrapping the shared fn; fail-safe to `"unlicensed"`, never panics.
  [`jni.rs:44`](../../src-tauri/src/license/jni.rs#L44)

- Module registered unconditionally; body gated to Android by inner attribute (mirrors `stt/jni_bridge`).
  [`mod.rs:42`](../../src-tauri/src/license/mod.rs#L42)

**Android consumer + the gate (highest blast radius)**

- The gate: trial anchored on OS install time, then non-Groq provider keys stripped when not allowed.
  [`KlarvoApi.kt:284`](../../android/kotlin-src/com/klarvo/voice/KlarvoApi.kt#L284)

- Kotlin status parser; fail-safe deny on native-unavailable/throw/malformed string.
  [`LicenseValidator.kt:72`](../../android/kotlin-src/com/klarvo/voice/LicenseValidator.kt#L72)

- `isLicensed` repurposed: was "any non-blank key passes" (the bug), now delegates to the real status.
  [`KlarvoApi.kt:164`](../../android/kotlin-src/com/klarvo/voice/KlarvoApi.kt#L164)

**Tests (peripheral)**

- The core-fix inversion + trial-boundary + LS-branch cases (Linux-runnable).
  [`mod.rs:1204`](../../src-tauri/src/license/mod.rs#L1204)

## On-Device Smoke Result — 2026-06-10 (GREEN, Andy gate passed)

Real device (WiFi adb, `100.112.41.70`), Claude-driven from WSL. Full
`android-build.sh --clean` (fresh `libklarvo_lib.so` carrying the new export
`Java_com_klarvo_voice_LicenseValidator_nativeComputeStatus`, verified via
`llvm-nm` on the installed APK) → `android-smoke.sh` (debug, debuggable APK,
24 JVM tests green, versionName gate passed).

Method: four `config.json` variants pushed via `adb push /data/local/tmp` +
`run-as cp` into `/data/data/com.klarvo.voice/`, `am force-stop` between each
(cache invalidation), one human dictation gesture per state, gate decision read
from logcat line `[license] Not licensed/trial -- alternative providers gated`
(tag `KlarvoApi`).

| State | config | Expected | logcat verdict | Paste |
|-------|--------|----------|----------------|-------|
| a | garbage key + `firstInstallAt` 2020 (trial expired) | UNLICENSED, deepseek→groq | gate line present ✓ | came ✓ |
| b | blank key + trial expired | UNLICENSED, deepseek→groq | gate line present ✓ | came ✓ |
| c | valid key `KLARVO-MFXG-…` | LICENSED, deepseek kept | no gate line ✓ | came ✓ |
| d | blank key + trial active (~10d) | LICENSED (trial) | no gate line ✓ | came ✓ |

JNI confirmed live: `LicenseValidator: Native library libklarvo_lib loaded for
license validation` in every round (no fail-safe-deny artifact). Device config
restored to original afterward.

**Testability note:** the trial clock uses `effectiveFirstInstall =
config.firstInstallAt if >0 else OS firstInstallTime` — the config value wins, so
trial-expired is reproducible by pushing a backdated `firstInstallAt` without
touching OS install time. This is what made a 4-state on-device smoke possible
without reinstall/time-travel (verification symmetry satisfied).
