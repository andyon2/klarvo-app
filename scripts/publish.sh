#!/bin/bash
# Exportiert den Produktcode (ohne Agent-Daten) ins Public Repo.
#
# Usage: ./scripts/publish.sh [public-repo-pfad]
# Default: ~/voxlit-app

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"
SOURCE_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="${1:-$HOME/voxlit-app}"

# --- Dateien die NICHT ins Public Repo gehoeren ---
EXCLUDE_LIST=(
  # Agent-Infrastruktur
  "CLAUDE.md"
  "main-agent.md"
  "project-status.md"
  ".claude/"
  "scripts/"
  "knowledge/"
  "briefings/"
  "feedback/"
  "sources/"
  "dispatches/"
  "marketing/"
  # Build-Artefakte und Dependencies (via .gitignore ignoriert, aber rsync kennt das nicht)
  "node_modules/"
  "dist/"
  "dist-ssr/"
  "target/"
  "src-tauri/target/"
  "src-tauri/gen/"
  "android/.gradle/"
  "android/build/"
  "android/app/build/"
  ".tauri/"
  "models/"
  # Secrets und generierte Dateien
  ".dev-keys"
  ".env"
  "voxlit-debug.keystore"
  "social-preview.png"
  "*.apk"
  "*.aab"
  "*.bin"
  "*.log"
)

# --- Erstes Setup ---
if [ ! -d "$TARGET_DIR/.git" ]; then
  echo "Public Repo existiert noch nicht. Clone $TARGET_DIR..."
  git clone https://github.com/andyon2/voxlit-app.git "$TARGET_DIR"
fi

# --- Sync ---
echo "Synce Produktcode nach $TARGET_DIR..."

RSYNC_EXCLUDES=""
for item in "${EXCLUDE_LIST[@]}"; do
  RSYNC_EXCLUDES="$RSYNC_EXCLUDES --exclude=$item"
done

rsync -av --delete \
  $RSYNC_EXCLUDES \
  --exclude=".git/" \
  "$SOURCE_DIR/" "$TARGET_DIR/"

# --- Scrub license secret from public copy ---
# Replace the real HMAC secret with dummy values so voxlit-app compiles
# but cannot generate keys valid for official builds.
echo "Scrubbing license secret..."
LICENSE_FILE="$TARGET_DIR/src-tauri/src/license/mod.rs"
if [ -f "$LICENSE_FILE" ]; then
  sed -i 's|b"voxlit-license-v1"|b"public-dummy-v1xx"|' "$LICENSE_FILE"
  sed -i 's|b"-2025-open-core!"|b"-xxxx-not-secret"|' "$LICENSE_FILE"
  echo "  License secret replaced with dummy values."
else
  echo "  Warning: license/mod.rs not found -- skipping."
fi

# --- Public .gitignore sicherstellen ---
# Die bestehende .gitignore wird aus dem Source uebernommen.
# Agent-Verzeichnisse muessen nicht rein, weil sie gar nicht gesynct werden.

# --- Instanz-Marker-Check ---
echo ""
echo "=== Instanz-Marker-Check ==="
MARKERS_FOUND=0

MARKER_PATTERNS=(
  "main-agent"
  "project-status"
  "CLAUDE.md"
  "briefings/"
  "knowledge/"
  "/feedback/"
  "dispatches/"
  "voxlit-tech-lead"
  "rust-core\.md"
  "product-strategist"
)

for pattern in "${MARKER_PATTERNS[@]}"; do
  HITS=$(grep -rl "$pattern" "$TARGET_DIR" --include="*.md" --include="*.sh" --include="*.json" --include="*.ts" --include="*.tsx" --exclude-dir=".git" --exclude-dir="node_modules" --exclude-dir="target" 2>/dev/null || true)
  if [ -n "$HITS" ]; then
    echo "⚠  Marker '$pattern' gefunden in:"
    echo "$HITS" | sed 's|^|   |'
    MARKERS_FOUND=1
  fi
done

if [ "$MARKERS_FOUND" -eq 0 ]; then
  echo "✓ Keine Agent-Marker gefunden."
else
  echo ""
  echo "⚠  AGENT-DATEN IM PUBLIC REPO! Dateien bereinigen oder auf Exclude-Liste setzen."
  echo "   Abbruch empfohlen. Trotzdem committen? Dann manuell weiter."
  exit 1
fi

# --- Status zeigen ---
echo ""
echo "=== Sync abgeschlossen ==="
cd "$TARGET_DIR"
git add -A
echo ""
git status

echo ""
echo "Naechste Schritte:"
echo "  cd $TARGET_DIR"
echo "  git diff --cached --stat   # Aenderungen pruefen"
echo "  git commit -m '...'        # Committen"
echo "  git push                   # Pushen"
