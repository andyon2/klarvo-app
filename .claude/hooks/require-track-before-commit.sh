#!/bin/bash
# Blockiert git commit wenn /track nicht vorher lief.
# Prueft ob .git/commit-via-skill Marker existiert.
# Marker wird von /commit gesetzt, Hook loescht ihn nach Pruefung.

MARKER="$CLAUDE_PROJECT_DIR/.git/commit-via-skill"

if [[ -f "$MARKER" ]]; then
  rm -f "$MARKER"
  cat <<'EOF'
{"decision": "allow"}
EOF
else
  cat <<'EOF'
{"decision": "block", "reason": "Commit blockiert: Nutze /commit statt manuellem git commit. /commit stellt sicher dass /track vorher laeuft."}
EOF
fi
