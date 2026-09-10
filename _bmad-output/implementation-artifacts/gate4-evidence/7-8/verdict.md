# GATE-4 verdict — Story 7-8 (parity-net close-out + twin hygiene), 2026-09-10

**Verdict: GREEN on the unattended proxy (headless AVD `klarvo-emu`, laptop, `emulator-5554`).**
Story type: logic/hygiene, not surface — no pixel layer to judge. Run by the story-conductor, not by a worker.

## Self-verification (what I ran, objective results)

| Round | Commit | JVM gate (all result XMLs) | Install | Rust-side JNI proof | Log |
|---|---|---|---|---|---|
| r1 | `00e771d` | 168 / 0 fail / 0 err (22 suites) | `primaryCpuAbi=arm64-v8a`, fresh (`lastUpdateTime` same minute) | `klarvo_lib` `[setup]` lines in the app process, 0 `UnsatisfiedLinkError` | `smoke-r1-00e771d.log`, `structure-install-r1.txt` |
| r2 | `27de205` | 168 / 0 / 0 | arm64 re-install ok (APK reused: no production Kotlin changed) | not re-driven | `smoke-r2-27de205.log` |
| r3 | `85aa0ee` | 168 / 0 / 0 | arm64 re-install ok | not re-driven | `smoke-r3-85aa0ee.log` |
| r4 | `5128432` (final) | 168 / 0 / 0, per suite in `jvm-suites-5128432.md` | arm64, fresh install after emulator reboot | `klarvo_lib` `[setup] Loaded config …` in process `4183`, 0 `UnsatisfiedLinkError` | `smoke-r4-5128432.log` |

Structural assertion (AC6b, the trap this story closes): on an `emulator-*` serial the smoke now installs with
`--abi arm64-v8a -r -g`; the installed package reports `primaryCpuAbi=arm64-v8a`, native libs load from
`base.apk!/lib/arm64-v8a` (`extractNativeLibs=false`, see nativeloader line in `structure-install-r1.txt`), and
the Rust core runs inside the app process. Before this story the x86_64 split was installed and every JNI
call died with `UnsatisfiedLinkError` while the smoke stayed green.

M9 endpoint probe from powerhouse (no key): `POST https://api.deepseek.com/v1/chat/completions` → 401,
`POST https://api.deepseek.com/chat/completions` → 401. Both paths are served; M9 is parity with Desktop
(`llm/mod.rs:719`), not a repair.

Note on the script banner: `android-smoke.sh` prints "24 Tests" (one result XML); the totals above are
aggregated over all XMLs — see `NOTE-jvm-test-counts.md`.

## What the proxy does NOT decide

- No dictation was performed: no microphone on the AVD, no LLM key seeded. The DeepSeek request path
  was never exercised end-to-end on Android.
- Real-device (HyperOS) behaviour.

## Residual for Andi (real-target gate, batched with the next fresh APK)

One dictation on the Xiaomi with DeepSeek cleanup returns cleaned text. That is the only observation
this run could not synthesize. Close it with a one-line docs commit on the story's Change Log, as 7-2 did.
