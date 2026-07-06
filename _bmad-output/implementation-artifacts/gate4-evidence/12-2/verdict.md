# GATE-4 evidence — Story 12-2 (Audio-retry history)

Date: 2026-07-06
Conductor run: autonomous (bmad-story-conductor), baseRef 46a473b, HEAD after close-out.

## Surface posture (why the structural oracle does not gate here)
The pending-entry surface is the MAIN-WINDOW dictation-history list (`is_note=0`),
NOT an application-overlay window. The project conductor contract's structural
visual-oracle (`dumpsys window windows`, overlay count/size/gravity) applies only to
overlay windows; `visual_oracle.pixel = false`. A history *list* is neither an overlay
nor a pixel-authoritative surface on the emulator. Therefore the emulator has nothing
decision-grade to assert about this surface, and no structural FAILED-smoke is possible
for it. This matches the story's own Testing Rules and the invoking directive (E9/E11).

## Machine-verifiable layer — SELF-VERIFIED GREEN
- G-A logic guards (Rust unit/integration, inline #[cfg(test)]):
    `cargo test --lib` => test result: ok. 654 passed; 0 failed; 0 ignored.
    Covers: additive migration backward-compat (pre-existing row -> status='done'/audio_path=NULL);
    pending-entry creation on terminal STT failure (deliver_outcome, audio preserved);
    promote_pending_to_done -> done + synced=0;
    discard -> row+WAV gone; discard tolerates already-missing WAV; discard rejects non-pending;
    sync push EXCLUDES pending rows (test_read_unsynced_excludes_pending_status);
    happy-path parity (add_entry defaults done/NULL, no WAV).
- Cross-platform compile:
    tsc --noEmit: clean; npm run build: OK (vite build).
    scripts/android-build.sh: BUILD OK — apksigner-verified signed release APK
      (Klarvo-v0.5.0-20260706-*.apk). Rust arm64 .so + Kotlin/Gradle both compiled.

## Residual for the HUMAN (Andi's batched real-machine gate) — NOT a proxy pass
- Windows: the pending-entry rendering (amber card, placeholder + reason + timestamp,
  the two actions "Erneut verarbeiten"/"Verwerfen", busy/disabled state, inline error)
  and the real on-outage behaviour (terminal STT failure -> pending entry appears;
  re-process success -> promotes + WAV deleted; discard -> gone). Real-machine visual +
  behavioural verdict only.
- Android: the dictation-history UI rendering of the pending entry and the reprocess/discard
  IPC reachability from the shared React surface via TauriActivity are the real-device
  residual (per story Dev Notes; the machine-verifiable Android deliverable here is the
  Kotlin WAV-persistence + pending-row logic + the sync status-filter parity, all compiled).

## Verdict
Machine layer GREEN and self-verified. Visual/behavioural layer is a genuine, named human
residual by the contract's own posture (overlay-only structural oracle, pixel=false).
No unattended FAILED smoke is possible for this surface. Proceed to close-out; carry the
residual to Andi.
