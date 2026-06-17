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

# LIFECYCLE (added 2026-06-17 after an orphaned emulator pegged ~8 cores)
# --------------------------------------------------------------------------
# The boot is `nohup ... &` ON PURPOSE — the emulator must outlive the booting
# shell so multiple test steps / subagents share one warm surface. The flip side
# is that nothing ever STOPPED it: a crashed/forgotten run left qemu reparented to
# init, burning CPU until a human killed it. Fix = the boot now also arms a
# detached, self-limiting WATCHDOG:
#   * hard TTL  (KLARVO_EMU_TTL_SECS, default 7200s/120min) — absolute max lifetime,
#     always on. Guarantees no infinite orphan even if the orchestrator dies.
#   * idle reaper (KLARVO_EMU_IDLE_SECS, default 0 = off) — kills early if the
#     heartbeat file is older than N seconds. Callers extend it with `bump`.
# A boot TOKEN in the marker file scopes each watchdog to its own boot: a newer
# boot (or `stop`) rewrites/removes the marker, and the stale watchdog sees the
# token change / marker vanish and exits WITHOUT killing the live emulator.
# Subcommands: up (default) | stop | bump | __watchdog <token> (internal).

set -euo pipefail

SELF="$(readlink -f "$0")"

ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-/home/andyon2/.android/avd}"
EMU="$ANDROID_HOME/emulator/emulator"
ADB="$ANDROID_HOME/platform-tools/adb"
AVD="${KLARVO_AVD:-klarvo-emu}"
SER="${KLARVO_EMU_SERIAL:-emulator-5554}"
PORT="${SER##*-}"

# Watchdog tunables (env-overridable; defaults are deliberately generous so the
# TTL never fires mid-run for realistic smoke/conductor durations).
TTL="${KLARVO_EMU_TTL_SECS:-7200}"        # hard max lifetime in seconds (120 min)
IDLE_SECS="${KLARVO_EMU_IDLE_SECS:-0}"    # 0 = idle reaper disabled
CHECK="${KLARVO_EMU_WD_CHECK:-30}"        # watchdog poll interval
MARKER="${KLARVO_EMU_MARKER:-/tmp/klarvo-emu.marker}"
HEARTBEAT="${KLARVO_EMU_HEARTBEAT:-/tmp/klarvo-emu.heartbeat}"
WD_PID="${KLARVO_EMU_WD_PID:-/tmp/klarvo-emu.watchdog.pid}"
WD_LOG="${KLARVO_EMU_WD_LOG:-/tmp/klarvo-emu-watchdog.log}"

export ANDROID_HOME ANDROID_AVD_HOME

log() { echo "[emu] $*" >&2; }

boot_completed() { [ "$("$ADB" -s "$SER" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; }

# Best-effort teardown: console kill first, then a targeted pkill of THIS avd's
# qemu as a fallback. Both no-op cleanly when nothing matches.
do_kill() {
    "$ADB" -s "$SER" emu kill >/dev/null 2>&1 || true
    sleep 2
    pkill -f "qemu-system.*-avd $AVD" 2>/dev/null || true
}

# Arm a fresh watchdog unless a live one already guards this boot.
ensure_watchdog() {
    if [ -f "$WD_PID" ] && kill -0 "$(cat "$WD_PID" 2>/dev/null)" 2>/dev/null; then
        log "Watchdog läuft bereits (pid $(cat "$WD_PID"))."
        return 0
    fi
    local token; token="$(date +%s)-$$-${RANDOM}"
    {
        echo "token=$token"
        echo "serial=$SER"
        echo "started=$(date +%s)"
        echo "ttl=$TTL"
    } > "$MARKER"
    touch "$HEARTBEAT"
    nohup "$SELF" __watchdog "$token" >>"$WD_LOG" 2>&1 &
    echo $! > "$WD_PID"
    log "Watchdog gestartet (pid $!, TTL ${TTL}s, idle $([ "$IDLE_SECS" -gt 0 ] && echo "${IDLE_SECS}s" || echo aus))."
}

cmd_watchdog() {
    local mytoken="${1:-}"
    [ -n "$mytoken" ] || exit 0
    while true; do
        [ -f "$MARKER" ] || exit 0                              # stopped → marker gone
        local tok started ttl now elapsed
        tok="$(sed -n 's/^token=//p' "$MARKER")"
        [ "$tok" = "$mytoken" ] || exit 0                       # superseded by newer boot
        started="$(sed -n 's/^started=//p' "$MARKER")"; started="${started:-0}"
        ttl="$(sed -n 's/^ttl=//p' "$MARKER")"; ttl="${ttl:-$TTL}"
        now="$(date +%s)"
        elapsed=$(( now - started ))
        if [ "$elapsed" -ge "$ttl" ]; then
            log "Watchdog: TTL ${ttl}s erreicht (elapsed ${elapsed}s) → Emulator wird beendet."
            do_kill; rm -f "$MARKER" "$WD_PID"; exit 0
        fi
        if [ "${IDLE_SECS:-0}" -gt 0 ] && [ -f "$HEARTBEAT" ]; then
            local hb idle
            hb="$(stat -c %Y "$HEARTBEAT" 2>/dev/null || echo "$now")"
            idle=$(( now - hb ))
            if [ "$idle" -ge "$IDLE_SECS" ]; then
                log "Watchdog: idle ${idle}s ≥ ${IDLE_SECS}s → Emulator wird beendet."
                do_kill; rm -f "$MARKER" "$WD_PID"; exit 0
            fi
        fi
        sleep "$CHECK"
    done
}

cmd_bump() { touch "$HEARTBEAT"; }

cmd_stop() {
    log "Stoppe Emulator $SER (AVD $AVD) ..."
    do_kill
    rm -f "$MARKER" "$WD_PID"        # marker removal makes any live watchdog exit
    log "Emulator gestoppt."
}

cmd_up() {
    # Already up? fast no-op — but make sure a watchdog guards it.
    if "$ADB" devices 2>/dev/null | grep -q "^${SER}[[:space:]].*device$" && boot_completed; then
        log "$SER already booted."
        ensure_watchdog
        echo "$SER"
        return 0
    fi

    # KVM access: after Andi's next WSL login the kvm group is native; in a session
    # started before that, wrap the launch in `sg kvm` (the group membership exists in
    # /etc/group even if not yet in this process's group set).
    local KVM_WRAP=()
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        log "KVM directly accessible."
    elif id -nG 2>/dev/null | tr ' ' '\n' | grep -qx kvm; then
        log "KVM via current group set."
    elif getent group kvm 2>/dev/null | grep -q ":.*\b$(id -un)\b"; then
        log "KVM via 'sg kvm' wrapper (group not yet active in this session)."
        KVM_WRAP=(sg kvm -c)
    else
        log "ERROR: no KVM access. One-time fix: sudo usermod -aG kvm \$USER && sudo chmod 666 /dev/kvm"
        return 1
    fi

    log "Booting AVD '$AVD' headless on port $PORT ..."
    local EMU_CMD="$EMU -avd $AVD -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -no-snapshot -port $PORT -accel on"
    if [ "${#KVM_WRAP[@]}" -gt 0 ]; then
        nohup "${KVM_WRAP[@]}" "$EMU_CMD" >/tmp/klarvo-emu-boot.log 2>&1 &
    else
        nohup $EMU_CMD >/tmp/klarvo-emu-boot.log 2>&1 &
    fi

    log "Waiting for device transport ..."
    "$ADB" -s "$SER" wait-for-device
    log "Waiting for boot_completed ..."
    local _
    for _ in $(seq 1 90); do
        if boot_completed; then
            log "Boot complete."
            ensure_watchdog
            echo "$SER"
            return 0
        fi
        sleep 3
    done
    log "ERROR: boot timed out (see /tmp/klarvo-emu-boot.log)."
    return 1
}

case "${1:-up}" in
    up|"")       cmd_up ;;
    stop|down)   cmd_stop ;;
    bump)        cmd_bump ;;
    __watchdog)  shift; cmd_watchdog "${1:-}" ;;
    *) log "Verwendung: $0 {up|stop|bump}"; exit 64 ;;
esac
