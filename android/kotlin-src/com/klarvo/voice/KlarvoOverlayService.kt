package com.klarvo.voice

import android.Manifest
import android.app.*
import android.content.*
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.os.*
import android.util.DisplayMetrics
import android.view.*
import android.widget.Toast
import androidx.core.content.ContextCompat
import java.io.File
import java.io.IOException
import java.util.concurrent.Executors
import kotlin.math.abs

/**
 * Foreground Service that manages the floating bubble overlay.
 *
 * Keyboard detection -- two-tier approach:
 *   PRIMARY:  KlarvoAccessibilityService calls onKeyboardVisibilityChanged() whenever
 *             it detects a TYPE_INPUT_METHOD window appearing/disappearing system-wide.
 *             This is the most reliable mechanism and works in all apps.
 *   FALLBACK: If the accessibility service is not active, we fall back to polling
 *             InputMethodManager.getInputMethodWindowVisibleHeight() via reflection.
 *
 * Bubble visibility modes (stored in SharedPreferences):
 *   KEYBOARD_ONLY (default): bubble appears only when the soft keyboard is visible.
 *   ALWAYS_VISIBLE: bubble is always on screen, regardless of keyboard state.
 *
 * Recording modes (switchable via notification action):
 *   HOLD:     Tap -> TAP surface [✗·waveform·➤]; Long-press -> HOLD Cancel surface (PTT,
 *               Story 9-14 re-scope 2026-07-01): hold=record, release anywhere=send,
 *               release on the ✗ Abbrechen target=cancel (no Sperren/lock target)
 *   TOGGLE:   Tap -> start (red circle), Tap again -> stop + process
 *   AUTOSTOP: Tap -> start, auto-stops after silence detected
 *   AUTO:     Tap -> start loop, auto-stops on silence then restarts, Tap -> stop loop
 *
 * Touch gestures in RECORDING state (ADR-0019 / Story 9.5):
 *   Tap bubble          -> Senden: stopAndProcessRecording() (all modes, all gestures)
 *   Red square (panel)  -> Abbrechen: cancelRecording() (discard audio, no paste)
 *   Drag                -> moves the bubble (drag threshold still applies)
 */
class KlarvoOverlayService : Service() {

    // -----------------------------------------------------------------------
    // Pure delivery / retry decision TYPES (story 13-2).
    //
    // Declared at class level, not inside the companion: a type nested in a
    // companion object is `KlarvoOverlayService.Companion.X` to a caller, and
    // the JVM tests that drive these decisions should name them the way the
    // shipped `RecordingState` is named.
    // -----------------------------------------------------------------------

    /**
     * What Step 4 of [processAudio] does with the finished text.
     *
     * @param paste            call `pasteIntoFocusedField()`.
     * @param showCopiedToast  show the short "Copied: …" toast.
     * @param success          the run really delivered — the only state
     *                         that earns the DONE checkmark
     *                         (see [terminalStateFor]).
     */
    data class DeliveryDecision(
        val paste: Boolean,
        val showCopiedToast: Boolean,
        val success: Boolean
    )

    /**
     * The verdict [Companion.classifySttSentinel] returns for one
     * `__ERROR_*` sentinel from `GroqSttBridge.nativeTranscribe`
     * (story 13-2, D9 / D-M5).
     *
     * Pure, so the whole ladder is JVM-testable; only the backoff loop itself
     * stays on-device. The sentinels are emitted by the real Rust mapping
     * (`stt/groq_jni.rs`), never forged in Kotlin — ADR-0017.
     */
    enum class SttVerdict { SUCCESS, RETRYABLE, NON_RETRYABLE }

    /** [Companion.classifySttSentinel]'s answer: the verdict plus the message to carry. */
    data class SttSentinel(val verdict: SttVerdict, val message: String)

    companion object {
        private const val TAG = "KlarvoOverlayService"

        private const val CHANNEL_ID    = "klarvo_overlay"
        private const val NOTIFICATION_ID = 1
        private const val PREFS_NAME    = "klarvo_bubble_prefs"
        private const val PREF_X        = "bubble_x"
        private const val PREF_Y        = "bubble_y"
        private const val PREF_SIDE     = "bubble_side"  // "left" or "right"

        // Keyboard jump-up: fixed nav-bar clearance in px (AR5d — do NOT use WindowInsetsCompat
        // or env(safe-area-inset-bottom); those are unreliable for overlay Services on API 24+)
        private const val NAV_BAR_CLEARANCE_PX = 56

        /** SharedPreference key: if true the bubble is always visible, not just when keyboard is open. */
        const val PREF_ALWAYS_VISIBLE = "bubble_always_visible"

        /** BroadcastReceiver actions. */
        const val ACTION_TOGGLE_BUBBLE = "com.klarvo.voice.TOGGLE_BUBBLE"

        // Debug harness — registered only in debug builds (BuildConfig.DEBUG).
        // Drive all four bubble states on-device without live audio/network:
        //   adb connect 100.112.41.70:5555
        //   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle
        //   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Hello world"
        //   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state transcribing --ef rms 0.2 --es transcript "Hello world"
        //   adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state done
        private const val ACTION_DEBUG_SET_STATE = "com.klarvo.voice.DEBUG_SET_STATE"
        private const val EXTRA_STATE      = "state"        // "idle"|"recording"|"transcribing"|"done"
        private const val EXTRA_RMS        = "rms"          // Float 0.0–1.0 (synthetic amplitude)
        private const val EXTRA_TRANSCRIPT = "transcript"   // String (synthetic raw text)
        private const val EXTRA_HOLD_MODE  = "hold_mode"   // Boolean: true → HOLD dock in recording state

        // Keyboard detection: poll InputMethodManager at this interval (ms)
        private const val KEYBOARD_CHECK_INTERVAL = 300L

        // Long-press threshold -- after this delay a held touch becomes push-to-talk
        private const val LONG_PRESS_TIMEOUT_MS = 500L

        // Debounce delay before showing the bubble. Gives checkForegroundBankingApp()
        // time to detect and block, preventing the brief "flash" that banking apps catch.
        private const val SHOW_DEBOUNCE_MS = 150L

        // Base bubble size in dp -- multiplied by config.bubbleSize scale factor
        private const val BASE_BUBBLE_SIZE_DP = 56

        /**
         * Story 11-3 (AC-3a, item 3, Task 5.1): fixed height of the listening-panel
         * WindowManager window — replaces the pre-11-3 WRAP_CONTENT + 200dp-minimum combination
         * that let the window grow unbounded with accumulated preview text (the root cause of
         * the "fills the screen" usability blocker). Reuses that same 200dp value as the sole,
         * fixed height (Task 4.2 first-pass proposal) — device-tunable at GATE-4.
         */
        private const val PANEL_FIXED_HEIGHT_DP = 200

        // Transparent padding around the visual squircle (as a fraction of the visual size) so the
        // soft drop shadow + outer ring render without being clipped by the overlay window bounds.
        // Floored so the window always meets the ≥48dp touch target even at the smallest size.
        private const val SHADOW_PAD_FRACTION = 0.22f
        private const val MIN_SHADOW_PAD_DP   = 8

        /** Live reference used by KlarvoAccessibilityService for paste. */
        var instance: KlarvoOverlayService? = null

        /**
         * Code-review fix F2 (2026-07-01, pure/testable): guards whether
         * [ListeningPanelView.applyAppearance] may be called at all. `applyAppearance` restyles
         * the listening panel's stock look (translucent bg, teal border, font size/family) --
         * it must only run when the user actually opted into Live Preview (AC-4: byte-identical
         * to pre-11-2 behavior when disabled; also keeps Auto/AutoStop visuals untouched, since
         * those modes never enable preview per [RecordingMode.shouldInstallPreviewFlush]).
         * Pure function -- no Android Context needed -- same testable-shape pattern as
         * [RecordingMode.shouldInstallPreviewFlush].
         */
        fun shouldApplyPreviewAppearance(livePreviewEnabled: Boolean): Boolean = livePreviewEnabled

        /**
         * Code-review fix P1 (11-3): sanitizes an incoming preview STT chunk before it is
         * accumulated in [appendPreviewText]. Two problems this closes:
         *  - The accumulated preview text is newline-joined per chunk (Task 4.2). If a single
         *    STT chunk itself contains an embedded newline, that would introduce a spurious
         *    extra line break. Embedded newlines are collapsed to a single space here so `"\n"`
         *    stays the ONLY inter-chunk separator.
         *  - A blank/whitespace-only chunk would otherwise still get joined in, wasting a
         *    transcript line on an empty entry.
         * Returns the cleaned chunk, or `null` if there is nothing worth appending (caller should
         * skip the append entirely in that case).
         */
        fun sanitizePreviewChunk(text: String): String? {
            val cleaned = text.replace(Regex("\\n+"), " ").trim()
            return cleaned.ifBlank { null }
        }

        /**
         * M2 (AC5, Story 7-2): resolves `minRecordingMs` for the pre-STT `nativeSilenceCheck`
         * JNI call from the cached config, falling back to `500L` (matching
         * [KlarvoApi.Config]'s own default) if config is somehow absent -- not a behavior
         * change from the previous hardcoded literal, just no longer ignoring a configured
         * value. Extracted (same testable-shape pattern as [sanitizePreviewChunk]) so a JVM
         * test can verify a non-default config value actually changes the resolved value,
         * without needing the native JNI call itself.
         */
        fun resolveMinRecordingMsForSilenceFilter(config: KlarvoApi.Config?): Long =
            config?.minRecordingMs ?: 500L

        /**
         * The message shown when cleanup failed and the raw transcript was left
         * in the clipboard (Story 7-10, Q2/Q5).
         *
         * Mirrors the desktop pill's generic degrade wording
         * (`pipeline::degrade_warn_msg`) MINUS its trailing "(Ctrl+V)" key hint:
         * GATE 2 (Andi, 2026-09-14) dropped the hint on Android because there is
         * no Ctrl+V on a phone. The cause-first half is identical on both
         * platforms, and the model-not-found form
         * (`Model '<id>' not found — in clipboard`) carries no key hint on either
         * platform, so it stays identical to the pill.
         *
         * Kept as a named constant so the parity with
         * `pipeline::degrade_warn_msg` is greppable from both sides.
         *
         * Recorded divergence (7-10 re-review, Andi 2026-09-14): the desktop's
         * second literal `pipeline::terminal_degrade_msg` ("Cleanup failed —
         * clipboard write failed", shown when the clipboard write itself fails
         * on the degrade path) has NO Kotlin twin. `copyToClipboard` has no
         * try/catch yet (deferred-work.md), so Android cannot observe that
         * failure; the twin follows once it can. Registered in docs/backlog.md.
         */
        const val CLEANUP_FAILED_CLIPBOARD_MSG =
            "Cleanup failed — raw text in clipboard"

        /**
         * Android has no on-device LLM cleanup provider.
         *
         * Kotlin twin of `pipeline::local_cleanup_available()`, which is
         * `cfg!(target_os = "windows")`. Per grundsatzurteil G3b the Android
         * local path is not built in this epic, so the constant is `false` and
         * [skipsCloudCleanup] turns a stored `llmProvider = "local"` into
         * "no cleanup" rather than a silent cloud call.
         */
        const val LOCAL_CLEANUP_AVAILABLE = false

        /**
         * The provider id that means "on-device", for BOTH chains.
         *
         * One literal, three G2a guards ([skipsCloudCleanup],
         * [RecordingMode.shouldInstallPreviewFlush], `flushPreviewDelta`'s
         * flush-time re-check). It was spelled out three times in the story
         * whose thesis is one rule, so a provider-id rename would have
         * disabled two of the three and left the third reporting green
         * (review finding). Twin of the `"local"` the Rust `offline_rule` and
         * `preview_flush_should_install` compare against.
         */
        const val LOCAL_PROVIDER_ID = "local"

        /**
         * **The** offline rule (story 13-2, E2 / G2a). `true` means this
         * dictation makes no cleanup call at all — the raw transcript is the
         * output.
         *
         * Rust↔Kotlin TWIN of `pipeline::offline_rule_with`, pinned row for row
         * by `test-fixtures/offline-rule-vectors.json` (read here by
         * `OfflineRuleVectorsTest` and in Rust by
         * `pipeline::tests::spec_offline_rule_matches_the_fixture_matrix`).
         *
         * Two clauses:
         * - a selected local cleanup that this platform does not have degrades
         *   to *no cleanup*, never to a cloud call (drift row D-M21);
         * - local STT implies local cleanup or none (G2a / drift row D-H10) —
         *   Android branched on `config.llmProvider == "local"` alone, so after
         *   a local transcript the cloud cleanup arm ran for any cloud
         *   `llmProvider`.
         *
         * [llmProvider] must be the EFFECTIVE provider
         * ([KlarvoApi.effectiveLlmProviderName]), so an active test provider is
         * still reached.
         */
        internal fun skipsCloudCleanup(
            sttProvider: String,
            llmProvider: String,
            localCleanupAvailable: Boolean
        ): Boolean {
            val localLlm = llmProvider == LOCAL_PROVIDER_ID
            if (localLlm && !localCleanupAvailable) return true
            return sttProvider == LOCAL_PROVIDER_ID && !localLlm
        }

        /**
         * Decides the Step-4 delivery (Story 7-10, AC2) — the Kotlin twin of the
         * desktop `pipeline::deliver_text` branch.
         *
         * When cleanup failed, the raw transcript goes to the clipboard ONLY: it
         * is never inserted into the focused field, so filler-laden text cannot
         * appear (or, on a desktop-style auto-send, be submitted) behind the
         * user's back. Q5: the "Copied: …" toast is suppressed on that path so
         * the single combined degrade toast is the newest one — on HyperOS the
         * newest toast wins.
         *
         * `llmCleanupFailed` is an EXPLICIT flag, not `degradeStatusMsg != null`:
         * that message is also set when the fallback provider *succeeded*
         * ("⚠ Cleanup-Anbieter gewechselt"), which is not a failure.
         *
         * Q4 (desktop parity): only true cleanup failures reach here. "No LLM key
         * configured" keeps today's paste, and a silent local-MNN failure is out
         * of scope (backlog).
         *
         * Extracted as a pure function because Step 4 itself needs a live
         * Service, a ClipboardManager and an AccessibilityService — the repo's
         * established seam pattern ([BankingGuard.shouldBlockPaste],
         * [sanitizePreviewChunk]). **The full integration path — real clipboard
         * write, real accessibility paste — is covered by the on-device smoke,
         * not by this function's unit test.**
         */
        /**
         * Step 4a, **before** the paste: may this text be inserted at all?
         *
         * Split out of [decideDelivery] by story 13-2 because the rest of the
         * decision now needs the paste OUTCOME, which does not exist yet at
         * this point. [clipboardOk] is new here too: after story 13-2 the
         * clipboard write is guarded, and a paste that would read a clipboard
         * that was never written is worse than no paste.
         */
        internal fun shouldAttemptPaste(
            llmCleanupFailed: Boolean,
            accessibilityConnected: Boolean,
            clipboardOk: Boolean
        ): Boolean = !llmCleanupFailed && accessibilityConnected && clipboardOk

        /**
         * Step 4b, **after** the paste: what the user sees.
         *
         * [pasteOutcome] is `null` when no paste was attempted
         * ([shouldAttemptPaste] said no, e.g. no accessibility service).
         *
         * Story 13-2 rows:
         * - **D4 / D-H20** — an attempted paste that did not land shows the
         *   shipped `"Copied: …"` toast and **no** DONE checkmark. Step 4 used
         *   to decide on `instance != null` alone, and `pasteIntoFocusedField`
         *   returned `Unit`, so a paste into a non-editable surface flashed
         *   success while nothing appeared anywhere.
         * - **D5 / D-M24** — a cleanup failure gets no DONE flash at all,
         *   which is what the flash's own comment has always claimed ("Only
         *   the success path gets the DONE state").
         * - **D6 / D-M12** — a clipboard write that threw shows neither the
         *   toast nor the check: nothing is on the clipboard to copy.
         *
         * Unchanged on purpose: when no accessibility service is connected,
         * nothing is attempted, the `"Copied: …"` toast fires and the run still
         * ends in DONE. That is the shipped clipboard-delivery ending and no
         * audit row re-opens it (Desktop's twin, `DoneClipboard`, is likewise a
         * terminal state rather than an error).
         */
        fun decideDelivery(
            llmCleanupFailed: Boolean,
            accessibilityConnected: Boolean,
            clipboardOk: Boolean,
            pasteOutcome: KlarvoAccessibilityService.PasteOutcome?
        ): DeliveryDecision {
            val attempted = shouldAttemptPaste(llmCleanupFailed, accessibilityConnected, clipboardOk)
            val pasted = pasteOutcome == KlarvoAccessibilityService.PasteOutcome.PASTED
            return DeliveryDecision(
                paste = attempted,
                // Pre-7-10 behaviour kept: a successful paste is silent; every
                // other delivery that DID reach the clipboard says so. Story
                // 7-10 Q5: on the cleanup-failure path the single degrade toast
                // is the only one.
                showCopiedToast = clipboardOk && !llmCleanupFailed && !pasted,
                success = clipboardOk && !llmCleanupFailed && (pasted || !attempted)
            )
        }

        /**
         * The terminal bubble state for a finished run.
         *
         * **No new [RecordingState] value** (Andi, 2026-09-21): a run that did
         * not deliver ends in the shipped IDLE state and its cause is carried
         * by the existing degrade toast. Desktop's `DoneClipboard` /
         * `DoneDegraded` are the conceptual twins but porting them would mean
         * designing a new Android bubble drawing, which this story does not do.
         */
        internal fun terminalStateFor(decision: DeliveryDecision): RecordingState =
            if (decision.success) RecordingState.DONE else RecordingState.IDLE

        /**
         * Maps one `nativeTranscribe` return value to a retry verdict.
         *
         * - a non-`__ERROR_` string is the transcript;
         * - `__ERROR_EMPTY_AUDIO__` — nothing to send, never retryable;
         * - `__ERROR_API:` — 4xx is permanent, everything else (5xx, an
         *   unparseable status) is retryable;
         * - `__ERROR_FORMAT:` — **story 13-2 / D9 / D-M5.** The answer arrived
         *   but is empty or unparseable (`SttError::ResponseFormat`), which
         *   `pipeline::is_retryable_stt_error` calls non-retryable. It used to
         *   have no sentinel of its own, fell into the `__ERROR_NETWORK:`
         *   catch-all, and cost three Groq calls plus ~7 s of backoff before
         *   reaching the same terminal state Desktop reaches on the first
         *   attempt. Reachable at defaults (quiet noise that passes the RMS
         *   gate and whose segments are all dropped by `no_speech_prob > 0.6`);
         *   reproducible with `advanced.testProviderStt = "empty"`.
         * - `__ERROR_NETWORK:` and any unknown sentinel — retryable.
         */
        internal fun classifySttSentinel(result: String): SttSentinel = when {
            !result.startsWith("__ERROR_") ->
                SttSentinel(SttVerdict.SUCCESS, result)

            result == "__ERROR_EMPTY_AUDIO__" ->
                SttSentinel(SttVerdict.NON_RETRYABLE, "Groq STT: empty audio")

            result.startsWith("__ERROR_FORMAT:") ->
                SttSentinel(
                    SttVerdict.NON_RETRYABLE,
                    "Groq STT failed: ${result.removeSurrounding("__ERROR_FORMAT:", "__")}"
                )

            result.startsWith("__ERROR_API:") -> {
                val msg = result.removeSurrounding("__ERROR_API:", "__")
                val statusCode = Regex("HTTP (\\d{3})").find(msg)
                    ?.groupValues?.getOrNull(1)?.toIntOrNull()
                if (statusCode != null && statusCode in 400..499) {
                    SttSentinel(SttVerdict.NON_RETRYABLE, "Groq STT failed: $msg")
                } else {
                    SttSentinel(SttVerdict.RETRYABLE, msg)
                }
            }

            result.startsWith("__ERROR_NETWORK:") ->
                SttSentinel(
                    SttVerdict.RETRYABLE,
                    result.removeSurrounding("__ERROR_NETWORK:", "__")
                )

            else -> SttSentinel(SttVerdict.RETRYABLE, result)
        }

        /**
         * Performs [write] and reports whether it landed; a throw is caught,
         * handed to [onFailure], and **never** propagated.
         *
         * Story 13-2 (D6 / D-M12). `copyToClipboard` was unguarded inside
         * `handler.post`, so a throwing `setPrimaryClip` — an OEM clipboard
         * service refusing or dying; HyperOS has its own clipboard policy
         * layer — was an uncaught main-thread exception and the app crashed
         * mid-delivery.
         *
         * [write] and [onFailure] are function parameters for exactly the
         * reason `KlarvoAudioRecorder.vadGateDecision`'s `isSpeech` is one: the
         * real bodies need a `Context`, a `ClipboardManager` and
         * `android.util.Log`, all of which throw "not mocked" on this
         * classpath. With the seam a JVM test drives the real catch semantics
         * with a throwing fake, instead of a human reading the try/catch.
         * The production [onFailure] (log + `pasteErrorCount`) still needs a
         * source tripwire — see `OverlayServiceSourceContractTest`.
         *
         * A real `setPrimaryClip` failure is not producible on Andi's device
         * and the test provider cannot inject it; ADR-0016 Amendment 4 records
         * D6 as agent-verified only (Weg 2).
         */
        internal fun guardedClipboardWrite(
            write: () -> Unit,
            onFailure: (Throwable) -> Unit,
        ): Boolean = try {
            write()
            true
        } catch (t: Throwable) {
            // `Throwable`, not `Exception`: the KDoc above and the D6 row both
            // promise "never an uncaught main-thread exception", and an OEM
            // clipboard service can surface as an `Error` (a `LinkageError`
            // from a provider stub, an `UnsatisfiedLinkError` behind a shim) —
            // which `catch (e: Exception)` lets past. Review finding.
            onFailure(t)
            false
        }

        /**
         * Classifies a cleanup failure as retryable, mirroring Rust's
         * `is_retryable_llm_error` (429 / 5xx / a transport failure with no
         * HTTP response at all).
         *
         * Moved to the companion by story 13-2 so a JVM test can drive it, and
         * widened in two ways:
         * - **D10 / D-M2**: a [org.json.JSONException] — a body that does not
         *   decode — is RETRYABLE. The single-call path used to gate the ladder
         *   on `e is IOException` first, and JSONException is not one, so a
         *   malformed answer on a dictation under [KlarvoApi.CHUNK_THRESHOLD]
         *   got no fallback at all while the chunked path (where
         *   `collectChunkResults` rewraps it) did. Rust lets `response.json()`
         *   fail into the retryable `LlmError::Request` for the same bytes.
         * - **D2 / D3**: the two answers that arrived but say nothing usable
         *   are named non-retryable explicitly. They carry no `HTTP <code>`, so
         *   the regex below would otherwise read them as transport failures and
         *   burn the ladder on a guaranteed repeat.
         *
         * Anything that is not an [IOException] or a [org.json.JSONException]
         * stays non-retryable, as before.
         */
        internal fun isRetryableCleanupFailure(e: Throwable): Boolean = when (e) {
            is KlarvoApi.CleanupResponseFormatException -> false
            is KlarvoApi.CleanupOutputTruncatedException -> false
            is org.json.JSONException -> true
            is IOException -> {
                val status = Regex("HTTP (\\d{3})").find(e.message ?: "")
                    ?.groupValues?.getOrNull(1)?.toIntOrNull()
                status == null || status == 429 || status >= 500
            }
            else -> false
        }
    }

    // Cached config -- populated by loadBubbleControls(), reused in processAudio().
    // Avoids redundant disk reads within a single dictation cycle.
    private var cachedConfig: KlarvoApi.Config? = null

    enum class RecordingMode(val label: String, val badge: String) {
        HOLD("Hold", "H"),
        TOGGLE("Toggle", "T"),
        AUTOSTOP("Auto Stop", "S"),
        AUTO("Auto", "A");

        fun next(): RecordingMode = entries[(ordinal + 1) % entries.size]

        companion object {
            /**
             * Maps a config string (case-insensitive) to a RecordingMode.
             * Falls back to HOLD for unknown values.
             */
            fun fromString(value: String): RecordingMode = when (value.lowercase()) {
                "toggle"   -> TOGGLE
                "autostop" -> AUTOSTOP
                "auto"     -> AUTO
                else       -> HOLD
            }

            /**
             * Selects the silence-detection duration for a recording session.
             *
             * Mirrors the desktop pipeline (pipeline.rs:640 AUTOSTOP→autostop_silence_secs,
             * :704 AUTO→auto_mode_silence_secs). AUTO and AUTOSTOP use the shared mode-level
             * fields; HOLD and TOGGLE fall back to the per-gesture values.
             *
             * Pure function — no Android context needed — so it is directly testable by JVM
             * tests (AC6, Story 9-7). Behavior is byte-identical to the inline block it
             * replaced in startRecording().
             *
             * @param mode           Active RecordingMode for this session.
             * @param gesture        Gesture that started the recording ("tap", "longpress", or null).
             * @param tapSilence     Per-gesture silence for tap (HOLD/TOGGLE only).
             * @param longPressSilence Per-gesture silence for long-press (HOLD/TOGGLE only).
             * @param autostopSilence Mode-level silence for AUTOSTOP.
             * @param autoModeSilence Mode-level silence for AUTO.
             * @return               Silence duration in seconds to pass to KlarvoAudioRecorder.
             */
            fun selectSilenceSecs(
                mode: RecordingMode,
                gesture: String?,
                tapSilence: Float,
                longPressSilence: Float,
                autostopSilence: Float,
                autoModeSilence: Float,
            ): Float = when (mode) {
                AUTO     -> autoModeSilence
                AUTOSTOP -> autostopSilence
                else -> when (gesture) {
                    "longpress" -> longPressSilence
                    else        -> tapSilence
                }
            }

            /**
             * Story 11-2 (AC-1/AC-3/AC-4, Task 2.1): pure guard deciding whether the repeatable
             * preview-flush callback should be installed for a recording session -- HOLD/TOGGLE
             * only, and only when the user has opted in via Settings (`livePreviewEnabled`).
             * Auto/AutoStop never get a preview flush (mirrors desktop FR4 parity) regardless of
             * the setting. Pure function -- no Android Context needed -- same testable-shape
             * pattern as [selectSilenceSecs] (AC-3/AC-4's JVM test lives next to
             * `RecordingModeSilenceSelectionTest.kt`).
             *
             * **Story 13-2 (E1 / D-H9) added [sttProvider].** With a stored
             * `sttProvider = "local"` the live preview used to upload every
             * pause's delta WAV to Groq — with or without a Groq key, because
             * `readConfig` admits `local` with a blank key and
             * `WhisperStt::transcribe` sends the whole multipart body before
             * the 401 comes back. Audio left the device under a setting called
             * "Offline". The desktop twin,
             * `pipeline::preview_flush_should_install`, has always taken
             * `stt_provider`; this closes the Android half.
             *
             * This is mandatory independently of 13-3: hiding the control
             * leaves the stored value behind (ADR-0016's gate definition), and
             * `flushPreviewDelta` re-checks at flush time as
             * `pipeline.rs` does, because the config can change between the
             * install and the pause.
             */
            fun shouldInstallPreviewFlush(
                mode: RecordingMode,
                livePreviewEnabled: Boolean,
                sttProvider: String
            ): Boolean =
                (mode == HOLD || mode == TOGGLE) &&
                    livePreviewEnabled &&
                    sttProvider != LOCAL_PROVIDER_ID
        }
    }

    /**
     * Story 13-2: `internal` (was `private`) so the pure [terminalStateFor]
     * seam can return it and a JVM test can assert the choice. **The set of
     * values is unchanged** — this story adds no state; a run that did not
     * deliver ends in the shipped IDLE.
     */
    internal enum class RecordingState { IDLE, RECORDING, TRANSCRIBING, DONE }

    private val handler = Handler(Looper.getMainLooper())

    /**
     * Story 11-2 code-review fix F3 (2026-07-01): serializes preview-chunk transcription so
     * appended order always matches speech order. Previously each [flushPreviewDelta] spawned an
     * independent `Thread`; variable Groq latency could let pause N+1's transcription land
     * before pause N's, scrambling the preview. A single-thread executor makes flushes FIFO
     * (one in-flight at a time) without blocking the caller (main thread just enqueues).
     */
    private val previewFlushExecutor = Executors.newSingleThreadExecutor()

    private lateinit var windowManager: WindowManager
    private lateinit var bubbleView: FloatingBubbleView
    private lateinit var bubbleParams: WindowManager.LayoutParams
    private var overlayType = 0

    private var currentState = RecordingState.IDLE

    /** Tracks whether the bubble view is currently attached to WindowManager. */
    private var isBubbleVisible = false

    // Listening panel (Story 9.5): second TYPE_APPLICATION_OVERLAY window shown during recording/transcribing.
    private var panelView: ListeningPanelView? = null
    private var panelParams: WindowManager.LayoutParams? = null
    private var panelVisible = false

    // Keyboard detection
    private var keyboardVisible = false

    /**
     * True when the bubble should be shown regardless of keyboard state.
     * Loaded from SharedPreferences; defaults to false (keyboard-only mode).
     */
    private var alwaysVisible = false

    /**
     * True once the AccessibilityService has called onKeyboardVisibilityChanged() at
     * least once. While this is false we trust the reflection-based fallback instead.
     */
    private var accessibilityServiceActive = false

    /** True while a banking/security app is in the foreground. Blocks bubble show. */
    private var bankingAppActive = false

    // Audio
    private var audioRecorder: KlarvoAudioRecorder? = null

    // Touch handling
    private var dragTouchStartX = 0f
    private var dragTouchStartY = 0f
    private var bubbleStartX = 0
    private var bubbleStartY = 0
    private var isDragging = false
    private var dragThresholdPx = 0f
    // Primary pointer lock (code review finding B): a second finger touching down during a HOLD
    // must not be able to change which target the release commits — only the pointer captured at
    // ACTION_DOWN drives hit-tracking and the release-to-commit decision.
    private var activePointerId = MotionEvent.INVALID_POINTER_ID

    // Story 11-4 (AC-1): set when the bubble-above-panel reorder (see showListeningPanel /
    // reorderBubbleAbovePanel) had to be deferred because a touch gesture was in flight on the
    // bubble window at the moment the panel was shown (e.g. push-to-talk's longPressRunnable
    // fires startRecording()/showListeningPanel() while the finger is still down). Consumed at
    // the next ACTION_UP/ACTION_CANCEL, once the gesture has actually ended.
    private var bubbleReorderPending = false

    // Per-gesture recording modes: tap and long-press are configured independently.
    private var tapMode = RecordingMode.TOGGLE
    private var longPressMode = RecordingMode.HOLD

    // Per-gesture silence-detection settings.
    private var tapSilenceSecs = 2.0f
    private var longPressSilenceSecs = 2.0f
    // Mode-level silence durations (AUTO/AUTOSTOP use these, parity with desktop pipeline.rs:640/704).
    private var autostopSilenceSecs = 2.0f
    private var autoModeSilenceSecs = 2.0f
    // Energy gate threshold for VAD pre-filter (AC1/AC2/AC3, Story 9-11).
    // Read from config.json "advanced.silenceThreshold"; default matches Rust default_silence_threshold() = 0.005.
    private var silenceThreshold = KlarvoAudioRecorder.DEFAULT_ENERGY_GATE_THRESHOLD

    /**
     * Tracks which gesture started the current recording session.
     * Used to select the correct per-gesture mode / silenceSecs when stopping.
     * (The auto-send flags this KDoc used to name were removed in story 7-9,
     * row M13 — Android has no auto-send path.)
     * "tap" or "longpress"; null when not recording.
     */
    private var activeGesture: String? = null

    // Auto-mode loop: true while the auto-loop is active (records, processes, repeats)
    private var autoLoopActive = false

    /**
     * Remembered Y position before a keyboard jump-up so we can restore it when the keyboard
     * closes in always-visible mode. Null when the bubble has not been moved for the keyboard.
     * Never written by savePosition() — a keyboard-shifted Y must not become the resting position.
     */
    private var preKeyboardY: Int? = null

    // Long-press / push-to-talk state
    private var longPressTriggered = false

    /**
     * True while the user is holding a long-press that triggered push-to-talk recording.
     * When the finger lifts we confirm (stop + process) instead of treating it as a tap.
     */
    private var pushToTalkActive = false

    /**
     * Synthetic transcript injected by the debug harness broadcast.
     * Stored for use by the listening-panel render in Story 9.5; ignored in 9.4.
     */
    private var debugTranscript: String = ""

    /**
     * Story 11-2: accumulated raw preview text for the CURRENT recording (HOLD/TOGGLE,
     * `livePreviewEnabled == true` only). Appended to by [flushPreviewDelta] on every pause;
     * display-only -- never feeds the paste path. Cleared on finish (AC-7) and cancel.
     */
    private var previewAccumulatedText: String = ""

    /**
     * Saved X position of the bubble window before expanding to the recording cluster.
     * Restored when leaving RECORDING state (TRANSCRIBING / DONE / IDLE).
     * Null when not currently in cluster mode.
     */
    private var preclusterBubbleX: Int? = null

    /**
     * Saved Y position of the bubble window before entering the HOLD Cancel surface.
     * adjustLayoutForState shifts Y so the anchor bubble's drawn center coincides with the idle
     * bubble's on-screen center (AC2); this field saves the original Y so it can be restored
     * exactly on stop/cancel via adjustLayoutForState's else-branch. Null when not in HOLD.
     */
    private var preclusterBubbleY: Int? = null

    /**
     * Debug-only broadcast receiver — drives the bubble through all four states on demand
     * without live audio or network. Only registered when BuildConfig.DEBUG == true.
     * Fast path for an already-running service process (avoids service-start overhead).
     * See ACTION_DEBUG_SET_STATE constants for adb commands.
     *
     * Cold-start path (dead process): DebugHarnessReceiver (manifest-declared, static) wakes
     * the process via startForegroundService and forwards extras; onStartCommand calls
     * applyHarnessState() once bubbleView is initialised. Keep both paths in sync via the
     * shared applyHarnessState() helper below.
     */
    private val debugStateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action != ACTION_DEBUG_SET_STATE) return
            val stateToken = intent.getStringExtra(EXTRA_STATE) ?: return
            val rms        = intent.getFloatExtra(EXTRA_RMS, -1f)
            val transcript = intent.getStringExtra(EXTRA_TRANSCRIPT)
            val holdMode   = intent.getBooleanExtra(EXTRA_HOLD_MODE, false)
            handler.post {
                applyHarnessState(stateToken, rms, transcript, holdMode)
            }
        }
    }

    /**
     * Shared harness-state application logic — called by both the dynamic debugStateReceiver
     * (already-running process fast path) and onStartCommand (cold-start via DebugHarnessReceiver).
     *
     * Must be called on the main thread (handler.post from dynamic receiver; handler.post from
     * onStartCommand). Idempotent: safe to call when the service is already in the target state.
     */
    private fun applyHarnessState(stateToken: String, rms: Float, transcript: String?, holdMode: Boolean = false) {
        if (!BuildConfig.DEBUG) return
        if (!::bubbleView.isInitialized) {
            KlarvoLogger.w(TAG, "[harness] bubbleView not ready — cannot apply state '$stateToken'")
            return
        }
        val newState = when (stateToken.lowercase()) {
            "idle"         -> RecordingState.IDLE
            "recording"    -> RecordingState.RECORDING
            "transcribing" -> RecordingState.TRANSCRIBING
            "done"         -> RecordingState.DONE
            else -> {
                KlarvoLogger.w(TAG, "[harness] DEBUG_SET_STATE: unknown token '$stateToken'")
                return
            }
        }
        // Reset amplitude when rms is absent so each harness broadcast starts clean.
        val coercedRms = if (rms >= 0f) rms.coerceIn(0f, 1f) else 0f
        if (transcript != null) debugTranscript = transcript
        // Force the bubble visible for harness use before setting state.
        // Capture previous state so adjustLayoutForState gets correct geometry.
        val previousState = currentState
        forceShowBubbleForHarness()
        // Story 9-14: set pushToTalkActive BEFORE adjustLayoutForState so the HOLD targets
        // window dimensions are used when hold_mode=true (adjustLayoutForState reads pushToTalkActive).
        // No drag-threshold init needed anymore (B-Sprache release-to-commit is pure hit-tracking,
        // not threshold-fire-on-move — finding 7's old init point is structurally gone).
        pushToTalkActive = (newState == RecordingState.RECORDING && holdMode)
        setState(newState)
        adjustLayoutForState(newState, previousState)
        // Apply the static wave level AFTER the state transition so that the
        // RECORDING branch of updateAnimators() (waveLevels.fill(0f)) has already
        // run before we fill the history — otherwise the reset would wipe our level.
        // Use setStaticWaveLevel so the harness fills all 20 history slots uniformly,
        // giving a visible uniform waveform at that level (not just one pushed slot).
        bubbleView.setStaticWaveLevel(coercedRms)

        // Story 9-14/9-15: apply HOLD dock visual and TAP surface props after state is applied.
        if (newState == RecordingState.RECORDING) {
            bubbleView.holdDockActive = holdMode
            setHoldModeOnPanel(holdMode)
            if (!holdMode) {
                // TAP surface harness: set dock side and start timer.
                bubbleView.dockSide = getDockSide()
                bubbleView.recordingStartMs = System.currentTimeMillis()
            }
        } else {
            bubbleView.holdDockActive = false
            setHoldModeOnPanel(false)
            bubbleView.recordingStartMs = 0L
        }

        // Story 9.5: sync listening panel to harness state.
        when (newState) {
            RecordingState.RECORDING -> {
                if (!panelVisible) {
                    showListeningPanel(ListeningPanelView.State.RECORDING)
                    // F4: guard timer start on actual successful attach
                    if (panelVisible) panelView?.startTimer()
                }
                panelView?.amplitude = coercedRms
                panelView?.rawTranscript = debugTranscript
            }
            RecordingState.TRANSCRIBING -> {
                if (!panelVisible) {
                    showListeningPanel(ListeningPanelView.State.TRANSCRIBING)
                } else {
                    panelView?.stopTimer()
                    panelView?.panelState = ListeningPanelView.State.TRANSCRIBING
                    panelView?.invalidate()
                }
            }
            RecordingState.IDLE, RecordingState.DONE -> {
                hideListeningPanel()
            }
        }

        KlarvoLogger.d(TAG, "[harness] state → $newState (rms=$rms, transcript=${transcript?.take(30)})")
    }

    /**
     * Debug-only direct-show path for the state harness.
     * Bypasses the SHOW_DEBOUNCE_MS delay and the banking-app guard so the bubble
     * appears synchronously on the emulator without any UI interaction.
     *
     * Only compiled/called in DEBUG builds (receiver is already DEBUG-gated;
     * this helper is an extra safety so the logic never leaks into release).
     */
    private fun forceShowBubbleForHarness() {
        if (!BuildConfig.DEBUG) return
        // Cancel any pending debounced show — we want immediate attach.
        pendingShowRunnable?.let { handler.removeCallbacks(it) }
        pendingShowRunnable = null
        if (!isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                reloadBubbleAppearance()
                windowManager.addView(bubbleView, bubbleParams)
                isBubbleVisible = true
                updateNotification()
                KlarvoLogger.d(TAG, "[harness] bubble force-shown for harness")
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "[harness] forceShowBubbleForHarness failed", e)
            }
        }
    }

    private val longPressRunnable = Runnable {
        if (!isDragging && currentState == RecordingState.IDLE) {
            longPressTriggered = true
            activeGesture      = "longpress"
            // Re-read config before deciding behavior.
            loadBubbleControls()
            // Only activate push-to-talk (stop on finger lift) when longPressMode is HOLD.
            pushToTalkActive = (longPressMode == RecordingMode.HOLD)
            // Enable auto-loop when longPressMode is AUTO.
            if (longPressMode == RecordingMode.AUTO) {
                autoLoopActive = true
            }
            // HOLD Cancel surface (Story 9-14 re-scope 2026-07-01): show holdDockActive surface +
            // update panel label. No drag-threshold init needed — release-to-commit is pure
            // hit-tracking (Task 6/7), not threshold-fire-on-move.
            if (pushToTalkActive) {
                bubbleView.holdDockActive = true
                setHoldModeOnPanel(true)
            }
            startRecording()
        }
    }

    /** Sets isHoldMode on the listening panel (if visible). No-op when panel is not attached. */
    private fun setHoldModeOnPanel(holdMode: Boolean) {
        panelView?.isHoldMode = holdMode
        panelView?.invalidate()
    }

    /**
     * Receives broadcast actions from the foreground notification.
     * Registered/unregistered dynamically -- no manifest entry needed.
     * Only handles ACTION_TOGGLE_BUBBLE (mode switching removed).
     */
    private val notificationActionReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == ACTION_TOGGLE_BUBBLE) toggleBubble()
        }
    }

    private val keyboardCheckRunnable = object : Runnable {
        override fun run() {
            checkKeyboardVisibility()
            handler.postDelayed(this, KEYBOARD_CHECK_INTERVAL)
        }
    }

    /**
     * Runnable that completes the DONE→IDLE flash after 800ms.
     * Named so it can be cancelled if a new recording starts before the delay fires
     * (AUTO mode, re-record, or harness) — preventing a stale callback from forcing
     * IDLE while a fresh recording is already live.
     */
    private val doneFlashRunnable = Runnable {
        if (currentState == RecordingState.DONE) {
            // Story 9.5: hide listening panel before returning to IDLE.
            hideListeningPanel()
            setState(RecordingState.IDLE)
            adjustLayoutForState(RecordingState.IDLE, RecordingState.DONE)
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        KlarvoLogger.init(this)
        dragThresholdPx = 10f * resources.displayMetrics.density

        overlayType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            @Suppress("DEPRECATION")
            WindowManager.LayoutParams.TYPE_PHONE
        }

        cleanupStalePendingWavFiles()
        loadBubbleControls()
        createNotificationChannel()
        startForegroundWithNotification()

        val filter = IntentFilter().apply {
            addAction(ACTION_TOGGLE_BUBBLE)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(notificationActionReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(notificationActionReceiver, filter)
        }

        // Debug harness: register state-override receiver only in debug builds.
        // Allows driving all four bubble states on-device without live audio/network.
        // RECEIVER_EXPORTED is intentional on Tiramisu+: adb shell am broadcast runs as shell
        // UID (2000), not the app UID — RECEIVER_NOT_EXPORTED would silently drop those
        // broadcasts. This is safe because the entire block is gated by BuildConfig.DEBUG;
        // release APKs never register this receiver at all (AC4).
        if (BuildConfig.DEBUG) {
            val debugFilter = IntentFilter(ACTION_DEBUG_SET_STATE)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                registerReceiver(debugStateReceiver, debugFilter, RECEIVER_EXPORTED)
            } else {
                registerReceiver(debugStateReceiver, debugFilter)
            }
            KlarvoLogger.d(TAG, "[harness] debug broadcast receiver registered")
        }

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        alwaysVisible = prefs.getBoolean(PREF_ALWAYS_VISIBLE, false)

        setupBubble()
        setupKeyboardDetector()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Cold-start harness path: DebugHarnessReceiver (manifest-declared static receiver) starts
        // the service with ACTION_DEBUG_SET_STATE when the process is dead. onCreate() runs first
        // and initialises bubbleView, so we can safely apply the harness state here on the main
        // thread via handler.post (gives the layout a frame to settle before we attach the bubble).
        if (BuildConfig.DEBUG && intent?.action == ACTION_DEBUG_SET_STATE) {
            val stateToken = intent.getStringExtra(EXTRA_STATE)
            if (stateToken != null) {
                val rms        = intent.getFloatExtra(EXTRA_RMS, -1f)
                val transcript = intent.getStringExtra(EXTRA_TRANSCRIPT)
                val holdMode   = intent.getBooleanExtra(EXTRA_HOLD_MODE, false)
                handler.post { applyHarnessState(stateToken, rms, transcript, holdMode) }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        instance = null
        // Story 11-4 F2: service teardown removes the bubble window below without a touch ever
        // reaching handleTouch's ACTION_UP/CANCEL consume -- clear the flag so it can't leak.
        bubbleReorderPending = false
        handler.removeCallbacks(keyboardCheckRunnable)
        handler.removeCallbacks(longPressRunnable)
        handler.removeCallbacks(doneFlashRunnable)
        try {
            unregisterReceiver(notificationActionReceiver)
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "Failed to unregister notificationActionReceiver (already unregistered?)", e)
        }
        if (BuildConfig.DEBUG) {
            try {
                unregisterReceiver(debugStateReceiver)
            } catch (e: IllegalArgumentException) {
                KlarvoLogger.w(TAG, "[harness] debugStateReceiver already unregistered", e)
            }
        }
        audioRecorder?.releaseImmediately()
        audioRecorder = null
        // Story 11-2 fix F3: tear down the preview-flush executor so no queued/in-flight preview
        // transcription outlives the service.
        previewFlushExecutor.shutdownNow()
        // F3: release panel window + its Handler + ValueAnimators on teardown
        hideListeningPanel()
        super.onDestroy()
        if (::bubbleView.isInitialized && isBubbleVisible) {
            try {
                windowManager.removeView(bubbleView)
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "Failed to remove bubbleView on destroy", e)
            }
            isBubbleVisible = false
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // --- Notification ---

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Klarvo Overlay",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps the Klarvo voice bubble visible"
                setShowBadge(false)
            }
            val nm = getSystemService(NotificationManager::class.java)
            nm.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        // Show current per-gesture mode configuration as status text.
        val statusText = "Tap: ${tapMode.label}, Hold: ${longPressMode.label}"

        // Tap on notification body = toggle bubble visibility
        val toggleIntent = Intent(ACTION_TOGGLE_BUBBLE).apply { setPackage(packageName) }
        val pendingToggle = PendingIntent.getBroadcast(
            this, 0, toggleIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        builder
            .setContentTitle("Klarvo")
            .setContentText(statusText)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setContentIntent(pendingToggle)
            .setOngoing(true)

        return builder.build()
    }

    private fun updateNotification() {
        val nm = getSystemService(NotificationManager::class.java)
        nm.notify(NOTIFICATION_ID, buildNotification())
    }

    private fun startForegroundWithNotification() {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            // Android 14+ (API 34+) blocks FOREGROUND_SERVICE_TYPE_MICROPHONE from background
            // callers (e.g. BroadcastReceiver context) unless the app is in a user-visible state,
            // even when RECORD_AUDIO is granted. In DEBUG builds the service may be cold-started
            // via DebugHarnessReceiver (background), so we catch the SecurityException and fall
            // back to type NONE for the initial notification. The microphone FGS type is not
            // needed for the visual-state harness (no audio is captured). This fallback is gated
            // to DEBUG so release builds always use the microphone type from the normal
            // MainActivity foreground-start path.
            if (BuildConfig.DEBUG && Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                try {
                    startForeground(
                        NOTIFICATION_ID, notification,
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
                    )
                } catch (e: SecurityException) {
                    // FOREGROUND_SERVICE_TYPE_NONE is also prohibited on targetSdk 34+.
                    // DATA_SYNC is allowed from background under the SYSTEM_ALERT_WINDOW exemption
                    // and does not require a user-visible state — safe for the visual harness.
                    KlarvoLogger.d(TAG, "[harness] microphone FGS type blocked from background, using DATA_SYNC fallback: ${e.message?.take(60)}")
                    startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
                }
            } else {
                startForeground(
                    NOTIFICATION_ID, notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
                )
            }
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    // --- Recording controls ---

    /**
     * Loads per-gesture recording controls from config.json.
     * Replaces the old single-mode loadRecordingMode().
     */
    private fun loadBubbleControls() {
        val config = KlarvoApi.readConfig(this)
        cachedConfig = config  // cache for processAudio() to avoid redundant disk read
        if (config != null) {
            tapMode = RecordingMode.fromString(config.bubbleTapMode)
            longPressMode = RecordingMode.fromString(config.bubbleLongPressMode)
            tapSilenceSecs = config.bubbleTapSilenceSecs
            longPressSilenceSecs = config.bubbleLongPressSilenceSecs
            autostopSilenceSecs = config.autostopSilenceSecs
            autoModeSilenceSecs = config.autoModeSilenceSecs
            silenceThreshold = config.silenceThreshold
            KlarvoLogger.d(TAG, "loadBubbleControls: tap=${config.bubbleTapMode}→$tapMode, lp=${config.bubbleLongPressMode}→$longPressMode, silenceThreshold=$silenceThreshold")
        } else {
            KlarvoLogger.w(TAG, "loadBubbleControls: config is NULL, using defaults tap=$tapMode, lp=$longPressMode")
        }
    }

    // --- Keyboard detection ---

    private fun setupKeyboardDetector() {
        if (alwaysVisible) {
            showBubble()
        } else {
            handler.post(keyboardCheckRunnable)
        }
    }

    fun onKeyboardVisibilityChanged(visible: Boolean) {
        handler.post {
            accessibilityServiceActive = true
            applyKeyboardState(visible)
        }
    }

    /**
     * Called by KlarvoAccessibilityService when a banking/security app enters or
     * leaves the foreground. When active, the bubble is forcefully hidden regardless
     * of keyboard state or alwaysVisible setting. This is a security feature and
     * cannot be disabled.
     */
    fun onBankingAppStateChanged(active: Boolean, packageName: String) {
        handler.post {
            KlarvoLogger.d(TAG, "Banking state change: active=$active (was=$bankingAppActive), pkg=$packageName")
            if (active == bankingAppActive) return@post
            bankingAppActive = active

            if (active) {
                KlarvoLogger.i(TAG, "Banking app detected: $packageName — hiding bubble")
                hideBubble()
                Toast.makeText(this, "Klarvo paused — you can dismiss any remaining security warning from your banking app.", Toast.LENGTH_LONG).show()
            } else {
                KlarvoLogger.i(TAG, "Banking app left foreground: $packageName — restoring bubble")
                // Re-apply normal visibility rules: show if keyboard is open or alwaysVisible.
                if (alwaysVisible || keyboardVisible) {
                    showBubble()
                }
            }
        }
    }

    private fun applyKeyboardState(isOpen: Boolean) {
        if (alwaysVisible) {
            // In always-visible mode we never show/hide, but we still need to restore the
            // bubble's Y position when the keyboard closes after a jump-up.
            if (!isOpen) {
                val saved = preKeyboardY
                if (saved != null && isBubbleVisible && ::bubbleView.isInitialized) {
                    preKeyboardY = null
                    bubbleParams.y = saved
                    updateBubbleLayout()
                } else {
                    preKeyboardY = null
                }
            }
            return
        }
        if (isOpen == keyboardVisible) return

        keyboardVisible = isOpen
        if (isOpen) {
            showBubble()
        } else {
            // Keyboard closed: clear any remembered pre-keyboard Y (not always-visible path,
            // so the bubble is about to be hidden; no restore needed).
            preKeyboardY = null
            if (currentState == RecordingState.IDLE) {
                hideBubble()
            }
        }
    }

    /**
     * Adjusts the bubble Y position upward if the keyboard would cover it.
     * Called by KlarvoAccessibilityService when the IME window bounds are known,
     * and by the reflection fallback path with the IMM-reported height.
     *
     * Nav-bar clearance: 56px fixed constant (AR5d). Do NOT use WindowInsetsCompat or
     * env(safe-area-inset-bottom) — those are unreliable for overlay Services on API 24+.
     */
    /**
     * Overlay-window size (px) for a given visual bubble size: the visual squircle plus transparent
     * shadow padding on every side. FloatingBubbleView draws the squircle centered, so the padding
     * is the room the soft shadow + outer ring need — without it they clip at the window edge (a
     * hard square cutoff, worst at large manual sizes). Always ≥48dp, so it also satisfies the
     * touch-target requirement (AC4). Single source of truth for ALL window-size math (set + snap +
     * save + keyboard) so they never diverge.
     */
    private fun bubbleWindowPx(visualDp: Int): Int {
        val dm = resources.displayMetrics
        val padDp = maxOf(MIN_SHADOW_PAD_DP, (visualDp * SHADOW_PAD_FRACTION).toInt())
        return ((visualDp + 2 * padDp) * dm.density).toInt()
    }

    fun adjustBubbleForKeyboard(keyboardHeightPx: Int) {
        handler.post {
            if (!isBubbleVisible || !::bubbleView.isInitialized) return@post
            if (panelVisible) {
                // Live-preview panel is up: AC-1's z-order reorder already keeps the bubble
                // on top of the panel, so keyboard-avoidance is unneeded and actively harmful
                // here — it repeatedly yanked the bubble out of the box the user is trying to
                // drag it into. Suppress the clamp, and clear any stash from before the panel
                // appeared so a later keyboard-close doesn't restore a stale pre-panel Y and
                // override the position the user just dragged into the box.
                preKeyboardY = null
                return@post
            }
            val (_, screenH) = getScreenDimensions()
            // Use the full window height (incl. shadow padding) so the real bottom edge clears
            // the keyboard, not just the smaller visual squircle.
            val windowPx = bubbleWindowPx(bubbleView.getBubbleSizeDp())
            val maxY = screenH - keyboardHeightPx - NAV_BAR_CLEARANCE_PX - windowPx
            if (bubbleParams.y > maxY) {
                if (preKeyboardY == null) {
                    preKeyboardY = bubbleParams.y
                }
                bubbleParams.y = maxY.coerceAtLeast(0)
                updateBubbleLayout()
            }
        }
    }

    private fun checkKeyboardVisibility() {
        if (accessibilityServiceActive) return

        try {
            val imm = getSystemService(INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            val method = imm.javaClass.getMethod("getInputMethodWindowVisibleHeight")
            val height = method.invoke(imm) as Int
            applyKeyboardState(height > 0)
            // Reflection fallback: also apply keyboard jump-up with the reported height
            if (height > 0) {
                adjustBubbleForKeyboard(height)
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "getInputMethodWindowVisibleHeight reflection failed: ${e.message}")
        }
    }

    fun isAlwaysVisible(): Boolean = alwaysVisible

    fun setAlwaysVisible(enabled: Boolean) {
        alwaysVisible = enabled
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putBoolean(PREF_ALWAYS_VISIBLE, enabled)
            .apply()

        if (enabled) {
            showBubble()
        } else {
            if (!keyboardVisible && currentState == RecordingState.IDLE) {
                hideBubble()
            }
        }
    }

    private fun toggleBubble() {
        handler.post {
            if (isBubbleVisible) hideBubble() else showBubble()
        }
    }

    /**
     * Pending show runnable for debounced bubble display. When showBubble() is called,
     * the actual WindowManager.addView is delayed by SHOW_DEBOUNCE_MS to give
     * checkForegroundBankingApp() time to detect and block. This prevents the brief
     * "flash" that banking apps like N26 catch as an active overlay.
     */
    private var pendingShowRunnable: Runnable? = null

    private fun showBubble() {
        if (bankingAppActive) return  // Never show while banking app is active
        // Cancel any previous pending show to avoid duplicates
        pendingShowRunnable?.let { handler.removeCallbacks(it) }
        val runnable = Runnable {
            pendingShowRunnable = null
            if (bankingAppActive) return@Runnable  // Re-check after delay
            if (!isBubbleVisible && ::bubbleView.isInitialized) {
                try {
                    reloadBubbleAppearance()
                    windowManager.addView(bubbleView, bubbleParams)
                    isBubbleVisible = true
                    updateNotification()
                } catch (e: Exception) {
                    KlarvoLogger.w(TAG, "Failed to add bubbleView to WindowManager", e)
                }
            }
        }
        pendingShowRunnable = runnable
        handler.postDelayed(runnable, SHOW_DEBOUNCE_MS)
    }

    private fun hideBubble() {
        // Cancel any pending show — banking detection may arrive before the debounce fires
        pendingShowRunnable?.let { handler.removeCallbacks(it) }
        pendingShowRunnable = null
        if (isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.removeView(bubbleView)
                isBubbleVisible = false
                updateNotification()
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "Failed to remove bubbleView from WindowManager", e)
            }
        }
        // Story 11-4 F2: the bubble window may be torn down here mid-gesture (e.g. banking-app
        // detection), without the touch ever reaching handleTouch's ACTION_UP/CANCEL branch that
        // normally consumes this flag. Clear it here too so a later unrelated gesture doesn't
        // fire a spurious reorder against a since-recreated bubble.
        bubbleReorderPending = false
    }

    // --- Bubble setup ---

    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        // bubbleSize scale factor is superseded by computeVisualSizeDp() as of Story 9.3;
        // idle opacity is fixed at 1.0 (canon) as of the 9.3 polish pass.
        val config = KlarvoApi.readConfig(this)

        // Responsive size formula: clamp(36, 0.11 × min(screenW_dp, screenH_dp), 44)
        val sizeDp = computeVisualSizeDp(config)
        bubbleView.setBubbleSize(sizeDp)
        // Idle bubble renders fully opaque to match the canon (.ab-bubble.idle has no opacity
        // reduction). The legacy 0.85 translucency washed the solid teal-gradient squircle out,
        // especially on light backgrounds. (Story 9.3 polish.)
        bubbleView.alpha = 1.0f

        val (screenW, screenH) = getScreenDimensions()
        val dm        = resources.displayMetrics
        val dp        = dm.density
        val bubblePx  = (sizeDp * dp).toInt()
        val marginPx  = (8 * dp).toInt()  // 8dp snap margin (tighter than startup default)

        // Window = visual squircle + shadow padding (always ≥48dp touch target). The squircle is
        // drawn centered; the padding gives the soft shadow + outer ring room (no clipping).
        val touchTargetPx = bubbleWindowPx(sizeDp)

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        // AC9 (Story 9.3): when edge-snap is OFF restore the raw saved X; when ON use saved side.
        val savedX: Int
        val savedY: Int
        if (config?.bubbleEdgeSnap != false) {
            // Restore position using saved side (left/right) rather than raw pixel X.
            // This ensures the bubble lands on the correct edge after screen rotation or reinstall.
            val savedSide = prefs.getString(PREF_SIDE, "right") ?: "right"
            // WindowManager positions the window (touchTargetPx wide), not the visual circle.
            // Use touchTargetPx for edge placement so the window fits within the screen edge.
            val defaultX = if (savedSide == "left") marginPx else screenW - touchTargetPx - marginPx
            savedX = prefs.getInt(PREF_X, defaultX)
        } else {
            // Edge-snap OFF: restore the raw drop X the user last placed the bubble at.
            val defaultX = screenW - touchTargetPx - marginPx
            savedX = prefs.getInt(PREF_X, defaultX)
        }
        savedY = prefs.getInt(PREF_Y, screenH / 2)

        bubbleParams = WindowManager.LayoutParams(
            touchTargetPx,
            touchTargetPx,
            overlayType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = savedX
            y = savedY
        }

        bubbleView.setOnTouchListener { _, event -> handleTouch(event) }

        isBubbleVisible = false
    }

    private fun getScreenDimensions(): Pair<Int, Int> {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val metrics = windowManager.currentWindowMetrics
            val bounds  = metrics.bounds
            Pair(bounds.width(), bounds.height())
        } else {
            val dm = DisplayMetrics()
            @Suppress("DEPRECATION")
            windowManager.defaultDisplay.getRealMetrics(dm)
            Pair(dm.widthPixels, dm.heightPixels)
        }
    }

    /**
     * Computes the visual bubble size in dp using the responsive formula:
     *   visualDp = clamp(36, (0.11 × min(screenW_dp, screenH_dp)).toInt(), 44)
     *
     * This supersedes the old BASE_BUBBLE_SIZE_DP × config.bubbleSize formula as of Story 9.3.
     * The bubbleSize config scale factor is no longer applied to the visual size (it becomes a
     * no-op). Document here so Story 9.5+ is aware. The config field is intentionally not
     * removed — it may be repurposed or restored in a future story.
     */
    private fun computeVisualSizeDp(cfg: KlarvoApi.Config? = null): Int {
        // AC8 (Story 9.3): when bubbleSizeDp > 0 the user has set a manual size — use it directly.
        // 0 = Auto: fall back to the responsive formula from AC3.
        // Uses the provided config first, then falls back to the cached config.
        val effectiveCfg = cfg ?: cachedConfig
        val manual = effectiveCfg?.bubbleSizeDp ?: 0
        if (manual > 0) return manual.coerceIn(32, 72)
        val dm = resources.displayMetrics
        val screenWdp = dm.widthPixels / dm.density
        val screenHdp = dm.heightPixels / dm.density
        val rawDp = (0.11f * minOf(screenWdp, screenHdp)).toInt()
        return rawDp.coerceIn(36, 44)
    }

    // --- WindowManager layout update ---

    /**
     * Pushes the current bubbleParams to WindowManager.
     * Must be called on the main thread whenever params change (size, position).
     */
    private fun updateBubbleLayout() {
        if (!isBubbleVisible) return
        try {
            windowManager.updateViewLayout(bubbleView, bubbleParams)
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "Failed to update bubble layout", e)
        }
    }

    /**
     * Returns "left" or "right" based on which screen half the idle bubble is currently on.
     * Uses the pre-expansion X position ([preclusterBubbleX] if already set, else [bubbleParams.x])
     * so that it reflects the idle position even after window expansion started.
     * Used to set [FloatingBubbleView.dockSide] before entering the TAP surface.
     */
    private fun getDockSide(): String {
        val (screenW, _) = getScreenDimensions()
        val visualDp = bubbleView.getBubbleSizeDp()
        val windowPx = bubbleWindowPx(visualDp)
        val idleX = preclusterBubbleX ?: bubbleParams.x
        return if (idleX + windowPx / 2 < screenW / 2) "left" else "right"
    }

    /**
     * Adjusts the WindowManager LayoutParams to match the current view state.
     *
     * IDLE / TRANSCRIBING / DONE -> explicit touchTargetPx × touchTargetPx (≥48dp touch target)
     *                               FloatingBubbleView draws the smaller visual circle centered.
     * RECORDING (TAP surface)    -> TAP_VISUAL_W/H + 2×TAP_SHADOW_PAD_DP each side.
     * RECORDING (HOLD Cancel)    -> holdVisualWidthDp/HeightDp + 2×HOLD_SHADOW_PAD_DP each side
     *                               (Story 9-14 re-scope 2026-07-01 — anchor bubble at its idle
     *                               size/position + ONE Abbrechen target growing diagonally).
     *
     * Dock-edge-anchor: when expanding to the TAP surface the right edge of the new window
     * aligns with the right edge of the idle bubble window (right dock), or the left edge
     * is clamped to 0 (left dock) — drawTapSurface() places Send on the correct side via dockSide.
     */
    private fun adjustLayoutForState(newState: RecordingState, previousState: RecordingState) {
        val dp            = resources.displayMetrics.density
        val visualDp      = bubbleView.getBubbleSizeDp()
        val touchTargetPx = bubbleWindowPx(visualDp)

        if (newState == RecordingState.RECORDING) {
            if (preclusterBubbleX == null) preclusterBubbleX = bubbleParams.x
            if (pushToTalkActive) {
                // HOLD Cancel surface window (Story 9-14 re-scope 2026-07-01, ADR-0019 Amendment):
                // the anchor bubble keeps its idle size+position (AC2) — the window only needs to
                // be just big enough to also hold the ONE Abbrechen target, which grows diagonally
                // up-and-toward-center from the bubble (see Dev Notes "Window geometry is now
                // diagonal, not vertical").
                bubbleView.dockSide = getDockSide()
                val btnDp        = bubbleView.recordingButtonSizeDp
                val bubbleSizeDp = visualDp
                val holdW = ((FloatingBubbleView.holdVisualWidthDp(btnDp, bubbleSizeDp)  + 2 * FloatingBubbleView.HOLD_SHADOW_PAD_DP) * dp).toInt()
                val holdH = ((FloatingBubbleView.holdVisualHeightDp(btnDp, bubbleSizeDp) + 2 * FloatingBubbleView.HOLD_SHADOW_PAD_DP) * dp).toInt()

                // X-anchor: dock-edge-anchor (same convention as the TAP branch below) — the bubble
                // sits at the same inset from the dock-side window edge as it did in the idle
                // window (Task 1.5: no separate HOLD-specific edge inset), so preserving the EDGE
                // position preserves the bubble's on-screen X automatically (AC2).
                bubbleParams.x      = maxOf(0, bubbleParams.x + touchTargetPx - holdW)
                bubbleParams.width  = holdW
                bubbleParams.height = holdH

                // Horizontal clamp toward whichever side the Abbrechen target now grows into (new
                // — the old purely-vertical layout never needed this, Task 6.3). For a right-docked
                // bubble the window extends LEFTWARD (already covered by the maxOf(0, ...) above);
                // for a left-docked bubble it extends RIGHTWARD and must not run past the screen's
                // right edge.
                val (screenW, _) = getScreenDimensions()
                bubbleParams.x = bubbleParams.x.coerceIn(0, maxOf(0, screenW - holdW))

                // Y-anchor: AC2 requires the bubble's drawn center to coincide EXACTLY with where
                // the idle bubble's center was (the thumb is still physically there), for EVERY
                // dock position (Code-Review Finding B, 2026-07-01): the old
                // `.coerceIn(0, maxHoldY)` here silently moved the anchor away from the thumb
                // whenever the window didn't fit above the bubble (high/low dock) — exactly the
                // "kein Größen-/Orts-Sprung" violation AC2 forbids and the bug that triggered this
                // re-scope in the first place. Fix: NEVER clamp the bubble away from idleCenterY.
                // Instead the Abbrechen target's GROWTH DIRECTION is dock-adaptive
                // ([FloatingBubbleView.holdGrowDirection]) — it grows upward when there's room
                // above (the normal case) and flips downward when docked too high for that to fit.
                // Both this window-sizing code and drawHoldTargets()/the ACTION_MOVE hit-test below
                // read the same [holdGrowDirection] field, so they can never diverge (same pattern
                // already established for [dockSide]).
                if (preclusterBubbleY == null) preclusterBubbleY = bubbleParams.y
                val idleCenterY = (preclusterBubbleY ?: bubbleParams.y) + touchTargetPx / 2

                val activeRPx = btnDp * FloatingBubbleView.HOLD_CANCEL_ACTIVE_SCALE / 2f * dp
                val targetSpanAbovePx = FloatingBubbleView.HOLD_CANCEL_OFFSET_Y_DP * dp + activeRPx +
                    FloatingBubbleView.HOLD_SHADOW_PAD_DP * dp
                // Grow up if the target's span fits above the bubble, else flip down. With the small
                // 48dp vertical offset (2026-07-01 re-tune) this span is short, so "up" fits for all
                // realistic dock heights and "down" (rare, very-high dock) only seats the ✗ ~48dp
                // below — neither branch reaches the header or dives into chat content anymore, which
                // is what the earlier 0.15·screenH threshold was compensating for with a big offset.
                bubbleView.holdGrowDirection = if (idleCenterY - targetSpanAbovePx >= 0f) "up" else "down"

                val bubbleCenterYPx = FloatingBubbleView.holdBubbleCenter(
                    bubbleView.dockSide, bubbleView.holdGrowDirection, holdW.toFloat(), holdH.toFloat(),
                    FloatingBubbleView.HOLD_SHADOW_PAD_DP * dp, bubbleSizeDp * dp
                ).y
                bubbleParams.y = (idleCenterY - bubbleCenterYPx).toInt()
            } else {
                // Compact cluster window (Story 9-16 revert of the 9-15 TAP surface): fixed
                // visual W/H + shadow pad on each side. The cluster is fixed-size — the size
                // slider (recordingButtonSizeDp) no longer affects this surface, only HOLD.
                val clusterW = ((FloatingBubbleView.CLUSTER_VISUAL_W_DP + 2 * FloatingBubbleView.CLUSTER_SHADOW_PAD_DP) * dp).toInt()
                val clusterH = ((FloatingBubbleView.CLUSTER_VISUAL_H_DP + 2 * FloatingBubbleView.CLUSTER_SHADOW_PAD_DP) * dp).toInt()
                // Right-edge-anchor: shift X left by the extra width so the dock-spot right edge stays
                // fixed. Clamp to 0 so the cluster stays on-screen when docked on the left side.
                bubbleParams.x      = maxOf(0, bubbleParams.x + touchTargetPx - clusterW)
                bubbleParams.width  = clusterW
                bubbleParams.height = clusterH
            }
        } else {
            // Restore single-bubble window.
            val savedX = preclusterBubbleX
            if (savedX != null && previousState == RecordingState.RECORDING) {
                bubbleParams.x = savedX
                preclusterBubbleX = null
            }
            // Restore Y if we were in the HOLD Cancel surface (preclusterBubbleY set by the HOLD
            // branch above). Null in TAP-only recording (Y was never shifted). Covers stop/cancel
            // while still in HOLD mode (finding 5).
            val savedY = preclusterBubbleY
            if (savedY != null && previousState == RecordingState.RECORDING) {
                bubbleParams.y = savedY
                preclusterBubbleY = null
            }
            bubbleParams.width  = touchTargetPx
            bubbleParams.height = touchTargetPx
        }
        updateBubbleLayout()
    }

    // --- Touch handling ---

    private fun handleTouch(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                // Lock onto the primary pointer (code review finding B): everything below this
                // point reads THIS pointer's coordinates, never an arbitrary/secondary one.
                activePointerId = event.getPointerId(0)
                dragTouchStartX = event.rawX
                dragTouchStartY = event.rawY
                bubbleStartX    = bubbleParams.x
                bubbleStartY    = bubbleParams.y
                isDragging         = false
                longPressTriggered = false
                pushToTalkActive   = false

                // Only arm long-press in IDLE state (push-to-talk)
                if (currentState == RecordingState.IDLE) {
                    handler.postDelayed(longPressRunnable, LONG_PRESS_TIMEOUT_MS)
                }
                return true
            }

            MotionEvent.ACTION_POINTER_DOWN, MotionEvent.ACTION_POINTER_UP -> {
                // A second finger touched down/lifted (code review finding B). Only the primary
                // pointer captured at ACTION_DOWN drives hit-tracking and release-to-commit below
                // — a secondary pointer must never be able to change which target the release
                // commits, or commit from the wrong location. No-op here by design.
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                // Resolve the primary pointer's current index (it can shift if another pointer
                // left the array before it) — ignore the event if it's no longer present.
                val pointerIndex = event.findPointerIndex(activePointerId)
                if (pointerIndex == -1) return true

                // During push-to-talk (HOLD Cancel surface, Story 9-14 re-scope 2026-07-01):
                // continuous hit-tracking against the single Abbrechen target, NOT
                // threshold-fire-on-move (see story Dev Notes "Release-to-commit is the core
                // mechanism change"). The window does not reposition during a hold, so the primary
                // pointer's (x, y) (view-local) is already in the same coordinate space
                // drawHoldTargets() uses. Also forwards the live finger position + the
                // dragged-away-from-bubble dead-zone flag (Task 4) so the View can render the
                // ghost-bubble/origin-fade dynamics (AC6) — no cancelRecording() here; that
                // happens on release (Task 5). A circular hit-test has no directional ambiguity
                // by construction, which structurally avoids the old code's diagonal-drag /
                // signed-direction-only bugs (findings 3/4) rather than just porting around them.
                if (pushToTalkActive) {
                    val touchX = event.getX(pointerIndex)
                    val touchY = event.getY(pointerIndex)
                    val dp = resources.displayMetrics.density
                    val shadowPad    = FloatingBubbleView.HOLD_SHADOW_PAD_DP * dp
                    val bubbleDiamPx = bubbleView.getBubbleSizeDp() * dp
                    val restRPx      = bubbleView.recordingButtonSizeDp * dp / 2f
                    val offsetXPx    = FloatingBubbleView.HOLD_CANCEL_OFFSET_X_DP * dp
                    val offsetYPx    = FloatingBubbleView.HOLD_CANCEL_OFFSET_Y_DP * dp

                    val bubbleCenter = FloatingBubbleView.holdBubbleCenter(
                        bubbleView.dockSide, bubbleView.holdGrowDirection,
                        bubbleView.width.toFloat(), bubbleView.height.toFloat(),
                        shadowPad, bubbleDiamPx
                    )
                    val cancelCenter = FloatingBubbleView.holdCancelCenter(
                        bubbleView.dockSide, bubbleView.holdGrowDirection, bubbleCenter, offsetXPx, offsetYPx
                    )

                    // Hit-test boundary stays the REST radius even when the target visually grows
                    // to ACTIVE (Task 4.2/AC4) — growing is feedback, not a hit-zone change; the
                    // finger is already inside once it crosses the REST circle.
                    bubbleView.holdTargetHit =
                        if (FloatingBubbleView.isInsideCircle(touchX, touchY, cancelCenter.x, cancelCenter.y, restRPx)) {
                            HoldTarget.CANCEL
                        } else {
                            HoldTarget.NONE
                        }

                    // Dead-zone (Task 3.5/4): reuses the existing free-drag dragThresholdPx
                    // convention (same ~10dp already tuned for that gesture) — once the finger
                    // has moved that far from the bubble, the ghost-bubble + origin-fade dynamics
                    // kick in (AC6).
                    val distFromBubble = Math.hypot(
                        (touchX - bubbleCenter.x).toDouble(), (touchY - bubbleCenter.y).toDouble()
                    )
                    bubbleView.holdDragging = distFromBubble > dragThresholdPx
                    // Clamp the DRAWN ghost position inside the overlay window so it can never be
                    // clipped away by the window edge (2026-07-01, Andi: "harte Kante unten — Bubble
                    // verschwindet"). The HOLD window spends its vertical budget on the target side
                    // of the bubble, leaving only ~shadowPad on the far side; dragging the finger
                    // past that edge drew the ghost outside the surface → it vanished. The hit-test
                    // above still uses the RAW touch coords, so target detection is unaffected — only
                    // the ghost's paint position is pinned to the window so it stays visible.
                    val ghostR = bubbleDiamPx * 0.92f / 2f
                    bubbleView.holdFingerX  = touchX.coerceIn(ghostR, bubbleView.width.toFloat()  - ghostR)
                    bubbleView.holdFingerY  = touchY.coerceIn(ghostR, bubbleView.height.toFloat() - ghostR)
                    return true
                }

                val dx = event.rawX - dragTouchStartX
                val dy = event.rawY - dragTouchStartY
                if (!isDragging && (abs(dx) > dragThresholdPx || abs(dy) > dragThresholdPx)) {
                    isDragging = true
                    // Moved too much -- cancel long-press
                    handler.removeCallbacks(longPressRunnable)
                }
                if (isDragging) {
                    bubbleParams.x = (bubbleStartX + dx).toInt()
                    bubbleParams.y = (bubbleStartY + dy).toInt()
                    try {
                        windowManager.updateViewLayout(bubbleView, bubbleParams)
                    } catch (e: Exception) {
                        KlarvoLogger.w(TAG, "Failed to update bubble position during drag", e)
                    }
                }
                return true
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                handler.removeCallbacks(longPressRunnable)
                // Primary pointer's index at release — may be -1 if it already left the screen
                // while a secondary pointer remained (ACTION_POINTER_UP, handled above as no-op);
                // guarded below wherever it's read.
                val pointerIndex = event.findPointerIndex(activePointerId)

                if (event.actionMasked == MotionEvent.ACTION_UP) {
                    when {
                        isDragging -> {
                            // AC9 (Story 9.3): edge-snap gated behind config.bubbleEdgeSnap.
                            // Default = true (canon snap + remembered-side). When OFF: raw drop X persists.
                            if (cachedConfig?.bubbleEdgeSnap != false) {
                                // Edge-snap: slide to nearest horizontal edge on drag release.
                                // 8dp margin from edge for a clean overlay feel.
                                val (screenW, _) = getScreenDimensions()
                                val dm = resources.displayMetrics
                                // WindowManager positions the window (windowPx wide, incl. shadow
                                // padding), not the visual squircle. All edge/midpoint math uses it.
                                val windowPx = bubbleWindowPx(bubbleView.getBubbleSizeDp())
                                val marginPx = (8 * dm.density).toInt()
                                val midScreen = screenW / 2
                                bubbleParams.x = if (bubbleParams.x + windowPx / 2 < midScreen) {
                                    marginPx              // snap left
                                } else {
                                    screenW - windowPx - marginPx  // snap right
                                }
                                updateBubbleLayout()
                            }
                            savePosition(bubbleParams.x, bubbleParams.y)
                        }
                        pushToTalkActive -> {
                            // Release-to-commit (Story 9-14 re-scope 2026-07-01, AC5): dispatch on
                            // where the finger WAS at release (holdTargetHit), not a
                            // threshold-fire mid-drag. Lands on Abbrechen -> cancel; anywhere else
                            // -> sends (no Sperren/lock target anymore — sending is the default).
                            // Mirrors handleTouch's HoldTarget import — same package as
                            // FloatingBubbleView, no qualification needed.
                            pushToTalkActive = false
                            when (bubbleView.holdTargetHit) {
                                HoldTarget.CANCEL -> {
                                    bubbleView.holdDockActive = false
                                    cancelRecording()
                                }
                                HoldTarget.NONE -> stopAndProcessRecording()
                            }
                            bubbleView.holdTargetHit = HoldTarget.NONE
                            bubbleView.holdDragging  = false
                        }
                        !longPressTriggered -> {
                            // Use the primary pointer's coordinates (finding B) — falls back to a
                            // no-op tap if it's already gone (see pointerIndex comment above).
                            if (pointerIndex != -1) {
                                handleTap(event.getX(pointerIndex), event.getY(pointerIndex))
                            }
                        }
                    }
                } else {
                    // ACTION_CANCEL while push-to-talk -> cancel recording. OS-interrupted touch
                    // (distinct from a normal finger lift) — not covered by AC5 (release
                    // semantics only); keeping the existing safe default is intentional, not an
                    // oversight (Task 7.2).
                    if (pushToTalkActive) {
                        pushToTalkActive = false
                        cancelRecording()
                    }
                    bubbleView.holdTargetHit = HoldTarget.NONE
                    bubbleView.holdDragging  = false
                }
                activePointerId = MotionEvent.INVALID_POINTER_ID
                // Story 11-4 (AC-1): the gesture just ended -- if a bubble-above-panel reorder was
                // deferred while it was in flight (see showListeningPanel), do it now.
                if (bubbleReorderPending) {
                    bubbleReorderPending = false
                    // F3: this fires synchronously from inside bubbleView's own ACTION_UP
                    // dispatch -- reorderBubbleAbovePanel() would remove/re-add that same view
                    // while it's still handling its own touch event. Post it instead so the
                    // reorder runs after dispatch has unwound.
                    handler.post {
                        // F4: the panel may have been hidden (e.g. AUTOSTOP/AUTO auto-stop)
                        // between the deferral and this post running. Reordering above an
                        // already-gone panel is a pointless removeView+addView (flicker,
                        // animator reset) with no z-order benefit -- skip it.
                        if (panelVisible) {
                            reorderBubbleAbovePanel()
                        }
                    }
                }
                return true
            }
        }
        return false
    }

    private fun savePosition(x: Int, y: Int) {
        val (screenW, _) = getScreenDimensions()
        // Side detection uses the full window width (incl. shadow padding), matching WindowManager
        // placement logic so left/right classification agrees with edge-snap math.
        val windowPx = bubbleWindowPx(bubbleView.getBubbleSizeDp())
        val side = if (x + windowPx / 2 < screenW / 2) "left" else "right"
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_X, x)
            .putInt(PREF_Y, y)
            .putString(PREF_SIDE, side)
            .apply()
    }

    // --- State machine ---

    /**
     * Handles a tap (no drag, no long-press).
     *
     * Behavior depends on [tapMode]:
     *
     * HOLD mode:
     *   IDLE -> expand to TAP surface
     *   RECORDING -> tap Send/Cancel circles on the TAP surface
     *
     * TOGGLE / AUTOSTOP mode:
     *   IDLE -> start recording (TAP surface)
     *   RECORDING -> tap Send/Cancel circles
     *
     * AUTO mode:
     *   IDLE -> start auto-loop (records, processes, repeats until tapped again)
     *   RECORDING -> stop loop + process current segment
     *
     * @param touchX Touch x-coordinate relative to the view's left edge.
     * @param touchY Touch y-coordinate relative to the view's top edge.
     *               Required for 2D circular hit detection on the TAP surface (Story 9-15).
     */
    private fun handleTap(touchX: Float, touchY: Float) {
        when (currentState) {
            RecordingState.IDLE -> {
                activeGesture = "tap"
                // Reload config before checking mode so Settings changes apply immediately.
                loadBubbleControls()
                if (tapMode == RecordingMode.AUTO) {
                    autoLoopActive = true
                }
                startRecording()
            }
            RecordingState.RECORDING -> {
                // Compact cluster (Story 9-16 revert): two 1-D X-band zones flanking the waveform.
                // pushToTalkActive: finger still held → release handles PTT confirm.
                if (pushToTalkActive) return
                when {
                    bubbleView.isTouchInConfirmZone(touchX) -> {
                        // ➤ Send (RIGHT)
                        if (tapMode == RecordingMode.AUTO) autoLoopActive = false
                        stopAndProcessRecording()
                    }
                    bubbleView.isTouchInCancelZone(touchX) -> {
                        // ✗ Cancel (LEFT)
                        cancelRecording()
                    }
                    // Waveform/dead area between the buttons: no-op.
                }
            }
            RecordingState.TRANSCRIBING -> {
                // Stop auto-loop so the cycle doesn't repeat after transcribing finishes.
                if (autoLoopActive) {
                    autoLoopActive = false
                    KlarvoLogger.d(TAG, "Auto-loop deactivated by tap during transcribing")
                }
            }
            RecordingState.DONE -> {
                // Placeholder state: no user action during the 800ms DONE flash.
            }
        }
    }

    // --- Audio recording ---

    private fun startRecording() {
        // Check runtime permission before starting audio capture.
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED
        ) {
            KlarvoLogger.e(TAG, "RECORD_AUDIO permission not granted at recording time")
            showToast("Microphone permission required. Please grant in app settings.")
            return
        }

        // Pre-check: verify that a valid config with API keys exists before we start recording.
        // Without this check, the pipeline would silently fail after the user has already
        // recorded audio -- which is confusing. Failing fast here gives immediate feedback.
        val preCheckConfig = cachedConfig ?: KlarvoApi.readConfig(this)
        if (preCheckConfig == null) {
            // config.json missing entirely -- app was never configured via the desktop UI.
            KlarvoLogger.w(TAG, "startRecording: config.json not found or incomplete -- aborting")
            showToast("No configuration found. Open Klarvo on your desktop and configure the app first.")
            return
        }
        if (preCheckConfig.sttProvider != "local" && preCheckConfig.groqApiKey.isBlank()) {
            // Cloud STT selected but no Groq key present.
            KlarvoLogger.w(TAG, "startRecording: Groq API key missing -- aborting")
            showToast("No API key configured. Open Klarvo Settings and add your Groq key.")
            return
        }

        // Determine which mode governs this recording session.
        val activeMode = when (activeGesture) {
            "longpress" -> longPressMode
            else        -> tapMode  // "tap" or null (auto-loop restart)
        }

        // Select the silence duration by the ACTIVE MODE via the pure companion function.
        // See RecordingMode.selectSilenceSecs() for the full rationale and desktop parity ref.
        val activeSilenceSecs = RecordingMode.selectSilenceSecs(
            mode              = activeMode,
            gesture           = activeGesture,
            tapSilence        = tapSilenceSecs,
            longPressSilence  = longPressSilenceSecs,
            autostopSilence   = autostopSilenceSecs,
            autoModeSilence   = autoModeSilenceSecs,
        )
        KlarvoLogger.d(TAG, "[pipeline] silence window: mode=$activeMode → ${activeSilenceSecs}s")

        val recorder = KlarvoAudioRecorder(
            context = this,
            onAmplitude = { amplitude ->
                handler.post {
                    bubbleView.amplitude = amplitude
                    panelView?.amplitude = amplitude
                }
            },
            silenceSecs = activeSilenceSecs,
            energyGateThreshold = silenceThreshold,
            previewPauseSilenceSecs = cachedConfig?.previewPauseSilenceSecs ?: 2.0f
        )

        // Wire up silence detection for AUTOSTOP / AUTO modes.
        if (activeMode == RecordingMode.AUTOSTOP || activeMode == RecordingMode.AUTO) {
            recorder.onSilenceDetected = {
                handler.post { onSilenceTriggered() }
            }
        }

        // Story 11-2 (AC-1/AC-3/AC-4, Task 2.3): repeatable preview-flush callback, HOLD/TOGGLE
        // only, opt-in via Settings. Fresh accumulator for this recording (AC-7's clear-on-finish
        // guarantees this is already empty, but reset defensively in case of an earlier bail-out).
        if (RecordingMode.shouldInstallPreviewFlush(
                activeMode,
                cachedConfig?.livePreviewEnabled == true,
                cachedConfig?.sttProvider ?: "groq"
            )
        ) {
            previewAccumulatedText = ""
            recorder.onPreviewPause = {
                handler.post { flushPreviewDelta() }
            }
        }

        try {
            recorder.start()
        } catch (e: SecurityException) {
            KlarvoLogger.e(TAG, "Permission denied when starting audio recording", e)
            showToast("Microphone permission denied. Please grant in app settings.")
            return
        } catch (e: IllegalStateException) {
            KlarvoLogger.w(TAG, "Failed to start audio recording", e)
            showToast("Cannot start recording: ${e.message}")
            return
        }

        audioRecorder = recorder

        // TAP surface: record start time for the chip timer display.
        bubbleView.recordingStartMs = System.currentTimeMillis()

        // TAP surface: expand window and set state.
        val preRecordingState = currentState
        setState(RecordingState.RECORDING)
        adjustLayoutForState(RecordingState.RECORDING, preRecordingState)

        // AUTO mode: paste-on-silence immediately — no panel, keyboard stays active.
        // All other modes: dismiss keyboard + show passive listening panel.
        if (activeMode != RecordingMode.AUTO) {
            (getSystemService(Context.INPUT_METHOD_SERVICE)
                as? android.view.inputmethod.InputMethodManager)
                ?.hideSoftInputFromWindow(bubbleView.windowToken, 0)
            showListeningPanel(ListeningPanelView.State.RECORDING)
            if (panelVisible) panelView?.startTimer()
        }
    }

    /**
     * Called when silence detection fires (AUTOSTOP / AUTO modes only).
     * Must be called on the main thread.
     */
    private fun onSilenceTriggered() {
        if (currentState != RecordingState.RECORDING) return

        // Story 11-1 (spike): capture the pause-signal timestamp right here -- this is
        // KlarvoAudioRecorder.onSilenceDetected reaching the service, i.e. the "pause"
        // moment the live-preview benchmark measures from. Threaded through to
        // processAudio() purely for logging; does not affect recording/STT behavior (AC4).
        // Monotonic (SystemClock.elapsedRealtime()) rather than wall-clock: a clock step
        // (NTP sync) between pause and end-capture must not corrupt the benchmark interval
        // (code review finding, 2026-07-01).
        val pauseSignalMs = SystemClock.elapsedRealtime()

        val activeMode = when (activeGesture) {
            "longpress" -> longPressMode
            else        -> tapMode
        }

        when (activeMode) {
            RecordingMode.AUTOSTOP -> {
                stopAndProcessRecording(pauseSignalMs)
            }
            RecordingMode.AUTO -> {
                // Stop current segment and process it, then start a new recording
                // if the auto-loop is still active.
                stopAndProcessRecording(pauseSignalMs)
                // processAudio will call onProcessingComplete which handles the loop restart.
            }
            else -> { /* should not happen */ }
        }
    }

    /**
     * Story 11-2 (AC-1/Task 2.4): called (via `handler.post`) whenever
     * [KlarvoAudioRecorder.onPreviewPause] fires. Takes the delta-since-last-flush WAV,
     * transcribes it RAW (no LLM cleanup, mirrors AC-1/FR1 parity) via the same
     * `transcribeWithRetry` + hallucination-filter chain [processAudio] uses, and appends the
     * result to the panel. Recording is NOT stopped, nothing is pasted -- fail-soft on any
     * error (log + skip, mirrors desktop AC-8 in Story 5-1).
     *
     * Code-review fix F3 (2026-07-01): dispatched onto [previewFlushExecutor] (single-thread,
     * FIFO) instead of an independent `Thread` per pause -- variable Groq latency could
     * otherwise let pause N+1's transcription complete before pause N's, scrambling the
     * appended order. One flush is in-flight at a time; the caller (main thread) never blocks.
     */
    private fun flushPreviewDelta() {
        if (currentState != RecordingState.RECORDING) return
        val recorder = audioRecorder ?: return
        val config = cachedConfig ?: return
        // Story 13-2 (E1 / D-H9): re-checked AT FLUSH TIME, not only at
        // install time, exactly as `pipeline::flush_preview_delta` does -- the
        // config can change between starting a recording and the first pause.
        // With a stored `local` STT provider no byte may leave the device, key
        // or no key.
        if (config.sttProvider == LOCAL_PROVIDER_ID) {
            KlarvoLogger.d(TAG, "[preview] offline STT stored -- flush suppressed, nothing uploaded")
            return
        }
        val wavBytes = recorder.deltaSnapshotWav() ?: return

        previewFlushExecutor.execute {
            try {
                val text = transcribeWithRetry(
                    wavBytes,
                    config.groqApiKey,
                    config.language,
                    "whisper-large-v3-turbo",
                    config.dictionaryTerms,
                    sttHintFor(config),
                    null, // preview chunks are display-only -- no pending-WAV backup needed
                    config.testProviderStt
                )
                if (text.isBlank()) return@execute
                if (GroqSttBridge.nativeIsHallucination(text)) {
                    KlarvoLogger.d(TAG, "[preview] chunk filtered as hallucination, skipping")
                    return@execute
                }
                handler.post { appendPreviewText(text) }
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "[preview] flush failed, skipping (fail-soft)", e)
            }
        }
    }

    /**
     * Story 11-2 (AC-5): appends [text] to the accumulated preview and pushes it to the panel.
     *
     * Code-review fix F1 (2026-07-01): a preview STT chunk is dispatched (Groq round-trip) while
     * still RECORDING, but can complete AFTER the user has already finished
     * ([stopAndProcessRecording], which clears `previewAccumulatedText`) or cancelled
     * ([cancelRecording], same reset) -- i.e. after the state has moved on to TRANSCRIBING/IDLE.
     * Without this recheck, that late chunk would re-populate stale preview text onto the panel,
     * which is still visible during TRANSCRIBING. Bail out and drop the chunk once we're no
     * longer RECORDING; this is called on the main thread ([handler.post]), same thread that
     * flips [currentState], so there is no race on the read itself.
     *
     * Story 11-3 (Task 4.2): chunks are newline-joined (`"\n"`), not space-joined (`" "`).
     * 11-2's code review accepted the space-join as a Low residual for *accuracy* reasons
     * (orientation surface, not accuracy) -- that acceptance does not extend to this story's
     * *display* mechanics. One line per flush chunk reads better in the scrollable transcript
     * (AC-3 pivot, 2026-07-08) than one run-on paragraph would.
     */
    private fun appendPreviewText(text: String) {
        if (currentState != RecordingState.RECORDING) return
        val cleaned = sanitizePreviewChunk(text) ?: return
        previewAccumulatedText = if (previewAccumulatedText.isBlank()) cleaned else "$previewAccumulatedText\n$cleaned"
        panelView?.rawTranscript = previewAccumulatedText
    }

    /**
     * Stops recording and discards the captured audio.
     * Returns the bubble to IDLE immediately without calling the STT pipeline.
     */
    private fun cancelRecording() {
        val recorder = audioRecorder ?: return
        audioRecorder = null
        autoLoopActive = false

        // Release the recorder on a background thread (stop() can block briefly)
        Thread {
            recorder.releaseImmediately()
        }.start()

        // Reset HOLD Cancel surface state (Story 9-14). holdTargetHit/holdDragging reset here too
        // — finding 2's lesson (a transient HOLD flag never cleared on stop/cancel can leak into
        // the next recording) applies to any HOLD-related transient state, not just the old
        // isLockedMode flag (removed in the 2026-07-01 re-scope — Sperren/lock is gone).
        bubbleView.holdDockActive = false
        bubbleView.holdTargetHit  = HoldTarget.NONE
        bubbleView.holdDragging   = false
        setHoldModeOnPanel(false)
        // Reset TAP surface timer (Story 9-15).
        bubbleView.recordingStartMs = 0L

        val previousState = currentState
        // Detect expanded mode: any non-square window means the TAP surface / HOLD Cancel surface
        // is shown (IDLE/TRANSCRIBING/DONE are always touchTargetPx × touchTargetPx — square).
        // NOT width>height: HOLD's window (Story 9-14) grows diagonally from the bubble, not
        // purely horizontally like TAP's wider-than-tall layout — a width>height check would
        // silently skip the shrink-back-to-idle resize for HOLD.
        val wasBarMode = bubbleView.width != bubbleView.height
        setState(RecordingState.IDLE)
        // Only adjust layout if we were in expanded mode.
        if (previousState == RecordingState.RECORDING && wasBarMode) {
            adjustLayoutForState(RecordingState.IDLE, previousState)
        }
        // Story 11-2: reset the preview accumulator so a cancelled recording never leaks its
        // partial preview text into the next one (the panel itself is a fresh instance per
        // showListeningPanel() call, so this is a defensive service-level reset, not a visible fix).
        previewAccumulatedText = ""
        // Story 9.5: hide listening panel on cancel.
        hideListeningPanel()
    }

    /**
     * Stops recording and starts the STT + cleanup pipeline.
     * This is the "confirm" action -- used by the ✓ button and push-to-talk release.
     *
     * @param pauseSignalMs Story 11-1 (spike): when non-null, this call was triggered by
     *   [onSilenceTriggered] (a VAD pause, not a manual tap/release) and holds the
     *   `SystemClock.elapsedRealtime()` at which the pause fired. Threaded through to
     *   [processAudio] purely so the benchmark log line can compute pause-to-text latency;
     *   null for manual stops (button tap, push-to-talk release), which are not "pause" events.
     */
    private fun stopAndProcessRecording(pauseSignalMs: Long? = null) {
        val recorder = audioRecorder ?: return
        audioRecorder = null

        // Reset HOLD Cancel surface state (Story 9-14). holdTargetHit/holdDragging reset here too
        // — finding 2's lesson (a transient HOLD flag never cleared on stop/cancel can leak into
        // the next recording) applies to any HOLD-related transient state, not just the old
        // isLockedMode flag (removed in the 2026-07-01 re-scope — Sperren/lock is gone).
        bubbleView.holdDockActive = false
        bubbleView.holdTargetHit  = HoldTarget.NONE
        bubbleView.holdDragging   = false
        setHoldModeOnPanel(false)
        // Reset TAP surface timer (Story 9-15).
        bubbleView.recordingStartMs = 0L

        val previousState = currentState
        setState(RecordingState.TRANSCRIBING)
        // Only adjust layout if we were in expanded mode (TAP surface / HOLD targets — any
        // non-square window; see cancelRecording()'s wasBarMode comment for why not width>height).
        if (previousState == RecordingState.RECORDING && bubbleView.width != bubbleView.height) {
            adjustLayoutForState(RecordingState.TRANSCRIBING, previousState)
        }

        // Story 9.5: transition panel to TRANSCRIBING (keep visible, change appearance).
        panelView?.let { panel ->
            panel.stopTimer()
            panel.panelState = ListeningPanelView.State.TRANSCRIBING
            panel.invalidate()
        }

        Thread {
            val wavBytes = recorder.stop()
            processAudio(wavBytes, pauseSignalMs)
        }.start()

        // Story 11-2 (AC-7): clear the accumulated preview text + implicitly the delta marker
        // (a fresh KlarvoAudioRecorder is constructed per recording in startRecording(), so its
        // marker already starts at 0 -- nothing to reset there). Placed here, right after the
        // finish/paste chain is kicked off, rather than inside processAudio()'s many branches --
        // the finish path itself (processAudio -> paste) stays completely untouched, and no
        // further preview flushes can occur once audioRecorder is null (already cleared above).
        previewAccumulatedText = ""
    }

    // --- API pipeline ---

    /**
     * @param pauseSignalMs Story 11-1 (spike): non-null only when this call originated from a
     *   VAD pause ([onSilenceTriggered] / AUTOSTOP+AUTO modes). Used solely to log the
     *   pause-to-raw-text latency benchmark below -- does not affect the pipeline itself (AC4).
     */
    private fun processAudio(wavBytes: ByteArray, pauseSignalMs: Long? = null) {
        val t0 = System.currentTimeMillis()
        if (wavBytes.isEmpty()) {
            handler.post {
                // Story 13-2 (D11 / D-M14): this toast STAYS while the other
                // four go. An empty capture buffer is a recorder/plumbing
                // fault, not a recognition result -- there is nothing for the
                // user to have said differently, and swallowing it would create
                // exactly the silent loss this story exists to remove. Andi
                // confirmed the four-of-five reading on 2026-09-21.
                showToast("No audio recorded")
                autoLoopActive = false
                hideListeningPanel()
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        // Pre-STT filter: discard mini-taps and silent recordings before the Groq API call.
        // Delegates to the shared Rust silence_skip via GroqSttBridge (ADR-0017, AC4).
        // M2 (AC5, Story 7-2): minRecordingMs/silenceThreshold are now read from cachedConfig
        // (populated by AC1's config wiring) instead of hardcoded literals -- 500L/0.005f are
        // now only the null-safe fallback default (unchanged values, not a behavior change if
        // config is somehow absent), matching KlarvoApi.Config's own defaults.
        run {
            val filterMinRecordingMs = resolveMinRecordingMsForSilenceFilter(cachedConfig)
            val filterSilenceThreshold = silenceThreshold
            val wavBase64ForFilter = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
            val silenceResult = GroqSttBridge.nativeSilenceCheck(wavBase64ForFilter, filterMinRecordingMs, filterSilenceThreshold)
            when {
                silenceResult.startsWith("TooShort:") -> {
                    val durationMs = silenceResult.removePrefix("TooShort:").toLongOrNull() ?: 0L
                    KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: TooShort (${durationMs}ms < ${filterMinRecordingMs}ms)")
                    handler.post {
                        // Story 13-2 (D11 / D-M14): silent, like Desktop's
                        // message-less `PipelineEvent::idle()`. See the note at
                        // "No audio recorded" above for why that one stays.
                        autoLoopActive = false
                        hideListeningPanel()
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                silenceResult.startsWith("Silent:") -> {
                    val rms = silenceResult.removePrefix("Silent:").toFloatOrNull() ?: 0f
                    KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: Silent (rms=$rms < $filterSilenceThreshold)")
                    handler.post {
                        // Story 13-2 (D11 / D-M14): silent, like Desktop.
                        autoLoopActive = false
                        hideListeningPanel()
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                else -> { /* "Pass" — proceed to STT */ }
            }
        }

        // Persist WAV to disk before any network call so audio survives an app kill or
        // transient network failure.  The file is cleaned up after a successful STT call.
        val pendingWavFile = savePendingWav(wavBytes)

        // Use cached config from loadBubbleControls() (called moments ago by handleTap/longPress).
        // Fall back to a fresh read if the cache is somehow stale (e.g. auto-loop restart path).
        val config = cachedConfig ?: KlarvoApi.readConfig(this)
        val tConfig = System.currentTimeMillis()
        KlarvoLogger.d(TAG, "[pipeline] config read: ${tConfig - t0}ms")

        if (config == null || (config.sttProvider != "local" && config.groqApiKey.isBlank())) {
            handler.post {
                showToast("No API keys configured. Please open Klarvo and add your Groq key in Settings.")
                autoLoopActive = false
                hideListeningPanel()
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        // Story 12-1 GATE-4 follow-up: status/fallback toasts fired during STT/cleanup are
        // overridden ~1s later by HyperOS's own "pasted from your clipboard" system toast when
        // the paste reads the clipboard (Android shows only one toast at a time). Deferring the
        // message and showing it AFTER the paste makes it the newest toast, so it wins.
        var degradeStatusMsg: String? = null
        // Story 7-10 (AC2): an EXPLICIT cleanup-failure flag. `degradeStatusMsg`
        // cannot serve as the predicate — it is also set when the fallback
        // provider SUCCEEDED ("⚠ Cleanup-Anbieter gewechselt"), and `finalText`
        // looks identical whether cleanup worked or degraded. Set only on the
        // true failure branches in Step 2; consumed by Step 4 via
        // [decideDelivery].
        var llmCleanupFailed = false

        try {
            // Step 1: STT -- cloud (Groq) or local (whisper.cpp via JNI)
            //
            // Story 13-1b: the TEST provider wins over `sttProvider`, exactly as
            // the Rust twin's `pipeline::resolve_stt_provider` returns it BEFORE
            // it reads `stt_provider`. Without the second condition a stored
            // `sttProvider = "local"` would take the local-whisper branch and the
            // `Test provider (STT)` row would silently do nothing on Android --
            // a state that was unreachable under 13-1 (`"debug"` is not
            // `"local"`) and became reachable when selection moved to its own
            // key. The value itself is still carried through uninspected: the
            // cloud branch below hands it to the Rust core (ADR-0017).
            val transcript = if (
                config.sttProvider == "local" &&
                config.testProviderStt == KlarvoApi.TEST_PROVIDER_OFF
            ) {
                val tLocalStart = System.currentTimeMillis()

                // Resolve model file path (shared with the automatic Groq-failure
                // safety net below, AC3/story 12-1 -- see resolveLocalWhisperModelFile).
                val modelFile = resolveLocalWhisperModelFile()

                KlarvoLogger.d(TAG, "[local-stt] model path: $modelFile, exists=${modelFile.exists()}")

                if (!modelFile.exists()) {
                    KlarvoLogger.e(TAG, "[local-stt] Whisper model not found: $modelFile")
                    handler.post {
                        showToast("Whisper model not downloaded. Please download in Settings.")
                        autoLoopActive = false
                        hideListeningPanel()
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }

                KlarvoLogger.d(TAG, "[local-stt] nativeAvailable=${LocalWhisperInference.isNativeAvailable()}, isModelLoaded=${LocalWhisperInference.isModelLoaded()}")

                if (!LocalWhisperInference.isModelLoaded()) {
                    KlarvoLogger.d(TAG, "[local-stt] loading model: ${modelFile.absolutePath}")
                    val loadOk = LocalWhisperInference.load(modelFile.absolutePath)
                    KlarvoLogger.d(TAG, "[local-stt] load result: $loadOk")
                    if (!loadOk) {
                        KlarvoLogger.e(TAG, "[local-stt] Failed to load whisper model: $modelFile")
                        handler.post {
                            showToast("Failed to load Whisper model")
                            autoLoopActive = false
                            hideListeningPanel()
                            val prev = currentState
                            setState(RecordingState.IDLE)
                            adjustLayoutForState(RecordingState.IDLE, prev)
                        }
                        return
                    }
                }

                val wavBase64 = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
                KlarvoLogger.d(TAG, "[local-stt] calling transcribeAudio, base64 len=${wavBase64.length}, lang=${config.language}")
                val result = LocalWhisperInference.transcribeAudio(wavBase64, config.language)
                KlarvoLogger.d(TAG, "[local-stt] transcribeAudio result: '${result.take(100)}' (len=${result.length})")
                val tLocalEnd = System.currentTimeMillis()
                KlarvoLogger.d(TAG, "[pipeline] local STT: ${tLocalEnd - tLocalStart}ms (${wavBytes.size / 1024}KB audio)")

                if (result.isBlank()) {
                    KlarvoLogger.e(TAG, "Local transcription returned empty result")
                    handler.post {
                        showToast("Transcription failed")
                        autoLoopActive = false
                        hideListeningPanel()
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                result
            } else {
                try {
                    transcribeWithRetry(
                        wavBytes,
                        config.groqApiKey,
                        config.language,
                        "whisper-large-v3-turbo", // H9: model comes from Rust config (sttModel not yet in Android AppConfig; default parity)
                        config.dictionaryTerms,
                        // Story 13-2 (B4 / D-H4): the conditioning hint. This
                        // was `config.customPrompt` -- the LLM cleanup
                        // instruction -- which replaced Whisper's language hint
                        // in the request AND became the input of both post-STT
                        // guards.
                        sttHintFor(config),
                        pendingWavFile,
                        config.testProviderStt
                    )
                } catch (sttEx: IOException) {
                    // AC3: automatic local-Whisper safety net after Groq's retries are
                    // exhausted. Rethrow the original exception when no local model is
                    // usable -- the outer catch below preserves `pendingWavFile` (already
                    // kept on disk by transcribeWithRetry) and shows a clear error, so the
                    // dictation is never silently lost.
                    //
                    // Finding D: only fall back for a RETRYABLE failure (5xx/transport,
                    // already exhausted by transcribeWithRetry's own backoff) -- mirrors
                    // Rust's is_retryable_stt_error gate. A non-retryable failure (empty
                    // audio, 4xx/auth) is a guaranteed repeat, so loading/running the local
                    // model for it would be pure waste; go straight to the terminal path.
                    if (!isRetryableSttFailure(sttEx)) {
                        throw sttEx
                    }
                    val modelFile = resolveLocalWhisperModelFile()
                    if (!modelFile.exists() || !LocalWhisperInference.isNativeAvailable()) {
                        throw sttEx
                    }
                    KlarvoLogger.w(TAG, "[pipeline] Groq STT failed after retries ($sttEx), falling back to local Whisper", sttEx)
                    if (!LocalWhisperInference.isModelLoaded() && !LocalWhisperInference.load(modelFile.absolutePath)) {
                        throw sttEx
                    }
                    val wavBase64 = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
                    val localResult = LocalWhisperInference.transcribeAudio(wavBase64, config.language)
                    if (localResult.isBlank()) {
                        throw sttEx
                    }
                    degradeStatusMsg = "⚠ Groq am Limit → lokale Transkription"
                    localResult
                }
            }
            val tStt = System.currentTimeMillis()
            // Story 11-1 (spike, code review fix): dedicated monotonic end-capture for the
            // benchmark only -- taken at the same point as tStt above, but via
            // SystemClock.elapsedRealtime() so the pause-to-text interval can't be corrupted
            // by a wall-clock step (NTP sync) between pause and here. tStt/tConfig above are
            // unchanged and still drive the existing [pipeline] logs.
            val benchmarkEndElapsedMs = SystemClock.elapsedRealtime()
            KlarvoLogger.d(TAG, "[pipeline] STT: ${tStt - tConfig}ms (${wavBytes.size / 1024}KB audio, provider=${config.sttProvider})")

            // STT succeeded -- safe to remove the pending WAV backup.
            pendingWavFile?.delete()

            // A blank transcript is also what the shared guard chain returns
            // when it drops the text as a prompt echo or a blocklist match
            // (`stt::groq_jni::guard_transcript_for_jni`), so this one branch
            // is the shipped ending for every "nothing recognised" outcome.
            if (transcript.isBlank()) {
                handler.post {
                    // Story 13-2 (D11 / D-M14): silent, like Desktop.
                    autoLoopActive = false
                    hideListeningPanel()
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            // Story 11-1 (spike): pause-to-raw-text latency benchmark. Only logged for
            // pause-triggered stops (AUTOSTOP/AUTO -- pauseSignalMs non-null) and only once the
            // transcript is confirmed non-blank (code review fix: a blank "no speech" round-trip
            // is not a real sample and must not contaminate the distribution). Log-only, no
            // behavior change (AC4). `stt=` is the already-computed STT sub-duration so
            // retry-inflated samples (transcribeWithRetry backs off >=2000ms on transient
            // failures) are identifiable and excludable.
            if (pauseSignalMs != null) {
                val pauseToTextMs = benchmarkEndElapsedMs - pauseSignalMs
                KlarvoLogger.d(TAG, "[benchmark-11-1] pause-to-text=${pauseToTextMs}ms stt=${tStt - tConfig}ms")
            }

            // Hallucination guard via shared Rust (ADR-0017, AC2).
            // Replaces HallucinationFilter.isHallucination() — same logic, single Rust source.
            if (GroqSttBridge.nativeIsHallucination(transcript)) {
                KlarvoLogger.d(TAG, "[pipeline] hallucination filtered (Rust): '${transcript.take(60)}'")
                handler.post {
                    // Story 13-2 (D11 / D-M14): silent, like Desktop.
                    autoLoopActive = false
                    hideListeningPanel()
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            // Tracks LLM cleanup latency for feedback metrics.
            // Remains null when cleanup is skipped or fails (no key, exception).
            var llmLatencyMs: Long? = null

            // Step 2: Text cleanup via configured LLM provider (optional -- skip if no key)
            //
            // Story 13-2 (E2 / G2a, rows D-H10 / D-M21): THE offline rule, the
            // twin of `pipeline::offline_rule_with`. It replaced
            // `config.llmProvider == "local"`, which read only half the
            // question: after a LOCAL transcript the cloud cleanup arm still
            // ran for any cloud `llmProvider`, so "Offline" uploaded the text
            // it had just kept on the device.
            //
            // The `cleanupLocal` (MNN) call is gone from this path rather than
            // guarded: [LOCAL_CLEANUP_AVAILABLE] is false, so the predicate
            // returns "no cleanup" for every `llmProvider = "local"` config and
            // the branch was unreachable. It was also inert (drift row D-H18) --
            // it threw and the catch degraded to the raw transcript, i.e. the
            // same output this takes directly. `KlarvoApi.cleanupLocal` itself
            // is left alone; hiding the control is 13-3's row and building the
            // local path is a separate decision (G3b).
            val effectiveLlm = KlarvoApi.effectiveLlmProviderName(config)
            val finalText = if (skipsCloudCleanup(config.sttProvider, effectiveLlm, LOCAL_CLEANUP_AVAILABLE)) {
                KlarvoLogger.i(
                    TAG,
                    "[pipeline] Offline rule: no cleanup (stt=${config.sttProvider}, llm=$effectiveLlm, localCleanup=$LOCAL_CLEANUP_AVAILABLE)"
                )
                KlarvoApi.sanitizeLlmOutput(transcript)
            } else {
                val llmProvider = KlarvoApi.resolveLlmProvider(config)
                if (llmProvider != null) {
                    try {
                        val result = KlarvoApi.cleanupChunked(
                            text = transcript,
                            provider = llmProvider,
                            style = config.cleanupStyle,
                            dictionaryTerms = config.dictionaryTerms.takeIf { it.isNotBlank() },
                            customInstructions = config.customPrompt.takeIf { it.isNotBlank() }
                        )
                        val tCleanup = System.currentTimeMillis()
                        llmLatencyMs = tCleanup - tStt
                        KlarvoLogger.d(TAG, "[pipeline] cleanup: ${tCleanup - tStt}ms (${llmProvider.model})")
                        result
                    } catch (e: Exception) {
                        // AC2: try a fallback cleanup provider (never Groq, never the one
                        // that just failed) before degrading to raw text -- mirrors the
                        // Rust pipeline's fallback-then-raw-text sequence, which this
                        // runtime-failure path previously skipped entirely.
                        //
                        // Finding H: catches ANY exception (not just IOException) so a
                        // non-IOException from cleanupChunked (e.g. RuntimeException /
                        // ExecutionException) still degrades to raw text instead of
                        // escaping and losing the already-obtained transcript.
                        KlarvoLogger.w(TAG, "Text cleanup failed -- trying fallback provider", e)
                        KlarvoApi.updateFeedbackMetrics(this) { m ->
                            m.copy(llmErrorCount = m.llmErrorCount + 1)
                        }
                        // Finding C: exclude the ACTUALLY resolved provider name
                        // (llmProvider.providerName), not config.llmProvider -- when
                        // resolveLlmProvider substituted a fallback because the configured
                        // provider had no key, config.llmProvider names a provider that
                        // never ran, so excluding it would let the fallback re-pick the
                        // same substitute that just failed.
                        //
                        // Finding D: only actually attempt a fallback for a RETRYABLE
                        // failure (429/5xx/transport) -- mirrors Rust's is_retryable_llm_error
                        // gate. A non-retryable (e.g. 400/401 config/auth) failure is a
                        // guaranteed repeat, so degrade straight to raw text instead of
                        // burning another provider's quota for nothing.
                        //
                        // Story 13-2 (D10 / D-M2): the `e is IOException &&` pre-gate is
                        // gone -- it WAS the bug. A malformed provider answer throws a bare
                        // JSONException, which is not an IOException, so on a dictation
                        // under CHUNK_THRESHOLD the ladder never ran, while the chunked
                        // path (where collectChunkResults rewraps it) fell back normally.
                        // The type decision now lives in the pure companion
                        // [isRetryableCleanupFailure], which classifies a JSONException as
                        // retryable (Rust's twin: an undecodable body becomes the retryable
                        // LlmError::Request) and keeps every other non-IOException
                        // non-retryable, as before.
                        val fallbackProvider = if (isRetryableCleanupFailure(e)) {
                            KlarvoApi.resolveFallbackLlmProvider(config, llmProvider.providerName)
                        } else {
                            null
                        }
                        if (fallbackProvider != null) {
                            try {
                                val result = KlarvoApi.cleanupChunked(
                                    text = transcript,
                                    provider = fallbackProvider,
                                    style = config.cleanupStyle,
                                    dictionaryTerms = config.dictionaryTerms.takeIf { it.isNotBlank() },
                                    customInstructions = config.customPrompt.takeIf { it.isNotBlank() }
                                )
                                val tCleanup = System.currentTimeMillis()
                                llmLatencyMs = tCleanup - tStt
                                KlarvoLogger.i(TAG, "[pipeline] cleanup fallback succeeded (${fallbackProvider.model})")
                                // Finding [copy]: this fires after a cleanup FAILURE (not
                                // slowness) that triggered a provider switch -- reword to
                                // reflect what actually happened.
                                degradeStatusMsg = "⚠ Cleanup-Anbieter gewechselt"
                                result
                            } catch (fallbackEx: Exception) {
                                KlarvoLogger.w(TAG, "Cleanup fallback also failed -- using raw transcript", fallbackEx)
                                // Story 7-10: primary AND fallback failed -> a true
                                // cleanup failure. Raw text goes to the clipboard only.
                                llmCleanupFailed = true
                                degradeStatusMsg = CLEANUP_FAILED_CLIPBOARD_MSG
                                KlarvoApi.sanitizeLlmOutput(transcript)
                            }
                        } else {
                            KlarvoLogger.w(TAG, "No cleanup fallback provider available -- using raw transcript")
                            llmCleanupFailed = true
                            degradeStatusMsg = CLEANUP_FAILED_CLIPBOARD_MSG
                            KlarvoApi.sanitizeLlmOutput(transcript)
                        }
                    }
                } else {
                    KlarvoLogger.d(TAG, "[pipeline] cleanup: skipped (no LLM provider key)")
                    // Notify the user that cleanup was skipped so they understand
                    // why the pasted text may still contain filler words or errors.
                    handler.post {
                        showToast("Text pasted without cleanup (no LLM key configured).")
                    }
                    KlarvoApi.sanitizeLlmOutput(transcript)
                }
            }

            // Story 13-2 (B3 / D-H7, second half): the POST-cleanup ghost
            // strip, in Rust. Cleanup rationalises a recognisable ghost
            // ("Klinge") into a convincing full stockphrase
            // ("Kleinschreibung") -- detectable junk becomes fluent junk.
            // Desktop has always run strip_stockphrase_ghosts once more after
            // sanitize_llm_output; Android had no post-cleanup strip at all
            // (`sanitizeLlmOutput` only removes control characters). Same Rust
            // function over the bridge, never a Kotlin twin (ADR-0017).
            //
            // Guarded with `Throwable`, not `Exception`: this is a NEW native
            // symbol, and a stale `libklarvo_lib.so` raises
            // `UnsatisfiedLinkError` -- an `Error`, which the outer
            // `catch (e: IOException)` does not catch. Unguarded, the worker
            // thread would die here, AFTER the paid STT and LLM calls: nothing
            // pasted, nothing stored, and the bubble stranded in TRANSCRIBING.
            // Degrading to the un-stripped text loses a ghost, which is the
            // pre-13-2 Android behaviour; losing the dictation is not.
            // Review finding.
            val deliveredText = try {
                GroqSttBridge.nativeStripStockphraseGhosts(finalText)
            } catch (t: Throwable) {
                KlarvoLogger.e(
                    TAG,
                    "[pipeline] post-cleanup ghost strip unavailable (stale .so?) -- delivering un-stripped",
                    t
                )
                finalText
            }
            if (deliveredText != finalText) {
                KlarvoLogger.d(TAG, "[pipeline] post-cleanup ghost strip removed a stockphrase")
            }

            // Story 13-2 (D2 / D-H19): blank text must not be delivered or
            // stored. The named cleanup failures above already degrade to the
            // raw transcript, so this is the last line of defence -- an answer
            // that sanitises away to nothing, or a ghost strip that consumed
            // the whole text, must end the run silently (Desktop's
            // `PipelineEvent::idle()`) rather than paste an empty field and
            // write "" to history.db as the dictation.
            if (deliveredText.isBlank()) {
                KlarvoLogger.w(TAG, "[pipeline] nothing left to deliver -- no paste, no history row")
                handler.post {
                    autoLoopActive = false
                    hideListeningPanel()
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            KlarvoLogger.d(TAG, "[pipeline] total before paste: ${System.currentTimeMillis() - t0}ms")

            // Step 4: Copy to clipboard and paste
            // Capture activeGesture before posting to main thread (it may change on next gesture).
            val gesture = activeGesture
            // Capture pipeline timing values for metrics (captured in lambda closure).
            val capturedT0          = t0
            val capturedTConfig     = tConfig
            val capturedTStt        = tStt
            val capturedLlmLatency  = llmLatencyMs
            val capturedTranscript  = transcript
            val capturedFinalText   = deliveredText
            val capturedDegradeMsg  = degradeStatusMsg
            val capturedLlmFailed   = llmCleanupFailed
            val capturedConfig      = config
            handler.post {
                // DIV-04 fix: abort paste if a banking/security app is focused at paste time.
                // The pipeline may have started before the app-switch; this guard ensures
                // nothing reaches the clipboard or the accessibility paste path.
                if (BankingGuard.shouldBlockPaste(bankingAppActive)) {
                    showToast("Paste blocked — banking app active.")
                    autoLoopActive = false
                    hideListeningPanel()
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                    return@post
                }

                // Story 13-2 (B1-Android half / D-H3): the history row and the
                // Turso push happen HERE, after the guard verdict -- they used
                // to run as Steps 3 and 3b, before this block, so a dictation
                // into a banking app was blocked from the paste but had already
                // been written to history.db and pushed to the cloud. The
                // verdict is read on the main looper (BankingGuard's KDoc:
                // `bankingAppActive` is main-looper-owned), so the writes are
                // posted back to a worker thread from here rather than run on
                // it. Nothing depended on the old order: `saveToHistory`
                // returns Unit and `pushToTurso` re-reads the unsynced rows
                // from history.db itself. The Desktop half of D-H3 (a process
                // blocklist at all) is story 13-6 and is not touched here.
                Thread {
                    val tBeforeHistory = System.currentTimeMillis()
                    try {
                        KlarvoApi.saveToHistory(
                            context   = this@KlarvoOverlayService,
                            finalText = capturedFinalText,
                            rawText   = capturedTranscript,
                            style     = capturedConfig.cleanupStyle,
                            language  = capturedConfig.language,
                            deviceId  = capturedConfig.deviceId
                        )
                        KlarvoLogger.d(
                            TAG,
                            "[pipeline] history save: ${System.currentTimeMillis() - tBeforeHistory}ms"
                        )
                    } catch (e: Exception) {
                        KlarvoLogger.w(TAG, "History save failed (non-blocking)", e)
                    }
                    if (capturedConfig.tursoUrl.isNotBlank() && capturedConfig.tursoToken.isNotBlank()) {
                        try {
                            KlarvoApi.pushToTurso(
                                this@KlarvoOverlayService,
                                capturedConfig.tursoUrl,
                                capturedConfig.tursoToken
                            )
                        } catch (e: Exception) {
                            KlarvoLogger.w(TAG, "Turso sync failed (non-blocking)", e)
                        }
                    }
                }.start()

                // Story 13-2 (D6 / D-M12): guarded, and its outcome is part of
                // the delivery decision -- a clipboard that was never written
                // must not be pasted from and must not be reported as success.
                val clipboardOk = copyToClipboard(capturedFinalText)

                // Story 7-10 (AC2): on a cleanup failure the text stops at the
                // clipboard — no accessibility paste. Story 13-2 (D4 / D-H20):
                // the paste now REPORTS, and the toast/terminal decision is
                // taken afterwards from that outcome instead of from
                // `instance != null`. Both halves are pure companion functions
                // a JVM test drives (`CleanupFailureDeliveryTest`).
                // ONE read of the live service reference (review finding).
                // Reading it twice let the service disconnect between the
                // check and the paste, which yields `attempted = true` with a
                // null outcome -- a delivery that reports no success and logs
                // no cause, the one ending D4 exists to remove.
                val accessibility = KlarvoAccessibilityService.instance
                val accessibilityConnected = accessibility != null
                // Guarded for the same reason `guardedClipboardWrite` is, and
                // against the same promise ("never an uncaught main-thread
                // exception", D6): this runs on the looper, and every call
                // inside `pasteIntoFocusedField` is an `AccessibilityNodeInfo`
                // operation, which throws `IllegalStateException` on a node
                // that was sealed or recycled under it -- reachable when the
                // service is torn down mid-delivery. Unguarded, the app would
                // crash AFTER the paid STT and LLM calls and after the
                // clipboard write. A throw degrades to ACTION_REFUSED, the
                // outcome that already means "attempted, did not land": the
                // user gets the shipped "Copied: ..." toast and no checkmark,
                // which is exactly what D4 asks for. Review finding 2026-09-21.
                val pasteOutcome = if (shouldAttemptPaste(capturedLlmFailed, accessibilityConnected, clipboardOk)) {
                    try {
                        accessibility?.pasteIntoFocusedField()
                    } catch (t: Throwable) {
                        KlarvoLogger.e(TAG, "[delivery] accessibility paste threw -- treating as refused", t)
                        KlarvoAccessibilityService.PasteOutcome.ACTION_REFUSED
                    }
                } else {
                    null
                }
                val decision = decideDelivery(
                    capturedLlmFailed,
                    accessibilityConnected,
                    clipboardOk,
                    pasteOutcome
                )

                val preview = if (capturedFinalText.length > 50) capturedFinalText.take(50) + "..." else capturedFinalText
                // Successful paste is silent (the text simply appears) so it can't
                // override a same-cycle status/fallback toast (story 12-1). The
                // clipboard-fallback case IS surfaced: it's real info that the paste
                // did not land and the text is on the clipboard instead.
                if (decision.showCopiedToast) showToast("Copied: $preview")

                // Story 12-1 GATE-4 follow-up: show the deferred status/fallback toast now,
                // AFTER the paste, so it postdates (and thus wins over) HyperOS's own
                // "pasted from your clipboard" system toast. LENGTH_LONG so it dwells long
                // enough to actually be read.
                //
                // Story 7-10 (Q5): on the clipboard-only path this is the ONE
                // toast — "Copied: …" is suppressed above, so this LENGTH_LONG
                // degrade message is the newest and wins over HyperOS's own
                // "pasted from your clipboard" system toast.
                if (capturedDegradeMsg != null) {
                    Toast.makeText(this@KlarvoOverlayService, capturedDegradeMsg, Toast.LENGTH_LONG).show()
                }

                // Write feedback metrics (fire-and-forget, off main thread).
                Thread {
                    KlarvoApi.updateFeedbackMetrics(this@KlarvoOverlayService) { m ->
                        m.copy(
                            lastSttLatencyMs   = capturedTStt - capturedTConfig,
                            lastLlmLatencyMs   = capturedLlmLatency,
                            lastTotalLatencyMs = System.currentTimeMillis() - capturedT0,
                            lastTargetApp      = null,   // Android has no foreground-window tracking
                            lastDictationAt    = java.text.SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", java.util.Locale.US).also { it.timeZone = java.util.TimeZone.getTimeZone("UTC") }.format(java.util.Date()),
                            lastRawText        = capturedTranscript,
                            lastCleanedText    = capturedFinalText
                            // Error counts left unchanged -- copy() retains them.
                        )
                    }
                }.start()

                // Terminal state. "Only the success path gets the DONE state;
                // error paths go straight to IDLE" is what this block's comment
                // has always claimed -- story 13-2 (D4/D5/D6) makes it true.
                // Until now DONE flashed on EVERY non-blocked exit: after a
                // cleanup failure (D-M24), after a paste that landed nowhere
                // (D-H20), and -- had the clipboard not crashed the process
                // first -- after a failed clipboard write (D-M12).
                //
                // No new RecordingState: a run that did not deliver ends in the
                // shipped IDLE, with the existing degrade toast carrying the
                // cause. The choice itself is the pure [terminalStateFor].
                val terminal = terminalStateFor(decision)
                // Cancel any prior pending flash before scheduling a new one (defensive).
                handler.removeCallbacks(doneFlashRunnable)
                if (terminal == RecordingState.DONE) {
                    val prevForDone = currentState
                    setState(RecordingState.DONE)
                    adjustLayoutForState(RecordingState.DONE, prevForDone)
                    handler.postDelayed(doneFlashRunnable, 800L)
                } else {
                    // The same teardown `doneFlashRunnable` performs 800 ms
                    // later on the success path, minus the checkmark.
                    hideListeningPanel()
                    val prevForIdle = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prevForIdle)
                }

                // AUTO mode: restart recording for next segment
                val activeMode = when (gesture) {
                    "longpress" -> longPressMode
                    else        -> tapMode
                }
                if (autoLoopActive && activeMode == RecordingMode.AUTO) {
                    startRecording()
                }
            }

        } catch (e: IOException) {
            KlarvoLogger.w(TAG, "STT/API pipeline failed", e)
            // Increment STT error counter (this catch covers STT failures;
            // LLM IOException is caught earlier and increments llmErrorCount there).
            KlarvoApi.updateFeedbackMetrics(this) { m ->
                m.copy(sttErrorCount = m.sttErrorCount + 1)
            }
            // AC3/AC5: if a pending WAV still exists (Groq retries exhausted, no local
            // model available), the audio was preserved -- surface the taxonomy terminal
            // message so the user knows it wasn't silently lost, rather than a raw
            // exception string.
            val audioPreserved = pendingWavFile?.exists() == true
            val toastMsg = if (audioPreserved) {
                "✗ Transkription fehlgeschlagen — Audio gesichert"
            } else {
                "Error: ${e.message?.take(80)}"
            }
            // Story 12-2 (AC3): mirror the Windows pipeline — a preserved WAV must be
            // discoverable in the dictation history as a `pending` entry, not just a toast.
            if (audioPreserved) {
                KlarvoApi.savePendingHistoryEntry(
                    context   = this,
                    audioPath = pendingWavFile!!.absolutePath,
                    language  = config.language,
                    deviceId  = config.deviceId
                )
            }
            handler.post {
                showToast(toastMsg)
                autoLoopActive = false
                hideListeningPanel()
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
        }
    }

    // --- Listening panel helpers (Story 9.5) ---

    /**
     * Creates and attaches the listening panel overlay window.
     * Must be called on the main thread. No-op if panel is already visible.
     */
    private fun showListeningPanel(initialState: ListeningPanelView.State) {
        if (panelVisible) {
            // Update state if already visible (e.g. RECORDING→TRANSCRIBING without hide/show)
            panelView?.panelState = initialState
            panelView?.invalidate()
            return
        }
        val panel = ListeningPanelView(this)
        panel.panelState = initialState
        // Story 11-2 (AC-9): apply the ported appearance config fresh at show-time (mirrors the
        // desktop 6.6 "separate-window reactive read" lesson -- never cache stale values).
        // Code-review fix F2 (2026-07-01): only restyle when Live Preview is actually enabled.
        // `applyAppearance` isn't just cosmetics-when-off -- it also changes the panel's stock
        // look (translucent bg, teal border, 13sp->11sp, monospace->sans) that Auto/AutoStop and
        // the disabled-by-default (AC-4) case must keep byte-identical to pre-11-2 behavior.
        (cachedConfig ?: KlarvoApi.readConfig(this))?.let { config ->
            if (shouldApplyPreviewAppearance(config.livePreviewEnabled)) panel.applyAppearance(config)
        }
        // Story 11-3 (AC-3a, item 3, Task 5.1): the panel window's height is FIXED
        // (PANEL_FIXED_HEIGHT_DP), not WRAP_CONTENT. This is the direct fix for the original
        // "growing panel fills the whole screen" usability blocker (Dev Notes "Why this got
        // upgraded"): a WRAP_CONTENT window re-measures to whatever the accumulated transcript
        // needs, which is unbounded. AC-3 pivot (2026-07-08): the transcript content itself is
        // now a bounded ScrollView (ListeningPanelView.transcriptScrollView) that scrolls inside
        // this fixed window instead of evicting older lines -- the fixed WINDOW height is what
        // guarantees the window itself never grows/shrinks (a WRAP_CONTENT window would still
        // (re-)measure on every text update even with scrollable content inside it).
        val dp = resources.displayMetrics.density
        val panelHeightPx = (PANEL_FIXED_HEIGHT_DP * dp).toInt()
        val params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            panelHeightPx,
            overlayType,
            // CAUTION: do NOT add FLAG_NOT_TOUCHABLE here. HyperOS/MIUI force-dims any
            // TYPE_APPLICATION_OVERLAY window that carries FLAG_NOT_TOUCHABLE to a window alpha
            // of 0.8 (verified via `dumpsys window windows`: NOT_TOUCHABLE → alpha=0.8, removing
            // it → alpha=1.0), which made the home screen bleed through the panel — the real
            // cause of the long-standing "transparent panel" defect. PixelFormat and the view's
            // opaque background were red herrings; the dim is applied at the window-composite
            // level and overrides both params.alpha and view.alpha. The cluster window escapes it
            // precisely because it is touchable. We match the cluster's flags exactly. Cost: the
            // panel now consumes touches in its bottom strip instead of passing them through —
            // acceptable because the panel is a passive display that overlaps no other surface
            // (the ➤/✗ cluster is a separate window at the bubble position), and it stays
            // NOT_FOCUSABLE so it never steals input focus.
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            android.graphics.PixelFormat.TRANSLUCENT
        ).apply {
            gravity = android.view.Gravity.BOTTOM
        }
        // Modell B (ADR-0019 §4′): panel is a PASSIVE display. Cancel/Send/Waveform all live in
        // the cluster (a separate bubble-position window). The panel has no touch listener, so
        // touches that land on it are simply absorbed (no-op).
        try {
            windowManager.addView(panel, params)
            panelView    = panel
            panelParams  = params
            panelVisible = true
            KlarvoLogger.d(TAG, "[panel] shown (state=$initialState)")
            // Story 11-4 (AC-1, Design Decision 2): both the bubble and the panel are
            // TYPE_APPLICATION_OVERLAY windows owned by this same Service -- Android z-orders
            // same-type overlay windows by add-order, with no priority field (11-3's
            // investigation). The panel was just added, so without this it would now be the
            // most-recently-added window and render/receive-touches ABOVE the bubble. Re-adding
            // the bubble here makes IT the most-recently-added window again, restoring the
            // required "bubble always on top" invariant on every actual panel show (including
            // every RECORDING->TRANSCRIBING->RECORDING hide/re-show cycle, since each cycle goes
            // through this addView call again -- the early-return branch above, for a panel that
            // is already visible, doesn't change window order and needs no reorder).
            if (activePointerId != MotionEvent.INVALID_POINTER_ID) {
                // A touch gesture is in flight on the bubble window right now (e.g. push-to-talk's
                // longPressRunnable calls startRecording() -> showListeningPanel() while the
                // finger is still down). windowManager.removeView() would tear down the window's
                // input channel and cancel that gesture (OS-level ACTION_CANCEL) -- defer the
                // reorder until the gesture actually ends (handleTouch's ACTION_UP/CANCEL).
                bubbleReorderPending = true
            } else {
                reorderBubbleAbovePanel()
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "[panel] addView failed", e)
        }
    }

    /**
     * Re-adds the bubble window to the WindowManager so it becomes the most-recently-added
     * TYPE_APPLICATION_OVERLAY window and therefore renders/receives-touches above the panel
     * (Story 11-4, AC-1). No-op if the bubble isn't currently shown -- showBubble() itself always
     * adds fresh, so a bubble shown after the panel is already correctly on top with no extra
     * step needed.
     */
    private fun reorderBubbleAbovePanel() {
        if (!isBubbleVisible || !::bubbleView.isInitialized) return
        try {
            windowManager.removeView(bubbleView)
            windowManager.addView(bubbleView, bubbleParams)
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "[panel] failed to re-add bubble above panel", e)
            // F1: removeView may have succeeded before addView threw, leaving the bubble window
            // detached from the WindowManager while isBubbleVisible still claims it's shown.
            // Reflect reality so a later showBubble() can cleanly re-add it instead of acting on
            // a stale "visible" flag for a window that's no longer attached.
            isBubbleVisible = false
        }
    }

    /**
     * Removes the listening panel overlay window with a 320ms slide-down collapse animation.
     * If called mid-collapse (e.g. a new show is requested), the in-flight animation is cancelled
     * and the window is removed immediately so state stays consistent (F9).
     *
     * Must be called on the main thread. No-op if panel is not visible.
     */
    private fun hideListeningPanel() {
        if (!panelVisible) return
        val panel = panelView ?: return
        // Mark not visible immediately so re-entrant calls (e.g. from onDestroy) are no-ops
        panelVisible = false
        panelView    = null
        panelParams  = null
        panel.hideWithAnimation {
            try {
                windowManager.removeView(panel)
                KlarvoLogger.d(TAG, "[panel] hidden")
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "[panel] removeView failed (already removed?)", e)
            }
        }
    }

    // --- Helpers ---

    private fun setState(newState: RecordingState) {
        currentState   = newState
        bubbleView.state = when (newState) {
            RecordingState.IDLE        -> FloatingBubbleView.State.IDLE
            RecordingState.RECORDING   -> FloatingBubbleView.State.RECORDING
            RecordingState.TRANSCRIBING -> FloatingBubbleView.State.TRANSCRIBING
            RecordingState.DONE        -> FloatingBubbleView.State.DONE
        }
        bubbleView.alpha = 1.0f  // all states fully opaque (idle matches canon — see setupBubble)
        // Story 9.5 fix: while the listening panel owns RECORDING/TRANSCRIBING, suppress the
        // bubble's own state visual to the idle squircle so the two overlays don't both render the
        // same state (real-device double-window defect). The bubble window stays alive for touch
        // (PTT release / taps). Driven here on EVERY transition so DONE/IDLE restore the normal
        // visual (DONE checkmark, then idle).
        bubbleView.suppressedForPanel =
            newState == RecordingState.RECORDING || newState == RecordingState.TRANSCRIBING
        if (newState == RecordingState.IDLE) {
            bubbleView.amplitude = 0f
            // Re-read config so bubble size/opacity changes from Settings take effect
            // without requiring a full app restart.
            reloadBubbleAppearance()
            // Regression fix (9-14/9-15 push-to-talk rework): the keyboard is dismissed mid-RECORDING
            // (line ~1484, non-AUTO modes), so applyKeyboardState(false) fires while state != IDLE and
            // its hide gate (line 758) is correctly skipped. Nothing re-checked keyboard visibility once
            // the state later returned to IDLE, stranding the idle bubble on screen with no keyboard.
            // Re-apply the same gate here on every return to IDLE (covers the DONE→IDLE flash and cancel).
            if (!alwaysVisible && !keyboardVisible) {
                hideBubble()
            }
        }
    }

    /**
     * Re-reads bubble size, opacity, and recording controls from config.json.
     * Called on every return to IDLE so Settings changes take effect after the next dictation.
     *
     * Bubble visual size uses the responsive formula (computeVisualSizeDp) as of Story 9.3;
     * config.bubbleSize scale factor is no longer applied here.
     */
    private fun reloadBubbleAppearance() {
        val config = KlarvoApi.readConfig(this) ?: return
        val newSizeDp = computeVisualSizeDp(config)
        bubbleView.setBubbleSize(newSizeDp)
        // AC8 (Story 9-15 Re-Scope): apply user-configured recording button size.
        bubbleView.recordingButtonSizeDp = config.recordingButtonSizeDp
        bubbleView.alpha = 1.0f  // idle fully opaque (canon); legacy bubbleOpacity no longer dims it

        // Keep window (visual + shadow padding) in sync with the (possibly changed) visual size
        val touchTargetPx = bubbleWindowPx(newSizeDp)
        bubbleParams.width  = touchTargetPx
        bubbleParams.height = touchTargetPx

        updateBubbleLayout()
        loadBubbleControls()
        updateNotification()
    }

    /**
     * Writes [text] to the system clipboard and says whether it landed.
     *
     * **Story 13-2 (D6 / D-M12).** This was unguarded inside `handler.post`,
     * so a throwing `setPrimaryClip` (an OEM clipboard service refusing or
     * dying — HyperOS has its own clipboard policy layer) was an uncaught
     * main-thread exception: the app crashed mid-delivery. `pasteErrorCount`
     * existed in [KlarvoApi.FeedbackMetrics], was serialised, and was never
     * incremented by anything.
     *
     * A failure here is terminal for the delivery: nothing is on the
     * clipboard, so nothing may be pasted and no success may be shown
     * ([decideDelivery] takes `clipboardOk`). Reuses the shipped IDLE ending —
     * no new toast text, and Android gets no equivalent of Desktop's
     * "TEXT LOST" card, which would be a new surface.
     */
    private fun copyToClipboard(text: String): Boolean = guardedClipboardWrite(
        write = {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = ClipData.newPlainText("Klarvo transcription", text)
            clipboard.setPrimaryClip(clip)
        },
        onFailure = { e ->
            KlarvoLogger.e(TAG, "[delivery] clipboard write failed -- text not delivered", e)
            Thread {
                KlarvoApi.updateFeedbackMetrics(this@KlarvoOverlayService) { m ->
                    m.copy(pasteErrorCount = m.pasteErrorCount + 1)
                }
            }.start()
        },
    )

    /**
     * The Whisper conditioning hint for [config]'s active language — the value
     * that crosses the JNI as `customPrompt` (story 13-2, B4 / D-H4).
     *
     * `""` means "let the Rust core use its built-in language hint". The
     * selection itself lives in [KlarvoApi.selectSttHintOverride], the twin of
     * `stt::select_stt_hint_override`, so the two platforms cannot disagree
     * about which of the three `advanced.sttPrompt*` values applies.
     *
     * This is NOT `config.customPrompt`: that is the LLM cleanup instruction
     * and it goes to the LLM only.
     */
    private fun sttHintFor(config: KlarvoApi.Config): String =
        KlarvoApi.selectSttHintOverride(
            config.language,
            config.sttPromptDe,
            config.sttPromptEn,
            config.sttPromptAuto
        )

    private fun showToast(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
    }

    // ---- Data-loss prevention helpers ----

    /**
     * Resolves the local Whisper model file path.
     *
     * Checks `filesDir/models/` first (matches Tauri's `app_data_dir()` download
     * location), falling back to `dataDir/models/` for older builds. Shared by
     * the explicit `sttProvider == "local"` path and the automatic
     * Groq-failure safety net (AC3, story 12-1) so both consumers use the
     * same presence check instead of two independently-drifting copies.
     */
    private fun resolveLocalWhisperModelFile(): File {
        val filesDirModels = File(filesDir, "models")
        val dataDirModels = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.N) {
            File(dataDir, "models")
        } else {
            File(applicationInfo.dataDir, "models")
        }
        val modelDir = when {
            filesDirModels.exists() -> filesDirModels
            dataDirModels.exists() -> dataDirModels
            else -> filesDirModels // default to filesDir (matches Tauri download path)
        }
        return modelDir.resolve("ggml-small.bin") // TODO: read model name from config
    }

    /**
     * Writes the WAV bytes to {dataDir}/pending/<timestamp>.wav before any network call.
     * Returns the File, or null if the write fails (non-fatal -- pipeline continues).
     */
    private fun savePendingWav(wavBytes: ByteArray): File? {
        return try {
            val pendingDir = File(dataDir, "pending")
            pendingDir.mkdirs()
            val f = File(pendingDir, "${System.currentTimeMillis()}.wav")
            f.writeBytes(wavBytes)
            KlarvoLogger.d(TAG, "[pending-wav] saved ${wavBytes.size / 1024}KB to ${f.name}")
            f
        } catch (e: IOException) {
            KlarvoLogger.w(TAG, "[pending-wav] failed to save backup WAV", e)
            null
        }
    }

    /**
     * Finding D (story 12-1 code review): classifies an [IOException] thrown by
     * [transcribeWithRetry] as retryable or not, mirroring Rust's
     * `is_retryable_stt_error`. `transcribeWithRetry` already does its own
     * classification internally (4xx = non-retryable, 5xx/network = retried,
     * empty audio = non-retryable) and encodes the outcome in the exception
     * message prefix, so this reads that prefix rather than re-parsing status
     * codes: only "failed after retries" means the retryable path was actually
     * exhausted -- everything else (empty audio, 4xx) is a permanent failure
     * that a local-Whisper attempt cannot fix.
     */
    private fun isRetryableSttFailure(e: IOException): Boolean {
        return e.message?.startsWith("Groq STT failed after retries:") == true
    }

    /**
     * Transcribes [wavBytes] via the shared Rust Groq STT path
     * (`GroqSttBridge.nativeTranscribe`).
     *
     * **Story 13-2 (D9 / D-M5, D-M6) changed two things.**
     *
     * 1. **The retry budget is 1** — [retryDelaysMs] is empty, so the loop runs
     *    exactly once and no backoff is slept. Desktop has always made a single
     *    STT attempt (30 s timeout) and then reaches for the local net or the
     *    pending WAV; Android made three, with 2 s + 5 s in between, arriving at
     *    the *same end state* up to ~70 s later (audit row D-M6, verdict
     *    "android an desktop anpassen"). The loop shape is kept rather than
     *    unrolled so restoring a budget is a one-line change and the "failed
     *    after retries" message — which [isRetryableSttFailure] keys on to gate
     *    the local-Whisper net — still fires for a retryable exhaustion.
     * 2. **The sentinel→verdict decision moved to the companion**
     *    ([classifySttSentinel]), so the whole ladder including the new
     *    non-retryable `__ERROR_FORMAT:` is JVM-testable. Only the backoff loop
     *    itself stays on-device.
     *
     * [sttHint] is the Whisper CONDITIONING hint (B4) — see [sttHintFor].
     *
     * ADR-0017: `KlarvoApi.transcribe` + `buildMultipartBody` are deleted; the
     * request, its guards and the error mapping are the shared Rust core.
     */
    private fun transcribeWithRetry(
        wavBytes: ByteArray,
        apiKey: String,
        language: String,
        sttModel: String,
        dictionaryTerms: String,
        sttHint: String,
        pendingWavFile: File?,
        testProviderStt: String
    ): String {
        val wavBase64 = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
        val retryDelaysMs = emptyList<Long>()
        var lastErrorMsg: String? = null

        for (attempt in 0..retryDelaysMs.size) {
            val result = GroqSttBridge.nativeTranscribe(
                wavBase64 = wavBase64,
                apiKey = apiKey,
                language = language,
                dictionaryTerms = dictionaryTerms,
                // Story 13-2 (B4): the conditioning hint, NOT the LLM cleanup
                // instruction. The Rust core rebuilds the guard hint from this
                // plus `language`, which is why the JNI arity did not change.
                customPrompt = sttHint,
                sttModel = sttModel,
                temperature = 0.0f,
                // Story 13-1b: ONE value now, carried through UNINSPECTED. The
                // Rust core decides whether a non-"off" value means anything --
                // ADR-0017 keeps every STT request and guard decision out of
                // Kotlin, and a canned transcript forged here would produce the
                // __ERROR_* sentinels instead of the real code emitting them.
                testProviderStt = testProviderStt
            )

            val sentinel = classifySttSentinel(result)
            when (sentinel.verdict) {
                SttVerdict.SUCCESS -> return result

                SttVerdict.NON_RETRYABLE -> {
                    KlarvoLogger.w(TAG, "[stt-retry] non-retryable: ${sentinel.message}")
                    throw IOException(sentinel.message)
                }

                SttVerdict.RETRYABLE -> {
                    lastErrorMsg = sentinel.message
                    if (attempt < retryDelaysMs.size) {
                        val delay = retryDelaysMs[attempt]
                        KlarvoLogger.w(
                            TAG,
                            "[stt-retry] attempt $attempt failed (${sentinel.message}), retrying in ${delay}ms"
                        )
                        Thread.sleep(delay)
                    } else {
                        KlarvoLogger.e(
                            TAG,
                            "[stt-retry] retry budget exhausted, pending WAV kept: ${pendingWavFile?.name}"
                        )
                    }
                }
            }
        }
        throw IOException("Groq STT failed after retries: $lastErrorMsg")
    }

    // shouldBlockPaste delegates to BankingGuard (see BankingGuard.kt) so that
    // unit tests can reach the real decision site without an Android context (AI-2).

    /**
     * Deletes pending WAV files older than 7 days.
     * Called once at service startup to keep the pending directory clean.
     */
    private fun cleanupStalePendingWavFiles() {
        try {
            val pendingDir = File(dataDir, "pending")
            if (!pendingDir.exists()) return
            val cutoff = System.currentTimeMillis() - 7L * 24 * 60 * 60 * 1000
            var deleted = 0
            pendingDir.listFiles()?.forEach { f ->
                if (f.isFile && f.lastModified() < cutoff) {
                    f.delete()
                    deleted++
                }
            }
            if (deleted > 0) {
                KlarvoLogger.i(TAG, "[pending-wav] cleaned up $deleted stale WAV file(s)")
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "[pending-wav] cleanup failed", e)
        }
    }
}
