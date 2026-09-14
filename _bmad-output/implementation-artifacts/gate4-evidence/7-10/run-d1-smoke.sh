#!/usr/bin/env bash
# Story 7-10 — review round 1, directive D1: runs the status-line proxy smoke.
#
# The preview mocks cannot produce a hotkey-driven `done + warning` event
# (`onStateChanged` returns `mockListen()` and never fires), so this applies the
# documented THROWAWAY EDIT to src/tauri-commands.ts, measures, and reverts it.
# `git status` must be clean for that file afterwards — the script checks.
#
# Usage: bash _bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$ROOT/src/tauri-commands.ts"

cd "$ROOT"

if ! git diff --quiet -- src/tauri-commands.ts; then
  echo "REFUSING: src/tauri-commands.ts already has uncommitted changes." >&2
  echo "The revert below would destroy them. Commit or stash first." >&2
  exit 1
fi

BACKUP="$(mktemp)"
cp "$TARGET" "$BACKUP"

cleanup() {
  cp "$BACKUP" "$TARGET"
  rm -f "$BACKUP"
  if git diff --quiet -- src/tauri-commands.ts; then
    echo "[run-d1-smoke] throwaway edit reverted; src/tauri-commands.ts clean."
  else
    echo "[run-d1-smoke] WARNING: src/tauri-commands.ts is NOT clean after revert." >&2
  fi
  if [[ -n "${PREVIEW_PID:-}" ]]; then kill "$PREVIEW_PID" 2>/dev/null || true; fi
}
trap cleanup EXIT

# --- throwaway edit: park the listener on window so the harness can emit ---
python3 - "$TARGET" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = """export function onStateChanged(
  _callback: (payload: StateChangedPayload) => void
): Promise<() => void> {
  if (isPreviewMode) return mockListen();"""
new = """export function onStateChanged(
  _callback: (payload: StateChangedPayload) => void
): Promise<() => void> {
  if (isPreviewMode) {
    // THROWAWAY (story 7-10 D1 smoke) — reverted by run-d1-smoke.sh.
    (window as unknown as Record<string, unknown>).__klarvoPreviewEmit = _callback;
    return mockListen();
  }"""
assert old in s, "onStateChanged preview branch not found — the patch is stale"
open(p, "w").write(s.replace(old, new, 1))
PY
echo "[run-d1-smoke] throwaway edit applied."

npm run preview >"$HERE/preview-server.log" 2>&1 &
PREVIEW_PID=$!

for _ in $(seq 1 60); do
  if curl -sf -o /dev/null http://localhost:1422/; then break; fi
  sleep 0.5
done

node "$HERE/d1-status-line-smoke.mjs"
