# ADR-0003: JNI-Spike Outcome — Dual-Surface bestätigt, separater Listener-Kontrakt

**Status:** Accepted (siehe Amendment 2 — 2026-05-01)
**Date:** 2026-04-18

## Context

Architecture-Doc Step 4 §3 (Zeile 255) und `project_jni_dual_surface.md` setzen eine Dual-Surface-Bridge voraus:
- Control-Plane: uniffi für Request/Response
- Data-Plane: raw `jni`-Crate für Kontinuierliche Events (uniffi hat verifiziert **kein** Stream-Support)

Phase-0-Spike-Gate: end-to-end Audio-Level-Meter-Flow (20 Hz) Linux-compilable, echter JVM-Attach im Rust-Unit-Test — nicht cfg-gated oder gemockt, sonst bleibt die JNI-Semantik ungeprüft.

## Decision

**Dual-Surface wird committed.** Spike-Implementation in `klarvo-bridge-jni/` (Commit ed14014-Sukzessor) bestätigt beide Surfaces compilen zusammen und funktionieren end-to-end auf Linux ohne NDK.

### 1. Version-Pins (analog ADR-0002, pre-1.0 Supply-Chain-Risk)

```toml
uniffi = { version = "=0.31.1", features = ["tokio"] }
jni    = "=0.22.4"
# dev:
jni    = { version = "=0.22.4", features = ["invocation"] }
tempfile = "3"
```

Rationale `=`-Pin: beide pre-1.0, cargo-update könnte silent Breaking-Changes einziehen. Upgrades gehen über separates ADR.

### 2. uniffi-Scaffolding-Modus

`uniffi::setup_scaffolding!()` in library-mode, **ohne** UDL-File, **ohne** `build.rs`. Proc-macros (`#[uniffi::export]`, `#[derive(uniffi::Object)]`) sind ausreichend für den Phase-0-Scope. UDL + `uniffi-bindgen` für Kotlin-Output kommt mit Phase-1 Android-Shell.

### 3. Callback-Registrierungs-Kontrakt (load-bearing)

**Entschieden: Separater Registrierungs-Pfad via raw-jni.** Kein Listener-Parameter in der uniffi-Control-Plane.

**Kotlin-Side (Zielbild):**
```kotlin
// Data-Plane: raw JNI, einmal beim App-Init registriert
external fun registerAudioLevelListener(listener: AudioLevelListener)
external fun unregisterAudioLevelListener()

// Control-Plane: uniffi-generated
val session = Session()
session.startMeter()
session.stopMeter()
```

**Rust-Side:**
- `Java_com_klarvo_bridge_Bridge_registerAudioLevelListener` (raw JNI) → speichert `Global<JObject>` in `Mutex<Option<...>>`
- `Session::start_meter` (uniffi) → spawnt Tokio-Producer + Bridge-Tasks auf shared Runtime, nutzt den registrierten Listener

Rationale:
- Architekturelle Separation hart halten: Control=uniffi, Data=raw-jni. Keine uniffi-Runtime im 20-Hz-Hotpath.
- Shell kann den Listener-Lifecycle unabhängig vom Session-Lifecycle steuern (z.B. App-Init registriert, Session ist Start/Stop-zyklisch).
- Kotlin-Shell wrappt beide Seiten in einen `callbackFlow`-Adapter für Consumer-Code.

Gegenargument (zwei-Schritte-UX) akzeptiert: Wrapper auf Kotlin-Seite blendet das aus.

### 4. Threading-Modell

- Tokio-Multi-Thread-Runtime als Crate-globaler Singleton (`OnceLock<Runtime>`, 2 worker threads, named `klarvo-bridge`).
- Bridge-Task verwendet `JavaVM::singleton()` + `vm.attach_current_thread(|env| ...)` pro Event. jni 0.22 hält den Thread nach erstem Aufruf permanent attached → Folgeaufrufe sind cheap TLS-Lookups.
- `Mutex<Option<Global<JObject<'static>>>>` wird während des JNI-Calls gehalten. Acceptable für Single-Producer-Single-Listener-Spike. **Known Limitation** bei Multi-Listener-Zukunft: Lock-Contention, muss dann auf RwLock oder Callback-Registry umgestellt werden.

### 5. FFI-Boundary-Pattern (jni 0.22-Redesign, erzwungen)

jni 0.22 hat FFI-Entry-Points von `JNIEnv<'local>` auf `EnvUnowned<'caller>` umgestellt + mandatory `unowned_env.with_env(...)` für JNI-API-Zugriff. Unsere raw-jni-Symbole folgen dem 0.22-Pattern:

```rust
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_klarvo_bridge_Bridge_registerAudioLevelListener<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    listener: JObject<'caller>,
) {
    unowned_env
        .with_env(|env| -> JniResult<()> { register_listener(env, &listener) })
        .resolve::<ThrowRuntimeExAndDefault>()
}
```

Agents, die Android-JNI-Tutorials (typisch 0.21-basiert) referenzieren: bitte gegen `docs/0.22-MIGRATION.md` im jni-Crate abgleichen, sonst kommen `JNIEnv`-vs-`EnvUnowned`-Verwirrungen.

## Spike-Messdaten

| Metrik | Zielwert | Gemessen | Ergebnis |
|--------|----------|----------|----------|
| Events in 10 s bei 20 Hz nominal | 200 ±5% (190–210) | **200** | **0 Drops** |
| Test-Compile + JVM-Boot-Overhead | <2 s | ~1.2 s | OK |
| `cargo check --workspace` | grün | grün | OK |
| `cargo clippy -- -D warnings` | grün | grün | OK |
| Exceptions während 200 Events | 0 | 0 (JVM `-Xcheck:jni` aktiv) | OK |

**Nicht gemessen im Spike (Follow-ups für Phase 1):**
- Latenz-Verteilung push→callback (p50/p95/p99) — Spike-Durchsatz impliziert <50 ms, nicht formal erfasst
- GlobalRef-Cleanup unter 10 k Events (Long-Soak-Test) — Drop-Semantik wird durch `Global`-`Drop`-Impl abgedeckt, Leak-Check separat in Phase 1
- Kotlin-`callbackFlow`-Adapter-Shape — Shell-Thema Phase 1

## Consequences

**Positiv:**
- VadProvider-Trait-Signatur (ADR-0001 offen-gelassen) kann ohne JNI-Blocker finalisiert werden — die VadDecision-Enum lässt sich 1:1 über den gleichen raw-jni-Pfad emittieren wie AudioLevel.
- Phase-1 Android-Shell hat kanonischen Kontrakt: `registerXxxListener(...)` + Control-Plane-uniffi-Commands.
- Tokio-Broadcast-Pattern als Event-Backbone validiert.

**Negativ:**
- jni 0.22 hat eine steile Lernkurve für Android-Dev-Agents; Tutorials online sind 0.21.
- `Mutex` im 20-Hz-Hotpath ist technische Schuld bei Multi-Listener.

**Mitigations:**
- Dieses ADR + Migration-Link als Briefing-Material für Android-Shell-Session.
- Multi-Listener-Umstellung als separates ADR wenn konkret benötigt (vorerst YAGNI).

## Smoke-Test & Reproducibility

> **Hinweis (post-Amendment 2):** `--test-threads=1` ist seit der TEST_MUTEX-Fix
> obsolet — der Mutex serialisiert die Tests intern. Das Flag bleibt unten als
> historisches Spike-Reproduzierbarkeits-Kommando dokumentiert; kanonischer
> Smoke-Befehl ist heute `cargo test -p klarvo-bridge-jni` (ohne Flag).

```bash
cargo test -p klarvo-bridge-jni -- --test-threads=1 --nocapture
# expected:
#   test listener_receives_events_smoke ... ok
#   test twenty_hz_over_ten_seconds_no_drops ... ok
#   [twenty_hz_over_ten_seconds_no_drops] final count = 200
```

Setup-Voraussetzung: JDK auf PATH (Linux-Dev-Maschine hatte OpenJDK 17 via `apt`). Tests kompilieren via `Command::new("javac")` eine minimale `TestListener.java` in ein Tempdir, spawnen die JVM via `InitArgsBuilder` mit dem Tempdir als classpath. Kein Android-Emulator, kein NDK.

## Next Action

1. Commit `klarvo-bridge-jni/` + ADR-0003 als eigenen Spike-Outcome-Commit.
2. Phase-1-Action-Item: Kotlin-Shell-Skeleton in `shells/android/` mit `registerAudioLevelListener` + `callbackFlow`-Adapter aufsetzen.
3. `VadProvider`-Trait-ADR-0001 von "Proposed" auf "Accepted" umstellen (JNI-Pfad gate-free).

---

## Amendment 2 — Test-Isolation-Fix (2026-05-01, Story 2.A.F2)

**Status-Update:** Proposed → Accepted

### Root-Cause der Phase-2-A-Regression

Nach 2026-04-20 schlug `cargo test -p klarvo-bridge-jni` (ohne `--test-threads=1`) fehl:
`listener_receives_events_smoke` lieferte 0 Events, `twenty_hz_over_ten_seconds_no_drops` 210 Events.

**Mechanismus:** Beide Tests teilen `LISTENER: Mutex<Option<Global<JObject<'static>>>>` und
`RUNTIME: OnceLock<Runtime>`. Rust-Standardausführung ist multi-threaded. Wenn beide parallel starten:
1. Test-A registriert Listener L_A via `register_listener`
2. Test-B registriert Listener L_B via `register_listener` → überschreibt L_A im static LISTENER
3. Beide Sessions senden Events an L_B (das aktuelle LISTENER-Ziel)
4. Test-A liest `L_A.count` → 0 (L_A hat nie Events erhalten)
5. Test-B liest `L_B.count` → ~210 (Events beider Sessions, zufällig innerhalb 190–210-Toleranz)

**Breaking-Commit:** Kein Breaking-Commit zwischen Spike (482c6c9) und HEAD — der Bug war seit dem
Spike latent, blieb aber unbemerkt, weil der Spike mit `--test-threads=1` durchgeführt wurde
(wie in der Smoke-Test-Sektion dieses ADR dokumentiert). Die Regression wurde erst sichtbar als
`cargo test -p klarvo-bridge-jni` ohne das Flag aufgerufen wurde (z.B. durch CI-Änderungen in
Phase-2-A E1-Story).

### Fix

`static TEST_MUTEX: Mutex<()>` in `tests/audio_level_callback.rs` serialisiert die beiden Tests,
die LISTENER teilen. Jeder Test hält den Lock für seine gesamte Laufzeit (Registrierung + Messung +
`unregister_listener`). Kein neues Crate-Dependency erforderlich.

**Kein Production-Code-Bug.** Die `LISTENER`-Mutex-Architektur für Single-Listener-Spike (§4
Threading-Modell) ist unverändert korrekt; die Known Limitation (Multi-Listener) bleibt Phase-3.

### Fix-Messwerte (2026-05-01, OpenJDK 17.0.18, unoptimized+debuginfo)

| Metrik | Zielwert | Gemessen | Ergebnis |
|--------|----------|----------|----------|
| `twenty_hz_over_ten_seconds_no_drops` Events in 10 s | 200 ±5% (190–210) | **200** | ✅ 0 Drops |
| `listener_receives_events_smoke` Events in 500 ms | ≥ 5 | **10** | ✅ OK |
| `cargo test -p klarvo-bridge-jni` (kein `--test-threads=1`) | alle grün | **2/2 grün** | ✅ Fix bestätigt |

### Konsequenz für CI

E1 windows-ci.yml: `--exclude klarvo-bridge-jni` entfernt (separater Commit, Story 2.A.F2-Closure).
`--test-threads=1` ist post-Amendment-2 obsolet und kann ersatzlos entfernt werden — TEST_MUTEX
serialisiert die Tests, die das `LISTENER`-Static teilen. Das Flag bleibt in der historischen
Smoke-Test-Sektion dieses ADR als Spike-Reproduzierbarkeits-Kontext dokumentiert (siehe
`> Hinweis`-Box dort).

Nicht in F2-Scope (siehe `deferred-work.md`):
- Linux-Epic-5-CI exkludiert `klarvo-bridge-jni` weiterhin (F2-W5) — separate CI-Hardening-Story.
- Production `register_listener` überschreibt Listener still bei doppeltem Aufruf (F2-W6) —
  Multi-Listener-Hardening kommt mit Phase-3-JNI-Rewrite (§4 Threading-Modell-Limitation oben).
