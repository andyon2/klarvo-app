# GATE-4 verdict — Story 11-2 (Android Live-Preview Port)

Date: 2026-07-01 · Conductor: bmad-story-conductor (interactive) · Range reviewed: `32a3770..beb005c`

## Proxy surface
Headless WSL emulator `emulator-5554` (AVD `klarvo-emu`, stock Android — NO HyperOS skin),
`BMAD_CONDUCTOR=1`, install `--abi arm64-v8a` (arm64 Rust `.so` via x86_64 native-bridge).
Post-fix APK built fresh (`assembleUniversalDebug`, 104 MB), installed, service woken,
states driven via `DEBUG_SET_STATE` harness.

## Structural assertion (the machine-checkable GATE) — PASS
`dumpsys window windows`, recording state — two `APPLICATION_OVERLAY` windows:
- **Panel** (Window #7): `Requested w=1080 h=525`, `gr=BOTTOM`, `fmt=TRANSLUCENT` — the
  ~1080×525 bottom panel occupying the keyboard footprint (design decision #2). ✅
- **Cluster** (Window #8): `435×178`, `gr=TOP START CENTER` — the ✗·waveform·➤ cluster at
  the bubble, separate from the preview (design decision #1). ✅
- Count = 2, no doubled/missing windows, no structural regression vs. Epic 9 overlay. ✅

Evidence: `structure-{idle,recording,transcribing}.txt`, `ist-{idle,recording,transcribing}.png`,
`smoke-run.log`.

## Runtime + behavioral (as far as the proxy can go) — PASS
- Post-fix APK builds + installs + service launches + overlay renders. ✅
- Panel renders the accumulated transcript text (harness-injected via the same `rawTranscript`
  field the real preview-append writes to); text is the pure text surface, no waveform/buttons
  in the panel. ✅ (`ist-recording.png`)
- Ran with `livePreviewEnabled` = default OFF: panel shows its STOCK look (opaque, monospace,
  no teal/rounded/translucent) → **F2 fix confirmed** (appearance not applied when disabled;
  AC-4 "byte-identical when off"). ✅

## NOT observable on this proxy — Andi's real-device gate (Xiaomi/HyperOS)
1. **Live preview on a REAL speech pause** — the delta-flush → Groq → append loop with real
   mic + real pauses (the proxy injected text via the debug harness, not the real STT path).
2. **Appearance with preview ON** — the styled dark card (teal border/colors/font) + that the
   color/font settings take effect.
3. **F2 on/off look on HyperOS specifically** — HyperOS force-dims `FLAG_NOT_TOUCHABLE` overlays
   to alpha 0.8 (stock emulator can't reproduce); confirm preview-OFF panel is byte-identical to
   today's build.
4. **Settings render on Android** — the Appearance category appears in mobile settings
   (toggle + pause + color/font), with the **width preset and Bg-blur controls hidden**.

## Verdict
Mechanical/structural layer **GREEN**. Visual + real-STT verdict is Andi's batched real-device
gate (a visual story is never "done" on emulator-green — Epic-9 lesson). Story stays at `review`
until Andi's device confirmation; then close-out flips both status fields to `done`.
