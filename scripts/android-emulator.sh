#!/usr/bin/env bash
# android-emulator.sh — Ensure the unattended Android test surface (WSL emulator) is up.
#
# WHY THIS EXISTS
# ---------------
# The real Xiaomi device over Tailscale is reachable from WSL, but it is NOT an
# *unattended* test surface: installing an APK trips MIUI's USER_RESTRICTED
# confirmation, overlay/permission grants need taps, the screen must be on. A
# night-run conductor has no finger at the phone. So unattended Android smoke runs
# against a headless WSL emulator instead; the real device stays Andi's morning
# aesthetic / MIUI-specific human gate.
#
# This script boots (idempotently) the `klarvo-emu` AVD headless with KVM, waits
# for boot_completed, and prints the serial on stdout (last line). Re-running when
# it is already up is a fast no-op.
#
# THE UNATTENDED SMOKE RECIPE (what callers do after this script) — see the memory
# file reference_android_unattended_emulator_smoke.md for the full rationale:
#   APK=...app-universal-debug.apk     # universal debug APK (built via android-smoke.sh path)
#   # CRITICAL: force arm64 ABI. The Rust core (libklarvo_lib.so) is built ONLY for
#   # arm64 (android-build.sh does --target aarch64); the x86_64 image runs it via
#   # the google_apis native-bridge (libndk_translation). A plain install picks the
#   # x86_64 split, which lacks libklarvo_lib.so -> UnsatisfiedLinkError on launch.
#   adb -s "$SER" install --abi arm64-v8a -r -g "$APK"
#   adb -s "$SER" shell appops set com.klarvo.voice SYSTEM_ALERT_WINDOW allow
#   adb -s "$SER" shell pm grant com.klarvo.voice android.permission.RECORD_AUDIO
#   adb -s "$SER" shell pm grant com.klarvo.voice android.permission.POST_NOTIFICATIONS
#   # Drive a bubble state without live audio/network (debug-only RECEIVER_EXPORTED):
#   adb -s "$SER" shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE \
#       --es state idle|recording|transcribing|done [--ef rms 0.7] [--es transcript "..."] \
#       -p com.klarvo.voice
#   adb -s "$SER" exec-out screencap -p > /tmp/shot.png
#
# NOTE (the 9-4 gap this surfaced): DEBUG_SET_STATE currently only sets STATE; it
# does NOT force-SHOW the bubble (the overlay is added to WindowManager on keyboard-
# open or when the `bubble_always_visible` pref is true). Until 9-4 makes the
# harness self-show, force visibility once via:
#   printf '%s\n' "<?xml version='1.0' encoding='utf-8' standalone='yes' ?>" "<map>" \
#     '    <boolean name="bubble_always_visible" value="true" />' "</map>" > /tmp/kbp.xml
#   adb -s "$SER" push /tmp/kbp.xml /data/local/tmp/kbp.xml
#   adb -s "$SER" shell "run-as com.klarvo.voice sh -c 'mkdir -p shared_prefs && cp /data/local/tmp/kbp.xml shared_prefs/klarvo_bubble_prefs.xml'"
#   # then (re)start the overlay service via the app (it is exported=false; launch
#   # MainActivity and clear the permission chain — accessibility dialog has SKIP).

set -euo pipefail

ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-/home/andyon2/.android/avd}"
EMU="$ANDROID_HOME/emulator/emulator"
ADB="$ANDROID_HOME/platform-tools/adb"
AVD="${KLARVO_AVD:-klarvo-emu}"
SER="${KLARVO_EMU_SERIAL:-emulator-5554}"
PORT="${SER##*-}"

export ANDROID_HOME ANDROID_AVD_HOME

log() { echo "[emu] $*" >&2; }

boot_completed() { [ "$("$ADB" -s "$SER" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; }

# Already up? fast no-op.
if "$ADB" devices 2>/dev/null | grep -q "^${SER}[[:space:]].*device$" && boot_completed; then
    log "$SER already booted."
    echo "$SER"
    exit 0
fi

# KVM access: after Andi's next WSL login the kvm group is native; in a session
# started before that, wrap the launch in `sg kvm` (the group membership exists in
# /etc/group even if not yet in this process's group set).
KVM_WRAP=()
if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    log "KVM directly accessible."
elif id -nG 2>/dev/null | tr ' ' '\n' | grep -qx kvm; then
    log "KVM via current group set."
elif getent group kvm 2>/dev/null | grep -q ":.*\b$(id -un)\b"; then
    log "KVM via 'sg kvm' wrapper (group not yet active in this session)."
    KVM_WRAP=(sg kvm -c)
else
    log "ERROR: no KVM access. One-time fix: sudo usermod -aG kvm \$USER && sudo chmod 666 /dev/kvm"
    exit 1
fi

log "Booting AVD '$AVD' headless on port $PORT ..."
EMU_CMD="$EMU -avd $AVD -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -no-snapshot -port $PORT -accel on"
if [ "${#KVM_WRAP[@]}" -gt 0 ]; then
    nohup "${KVM_WRAP[@]}" "$EMU_CMD" >/tmp/klarvo-emu-boot.log 2>&1 &
else
    nohup $EMU_CMD >/tmp/klarvo-emu-boot.log 2>&1 &
fi

log "Waiting for device transport ..."
"$ADB" -s "$SER" wait-for-device
log "Waiting for boot_completed ..."
for _ in $(seq 1 90); do
    boot_completed && { log "Boot complete."; echo "$SER"; exit 0; }
    sleep 3
done
log "ERROR: boot timed out (see /tmp/klarvo-emu-boot.log)."
exit 1
