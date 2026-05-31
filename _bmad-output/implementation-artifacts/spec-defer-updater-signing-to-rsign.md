---
title: 'Defer updater signing to rsign (createUpdaterArtifacts: false)'
type: 'chore'
created: '2026-05-31'
status: 'done'
baseline_commit: 'ae4068f13385ac36a7a02cfe97399a2b2b71a36f'
context: ['{project-root}/src-tauri/tauri.conf.json']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `tauri.conf.json` sets `bundle.createUpdaterArtifacts: true` alongside an updater `pubkey`. In Tauri v2 that combination forces Tauri to sign the updater artifacts at *build* time and hard-requires `TAURI_SIGNING_PRIVATE_KEY` — so the Windows build aborts ("A public key has been found, but no private key", or "incorrect updater private key password" when a wrong one is present). This contradicts the project's design, which defers signing to rsign (`sign-installer.sh`) *after* the build. The contradiction silently broke the build, which in turn caused stale binaries to be smoke-tested (Story 1.2 corrupt-config backup appeared missing because the running .exe predated the fix).

**Approach:** Set `bundle.createUpdaterArtifacts` to `false` so Tauri skips build-time updater signing and no private key is required. Keep the updater `pubkey`, `endpoints`, and the NSIS bundle untouched; rsign continues to produce the matching `.sig` post-build.

## Boundaries & Constraints

**Always:** Keep `plugins.updater.pubkey` and `plugins.updater.endpoints` byte-for-byte unchanged (runtime updater verifies against this key). Keep `bundle.targets` and the `bundle.windows.nsis` block unchanged (the NSIS installer must still build). Limit the diff to the single `createUpdaterArtifacts` boolean.

**Ask First:** Re-enabling build-time signing, removing/altering the updater plugin, pubkey, or endpoints, or adding a CI/release workflow.

**Never:** Touch the Story 1.2 work (`config/mod.rs`, `lib.rs` backup-on-corrupt) — separate, still in `review`. Modify the `.env` signing keys (already commented out). Auto-generate, sign, or commit a `latest.json`. Commit or push (human-controlled).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Build, no signing key | `createUpdaterArtifacts:false`, `TAURI_SIGNING_PRIVATE_KEY` unset | `tauri build` succeeds; produces `target/release/klarvo.exe` + NSIS `Klarvo_<ver>_x64-setup.exe`; Tauri emits no `.sig`/`latest.json` | N/A — no key needed |
| Post-build signing | NSIS installer present | `sign-installer.sh` (rsign) produces a pubkey-matching `.sig` | unchanged (script aborts if installer/key missing) |

</frozen-after-approval>

## Code Map

- `src-tauri/tauri.conf.json` — ONLY file changed; `bundle.createUpdaterArtifacts` flag (line 31). `pubkey`/`endpoints` at the `plugins.updater` block must stay intact.
- `src-tauri/src/lib.rs:698` — `tauri_plugin_updater` registration; unchanged (context: the runtime updater stays active).
- `teams/klarvo/scripts/sign-installer.sh` — out-of-repo post-build rsign signer; unchanged (context: this is the deferred signer the flag change defers to).
- `teams/klarvo/scripts/sync-and-build.ps1` — out-of-repo build script; carries a now-redundant `--config createUpdaterArtifacts:false` override after this change. Note only; do not edit from this repo.

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/tauri.conf.json` — change `bundle.createUpdaterArtifacts` from `true` to `false` — removes the build-time signing requirement that contradicts the deferred-rsign design.

**Acceptance Criteria:**
- Given `createUpdaterArtifacts:false` and no `TAURI_SIGNING_PRIVATE_KEY` in the environment, when `tauri build` runs, then it completes without the "no private key" / password errors and writes `target/release/klarvo.exe`.
- Given the change, when the bundler runs, then the NSIS installer is still produced (targets/NSIS config unchanged).
- Given the change, when inspecting `plugins.updater`, then `pubkey` and `endpoints` are unchanged, so the runtime updater verifies against the same key.
- Given the file is parsed, when validated as JSON, then it is well-formed (no trailing-comma / syntax breakage from the edit).

## Spec Change Log

## Design Notes

`createUpdaterArtifacts:true` makes Tauri generate the updater manifest (`latest.json`) and sign it at build time. Setting it `false` skips that generation. This regresses no working flow: there is **no** `.github/workflows/` generating `latest.json`, and the build that would have generated it has been failing anyway. Early Access is withdrawn → no active auto-update consumers. **Deferred follow-up (not in scope):** before any future auto-update release, `latest.json` must be assembled manually with the rsign-produced signature (or build-time signing re-enabled with a valid key). This belongs in the release runbook, not this change.

## Verification

**Commands:**
- `python3 -c "import json;c=json.load(open('src-tauri/tauri.conf.json'));assert c['bundle']['createUpdaterArtifacts'] is False;assert c['plugins']['updater']['pubkey'] and c['plugins']['updater']['endpoints'];print('ok')"` — expected: prints `ok` (flag is false, pubkey + endpoints intact, JSON well-formed).

**Manual checks (authoritative — runs on Windows, by Andi):**
- Re-run `sync-and-build.ps1`: build completes with "Done! Fresh build verified.", **no** signing error, and `target/release/klarvo.exe` is freshly written (newer timestamp).

## Suggested Review Order

- The one-line flag flip that removes Tauri's build-time updater-signing requirement; `pubkey`/`endpoints`/NSIS deliberately untouched.
  [`tauri.conf.json:31`](../../src-tauri/tauri.conf.json#L31)
