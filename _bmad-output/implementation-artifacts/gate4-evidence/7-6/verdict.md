# Story 7-6 — GATE 4 evidence

**Gate chosen by Andi at GATE 1 (2026-09-16):** machine proof **plus** a Windows release build
**plus** Andi's own real Chat-style dictation with a dictionary term.

## Part 1 — machine proof: GREEN (conductor-verified)

Run by the dev and fix workers on Linux, re-checked by the conductor in the committed range
`d9f83ef..dc6e08b`.

| Run | Result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | 707 passed, 0 failed (707 before: one test removed, one added) |
| `cargo test … --lib spec_m12` | 1 passed — the desktop column of the M12 fixture against the real prompt |
| `cargo test … --lib chat` | 3 passed |
| `cargo build … --lib` | Finished, 13 warnings (pre-existing) |
| RED proof, both directions | shown RED, then reverted (vector-only flip; code-only change) |
| `cargo clippy` | **blocked** — not installed on this host; not installed around (R6) |

**What this proves:** wiring, logic and structure of the desktop prompt text under Linux — the Chat
arm now interpolates the dictionary section, and the fixture vector follows without editing the 7-8 test.
**What it does not prove:** the Windows release build, the Kotlin column (read, not asserted), any real
model call, any device, or that a real dictation improves.

## Part 2 — Windows release build: GREEN (conductor-run, 2026-09-16)

    scripts/windows-build.sh    →  exit 0

    == 5/5  Frische prüfen
        Commit gebaut : 2026-09-16 11:41:30
        exe geschrieben: 2026-09-16 14:04:31
    == FERTIG. klarvo.exe enthält fdf0db4 (conductor/story-7-6).

    klarvo.exe : D:\apps\klarvo\src-tauri\target\release\klarvo.exe   (44 MB)
    bundles    : Klarvo_0.5.0_x64_en-US.msi · Klarvo_0.5.0_x64-setup.exe (rsign-signiert, verifiziert)
    installer  : D:\Dropbox\App Development\klarvo\releases\v0.5.0\

**Freshness proven:** the exe is younger than the commit it was built from, which is the script's own
exit-3 guard (cargo reusing a stale object). The build carries `fdf0db4`, the branch HEAD.

**What this proves:** the code compiles, links and bundles on Windows — a statement neither `cargo check`
nor a Linux test run delivers. **What it does not prove:** pixels, font rasterization, or that a dictation
behaves better. A green build is never a design or behaviour gate.

### First attempt, for the record (2026-09-16, earlier)

The first run exited 1 in step 3/5: the laptop was powered off (`tailscale status` reported both nodes
offline ~5 h; ping 100 % loss). Not a code failure, and nothing was mutated on any host to get around it
(R6). The push in step 2 had already succeeded, so the re-run only had to check out and build.

### Branch note (honest record)

Two commits on this branch are **not** story 7-6 work: `3a306e8` (contract pointer to the skill-delivery
fix) and `fdf0db4` (closing the `bash`-wrapper deny gap recorded on 2026-09-14). Both are docs/config, no
product code. They were committed here because the conductor was standing on this branch; they belong on
`v1-ship` and land there with the merge. Named rather than hidden.

## Part 3 — Andi's real check: GREEN (2026-09-16)

Run by Andi on the real Windows build (`fdf0db4`):

1. Dictionary term entered in Settings.
2. Cleanup style set to **Chat**.
3. Sentence dictated containing that term.
4. **Result: the term arrives unchanged — in Chat style too.** Andi's words: "gruen, Begriff kommt
   unveraendert an, auch in Chat".

This is the statement no machine gate could make. The Linux tests proved the Chat prompt *carries* the
dictionary sentence; this proves the behaviour a user actually sees. M12 is closed on both platforms.

**GATE 4: GREEN, both parts. Story → `done` in both status fields.**
