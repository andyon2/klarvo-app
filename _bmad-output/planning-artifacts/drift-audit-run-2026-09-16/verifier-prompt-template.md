# Drift-audit VERIFIER — subsystem: {{SUBSYSTEM}} — Klarvo, frozen tree `e9543fe` (v1-ship), 2026-09-16

Two independent discovery readers (A = Fable 5.1, B = Opus 5) each produced a list of claimed
Desktop(Rust) ↔ Android(Kotlin) divergences. You verify the claims that fall into YOUR subsystem
against the real code, claim by claim, and then do an independent recall sweep of the same subsystem.

Repo: /home/andyon2/workspace/products/klarvo — READ-ONLY. No edits, no state-changing git, no builds.
Desktop = `src-tauri/src/`; Android = `android/kotlin-src/com/klarvo/voice/`; shared React UI =
`src/components/` (runs on BOTH platforms; `isDesktop`/`isMobile` from `src/platform.ts` gate controls).

## Your subsystem and its files
{{FILES}}

## Definition of a divergence (ADR-0016 scope)
Same config + same input → different observable behavior across platforms, with no error shown.
Classes: core-output determinism · config-contract integrity (a key the user can SET on that platform's
React UI but the runtime never READS, or reads with other default/unit) · degrade/failure-path outcome ·
persistence & sync semantics · guards (license, banking, sanitization, hallucination, prompt guards).
Not a divergence: platform genuinely cannot do it AND no control pretends it can; pure look & feel.

Severity: CRITICAL data integrity/security/license/silent loss · HIGH core output differs at defaults, or a
set value silently dies on one side · MEDIUM only when tuned / edge case · LOW cosmetic or wire-level.

## Claims to verify (union of A and B for this subsystem; duplicates already grouped where obvious)
{{CLAIMS}}

## What to do
1. For EVERY claim, open both cited locations (and search around them — line numbers may be off by a
   few). Verdict ∈ **CONFIRMED** · **REFUTED** (say exactly why, with file:line) · **PARTIAL** (write the
   corrected statement) · **DUPLICATE-OF <id>**. Re-assess severity yourself; do not inherit it.
   A claim without evidence on both sides is not confirmed — go find the evidence or refute.
2. Where a claim says "Kotlin never reads key X", grep the whole Kotlin tree for X (and its snake/camel
   variants) before agreeing. Where a claim says a React control "has no platform gate", read the JSX
   ancestors of that control (the gate is often 20–60 lines above).
3. **Recall sweep:** after the claims, read your subsystem's twin code side by side once more and list
   divergences NEITHER reader reported. Same evidence bar. Tag them `V-{{TAG}}-1`, `V-{{TAG}}-2`, …
4. Note any claim that is really outside your subsystem — do not verify it, just say so.

## Output — write ONE markdown file to: {{OUT}}
Sections:
### 1. Verdicts
`| claim id(s) | verdict | severity (yours) | corrected statement (one sentence, user-visible effect) | Desktop file:line | Android file:line | React gate |`
### 2. Recall sweep (new rows)
Same columns, ids `V-{{TAG}}-n`, plus a confidence column.
### 3. Out-of-subsystem claims (not verified)
### 4. Reader quality note
Two or three sentences: which reader's claims in this subsystem were precise, which were sloppy (wrong
lines, unsupported), any systematic error pattern.

Be terse. Every cell that names code carries file:line.
