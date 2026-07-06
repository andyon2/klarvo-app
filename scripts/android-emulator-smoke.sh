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

# 2b. Debug harness: inject DebugHarnessReceiver via the debug source-set manifest overlay.
#     The manifest merger merges src/debug/AndroidManifest.xml over src/main for debug variants,
#     so the receiver lands ONLY in debug APKs and never in release (AC4).
#
#     This replaces the old broken sed approach (which failed because the service attributes are
#     on separate lines) AND the old start-foreground-service approach (which failed because a
#     dynamically-registered receiver on a dead process is silently dropped).
#
#     The static DebugHarnessReceiver wakes the dead process on the first broadcast and starts
#     KlarvoOverlayService; no explicit start-foreground-service call is needed.
DEBUG_MANIFEST_DIR="$APP_DIR/src/debug"
mkdir -p "$DEBUG_MANIFEST_DIR"
cat > "$DEBUG_MANIFEST_DIR/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<!-- Debug source-set overlay: merged over src/main only for debug builds.
     Adds the static DebugHarnessReceiver so adb broadcasts wake a dead process.
     Also adds dataSync to KlarvoOverlayService foregroundServiceType so the
     service can cold-start from background (broadcast context) on API 34+ when the
     microphone type is blocked — the DATA_SYNC fallback in startForegroundWithNotification
     requires this to be declared in the manifest.
     Never included in release APKs. -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
          xmlns:tools="http://schemas.android.com/tools">
    <!-- Permission required for dataSync FGS type fallback on API 34+ (harness cold-start). -->
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
    <application>
        <receiver
            android:name=".DebugHarnessReceiver"
            android:exported="true"
            android:enabled="true">
            <intent-filter>
                <action android:name="com.klarvo.voice.DEBUG_SET_STATE" />
            </intent-filter>
        </receiver>
        <!-- Extend foregroundServiceType so the DATA_SYNC fallback is allowed on API 34+
             when starting from background broadcast context. tools:node="merge" appends
             to the existing declaration in src/main without replacing it. -->
        <service
            android:name=".KlarvoOverlayService"
            android:foregroundServiceType="microphone|dataSync"
            tools:node="merge" />
    </application>
</manifest>
EOF
log "manifest: debug overlay written → $DEBUG_MANIFEST_DIR/AndroidManifest.xml (DebugHarnessReceiver)"

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
log "permissions granted."

# 6. Debug harness: ready. Wake the service once so subsequent broadcasts hit the running
#    dynamic receiver (fast path). Two steps needed for a freshly installed APK:
#      a) Set app standby bucket to ACTIVE so Android doesn't rate-limit broadcasts.
#      b) adb install sets stopped=true (Android blocks broadcasts to stopped apps).
#         --include-stopped-packages clears that flag on the very first delivery.
#    After the first broadcast the service is running and the dynamic receiver handles
#    subsequent broadcasts without needing the flag.
"$ADB" -s "$SER" shell am set-standby-bucket "$PKG" active >/dev/null 2>&1 || true
log "harness: waking service via first idle broadcast (--include-stopped-packages clears stopped=true) ..."
"$ADB" -s "$SER" shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle \
    -p "$PKG" --include-stopped-packages >/dev/null 2>&1 || true
sleep 2   # give onCreate + bubbleView time to initialise before first harness state
log "harness ready. Drive states via DEBUG_SET_STATE broadcasts:"
log "  adb -s $SER shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle -p $PKG"

echo "$SER"
