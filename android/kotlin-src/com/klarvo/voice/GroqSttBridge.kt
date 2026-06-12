package com.klarvo.voice

/**
 * JNI bridge to the shared Rust Groq STT request + guard path (ADR-0017).
 *
 * This object provides the single consolidated STT path for Android.
 * The Kotlin twins (KlarvoApi.transcribe, buildMultipartBody, HallucinationFilter,
 * SilencePreFilter) are deleted and replaced by calls to this bridge.
 *
 * ## Rust implementation
 * All logic lives in `src-tauri/src/stt/groq_jni.rs`. This object only declares
 * the native method signatures; the implementation is in the shared Rust core.
 *
 * ## Error codes in nativeTranscribe return value
 * The Rust side embeds machine-readable error codes for distinguishable failure modes:
 * - `"__ERROR_EMPTY_AUDIO__"` — WAV decoded to zero bytes.
 * - `"__ERROR_API:HTTP <status>: <message>__"` — Groq API non-2xx. NOT retried by caller.
 * - `"__ERROR_NETWORK:<message>__"` — network failure. Caller may retry.
 * Empty string on unexpected failure.
 *
 * ## Retry semantics (preserved from transcribeWithRetry)
 * The Kotlin retry wrapper (transcribeWithRetry in KlarvoOverlayService) continues to
 * own the retry loop. It distinguishes retriable vs. non-retriable errors by inspecting
 * these error code prefixes, preserving the original 4xx-no-retry contract.
 *
 * ## nativeSilenceCheck return values
 * - `"Pass"` — recording is long enough and loud enough.
 * - `"TooShort:<durationMs>"` — shorter than minRecordingMs.
 * - `"Silent:<rms>"` — RMS below silenceThreshold.
 */
object GroqSttBridge {

    init {
        // The shared Rust library is loaded by the Tauri runtime at startup.
        // Android loads it before any JNI call via System.loadLibrary in the Application class.
    }

    /**
     * Transcribes a Base64-encoded WAV using the shared Rust Groq STT path.
     *
     * @param wavBase64        RFC-4648 Base64-encoded 16 kHz mono WAV.
     * @param apiKey           Groq API Bearer token.
     * @param language         ISO-639-1 code ("de", "en") or empty for auto-detect.
     * @param dictionaryTerms  Comma-separated user dictionary (or empty).
     * @param customPrompt     User custom STT hint (or empty).
     * @param sttModel         Groq model name (e.g. "whisper-large-v3-turbo").
     * @param temperature      Whisper sampling temperature (0.0 = deterministic).
     * @return Transcribed text, or an error code string (see class-level doc).
     */
    @JvmStatic
    external fun nativeTranscribe(
        wavBase64: String,
        apiKey: String,
        language: String,
        dictionaryTerms: String,
        customPrompt: String,
        sttModel: String,
        temperature: Float
    ): String

    /**
     * Returns true if [text] is a Whisper hallucination artifact.
     * Replaces HallucinationFilter.isHallucination() — same logic, shared Rust source.
     */
    @JvmStatic
    external fun nativeIsHallucination(text: String): Boolean

    /**
     * Returns true if [transcription] echoes the STT conditioning [sttHint].
     * Replaces the implicit echo check in the Kotlin pipeline path (H6).
     */
    @JvmStatic
    external fun nativeIsPromptEcho(transcription: String, sttHint: String): Boolean

    /**
     * Strips conditioning-prompt fragments and stockphrase ghosts from [text].
     * Replaces the Kotlin prompt-strip logic (H7 / AC7).
     */
    @JvmStatic
    external fun nativeStripPromptFragments(text: String, sttHint: String): String

    /**
     * Pre-STT silence and duration filter.
     *
     * @param wavBase64        RFC-4648 Base64-encoded WAV.
     * @param minRecordingMs   Minimum recording duration in milliseconds.
     * @param silenceThreshold RMS threshold below which the recording is considered silent.
     * @return "Pass", "TooShort:<durationMs>", or "Silent:<rms>".
     */
    @JvmStatic
    external fun nativeSilenceCheck(
        wavBase64: String,
        minRecordingMs: Long,
        silenceThreshold: Float
    ): String
}
