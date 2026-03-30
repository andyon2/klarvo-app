# Toolchain overlay: force ggml to NOT build AVX-512 variants.
# Used via CMAKE_TOOLCHAIN_FILE env var in sync-and-build.ps1.
# Reason: Build machine (AMD Ryzen 7) supports AVX-512, but end-user
# machines may not (e.g. Intel 8th gen). Without this, ggml auto-detects
# AVX-512 and builds a variant that crashes on older CPUs.
set(GGML_NATIVE OFF CACHE BOOL "Disable native CPU detection" FORCE)
set(GGML_AVX512 OFF CACHE BOOL "Disable AVX-512" FORCE)
set(GGML_AVX512_VBMI OFF CACHE BOOL "" FORCE)
set(GGML_AVX512_VNNI OFF CACHE BOOL "" FORCE)
set(GGML_AVX512_BF16 OFF CACHE BOOL "" FORCE)
set(GGML_AMX_TILE OFF CACHE BOOL "" FORCE)
set(GGML_AMX_INT8 OFF CACHE BOOL "" FORCE)
set(GGML_AMX_BF16 OFF CACHE BOOL "" FORCE)
