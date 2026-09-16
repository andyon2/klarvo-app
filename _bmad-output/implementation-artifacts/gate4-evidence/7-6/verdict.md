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

## Part 2 — Windows release build: BLOCKED (not a code failure)

    scripts/windows-build.sh    →  exit 1

    == 1/5  Lokalen Baum prüfen
        conductor/story-7-6 = dc6e08b62db011bf94a06d58a831eac134ed7e3f
    == 2/5  Nach origin pushen
        * [new branch]      conductor/story-7-6 -> conductor/story-7-6
    == 3/5  Auf laptop auschecken
        ssh: connect to host 100.119.146.126 port 22: Connection timed out
    ABBRUCH: Checkout auf laptop fehlgeschlagen

**Cause, verified:** the laptop is powered off, not misconfigured. `tailscale status` reports both of
its nodes offline, last seen ~5 h ago:

    100.123.144.12   laptop-q0pka3ta-1   windows   offline, last seen 5h ago
    100.119.146.126  laptop-q0pka3ta     linux     active; relay "fra"; offline, last seen 5h ago

`ping 100.119.146.126` → 100 % packet loss.

Exit code 1 is the script's "precondition violated" arm. Step 2 succeeded, so the branch **is** on
`origin` and the laptop will check it out as soon as it is reachable. Nothing was mutated on any host
to get around this (R6).

## Part 3 — Andi's real check: NOT YET RUN

Blocked behind Part 2: there is no fresh Windows build to test against.

When the laptop is on, the conductor re-runs `scripts/windows-build.sh`, and Andi then does:

1. Open Settings and add a dictionary term, e.g. `Kubernetes`.
2. Set the cleanup style to **Chat**.
3. Dictate a sentence containing that term.
4. Confirm the term arrives written exactly as entered.

**Status of the story:** stays `review` in both status fields. GATE 4 is open.
