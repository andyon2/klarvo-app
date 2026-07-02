# GATE-4 verdict — Story 12-1 (robust LLM/STT fallback ladder + pillbar status)

Date: 2026-07-02 · Conductor: interactive · Branch: conductor/epic-12 · Range: 6bc4971..32c42f0

## Self-verification (machine-produced, done)

- **Rust:** `cargo test --lib` → `test result: ok. 640 passed; 0 failed` (incl. new tests:
  transport-error retryable classification / inversion, non-retryable STT audio-preservation,
  fallback-provider selection). `cargo clippy` → no new warnings.
- **Android:** `./gradlew :app:testUniversalDebugUnitTest` → 153 tests, 0 failures (incl. new
  `LlmFallbackProviderTest` cases: Groq excluded from cleanup fallback; the actually-run substitute
  provider is excluded, not the configured name). `assembleUniversalDebug` → APK built. Theme in sync.
- **Frontend:** `npx tsc --noEmit` clean; `npm run build` succeeds.
- **Code-level path audit (conductor, Opus):** verified on both platforms — transport errors now
  fallback-eligible; Groq removed from cleanup fallback (Rust `resolve_fallback_provider` + Kotlin
  `cleanupFallbackCandidates`); STT→local-Whisper on retryable-exhausted only (Kotlin classifier
  co-located with and consistent with `transcribeWithRetry`'s throw messages; mirrors Rust
  `is_retryable_stt_error`); non-IOException cleanup failures degrade to raw text (Kotlin catches
  broadened to `Exception`); Windows pill now renders `payload.error` (was hardcoded "Error");
  warning-state safety timer added; happy-path WAV clone removed (trait borrows `&[u8]`).

## Structural window oracle — N/A for this story

The contract's unattended GATE (`dumpsys window windows` — overlay count/size/gravity/visibility)
asserts overlay-WINDOW STRUCTURE. Story 12-1 introduces **no overlay-window-structure change**: its
surface delta is (a) toast messages via the existing `showToast` and (b) a new transient text-STATE
in the existing FloatingBar pill (amber warning/error label). Neither changes window count, size, or
gravity. So the structural assertion has nothing new to check versus the 11-2 baseline; booting the
emulator would only re-confirm unchanged structure. No emulator smoke was run for this reason
(documented, not skipped silently).

## Residual for the human (Andi's real-machine batched gate — provably not machine-producible here)

The genuine gate for this feature is real fallback behaviour under a real provider outage + the
user actually SEEING the status. This needs a simulated outage and a human eye; it cannot be
produced from the unattended WSL side (no way to induce Andi's network outage; the real Xiaomi is
contract-detached from conductor runs). To verify:

1. **Windows:** with the app running, force a cleanup-provider outage (e.g. temporarily set an
   invalid DeepSeek key, or block `api.deepseek.com`), dictate → confirm: the amber status appears
   in the pill AND raw text is still pasted (no 30 s hang before degrade). If an OpenAI/OpenRouter
   key is set, confirm it falls back there first; otherwise confirm the raw-text status.
2. **Windows STT outage:** force a Groq (STT) outage → confirm the pill shows the terminal
   "✗ Transkription fehlgeschlagen — Audio gesichert" message (audio preserved for Story 12-2).
3. **Android real device:** same two scenarios → confirm the corresponding toasts appear
   (`⚠ …`) and behaviour matches Windows.
