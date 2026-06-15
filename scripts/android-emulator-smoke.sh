#!/usr/bin/env bash
# android-emulator-smoke.sh — Non-interactive Kotlin build + install onto the WSL
# emulator. The unattended sibling of android-smoke.sh (which targets the real
# device and BLOCKS on a `read -rsp` keypress — fatal for a night conductor).
#
# Use this from any unattended agent / epic-conductor GATE-4. It:
#   1. ensures the headless emulator is up (scripts/android-emulator.sh)
#   2. syncs Kotlin + fonts into gen/android
#   3. builds the universal debug APK (gradle, Rust .so from cache)
#   4. installs --abi arm64-v8a  (arm64 Rust core via the x86_64 native-bridge;
#      a plain install picks the x86_64 split which lacks libklarvo_lib.so -> crash)
#   5. grants overlay/mic/notifications + enables the a11y service via adb
#
# Prereq: scripts/android-build.sh has run once (produces gen/android + .so).
# Output: prints the serial as the LAST stdout line. Never prompts. Non-zero exit
# on failure (message to stderr). See memory reference_android_unattended_emulator_smoke.md.

set -euo pipefail
cd "$(dirname "$0")/.."

export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}"
export ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"
PKG="com.klarvo.voice"
GEN_ANDROID="src-tauri/gen/android"
APP_DIR="$GEN_ANDROID/app"
APK_PATH="$APP_DIR/build/outputs/apk/universal/debug/app-universal-debug.apk"

log()  { echo "[emu-smoke] $*" >&2; }
die()  { echo "[emu-smoke] FAIL: $*" >&2; exit 1; }

[ -d "$APP_DIR" ] || die "gen/android fehlt — zuerst scripts/android-build.sh laufen lassen."
[ "$(find "$APP_DIR/src/main/jniLibs" -name '*.so' 2>/dev/null | wc -l)" -gt 0 ] \
    || die ".so fehlen — zuerst scripts/android-build.sh (einmalig)."

# 1. Emulator up
log "ensuring emulator ..."
SER="$(scripts/android-emulator.sh | tail -1)"
[ -n "$SER" ] || die "Emulator-Boot fehlgeschlagen."
log "serial: $SER"

# 2. Sync Kotlin + fonts (+ tests if present)
cp android/kotlin-src/com/klarvo/voice/*.kt "$APP_DIR/src/main/java/com/klarvo/voice/"
mkdir -p "$APP_DIR/src/main/res/font"; cp android/res-font/*.ttf "$APP_DIR/src/main/res/font/" 2>/dev/null || true
if ls android/kotlin-test/com/klarvo/voice/*.kt >/dev/null 2>&1; then
    mkdir -p "$APP_DIR/src/test/java/com/klarvo/voice"
    cp android/kotlin-test/com/klarvo/voice/*.kt "$APP_DIR/src/test/java/com/klarvo/voice/"
fi
log "kotlin + fonts synced."

# 3. Build (Rust from cache)
log "building universal debug APK ..."
( cd "$GEN_ANDROID" && ./gradlew :app:assembleUniversalDebug \
    -x :app:rustBuildUniversalDebug -x :app:rustBuildArm64Debug -x :app:rustBuildArmDebug \
    -x :app:rustBuildX86Debug -x :app:rustBuildX86_64Debug --quiet ) || die "gradle assembleUniversalDebug fehlgeschlagen."
[ -f "$APK_PATH" ] || die "APK nicht erzeugt: $APK_PATH"
log "APK: $APK_PATH ($(du -m "$APK_PATH" | cut -f1) MB)"

# 4. Install (force arm64 split for native-bridge)
log "installing --abi arm64-v8a onto $SER ..."
OUT="$("$ADB" -s "$SER" install --abi arm64-v8a -r -g "$APK_PATH" 2>&1)" || true
if echo "$OUT" | grep -q INSTALL_FAILED_UPDATE_INCOMPATIBLE; then
    "$ADB" -s "$SER" uninstall "$PKG" >/dev/null 2>&1 || true
    OUT="$("$ADB" -s "$SER" install --abi arm64-v8a -g "$APK_PATH" 2>&1)" || true
fi
echo "$OUT" | grep -q "Success" || die "adb install fehlgeschlagen: $OUT"

# 5. Grants + a11y (idempotent)
"$ADB" -s "$SER" shell appops set "$PKG" SYSTEM_ALERT_WINDOW allow >/dev/null 2>&1 || true
"$ADB" -s "$SER" shell pm grant "$PKG" android.permission.RECORD_AUDIO >/dev/null 2>&1 || true
"$ADB" -s "$SER" shell pm grant "$PKG" android.permission.POST_NOTIFICATIONS >/dev/null 2>&1 || true
"$ADB" -s "$SER" shell settings put secure enabled_accessibility_services "$PKG/$PKG.KlarvoAccessibilityService" >/dev/null 2>&1 || true
"$ADB" -s "$SER" shell settings put secure accessibility_enabled 1 >/dev/null 2>&1 || true
"$ADB" -s "$SER" shell dumpsys deviceidle whitelist +$PKG >/dev/null 2>&1 || true
log "installed + permissions granted. Drive states via DEBUG_SET_STATE; screencap via exec-out."

echo "$SER"
