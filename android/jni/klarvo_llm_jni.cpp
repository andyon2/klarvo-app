#include <jni.h>
#include <string>
#include <sstream>
#include <android/log.h>
#include "llm/llm.hpp"

#define LOG_TAG "KlarvoMNN"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

using namespace MNN::Transformer;

// Single global LLM instance. Not thread-safe -- callers must serialize.
static Llm* g_llm = nullptr;

extern "C" {

JNIEXPORT jboolean JNICALL
Java_com_klarvo_voice_LocalLlmInference_loadModel(JNIEnv* env, jobject /*thiz*/, jstring configPath) {
    // Release existing instance before loading a new one.
    if (g_llm) {
        LOGI("Releasing previous LLM instance before reload");
        delete g_llm;
        g_llm = nullptr;
    }

    const char* path = env->GetStringUTFChars(configPath, nullptr);
    if (!path) {
        LOGE("loadModel: null config path");
        return JNI_FALSE;
    }

    LOGI("Loading model from: %s", path);
    g_llm = Llm::createLLM(std::string(path));
    env->ReleaseStringUTFChars(configPath, path);

    if (!g_llm) {
        LOGE("createLLM returned null");
        return JNI_FALSE;
    }

    bool ok = g_llm->load();
    if (!ok) {
        LOGE("Llm::load() failed");
        delete g_llm;
        g_llm = nullptr;
        return JNI_FALSE;
    }

    LOGI("Model loaded successfully");
    return JNI_TRUE;
}

JNIEXPORT jstring JNICALL
Java_com_klarvo_voice_LocalLlmInference_generate(JNIEnv* env, jobject /*thiz*/, jstring prompt) {
    if (!g_llm) {
        LOGE("generate called but no model is loaded");
        return env->NewStringUTF("");
    }

    const char* p = env->GetStringUTFChars(prompt, nullptr);
    if (!p) {
        LOGE("generate: null prompt");
        return env->NewStringUTF("");
    }

    std::string promptStr(p);
    env->ReleaseStringUTFChars(prompt, p);

    // Reset KV cache between independent cleanup requests.
    g_llm->reset();

    std::ostringstream oss;
    // response(string, ostream*) applies the model's built-in chat template.
    // For Qwen2.5 MNN models the template wraps the input with <|im_start|> tokens.
    // We pass the raw user text so the model formats it correctly.
    g_llm->response(promptStr, &oss);

    std::string result = oss.str();
    LOGI("generate: input=%zu chars, output=%zu chars", promptStr.size(), result.size());

    return env->NewStringUTF(result.c_str());
}

JNIEXPORT void JNICALL
Java_com_klarvo_voice_LocalLlmInference_releaseModel(JNIEnv* /*env*/, jobject /*thiz*/) {
    if (g_llm) {
        LOGI("Releasing LLM model");
        delete g_llm;
        g_llm = nullptr;
    }
}

JNIEXPORT jboolean JNICALL
Java_com_klarvo_voice_LocalLlmInference_isLoaded(JNIEnv* /*env*/, jobject /*thiz*/) {
    return (g_llm != nullptr) ? JNI_TRUE : JNI_FALSE;
}

} // extern "C"
