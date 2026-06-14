#!/usr/bin/env bash
# android-smoke.sh — Schneller Kotlin-only Debug-Build + adb Install für Smoke-Tests.
#
# Nutzen: nach Kotlin-Änderungen (kein Rust geändert) ~ 2-3 Minuten statt 20-30.
# Voraussetzung: android-build.sh muss mindestens einmal gelaufen sein
#                (erzeugt gen/android/ + .so-Dateien für den Rust-Teil).
#
# adb-Verbindung (einmalig einrichten):
#   USB:   Gerät per USB anschließen, USB-Debugging in Entwickleroptionen aktivieren.
#          In WSL2 verbindet sich der WSL-adb automatisch mit dem Windows-adb-Server
#          auf localhost:5037, falls Windows-adb läuft (Android Studio / SDK).
#   WiFi:  Kabelfrei bevorzugt? Auf dem Handy "Drahtloses Debugging" aktivieren.
#          Einmalig: adb pair <ip>:<pair-port>   (Pin aus Handy eingeben)
#          Dann:     adb connect <ip>:5555
#          Danach funktioniert dieses Script ohne USB.

set -euo pipefail
cd "$(dirname "$0")/.."

GEN_ANDROID="src-tauri/gen/android"
APP_DIR="$GEN_ANDROID/app"

# ---------------------------------------------------------------------------
# Hilfsfunktionen
# ---------------------------------------------------------------------------
fail() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    printf "║  SMOKE FAILED: %-51s║\n" "$1"
    echo "║  Keine neue APK auf dem Gerät.                                   ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    read -rsp $'\nTaste druecken zum Schliessen...' -n1 || true
    exit 1
}
trap 'fail "Unerwarteter Fehler in Zeile $LINENO"' ERR

ok()   { echo "[ok]    $*"; }
info() { echo "[info]  $*"; }
warn() { echo "[warn]  $*"; }
step() { echo ""; echo "── $* ──────────────────────────────────────────────"; }

# ---------------------------------------------------------------------------
# 0. Umgebung
# ---------------------------------------------------------------------------
export JAVA_HOME="/usr/lib/jvm/java-17-openjdk-amd64"
export ANDROID_HOME="${ANDROID_HOME:-/home/andyon2/workspace/tools/android-sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"

# ---------------------------------------------------------------------------
# 1. Voraussetzungen
# ---------------------------------------------------------------------------
step "Voraussetzungen prüfen"

if [ ! -d "$APP_DIR" ]; then
    fail "gen/android/ fehlt — zuerst 'scripts/android-build.sh' laufen lassen"
fi

SO_COUNT=$(find "$APP_DIR/src/main/jniLibs" -name "*.so" 2>/dev/null | wc -l)
if [ "$SO_COUNT" -eq 0 ]; then
    fail ".so-Dateien fehlen — zuerst 'scripts/android-build.sh' laufen lassen (einmalig)"
fi
ok "$SO_COUNT .so-Dateien vorhanden (Rust-Seite gecacht)"

if [ ! -x "$ADB" ]; then
    fail "adb nicht gefunden: $ADB"
fi

# Gerät prüfen
DEVICES=$(${ADB} devices 2>/dev/null | grep -c "device$" || true)
if [ "$DEVICES" -eq 0 ]; then
    echo ""
    echo "  Kein Android-Gerät erreichbar. Optionen:"
    echo ""
    echo "  USB:   USB-Debugging in Entwickleroptionen aktivieren."
    echo "         In WSL2: Windows-adb muss laufen (Android Studio öffnen reicht)."
    echo "         Dann: ${ADB} devices"
    echo ""
    echo "  WiFi:  'Drahtloses Debugging' auf dem Handy aktivieren."
    echo "         Einmalig pairen:  ${ADB} pair <ip>:<pair-port>"
    echo "         Verbinden:        ${ADB} connect <ip>:5555"
    echo ""
    fail "Kein Gerät gefunden"
fi
DEVICE_SERIAL=$(${ADB} devices 2>/dev/null | grep "device$" | head -1 | awk '{print $1}')
ok "Gerät: $DEVICE_SERIAL"

# ---------------------------------------------------------------------------
# 2. Kotlin-Quellen synchronisieren
# ---------------------------------------------------------------------------
step "Kotlin-Quellen synchronisieren"

SRC="android/kotlin-src/com/klarvo/voice"
DST="$APP_DIR/src/main/java/com/klarvo/voice"
cp "$SRC"/*.kt "$DST/"
ok "$(ls -1 "$SRC"/*.kt | wc -l) Produktions-Dateien kopiert"

# Font-Ressourcen (Geist + Geist Mono)
FONT_SRC="android/res-font"
FONT_DST="$APP_DIR/src/main/res/font"
mkdir -p "$FONT_DST"
cp "$FONT_SRC"/*.ttf "$FONT_DST/"
ok "$(ls -1 "$FONT_SRC"/*.ttf | wc -l) Font-Dateien kopiert"

# Test-Quellen (git-tracked in android/kotlin-test/)
TEST_SRC="android/kotlin-test/com/klarvo/voice"
TEST_DST="$APP_DIR/src/test/java/com/klarvo/voice"
if [ -d "$TEST_SRC" ] && [ "$(ls -1 "$TEST_SRC"/*.kt 2>/dev/null | wc -l)" -gt 0 ]; then
    mkdir -p "$TEST_DST"
    cp "$TEST_SRC"/*.kt "$TEST_DST/"
    ok "$(ls -1 "$TEST_SRC"/*.kt | wc -l) Test-Dateien kopiert"
fi

# ---------------------------------------------------------------------------
# 3. Unit-Tests (Gate — Logik-Regression wird hier gefangen, nicht auf dem Gerät)
# ---------------------------------------------------------------------------
step "JVM-Unit-Tests"

cd "$GEN_ANDROID"
if ./gradlew :app:testUniversalDebugUnitTest --quiet 2>&1; then
    TEST_XML=$(find app/build/test-results -name "*.xml" 2>/dev/null | head -1)
    if [ -n "$TEST_XML" ]; then
        TOTAL=$(grep -o 'tests="[0-9]*"' "$TEST_XML" | head -1 | grep -o '[0-9]*')
        FAIL=$(grep -o 'failures="[0-9]*"' "$TEST_XML" | head -1 | grep -o '[0-9]*')
        ok "$TOTAL Tests, $FAIL Failures — alle grün"
    else
        ok "Tests grün"
    fi
else
    cd -
    fail "Unit-Tests gescheitert — APK nicht installiert"
fi
cd -

# ---------------------------------------------------------------------------
# 4. Debug-APK bauen (Kotlin neu, Rust aus Cache)
# ---------------------------------------------------------------------------
step "Debug-APK bauen"

APK_PATH="$APP_DIR/build/outputs/apk/universal/debug/app-universal-debug.apk"
APK_BEFORE_TS=0
[ -f "$APK_PATH" ] && APK_BEFORE_TS=$(stat -c %Y "$APK_PATH")

BUILD_START=$(date +%s)
cd "$GEN_ANDROID"
./gradlew :app:assembleUniversalDebug \
    -x :app:rustBuildUniversalDebug \
    -x :app:rustBuildArm64Debug \
    -x :app:rustBuildArmDebug \
    -x :app:rustBuildX86Debug \
    -x :app:rustBuildX86_64Debug \
    --quiet
cd -
BUILD_END=$(date +%s)
BUILD_SECS=$((BUILD_END - BUILD_START))

if [ ! -f "$APK_PATH" ]; then
    fail "APK nicht erzeugt: $APK_PATH"
fi

APK_AFTER_TS=$(stat -c %Y "$APK_PATH")
if [ "$APK_AFTER_TS" -le "$APK_BEFORE_TS" ]; then
    warn "APK-Timestamp nicht aktualisiert — Gradle hat inkrementell nichts neu gebaut."
    warn "Kotlin-Änderungen evtl. nicht drin. Wenn nötig:"
    warn "  cd $GEN_ANDROID && ./gradlew :app:assembleUniversalDebug --rerun-tasks"
else
    ok "Frische APK erzeugt in ${BUILD_SECS}s"
fi

APK_MB=$(du -m "$APK_PATH" | cut -f1)
ok "APK: $APK_PATH (${APK_MB} MB)"

# ---------------------------------------------------------------------------
# 5. adb install (AI-1 Gate: frisches Binary aufs Gerät)
# ---------------------------------------------------------------------------
step "adb install"

INSTALL_OUT=$(${ADB} -s "$DEVICE_SERIAL" install -r "$APK_PATH" 2>&1) || true
if echo "$INSTALL_OUT" | grep -q "INSTALL_FAILED_UPDATE_INCOMPATIBLE"; then
    warn "Signatur-Konflikt — alte App wird deinstalliert (Daten gehen verloren)"
    ${ADB} -s "$DEVICE_SERIAL" uninstall com.klarvo.voice || true
    ${ADB} -s "$DEVICE_SERIAL" install "$APK_PATH"
elif echo "$INSTALL_OUT" | grep -q "INSTALL_FAILED_USER_RESTRICTED"; then
    echo "$INSTALL_OUT"
    fail "Handy hat Installation abgebrochen — Bestaetigungs-Dialog am Geraet pruefen und nochmal starten"
elif echo "$INSTALL_OUT" | grep -q "INSTALL_FAILED\|Error\|error"; then
    echo "$INSTALL_OUT"
    fail "adb install gescheitert"
fi
ok "Installiert auf $DEVICE_SERIAL"

# ---------------------------------------------------------------------------
# 6. Verifikation: versionName auf dem Gerät (AI-1 Beweis)
# ---------------------------------------------------------------------------
step "Verifikation"

BUILT_VERSION=$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed 's/.*"\([0-9.]*\)".*/\1/')
DEVICE_VERSION=$(${ADB} -s "$DEVICE_SERIAL" shell dumpsys package com.klarvo.voice 2>/dev/null \
    | grep versionName | head -1 | sed 's/.*versionName=\([^ ]*\).*/\1/' | tr -d '\r')

if [ -z "$DEVICE_VERSION" ]; then
    warn "versionName konnte nicht ausgelesen werden (App evtl. noch nicht gestartet)"
elif [ "$DEVICE_VERSION" = "$BUILT_VERSION" ]; then
    ok "versionName auf Gerät: $DEVICE_VERSION ✓  (AI-1 Gate bestanden)"
else
    warn "versionName auf Gerät: $DEVICE_VERSION — erwartet: $BUILT_VERSION"
    warn "Möglicherweise läuft noch die alte Version. App neu starten."
fi

# ---------------------------------------------------------------------------
# Abschluss
# ---------------------------------------------------------------------------
echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
printf "║  SMOKE BUILD OK  v%-47s║\n" "${BUILT_VERSION}"
printf "║  Gerät:  %-56s║\n" "$DEVICE_SERIAL"
printf "║  Dauer:  %-51s║\n" "${BUILD_SECS}s"
echo "║                                                                  ║"
echo "║  Nächste Schritte:                                               ║"
echo "║  1. App auf dem Gerät öffnen                                     ║"
echo "║  2. Phantom-Phrase sprechen → kein Paste, Toast erscheint        ║"
echo "║  3. Echten Satz sprechen → Paste erfolgt normal                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
read -rsp $'\nTaste druecken zum Schliessen...' -n1 || true
