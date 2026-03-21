#!/usr/bin/env bash
# Build Voxlit for Android.
# Copies Kotlin source files from android/kotlin-src/ to gen/android/ before building,
# because Tauri does not manage custom Kotlin files automatically.

set -euo pipefail
cd "$(dirname "$0")/.."

# --- Sync Kotlin sources ---
SRC="android/kotlin-src/com/voxlit/voice"
DST="src-tauri/gen/android/app/src/main/java/com/voxlit/voice"

if [ ! -d "$DST" ]; then
    echo "Error: $DST does not exist. Run 'npx tauri android init' first."
    exit 1
fi

echo "[sync] Copying Kotlin files from kotlin-src/ to gen/android/"
cp "$SRC"/*.kt "$DST/"
echo "[sync] Done: $(ls -1 "$SRC"/*.kt | wc -l) files copied"

# --- Set up environment ---
export JAVA_HOME="/usr/lib/jvm/java-17-openjdk-amd64"
export ANDROID_HOME="/home/andyon2/android-sdk"
export NDK_HOME="/home/andyon2/android-sdk/ndk/27.0.12077973"
export CC_aarch64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
export AR_aarch64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"

# --- Build ---
echo "[build] Starting Android build..."
npx tauri android build --target aarch64

# --- Read version ---
VERSION=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed 's/.*"\([0-9.]*\)".*/\1/')
echo "[version] Building v${VERSION}"

# --- Sign + deploy ---
APK_IN="src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"
APK_ALIGNED="/tmp/voxlit-aligned.apk"
APK_DIR="/mnt/d/Dropbox/App Development/voxlit/releases/v${VERSION}"
mkdir -p "$APK_DIR"
APK_OUT="$APK_DIR/Voxlit-v${VERSION}.apk"

echo "[sign] Aligning and signing APK..."
"$ANDROID_HOME/build-tools/34.0.0/zipalign" -f -p 4 "$APK_IN" "$APK_ALIGNED"
"$ANDROID_HOME/build-tools/34.0.0/apksigner" sign \
    --ks voxlit-debug.keystore \
    --ks-pass pass:dikta123 \
    --key-pass pass:dikta123 \
    --out "$APK_OUT" \
    "$APK_ALIGNED"

echo "[done] APK ready at: $APK_OUT"
