#!/usr/bin/env bash
# adb-pin.sh — Handy nach einem Neustart in EINEM Schritt wieder auf den
# Festport 5555 pinnen, damit scripts/android-smoke.sh künftig per Tailscale
# auto-connectet.
#
# Hintergrund: Android 11+ "Drahtloses Debugging" vergibt bei jedem Reboot/Toggle
# einen ZUFÄLLIGEN Connect-Port. Das Pairing bleibt erhalten (du koppelst nur das
# erste Mal), aber der Port wechselt. `adb tcpip 5555` pinnt das Handy bis zum
# nächsten Neustart auf 5555 — ein reboot-fester Festport ginge nur mit Root
# (persist.adb.tcp.port).
#
# Der zufällige Debug-Port lauscht auf ALLEN Interfaces des Handys, also auch auf
# der Tailscale-IP. Die Tailscale-IP ist reboot-fest — darum musst du nach einem
# Neustart effektiv nur noch die PORT-ZAHL vom Screen ablesen; die IP setzt dieses
# Script automatisch davor.
#
# BEDIENUNG (interaktiv, z.B. per Desktop-Shortcut "Klarvo ADB Pin"):
#   1. Handy: Entwickleroptionen → "Drahtloses Debugging" einschalten
#   2. Den dort gezeigten Port (hinter der IP) ablesen
#   3. Dieses Script starten → Port eingeben, wenn gefragt.
#      (Beim allerersten Mal fragt es zusätzlich nach dem Pairing-Port + Code.)
#
# Nicht-interaktiv:  scripts/adb-pin.sh <port>            (Tailscale-IP davor)
#               oder scripts/adb-pin.sh <ip>:<port>       (volle Adresse)

set -uo pipefail
cd "$(dirname "$0")/.."

export ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"

# Stabile Tailscale-IP des Handys (überschreibbar) + Pin-Ziel auf Festport 5555.
PIN_HOST="${KLARVO_ADB_HOST:-100.112.41.70}"
PIN_TARGET="${KLARVO_ADB_TARGET:-$PIN_HOST:5555}"

[ -x "$ADB" ] || { echo "FEHLER: adb nicht gefunden: $ADB"; exit 1; }

# Bare Port -> Tailscale-IP davor; volle "host:port"-Eingabe wird verbatim genutzt.
to_addr() {
    case "$1" in
        *:*) printf '%s' "$1" ;;
        *)   printf '%s:%s' "$PIN_HOST" "$1" ;;
    esac
}

# --- Connect-Port beziehen: Argument ODER interaktiv erfragen ---------------
if [ $# -ge 1 ]; then
    EPHEMERAL="$(to_addr "$1")"
else
    echo "Handy: Entwickleroptionen → 'Drahtloses Debugging' → Port hinter der IP ablesen."
    read -rp "Connect-Port (z.B. 39555), oder volle ip:port: " PORTIN
    [ -n "$PORTIN" ] || { echo "FEHLER: kein Port eingegeben."; exit 2; }
    EPHEMERAL="$(to_addr "$PORTIN")"
fi

connected() { "$ADB" devices | grep -q "device$"; }

# --- 1. Verbinden (bei Bedarf vorher koppeln) ------------------------------
echo ""
echo "[1/3] connect $EPHEMERAL …"
OUT=$("$ADB" connect "$EPHEMERAL" 2>&1); echo "      $OUT"

if ! echo "$OUT" | grep -qiE 'connected'; then
    echo ""
    echo "      Verbindung fehlgeschlagen — Gerät evtl. noch nicht mit diesem PC gekoppelt."
    read -rp "      Pairing nötig? Pair-Port eingeben (Enter = überspringen): " PAIRIN
    if [ -n "$PAIRIN" ]; then
        PAIRADDR="$(to_addr "$PAIRIN")"
        read -rp "      6-stelliger Pairing-Code (aus 'Gerät mit Code koppeln'): " PCODE
        "$ADB" pair "$PAIRADDR" "$PCODE"
        echo "[1b]  connect $EPHEMERAL (nach Pairing) …"
        "$ADB" connect "$EPHEMERAL"
    fi
fi

connected || { echo ""; echo "FEHLER: kein Gerät verbunden. Port prüfen (ändert sich bei jedem Toggle/Reboot)."; exit 1; }

# --- 2. Auf Festport 5555 pinnen -------------------------------------------
echo ""
echo "[2/3] tcpip 5555 (auf Festport pinnen) …"
"$ADB" tcpip 5555
sleep 2

# --- 3. Stabile Tailscale-Adresse verifizieren -----------------------------
echo ""
echo "[3/3] connect $PIN_TARGET (stabile Tailscale-Adresse) …"
"$ADB" connect "$PIN_TARGET" 2>&1 | sed 's/^/      /'

echo ""
echo "Verbundene Geräte:"
"$ADB" devices

echo ""
echo "OK — Festport 5555 gesetzt. 'Klarvo Android Smoke' connectet jetzt automatisch,"
echo "bis das Handy neu startet (dann dieses Script erneut)."
