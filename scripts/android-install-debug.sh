#!/usr/bin/env bash
# android-install-debug.sh — bringt einen frischen DEBUG-signierten Build auf Andis Handy,
# von powerhouse aus, ohne die App zu deinstallieren.
#
# WARUM ES DIESES SKRIPT GIBT
# Auf dem Xiaomi liegt ein debug-signierter Build (Andis Entscheidung 2026-09-21: das bleibt so,
# weil nur ein Debug-Build `run-as com.klarvo.voice` erlaubt — config.json und klarvo.log live
# lesen hat den 13-1-Geraete-Check zweimal gerettet). Das signierte Release-APK aus
# `android-build.sh` scheitert darauf mit INSTALL_FAILED_UPDATE_INCOMPATIBLE; die Alternative
# waere Deinstallieren = Settings, Lizenz und Groq-Key weg.
# Der Android-Build lebt auf dem Laptop (SDK, NDK, gen/android). Der Laptop erreicht das Handy
# ueber Tailscale aber nicht, powerhouse schon. Also: dort bauen, hier installieren.
#
# AUFRUF
#   scripts/android-install-debug.sh <ip:port>          # Debug-APK aus dem vorhandenen Rust-Stand
#   scripts/android-install-debug.sh <ip:port> --full   # vorher android-build.sh --clean (Rust/JNI/Frontend neu)
#
#   <ip:port> = Handy -> Entwickleroptionen -> Drahtloses Debugging -> "IP-Adresse & Port".
#   Der Port wechselt nach jeder Bildschirmsperre.
#
# --full ist PFLICHT, wenn sich Rust, die JNI-Signatur oder das Frontend geaendert haben:
# das Debug-APK nimmt libklarvo_lib.so (mit eingebettetem Frontend) aus dem letzten vollen Build.
# Ohne --full muss der Laptop-Checkout schon auf dem gewuenschten Commit stehen
# (`scripts/windows-build.sh` erledigt Push + Checkout nebenbei).
#
# EXIT-CODES: 0 installiert · 1 Vorbedingung verletzt · 2 Build gescheitert · 3 Installation gescheitert

set -euo pipefail
cd "$(dirname "$0")/.."

REMOTE="${KLARVO_BUILD_HOST:-laptop}"
REMOTE_REPO='$HOME/workspace/products/klarvo'
ADB="${ANDROID_HOME:-$HOME/workspace/tools/android-sdk}/platform-tools/adb"
REMOTE_APK='workspace/products/klarvo/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk'
LOCAL_APK="${TMPDIR:-/tmp}/klarvo-universal-debug.apk"

SERIAL=""
FULL=0
for arg in "$@"; do
  case "$arg" in
    --full) FULL=1 ;;
    -*)     echo "Unbekannte Option: $arg" >&2; exit 1 ;;
    *)      SERIAL="$arg" ;;
  esac
done

say() { printf '\n\033[36m== %s\033[0m\n' "$*"; }
die() { printf '\n\033[31mABBRUCH: %s\033[0m\n' "$*" >&2; exit "${2:-1}"; }

[ -n "$SERIAL" ] || die "Handy-Adresse fehlt. Aufruf: scripts/android-install-debug.sh <ip:port> [--full]"
[ -x "$ADB" ] || die "adb nicht gefunden: $ADB"

# --- 1. Handy zuerst: ein 20-Minuten-Build fuer ein unerreichbares Geraet ist verschenkt ----
say "1/4  Handy verbinden ($SERIAL)"
"$ADB" connect "$SERIAL" >/dev/null || true
"$ADB" -s "$SERIAL" get-state >/dev/null 2>&1 \
  || die "Handy nicht erreichbar. Drahtloses Debugging an? Port noch aktuell?"

# --- 2. Der Laptop muss denselben Commit tragen --------------------------------------------
say "2/4  Stand auf $REMOTE pruefen"
LOCAL_SHA="$(git rev-parse HEAD)"
REMOTE_SHA="$(ssh "$REMOTE" "cd $REMOTE_REPO && git rev-parse HEAD")"
echo "    powerhouse $LOCAL_SHA"
echo "    $REMOTE     $REMOTE_SHA"
[ "$REMOTE_SHA" = "$LOCAL_SHA" ] \
  || die "SHA-Drift. Erst pushen und auf $REMOTE auschecken (scripts/windows-build.sh macht beides)."

# --- 3. Bauen ------------------------------------------------------------------------------
say "3/4  Debug-APK bauen$([ "$FULL" -eq 1 ] && echo ' (voller Build zuerst, das dauert)')"
if [ "$FULL" -eq 1 ]; then
  ssh "$REMOTE" "cd $REMOTE_REPO && scripts/android-build.sh --clean" \
    || die "android-build.sh ist gescheitert (Fehler steht weiter oben)." 2
fi
# Die rustBuild-Tasks bleiben aus: der Debug-Variant nimmt die .so des vollen Builds aus jniLibs.
ssh "$REMOTE" "cd $REMOTE_REPO/src-tauri/gen/android \
  && export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64 ANDROID_HOME=\$HOME/workspace/tools/android-sdk \
  && ./gradlew :app:assembleUniversalDebug \
       -x :app:rustBuildUniversalDebug -x :app:rustBuildArm64Debug -x :app:rustBuildArmDebug \
       -x :app:rustBuildX86Debug -x :app:rustBuildX86_64Debug --quiet" \
  || die "gradle assembleUniversalDebug ist gescheitert." 2
ssh "$REMOTE" "ls -laL --time-style=long-iso $REMOTE_REPO/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/libklarvo_lib.so" \
  | sed 's/^/    Rust-Bibliothek: /'

# --- 4. Holen und installieren ---------------------------------------------------------------
say "4/4  Installieren (App-Daten bleiben erhalten)"
scp -q "$REMOTE:$REMOTE_APK" "$LOCAL_APK" || die "APK nicht vom $REMOTE geholt." 3
OUT="$("$ADB" -s "$SERIAL" install -r "$LOCAL_APK" 2>&1)" || true
echo "    $OUT" | tail -1
case "$OUT" in
  *Success*) ;;
  *INSTALL_FAILED_UPDATE_INCOMPATIBLE*)
    die "Signatur passt nicht — auf dem Handy liegt KEIN Debug-Build. Nicht deinstallieren ohne Andis Ja (Daten gehen verloren)." 3 ;;
  *INSTALL_FAILED_USER_RESTRICTED*)
    die "Das Handy hat die Installation abgelehnt — Bestaetigungs-Dialog am Geraet pruefen." 3 ;;
  *) die "adb install ist gescheitert." 3 ;;
esac
rm -f "$LOCAL_APK"

printf '\n\033[32m== FERTIG. %s auf %s.\033[0m\n' "${LOCAL_SHA:0:7}" "$SERIAL"
"$ADB" -s "$SERIAL" shell dumpsys package com.klarvo.voice | grep -m1 lastUpdateTime | sed 's/^ */   /'
