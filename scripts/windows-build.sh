#!/usr/bin/env bash
# windows-build.sh — stößt den Windows-Release-Build auf dem Laptop an, von powerhouse aus.
#
# WARUM ES DIESES SKRIPT GIBT
# Der Windows-Build lebt auf dem Laptop: `sync-and-build.ps1` robocopiert aus dem
# WSL-Checkout DES LAPTOPS nach D:\apps\klarvo und baut dort. powerhouse hat kein
# Windows. Der Code muss also erst über `origin` zum Laptop wandern. Diese drei
# Schritte — pushen, dort auschecken, bauen — hat bisher jeder Agent von Hand
# improvisiert, und Schritt 1 wurde regelmäßig vergessen (Agenten committen, aber
# pushen laut Werkzeug-Default nur auf Ansage). Dann baut der Laptop still den
# ALTEN Stand. Dieses Skript macht die Kette unteilbar und beweist die Frische.
#
# AUFRUF
#   scripts/windows-build.sh                # baut den aktuellen Branch
#   scripts/windows-build.sh feat/8-6-xyz   # baut einen bestimmten Branch
#   scripts/windows-build.sh --clean        # erzwingt Neuübersetzung (cargo clean -p klarvo)
#   scripts/windows-build.sh --skip-npm     # überspringt `npm ci` (schneller, wenn JS unverändert)
#
# DAUER: mehrere Minuten bis eine knappe halbe Stunde. Agenten starten das im
# Hintergrund, nicht im Vordergrund.
#
# EXIT-CODES
#   0  Build fertig UND nachweislich frisch (exe jünger als der gebaute Commit)
#   1  Vorbedingung verletzt (dreckiger Baum, Laptop nicht erreichbar, SHA-Drift)
#   2  Der Windows-Build selbst ist gescheitert — Compiler-Fehler stehen im Log
#   3  Build meldete Erfolg, aber die exe ist älter als der Commit (stale binary)

set -euo pipefail
cd "$(dirname "$0")/.."

# --- Konstanten -------------------------------------------------------------
# Der SSH-Alias landet direkt in der WSL-Ubuntu des Laptops (kein `wsl`-Wrapper).
REMOTE="${KLARVO_BUILD_HOST:-laptop}"
REMOTE_REPO="${KLARVO_BUILD_REPO:-\$HOME/workspace/products/klarvo}"
# powershell.exe steht in einer nicht-interaktiven SSH-Sitzung NICHT im PATH.
# Der absolute Pfad ist der einzige verlässliche Weg.
PS_EXE='/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe'
PS_SCRIPT='\\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\sync-and-build.ps1'
WIN_EXE='D:\apps\klarvo\src-tauri\target\release\klarvo.exe'

# --- Argumente --------------------------------------------------------------
REF=""
PS_ARGS=""
for arg in "$@"; do
  case "$arg" in
    --clean)    PS_ARGS="$PS_ARGS -Clean" ;;
    --skip-npm) PS_ARGS="$PS_ARGS -SkipNpm" ;;
    -*)         echo "Unbekannte Option: $arg" >&2; exit 1 ;;
    *)          REF="$arg" ;;
  esac
done
[ -n "$REF" ] || REF="$(git rev-parse --abbrev-ref HEAD)"

say() { printf '\n\033[36m== %s\033[0m\n' "$*"; }
die() { printf '\n\033[31mABBRUCH: %s\033[0m\n' "$*" >&2; exit "${2:-1}"; }

# --- 1. Vorbedingung: der lokale Baum muss den zu bauenden Stand tragen ------
# Ungespeicherte Änderungen an versionierten Dateien kommen nicht in den Commit,
# also auch nicht in den Build. Ohne diesen Riegel testet Andi eine exe, die
# seine letzte Änderung nicht enthält, und niemand merkt es.
say "1/5  Lokalen Baum prüfen"
if ! git diff --quiet || ! git diff --cached --quiet; then
  git status --short | grep -v '^??' || true
  die "versionierte Dateien sind geändert und nicht committet. Committe sie, sonst baut der Laptop einen Stand OHNE diese Änderungen."
fi
LOCAL_SHA="$(git rev-parse "$REF")"
COMMIT_EPOCH="$(git log -1 --format=%ct "$LOCAL_SHA")"
echo "    $REF = $LOCAL_SHA"

# --- 2. Nach origin pushen ---------------------------------------------------
say "2/5  Nach origin pushen"
git push origin "$REF:$REF"

# --- 3. Auf dem Laptop auschecken -------------------------------------------
say "3/5  Auf $REMOTE auschecken"
ssh "$REMOTE" "
  set -e
  cd $REMOTE_REPO
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo 'DIRTY: der Checkout auf dem Laptop hat geänderte versionierte Dateien.' >&2
    git status --short | grep -v '^??' >&2 || true
    exit 1
  fi
  git fetch origin --prune --quiet
  # \`checkout -B\` setzt den lokalen Branch hart auf origin. Der Laptop ist ein
  # Build-Spiegel, das ist gewollt — aber es würde dort entstandene Commits
  # verwerfen. Also erst prüfen, ob welche existieren.
  cur=\$(git rev-parse --abbrev-ref HEAD)
  if git rev-parse --verify --quiet \"origin/\$cur\" >/dev/null; then
    ahead=\$(git rev-list --count \"origin/\$cur..HEAD\")
    if [ \"\$ahead\" -gt 0 ]; then
      echo \"UNGEPUSHT: \$cur hat \$ahead Commit(s), die origin nicht kennt. Erst dort pushen.\" >&2
      exit 1
    fi
  fi
  git checkout --quiet -B '$REF' 'origin/$REF'
  echo \"    HEAD = \$(git rev-parse HEAD)\"
" || die "Checkout auf $REMOTE fehlgeschlagen (siehe Meldung oben)."

REMOTE_SHA="$(ssh "$REMOTE" "cd $REMOTE_REPO && git rev-parse HEAD")"
[ "$REMOTE_SHA" = "$LOCAL_SHA" ] || \
  die "SHA-Drift: powerhouse $LOCAL_SHA, $REMOTE $REMOTE_SHA. Der Laptop würde etwas anderes bauen."

# --- 4. Bauen ----------------------------------------------------------------
say "4/5  Windows-Build starten (das dauert)"
set +e
ssh "$REMOTE" "cd /mnt/c && '$PS_EXE' -NoProfile -ExecutionPolicy Bypass -File '$PS_SCRIPT'$PS_ARGS"
BUILD_RC=$?
set -e
[ "$BUILD_RC" -eq 0 ] || die "der Windows-Build ist gescheitert (Exit $BUILD_RC). Der Compiler-Fehler steht weiter oben. klarvo.exe wurde NICHT ersetzt." 2

# --- 5. Frische beweisen -----------------------------------------------------
# `sync-and-build.ps1` warnt bei unveränderter Zeitmarke nur. Hier wird daraus ein
# harter Riegel: die exe muss jünger sein als der Commit, den sie enthalten soll.
say "5/5  Frische prüfen"
EXE_EPOCH="$(ssh "$REMOTE" "cd /mnt/c && '$PS_EXE' -NoProfile -Command \"[int](Get-Item '$WIN_EXE').LastWriteTimeUtc.Subtract([datetime]'1970-01-01').TotalSeconds\"" | tr -d '\r')"
echo "    Commit gebaut : $(date -d "@$COMMIT_EPOCH" '+%Y-%m-%d %H:%M:%S')"
echo "    exe geschrieben: $(date -d "@$EXE_EPOCH" '+%Y-%m-%d %H:%M:%S')"
[ "$EXE_EPOCH" -ge "$COMMIT_EPOCH" ] || \
  die "klarvo.exe ist ÄLTER als der gebaute Commit — cargo hat ein altes Objekt wiederverwendet. Noch einmal mit --clean laufen lassen." 3

printf '\n\033[32m== FERTIG. klarvo.exe enthält %s (%s).\033[0m\n' "${LOCAL_SHA:0:7}" "$REF"
echo "   Gegenprobe in der App: Einstellungen -> Über zeigt Build-Hash und Zeitmarke."
