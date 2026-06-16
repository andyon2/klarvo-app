#!/usr/bin/env bash
# conductor-guard.sh — mechanische Lauf-Garantien für den BMAD-Epic-Conductor.
#
# Hintergrund: Postmortem 2026-06-15 (docs/postmortem-2026-06-15-epic-conductor.md).
# Kernlehre: Garantien müssen aus der UMGEBUNG kommen (Locks, Erreichbarkeit), nicht
# aus Prosa-Bitten an Subagenten — eine Bitte gegen den eingebauten Ablauf eines
# bmad-Skills ist nicht bindend. Dieses Script ist die mechanische Form von Naht 2
# (Test-Isolation) und Naht 4 (Run-Isolation).
#
# Subcommands:
#   acquire        Naht 4: Lock setzen (kein überlappender Lauf). Naht 2: echtes
#                  Android-Gerät abkoppeln, damit Worker es physisch NICHT treffen
#                  können (ein Worker, der android-smoke.sh ruft, kann das Telefon
#                  dann nicht installieren). Setzt expectedHead = aktueller HEAD.
#   expect <sha>   HEAD-Wächter füttern: erwarteten HEAD nach EIGENEM Commit setzen
#                  (der Conductor ruft das nach jedem seiner Story-Commits).
#   check-head     HEAD-Wächter: bewegte sich HEAD fremd (!= expectedHead)? exit 4.
#   release        Lock entfernen + echtes Gerät wieder verbinden (Lauf-Ende).
#   break          verwaisten Lock eines abgebrochenen Laufs hart entfernen.
#   status         Lock-Zustand zeigen (exit 0 = frei, 3 = gehalten).
#
# Env:
#   KLARVO_REAL_DEVICE   echtes Gerät zum Ab-/Ankoppeln (default 100.112.41.70:5555)
#   KLARVO_GUARD_NO_ADB  =1 → adb-Schritte überspringen (Tests / Nicht-Android-Epics)
#
# Der Lock ist ein LOGISCHER Lauf-Marker (ein Conductor-Lauf erstreckt sich über
# viele kurzlebige Agenten-Schritte, nicht einen Prozess) — daher KEINE PID-Liveness:
# ein vorhandener Lock heißt "Lauf aktiv → abbrechen"; ein verwaister Lock wird mit
# 'break' bewusst entfernt.

set -euo pipefail
cd "$(dirname "$0")/.."

LOCK=".conductor-lock"
REAL_DEVICE="${KLARVO_REAL_DEVICE:-100.112.41.70:5555}"

ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"

log() { echo "[conductor-guard] $*"; }
err() { echo "[conductor-guard] $*" >&2; }

adb_available() {
    [ "${KLARVO_GUARD_NO_ADB:-0}" != "1" ] && [ -x "$ADB" ]
}

disconnect_device() {
    if ! adb_available; then
        log "adb-Schritt übersprungen (KLARVO_GUARD_NO_ADB / kein adb)."
        return 0
    fi
    if [ -z "$REAL_DEVICE" ]; then
        log "kein echtes Gerät konfiguriert — nichts abzukoppeln."
        return 0
    fi
    # Naht 2: echtes Gerät für die Lauf-Dauer unerreichbar machen.
    "$ADB" disconnect "$REAL_DEVICE" >/dev/null 2>&1 || true
    log "echtes Gerät abgekoppelt: $REAL_DEVICE (Naht 2 — Worker können es nicht treffen)."
}

reconnect_device() {
    if ! adb_available; then return 0; fi
    if [ -z "$REAL_DEVICE" ]; then return 0; fi
    "$ADB" connect "$REAL_DEVICE" >/dev/null 2>&1 || true
    log "echtes Gerät wieder verbunden: $REAL_DEVICE."
}

lock_get() { sed -n "s/^$1=//p" "$LOCK" 2>/dev/null | head -1; }

cmd_acquire() {
    if [ -f "$LOCK" ]; then
        err "LOCK vorhanden — laut Lock ist ein Lauf aktiv. Naht 4: kein überlappender Lauf."
        cat "$LOCK" >&2
        err "Falls verwaist (abgebrochener Lauf): 'scripts/conductor-guard.sh break', dann erneut acquire."
        exit 3
    fi
    local sha
    sha=$(git rev-parse HEAD 2>/dev/null || echo "?")
    {
        echo "pid=$$"
        echo "startSha=$sha"
        echo "expectedHead=$sha"
        echo "startedAt=$(date -Iseconds)"
        echo "realDevice=$REAL_DEVICE"
    } > "$LOCK"
    log "Lock gesetzt (startSha=$sha)."
    disconnect_device
}

cmd_expect() {
    [ -f "$LOCK" ] || { err "kein Lock — erst acquire."; exit 1; }
    local sha="${1:-}"
    [ -n "$sha" ] || { err "expect braucht einen SHA: conductor-guard.sh expect <sha>"; exit 1; }
    # expectedHead-Zeile ersetzen (oder anhängen).
    if grep -q '^expectedHead=' "$LOCK"; then
        sed -i "s/^expectedHead=.*/expectedHead=$sha/" "$LOCK"
    else
        echo "expectedHead=$sha" >> "$LOCK"
    fi
    log "HEAD-Wächter: expectedHead=$sha."
}

cmd_check_head() {
    [ -f "$LOCK" ] || { err "kein Lock — erst acquire."; exit 1; }
    local expected current
    expected=$(lock_get expectedHead)
    current=$(git rev-parse HEAD 2>/dev/null || echo "?")
    if [ -z "$expected" ]; then
        err "kein expectedHead im Lock — HEAD-Wächter nicht initialisiert."
        exit 1
    fi
    if [ "$current" != "$expected" ]; then
        err "HEAD-DRIFT: HEAD=$current, erwartet=$expected — Fremdmutation (Naht 4). HALTEN + melden."
        exit 4
    fi
    log "HEAD ok ($current == erwartet)."
}

cmd_release() {
    reconnect_device
    rm -f "$LOCK"
    log "Lock entfernt + Gerät wieder verbunden (Lauf-Ende)."
}

cmd_break() {
    rm -f "$LOCK"
    log "Lock entfernt (break — verwaister Lauf)."
}

cmd_status() {
    if [ -f "$LOCK" ]; then
        log "Lock GEHALTEN:"
        cat "$LOCK"
        exit 3
    fi
    log "kein Lock — frei."
    exit 0
}

case "${1:-}" in
    acquire)    cmd_acquire ;;
    expect)     cmd_expect "${2:-}" ;;
    check-head) cmd_check_head ;;
    release)    cmd_release ;;
    break)      cmd_break ;;
    status)     cmd_status ;;
    *) err "Verwendung: $0 {acquire|expect <sha>|check-head|release|break|status}"; exit 64 ;;
esac
