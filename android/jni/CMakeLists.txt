cmake_minimum_required(VERSION 3.22.1)
project("klarvo_mnn")

# MNN source tree root (contains /include and /transformers).
set(MNN_ROOT "/tmp/mnn")

# Headers: MNN core + LLM engine.
include_directories("${MNN_ROOT}/include")
include_directories("${MNN_ROOT}/transformers/llm/engine/include")

# Pre-built MNN shared libraries (arm64-v8a Release build).
add_library(MNN SHARED IMPORTED)
set_target_properties(MNN PROPERTIES
    IMPORTED_LOCATION "${MNN_ROOT}/project/android/build_64/libMNN.so"
)

add_library(llm SHARED IMPORTED)
set_target_properties(llm PROPERTIES
    IMPORTED_LOCATION "${MNN_ROOT}/project/android/build_64/libllm.so"
)

# JNI wrapper -- loaded by LocalLlmInference via System.loadLibrary("klarvo_mnn").
add_library(klarvo_mnn SHARED klarvo_llm_jni.cpp)

target_link_libraries(klarvo_mnn
    android   # ANativeActivity, AAssetManager
    log       # __android_log_print
    MNN
    llm
)
