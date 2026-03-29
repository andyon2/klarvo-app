package com.klarvo.voice

import android.util.Log

/**
 * Local LLM inference via MNN (offline text cleanup).
 * Uses Qwen2.5-1.5B-Instruct model in MNN format.
 *
 * The model is loaded lazily on first use and kept in memory until release() is called.
 * All public methods are synchronized -- not safe for concurrent callers, but safe
 * for the single-threaded recording flow.
 *
 * Model path: context.filesDir/models/qwen2.5-1.5b-mnn/config.json
 * (populated by the user downloading the model bundle).
 *
 * Native library: libklarvo_mnn.so (JNI wrapper around MNN Llm).
 */
object LocalLlmInference {
    private const val TAG = "LocalLlmInference"

    private var loaded = false

    // Native library availability -- set to false permanently if System.loadLibrary fails.
    private var nativeAvailable = false

    init {
        try {
            System.loadLibrary("klarvo_mnn")
            nativeAvailable = true
            Log.i(TAG, "Native library libklarvo_mnn loaded")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load native library libklarvo_mnn: ${e.message}")
            // nativeAvailable stays false -- every public method will return a safe default.
        }
    }

    // JNI entry points -- names must match Java_com_klarvo_voice_LocalLlmInference_* in the C++ file.
    private external fun loadModel(configPath: String): Boolean
    private external fun generate(prompt: String): String
    private external fun releaseModel()
    private external fun isLoaded(): Boolean

    /**
     * Loads the model from the given config.json path.
     * No-op if the model is already loaded.
     *
     * @param configPath Absolute path to the model's config.json file.
     * @return true on success, false if native library is unavailable or loading failed.
     */
    @Synchronized
    fun load(configPath: String): Boolean {
        if (!nativeAvailable) {
            Log.w(TAG, "load: native library not available")
            return false
        }
        if (loaded) {
            Log.d(TAG, "load: model already loaded, skipping")
            return true
        }
        loaded = loadModel(configPath)
        if (loaded) {
            Log.i(TAG, "Model loaded from: $configPath")
        } else {
            Log.e(TAG, "Failed to load model from: $configPath")
        }
        return loaded
    }

    /**
     * Runs text cleanup using the local model.
     * The prompt must already be formatted as a ChatML string:
     *
     *   <|im_start|>system\n{system_prompt}<|im_end|>\n
     *   <|im_start|>user\n{raw_text}<|im_end|>\n
     *   <|im_start|>assistant\n
     *
     * Returns an empty string if the model is not loaded or native is unavailable.
     *
     * @param prompt ChatML-formatted prompt string.
     * @return Generated text (assistant reply only, no surrounding tokens).
     */
    @Synchronized
    fun cleanup(prompt: String): String {
        if (!nativeAvailable) {
            Log.w(TAG, "cleanup: native library not available")
            return ""
        }
        if (!loaded) {
            Log.w(TAG, "cleanup: model not loaded")
            return ""
        }
        return generate(prompt)
    }

    /**
     * Releases the model and frees native memory.
     * After this call, load() must be called again before cleanup() works.
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
}
