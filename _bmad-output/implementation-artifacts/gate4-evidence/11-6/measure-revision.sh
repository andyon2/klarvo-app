#!/usr/bin/env bash
# Misst den gerenderten Zeilenabstand aller drei Stufen nach der GATE-4-Revision.
#
# Vorhersage VOR der Messung (falsifizierbar): der Multiplikator in
# ListeningPanelView.kt ist desktop_wert / 1.3285, und setLineSpacing multipliziert
# den natuerlichen Zeilenkasten (gemessen 1.3285 x Schriftgroesse). Beides hebt sich
# auf, also MUSS der gerenderte Abstand exakt desktop_wert x Schriftgroesse sein:
#   Schrift "small" = 11sp @ density 440, font_scale 1.0 = 30.25 px
#   Kompakt 1.350 x 30.25 = 40.84 px
#   Normal  1.625 x 30.25 = 49.16 px
#   Locker  1.925 x 30.25 = 58.23 px
#
# NICHT `monkey` zum Starten benutzen: `monkey -p ... 1` injiziert nach dem Start
# einen ZUFAELLIGEN Tap und verlaesst damit den Bildschirm, den man messen wollte.
set -euo pipefail

ADB=/home/andyon2/workspace/tools/android-sdk/platform-tools/adb
DEV=100.112.41.70:5555
SP="$(cd "$(dirname "$0")" && pwd)"
PKG=com.klarvo.voice
TEXT="Ueber groessere Hoefe fliegen praechtige Voegel; jeder Uebergang zeigt Punkte und Unterlaengen fuer die Messung der Zeilenabstaende im Panel."

# Tippt einen Knopf anhand seines Textes -- Koordinaten werden gelesen, nie geraten.
tap_by_text() {
    local needle="$1"
    $ADB -s $DEV shell uiautomator dump /sdcard/ui.xml >/dev/null 2>&1 || return 1
    local xy
    xy=$($ADB -s $DEV shell cat /sdcard/ui.xml 2>/dev/null | python3 -c "
import re, sys
xml = sys.stdin.read()
needle = '''$needle'''.upper()
for m in re.finditer(r'text=\"([^\"]*)\"[^>]*bounds=\"\[(\d+),(\d+)\]\[(\d+),(\d+)\]\"', xml):
    if needle in m.group(1).upper():
        x1, y1, x2, y2 = map(int, m.groups()[1:])
        print((x1 + x2) // 2, (y1 + y2) // 2)
        break
")
    [ -n "$xy" ] || return 1
    $ADB -s $DEV shell input tap $xy
    return 0
}

for STUFE in small medium large; do
    echo "── Stufe: $STUFE ──"
    $ADB -s $DEV shell am force-stop $PKG
    python3 - "$STUFE" <<PY
import json, sys
c = json.load(open("$SP/config-backup.json"))
c["previewLineSpacing"] = sys.argv[1]
json.dump(c, open("$SP/config-live.json", "w"), ensure_ascii=False)
PY
    $ADB -s $DEV shell "run-as $PKG sh -c 'cat > /data/data/$PKG/config.json'" < "$SP/config-live.json"
    $ADB -s $DEV shell am start -n $PKG/.MainActivity >/dev/null 2>&1
    sleep 4
    if tap_by_text "SKIP FOR NOW"; then echo "  Onboarding-Dialog weggetippt"; sleep 2; fi
    # Der Harness erwartet den State KLEIN geschrieben ('recording'), und das ganze
    # Remote-Kommando muss gequotet sein -- sonst zerlegt die Geraete-Shell den
    # Transkript-Text in Argumente und '-p' erwischt ein Wort daraus als Paketnamen.
    $ADB -s $DEV shell "am broadcast -a $PKG.DEBUG_SET_STATE --es state recording --ef rms 0.8 --es transcript '$TEXT' -p $PKG" 2>&1 | grep -io "result=[0-9-]*" || true
    sleep 3
    $ADB -s $DEV exec-out screencap -p > "$SP/rev-$STUFE.png"
    echo "  -> rev-$STUFE.png  sha=$(sha256sum "$SP/rev-$STUFE.png" | cut -c1-12)"
done
echo "Fertig."
