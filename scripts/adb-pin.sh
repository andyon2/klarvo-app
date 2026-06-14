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
#
# Finding 1 fix: error goes to stderr + return 2 (not exit in subshell, which would
# only kill the subshell and capture the error text into the caller's variable).
# Finding 5 fix: "host:port" path also validates the port portion (last colon field)
# with the same numeric check — rejects IPv6 (multiple colons) + malformed forms.
to_addr() {
    case "$1" in
        *:*)
            # Split on last colon: everything before is host, after is port.
            local host port
            host="${1%:*}"
            port="${1##*:}"
            # Reject multiple-colon forms (bare IPv6, ip:port:extra, etc.)
            case "$host" in
                *:*) echo "FEHLER: Ungültiges Format (mehrere Doppelpunkte): '$1' — bitte 'ip:port' eingeben." >&2; return 2 ;;
            esac
            # Reject empty host (e.g. input ':5555')
            case "$host" in
                '') echo "FEHLER: Host darf nicht leer sein: '$1'" >&2; return 2 ;;
            esac
            # Reject empty port or non-numeric port
            case "$port" in
                '') echo "FEHLER: Port darf nicht leer sein: '$1'" >&2; return 2 ;;
                *[!0-9]*) echo "FEHLER: Port nicht numerisch: '$port' in '$1'" >&2; return 2 ;;
            esac
            printf '%s' "$1" ;;
        *)
            # Reject empty bare-port input
            case "$1" in
                '') echo "FEHLER: Port darf nicht leer sein." >&2; return 2 ;;
                *[!0-9]*) echo "FEHLER: Port nicht numerisch: '$1'" >&2; return 2 ;;
            esac
            printf '%s:%s' "$PIN_HOST" "$1" ;;
    esac
}

# --- Connect-Port beziehen: Argument ODER interaktiv erfragen ---------------
if [ $# -ge 1 ]; then
    EPHEMERAL="$(to_addr "$1")" || exit 2
else
    echo "Handy: Entwickleroptionen → 'Drahtloses Debugging' → Port hinter der IP ablesen."
    read -rp "Connect-Port (z.B. 39555), oder volle ip:port: " PORTIN
    [ -n "$PORTIN" ] || { echo "FEHLER: kein Port eingegeben."; exit 2; }
    EPHEMERAL="$(to_addr "$PORTIN")" || exit 2
fi

# Finding 3 fix: scope the check to $EPHEMERAL transport, not just any device.
connected() { "$ADB" devices | grep -q "^${EPHEMERAL}[[:space:]].*device$"; }

# --- 1. Verbinden (bei Bedarf vorher koppeln) ------------------------------
echo ""
echo "[1/3] connect $EPHEMERAL …"
OUT=$("$ADB" connect "$EPHEMERAL" 2>&1); echo "      $OUT"

if ! echo "$OUT" | grep -qiE 'connected'; then
    echo ""
    echo "      Verbindung fehlgeschlagen — Gerät evtl. noch nicht mit diesem PC gekoppelt."
    read -rp "      Pairing nötig? Pair-Port eingeben (Enter = überspringen): " PAIRIN
    if [ -n "$PAIRIN" ]; then
        PAIRADDR="$(to_addr "$PAIRIN")" || exit 2
        read -rp "      6-stelliger Pairing-Code (aus 'Gerät mit Code koppeln'): " PCODE
        # Finding 4 fix: capture and check pair output; surface failure clearly.
        PAIR_OUT=$("$ADB" pair "$PAIRADDR" "$PCODE" 2>&1)
        echo "      pair: $PAIR_OUT"
        if ! echo "$PAIR_OUT" | grep -qiE 'successfully paired|bereits gekoppelt'; then
            echo "" >&2
            echo "FEHLER: Pairing fehlgeschlagen (falscher/abgelaufener Code?)." >&2
            echo "        Ausgabe: $PAIR_OUT" >&2
            exit 1
        fi
        echo "[1b]  connect $EPHEMERAL (nach Pairing) …"
        CONN_OUT=$("$ADB" connect "$EPHEMERAL" 2>&1)
        echo "      $CONN_OUT"
        if ! echo "$CONN_OUT" | grep -qiE 'connected'; then
            echo "" >&2
            echo "FEHLER: Verbindung nach Pairing fehlgeschlagen." >&2
            echo "        Ausgabe: $CONN_OUT" >&2
            exit 1
        fi
    fi
fi

connected || { echo ""; echo "FEHLER: kein Gerät verbunden. Port prüfen (ändert sich bei jedem Toggle/Reboot)."; exit 1; }

# --- 2. Auf Festport 5555 pinnen -------------------------------------------
echo ""
echo "[2/3] tcpip 5555 (auf Festport pinnen) …"
# Finding 1 fix: capture adb exit status BEFORE the echo (echo always exits 0,
# so testing $? after the echo was always 0 — the numeric guard was dead).
TCPIP_OUT=$("$ADB" -s "$EPHEMERAL" tcpip 5555 2>&1)
TCPIP_RC=$?
echo "      $TCPIP_OUT"
if [ $TCPIP_RC -ne 0 ] || echo "$TCPIP_OUT" | grep -qiE 'error|failed|cannot'; then
    echo "" >&2
    echo "FEHLER: 'adb tcpip 5555' fehlgeschlagen — Port 5555 NICHT gesetzt." >&2
    echo "        Ausgabe: $TCPIP_OUT" >&2
    exit 1
fi
sleep 2

# Finding 3 fix: disconnect ephemeral transport before final connect, so only
# the stable :5555 transport remains (prevents android-smoke.sh from picking the
# stale ephemeral entry via blind 'grep device$ | head -1').
echo "      [disconnect ephemeral $EPHEMERAL]"
"$ADB" disconnect "$EPHEMERAL" >/dev/null 2>&1 || true

# --- 3. Stabile Tailscale-Adresse verifizieren -----------------------------
echo ""
echo "[3/3] connect $PIN_TARGET (stabile Tailscale-Adresse) …"
# Finding 2 fix: check final connect and gate the success banner on a real verify.
FINAL_OUT=$("$ADB" connect "$PIN_TARGET" 2>&1)
echo "      $FINAL_OUT"
if ! echo "$FINAL_OUT" | grep -qiE 'connected'; then
    echo "" >&2
    echo "FEHLER: Finale Verbindung zu $PIN_TARGET fehlgeschlagen." >&2
    echo "        Ausgabe: $FINAL_OUT" >&2
    echo "        Handy-Zustand prüfen: PIN-Transport eventuell noch nicht bereit?" >&2
    exit 1
fi

echo ""
echo "Verbundene Geräte:"
"$ADB" devices

echo ""
echo "OK — Festport 5555 gesetzt. 'Klarvo Android Smoke' connectet jetzt automatisch,"
echo "bis das Handy neu startet (dann dieses Script erneut)."
