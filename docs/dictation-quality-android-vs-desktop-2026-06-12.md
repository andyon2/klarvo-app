# Dictation Quality: Android (local Whisper) vs Desktop (Groq cloud)

**Date:** 2026-06-12
**Author:** Claude (evidence run for the STT-hallucination correct-course)
**Status:** Evidence report — input to a `bmad-correct-course` decision on Epic 7 / STT architecture

> **⚠️ CORRECTION 2026-06-12 (added after the run below).** The premise that the **phone uses
> local whisper** is **FALSE**. Three independent sources agree the phone uses **Groq cloud**,
> identical to desktop: (1) phone `config.json` → `sttProvider: "groq"`, `whisperMode: false`;
> (2) code dispatch `if (sttProvider=="local") … else → Groq`; (3) runtime log → **46/46 STT
> runs `provider=groq`, 0 local**. **Both platforms run the same engine and model
> (`whisper-large-v3-turbo`).** There is no engine difference, so the "local hallucinates less"
> conclusion is retracted. Furthermore, the Android organic sample (0/48) is **statistically
> indistinguishable** from the desktop rate (P(0 hits | 1.5%, n=48) ≈ 0.48). See the corrected
> **Conclusion** and **Implications** sections. The raw marker counts below remain valid as
> data; only the engine-attribution interpretation was wrong.

---

## Question

Andy's lived experience: dictation on the **phone feels better** than on the **laptop**.
Phone uses **local `whisper.cpp`** (via the shared Rust core over JNI); desktop uses **Groq
cloud Whisper** (`response_format=json`). Is the perceived quality gap real, and is it the
**engine** (Groq vs local) or just **usage** (clip length, environment)?

## Data

- **Desktop corpus:** `~/AppData/Roaming/com.klarvo.voice/history.db` — 2988 verbatim rows
  (full), 310 in the comparison window 2026-06-01..06-10.
- **Android corpus:** pulled live via `adb exec-out run-as com.klarvo.voice cat history.db`
  (Tailscale, port 38301) — 61 rows, 2026-06-01..06-10. Single `device_id`.
- Both DBs share the schema incl. `raw_text` (pure STT output, **before** LLM cleanup) and
  `text` (after cleanup). **Online sync is off**, so each file is single-platform — clean
  source separation.

## Method

Hallucination is detected by **markers** (no ground-truth of what was actually said), applied
to **`raw_text` only** — this is style-independent and isolates the STT engine. Detectors,
defined before measuring:

1. **Stockphrase ghosts** — known Whisper training-data artifacts: `Groß- und
   Kl(inge|ingel|einschreibung) [, Satzzeichen und Interpunktion]`, `Untertitelung des ZDF`,
   `amara.org`, subtitle/credit lines, `Musik Musik…`, `[Musik]`, subscribe/thank-you sign-offs.
2. **Repetition loops** — a consecutively repeated 4–7 word sequence, or a duplicated final
   sentence (catches the "…zu schoko, Puste. …zu schoko, Puste." loop).
3. **Trailing ghosts** — a short phantom fragment appended after a sentence-final punctuation.

## Confounders (named, not hidden)

- **Cleanup style differs:** Android corpus is `polished` (61/61), desktop is `verbatim`
  (310/310). → Controlled by measuring **`raw_text`**, which is pre-cleanup.
- **Clip length differs sharply:** Android skews short (31/61 < 60 chars), desktop skews long
  (162/310 ≥ 500 chars). → Controlled by computing rates **per length bucket**.
- **Test contamination:** all 8 Android marker rows fall in one window, 2026-06-01
  11:26–12:13 — a deliberate hallucination-filter test session (Story 2-1 Android port,
  ZDF sentences fed on purpose). → **Excluded** from organic rates.
- **Residual, not eliminated:** local model version vs Groq large-v3 differ (this *is* the
  engine-as-deployed comparison, by design); Android also runs `SilencePreFilter.kt` before
  STT — so part of the gap may be **better silence trimming**, not the engine per se (this is
  good news: silence-trim is portable). Android long-clip sample is small (n=9) → the long
  bucket is **underpowered**.

## Results

### Headline (organic, raw_text)

| | Hallucination-marker rate (raw STT) |
|---|---|
| **Android / local whisper** (organic, n=48) | **0 / 48 = 0 %** |
| **Desktop / Groq** (same window, n=310) | **9 / 310 = 2.9 %** |
| Desktop / Groq (full verbatim, n=2988) | 46 / 2988 = 1.5 % |

### Length-controlled (the decisive cut)

| Clip length | Desktop/Groq (full) | Desktop/Groq (window) | Android/local (organic) |
|---|---|---|---|
| **< 60 chars** | **8.6 %** (13/152) | 16.1 % (5/31) | **0 % (0/19)** |
| < 200 | 0.5 % | 2.6 % | 0 % (0/11) |
| < 500 | 1.1 % | 0.0 % | 0 % (0/9) |
| ≥ 500 | 1.1 % | 1.9 % | 0 % (0/9) |

**The short-clip bucket is the clincher.** Short/silent clips are the textbook Whisper
hallucination worst-case — the confounder should *penalize* whichever engine sees more of
them. Groq is worst there (8.6%), local whisper is clean (0/19). So the gap is **not** a
length/usage artifact; the cloud engine is a genuine hallucination source on Andy's audio.

### Failure-mode profile (Desktop/Groq)

- **Short/silent clips → whole-clip ghost:** `Untertitelung des ZDF, 2020`, `Groß- und Klinge`.
- **Long clips → trailing ghost** (~1.1%): real dictation + appended `… Groß- und Klinge.` /
  `… und Klingel.` The existing blocklist filter **cannot** catch these: the ≤8-word gate
  opens for long text, so the filter never runs.
- **Cleanup amplification:** LLM cleanup rationalizes the recognizable ghost `Klinge` into the
  *convincing* full stockphrase `Kleinschreibung` (e.g. desktop id 2708, 2891, 2777) — turning
  detectable junk into fluent, undetectable junk.

## Conclusion (corrected)

**Both platforms use the same engine** (Groq cloud, `whisper-large-v3-turbo`). The original
"local hallucinates less" reading is **retracted** — there is no engine to attribute a
difference to.

On the data itself: the desktop hallucination markers are **real** (`Groß- und Klinge` ghosts,
`Untertitelung des ZDF`, the `Puste…Puste` loop), concentrated on **short/silent clips
(8.6%)** and as **trailing ghosts on long clips (~1.1%)**. But the Android "0/48" is **not**
evidence of better quality: at the desktop base rate of 1.5%, P(0 hits in 48) ≈ 0.48 — pure
small-sample noise. Even the short-bucket gap (0/19 vs 8.6%) is only weakly suggestive
(P(0)≈0.18).

**Most likely truth:** STT quality is roughly the same on both (same engine/model). The
"phone feels better" perception is best explained by **silence handling** (Android runs
`SilencePreFilter.kt` + its own silence-second settings before the identical Groq call),
**usage** (phone = short clips, laptop = long clips, and Groq's failure modes are
length-dependent), **mic/audio capture**, and perception — **not** the engine. This data
cannot cleanly separate those; a controlled A/B (same audio → both pipelines) would be needed
to claim any real phone-vs-laptop quality delta.

## Implications for the correct-course (corrected)

"Converge the engine" is **moot** — both are already Groq. The real, defensible levers:

1. **Guard logic (the actual bug):** consolidate the hallucination filter into the Rust core +
   JNI (delete the Kotlin twin), and harden it: blocklist the `Groß- und Kleinschreibung,
   Satzzeichen und Interpunktion` stockphrase family; switch Groq to `verbose_json` and drop
   segments by `no_speech_prob` / `compression_ratio` / `avg_logprob`; stop cleanup from
   inventing (`Klinge`→`Kleinschreibung`).
2. **The two-strands problem is REAL and STRENGTHENED:** there are **two separate STT request
   implementations** hitting the same Groq endpoint — Rust `GroqWhisper` and Kotlin
   `KlarvoApi.transcribe` — which can (and do) send **different parameters** (silence
   handling, `response_format`, prompt conditioning). That parameter drift is exactly the
   divergence to kill. One shared Rust STT path consumed by Android via JNI makes both
   platforms hit Groq **identically** and inherit the same guards by construction.
3. **Cheap portable win regardless:** unify the silence pre-filter so both platforms trim the
   same way before the Groq call — a likely contributor to the perceived difference.

The architectural decision for the correct-course is therefore **not** "which engine" but
**"one shared STT request + guard path in the Rust core vs. two divergent ones"** — which is
the original structural concern, now backed by the finding that the two paths already differ
in their parameters against an identical backend.

## Recurring cross-platform quality audit (anchoring proposal)

The instrument now exists: pull both `history.db` files, run the marker detectors on
`raw_text`, report per-platform / per-bucket rates and drift. Recommended anchor:

- Commit the analysis as a repo tool (e.g. `scripts/dictation-quality-audit.py` or an
  `xtask` subcommand) with the detector set under version control.
- Run **manually on a cadence** (e.g. monthly, or before each release) — it needs the phone
  reachable over adb/Tailscale, so a headless cloud cron won't work.
- Treat it as the **quality-layer sibling** of Epic 7's golden-vector parity net (7-7):
  the parity net pins config-contract equality; this pins output-quality drift between
  engines while we remain dual-engine.

### Reproduction

```
adb connect <phone-ip>:<port>
adb exec-out run-as com.klarvo.voice cat history.db > history-android.db
python3 scripts/dictation-quality-audit.py history-android.db <desktop-history.db>
```
