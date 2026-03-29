package com.klarvo.voice

import android.util.Log

/**
 * Local Whisper STT inference via whisper-rs JNI bridge (offline transcription).
 * Uses a GGML model file (e.g. ggml-small.bin) loaded from the app's model directory.
 *
 * The model is loaded lazily on first use and kept in memory until release() is called.
 * All public methods are synchronized -- not safe for concurrent callers, but safe
 * for the single-threaded recording flow.
 *
 * Model path: context.dataDir/models/ggml-small.bin
 * (populated by the user downloading the model in Settings).
 *
 * Native library: libklarvo_lib.so (Tauri Rust library with whisper-rs JNI exports).
 *
 * JNI function names on the Rust side must be:
 *   Java_com_klarvo_voice_LocalWhisperInference_loadModel
 *   Java_com_klarvo_voice_LocalWhisperInference_nativeTranscribe
 *   Java_com_klarvo_voice_LocalWhisperInference_releaseModel
 *   Java_com_klarvo_voice_LocalWhisperInference_isLoaded
 */
object LocalWhisperInference {
    private const val TAG = "LocalWhisperInference"

    private var loaded = false

    // Native library availability -- set to false permanently if System.loadLibrary fails.
    private var nativeAvailable = false

    init {
        try {
            // The Tauri Rust library already contains whisper-rs + JNI exports.
            // If Tauri already loaded it, this is a no-op (Java spec).
            System.loadLibrary("klarvo_lib")
            nativeAvailable = true
            Log.i(TAG, "Native library libklarvo_lib loaded for whisper")
        } catch (e: Throwable) {
            // Catch Throwable (not just UnsatisfiedLinkError) to handle:
            // - UnsatisfiedLinkError (library not found)
            // - ExceptionInInitializerError (wrapped init failures)
            // - SecurityException (classloader issues)
            Log.e(TAG, "Failed to load native library libklarvo_lib: ${e.javaClass.simpleName}: ${e.message}")
            // nativeAvailable stays false -- every public method will return a safe default.
        }
    }

    // JNI entry points -- names must match Java_com_klarvo_voice_LocalWhisperInference_* in Rust.
    // Note: nativeTranscribe avoids a name collision with the public transcribeAudio method.
    private external fun loadModel(modelPath: String): Boolean
    private external fun nativeTranscribe(wavBase64: String, language: String): String
    private external fun releaseModel()
    private external fun isLoaded(): Boolean

    /**
     * Loads the Whisper GGML model from the given file path.
     * No-op if the model is already loaded.
     *
     * @param modelPath Absolute path to the GGML model file (e.g. ggml-small.bin).
     * @return true on success, false if native library is unavailable or loading failed.
     */
    @Synchronized
    fun load(modelPath: String): Boolean {
        if (!nativeAvailable) {
            Log.w(TAG, "load: native library not available")
            return false
        }
        if (loaded) {
            Log.d(TAG, "load: model already loaded, skipping")
            return true
        }
        loaded = loadModel(modelPath)
        if (loaded) {
            Log.i(TAG, "Model loaded from: $modelPath")
        } else {
            Log.e(TAG, "Failed to load model from: $modelPath")
        }
        return loaded
    }

    /**
     * Transcribes Base64-encoded WAV audio using the local Whisper model.
     *
     * @param wavBase64 Base64-encoded WAV bytes (16kHz mono, Base64.NO_WRAP encoding).
     * @param language  ISO-639-1 language code ("de", "en") or empty string for auto-detect.
     * @return Transcribed text, or empty string if the model is not loaded or on failure.
     */
    @Synchronized
    fun transcribeAudio(wavBase64: String, language: String): String {
        if (!nativeAvailable) {
            Log.w(TAG, "transcribeAudio: native library not available")
            return ""
        }
        if (!loaded) {
            Log.w(TAG, "transcribeAudio: model not loaded")
            return ""
        }
        return nativeTranscribe(wavBase64, language)
    }

    /**
     * Releases the model and frees native memory.
     * After this call, load() must be called again before transcribeAudio() works.
     */
    @Synchronized
    fun release() {
        if (!nativeAvailable) return
        releaseModel()
        loaded = false
        Log.i(TAG, "Model released")
    }

    /**
     * Returns true if the model is currently loaded and ready for inference.
     */
    @Synchronized
    fun isModelLoaded(): Boolean = nativeAvailable && loaded && isLoaded()

    /**
     * Returns true if the native library was loaded successfully.
     * Useful for diagnostics.
     */
    fun isNativeAvailable(): Boolean = nativeAvailable
}
