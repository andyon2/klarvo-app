# ADR-0005: HTTPS Client + HTTP-Mock Stack for Cloud-Provider-Plugins

**Status:** Accepted
**Date:** 2026-04-19

## Context

Phase 1 (Epic 1B) führt die erste Cloud-Provider-Plugin-Implementation ein: `klarvo-plugin-groq` (Story 1B.4, STT via Groq Whisper HTTPS-API). Die Wahl von HTTPS-Client und HTTP-Mock-Framework ist load-bearing, weil sie das Pattern für alle Folge-Plugins setzt: `klarvo-plugin-deepseek` (LLM), `klarvo-plugin-openai`, `klarvo-plugin-anthropic`, `klarvo-plugin-openrouter` (Phase-2+). Divergierende Stacks pro Plugin würden Wartungsaufwand, Build-Surface und Test-Harness-Komplexität multiplizieren.

`output/planning-artifacts/architecture.md` listet `reqwest` nur als v1-Inventory-Dependency (Zeile 85), ohne v2-Mandat. Kein ADR fixiert die Entscheidung. Zeile 1269 benennt „HTTPS (BYOK, Keys aus OS-Keystore)" ohne Client-Crate-Commitment. Dieses ADR schließt die Lücke vor Story 1B.4-Impl.

Rahmenbedingungen aus Memory + PRD:
- `SttProvider::transcribe` ist `async fn` (epics.md:483) → Async-native-Client benötigt.
- Zielgruppe Power-User/Devs/Institute (`project_market_positioning.md`). Institute implizieren Corporate-CA/Custom-Root-Szenarien.
- Keine Remote-Telemetrie (`project_no_remote_telemetry.md`).
- Windows-Build-Reproducibility ohne System-OpenSSL/SChannel-Divergenz zwischen Build-Hosts.
- BYOK-Keys werden im Plugin als `SecretString` gehalten (`project_api_key_os_keystore_mvp.md`, ADR-0004 Decision #2) → Bearer-Header-Konstruktion muss SecretString-sicher sein.

## Decision

### 1. HTTPS-Client: `reqwest` 0.12.x

**Gewählt:** `reqwest` als universeller HTTPS-Client für alle Cloud-Provider-Plugins.

**Alternativen verworfen:**
- `ureq` — blocking-API. `SttProvider::transcribe` ist `async fn` → jeder Call erzwingt `tokio::task::spawn_blocking` in Plugin-Impl. Runtime-Friction + zusätzliche Thread-Kosten pro Call.
- `hyper` — HTTP-Protokoll-Primitive, kein High-Level-Client. Plugin müsste Header-Management, JSON-Body-Encoding, Redirect-Handling, Connection-Pool selbst implementieren. Entspricht dem Nachbau von `reqwest`.

### 2. TLS-Backend: `rustls-tls-native-roots`

**Gewählt:** `rustls` (pure-Rust-TLS) mit OS-Truststore-Root-Source.

**Alternativen verworfen:**
- `native-tls` (OpenSSL/SChannel) — Build-Reproducibility-Risiko auf Windows (SChannel-Version-Abhängigkeit), OpenSSL-System-Dep auf Linux-Build-Hosts.
- `rustls-tls` mit `webpki-roots` (Mozilla-Bundle) — pure-Rust, reproducible, aber **kein** Support für Corporate-CA-Custom-Roots. Institute-User in Corporate-Umgebungen mit Custom-Root-CA (MITM-Proxy) würden TLS-Handshakes fehlschlagen sehen, ohne User-behebbare Option.

`rustls-tls-native-roots` kombiniert pure-Rust-TLS-Implementation mit OS-Truststore-Zugriff (Windows-Cert-Store, macOS-Keychain, Linux `/etc/ssl/certs`). Corporate-User, die eine Custom-Root-CA im System installiert haben, werden automatisch getrustet.

### 3. `reqwest` default-features = false + explizite Feature-Liste

**Cargo.toml-Pattern für alle Cloud-Provider-Plugins:**

```toml
[dependencies]
reqwest = { version = "0.12", default-features = false, features = [
    "rustls-tls-native-roots",
    "json",
] }
```

**Warum explizit:** `reqwest` default-features aktiviert `native-tls`, was Windows-Build an SChannel und Linux an System-OpenSSL bindet. `default-features = false` schaltet das ab. Minimale Feature-Liste:
- `rustls-tls-native-roots` — TLS-Backend per Decision #2.
- `json` — Groq/DeepSeek/OpenAI/Anthropic antworten JSON; `reqwest::Response::json::<T>()` wird in jedem Plugin benötigt.

Weitere Features (`multipart`, `stream`, `gzip`, …) werden **nicht** auf Workspace-/Core-Ebene aktiviert. Einzelne Plugins dürfen sie in ihrer Cargo.toml hinzufügen, wenn Provider-spezifisch nötig (z. B. `multipart` für Groq-Whisper-Audio-Upload-Form). Begründung pro Plugin dann im Cargo-Toml-Kommentar.

### 4. Client-Lifetime-Pattern: per-Plugin-Instance, nicht per-Call

**Regel:** Plugin-Structs halten `reqwest::Client` als Field und re-usen den Client über alle Calls hinweg.

```rust
pub struct GroqStt {
    client: reqwest::Client,        // konstruiert einmal in ::new()
    api_key: SecretString,
    endpoint: String,
    // ...
}
```

**Warum:** `reqwest::Client` besitzt einen Connection-Pool mit Keep-Alive-Sockets. Per-Call-Instanziierung (`reqwest::Client::new()` innerhalb von `transcribe`) erzwingt DNS-Lookup + TLS-Handshake pro Call — bei Dictation-Pipeline-Load (viele kurze Aufnahmen) messbarer Latenz-Hit. Ein Client pro Plugin-Instance amortisiert Connection-Setup über alle Calls.

**Verboten:** `lazy_static!`-Global-Client oder `OnceCell` auf Crate-Ebene. Jede Plugin-Instance ist eigenständig (mehrere STT-Plugins in Pipeline möglich, Plugin-Registry hält Instances).

### 5. HTTP-Mock-Framework: `wiremock` 0.6.x

**Gewählt:** `wiremock` als Test-Harness für Integration-Tests gegen Cloud-Provider-Plugins.

```toml
[dev-dependencies]
wiremock = "0.6"
```

**Alternativen verworfen:**
- `mockito` — blocking-biased, async-Support kam spät, API-Surface inkonsistent zwischen sync/async-Modi.
- `httpmock` — async-fähig, kleineres Ökosystem, weniger Community-Resources als wiremock.

`wiremock` matched den reqwest-async-Stack direkt (tokio-basierter Mock-Server), Test-Code läuft ohne `block_on`-Friction.

**Integration:** Mock-Server wird in `klarvo-test-fixtures` als Helper-Primitive bereitgestellt (analog `MockKeyStore`, `MockSttProvider`). Story 1B.4 AC verankert den Helper-Entry-Point; Phase-2-Folge-Plugins (deepseek/openai) re-usen denselben Helper-Baustein.

### 6. Scope-Deferrals (explizit Non-Goals oder Later-Phase)

- **Connection-Pool-Sizing per Plugin** → Phase-2-Polish. Default `reqwest`-Pool-Config ausreichend für Phase-1-Load.
- **Request-Timeouts** → Plugin-Config-Layer (Settings-Store, Epic 4), nicht Architektur-Layer. Default `reqwest`-Timeout (kein Client-wide-default, nur Per-Request) in Plugin-Impl mit vernünftigem Start-Wert (z. B. 30 s für Groq-Whisper).
- **Retry-Policy** → Epic 2 FR29 Graceful-Recovery auf Pipeline-Level, nicht Client-Level. 1B.4 liefert nur die Error-Surface (`AppError::kind::UpstreamUnavailable` mit Cause-Chain).
- **Certificate-Pinning** → Non-Goal für Phase 1+2. BYOK-Keys (User-owned) sind sensibler als Cert-Pinning-Gewinn; Corporate-CA-Szenarien würden durch Pinning gebrochen.
- **`reqwest-middleware`-Crate** → Known-future-extension-point für Retry-/Logging-Middleware (Epic 2 FR29, Epic 6 Observability). Kein Adoption in Phase 1.
- **WebSocket-Support** → `reqwest` hat keins; Phase-2+-Plugin, das WebSocket braucht (z. B. Realtime-STT), führt separate Dep (`tokio-tungstenite`) in eigener ADR-Amendment.
- **Outgoing-Telemetry aus HTTPS-Stack** → Verboten per `project_no_remote_telemetry`. `reqwest` default hat keine Telemetrie; `reqwest-tracing`-Crate wird **nicht** adoptiert. Lokales Logging via `tracing`-Crate innerhalb Plugin-Impl ist erlaubt (strukturierte Request-Logs auf DEBUG, Error-Logs auf ERROR).

### 7. Divergenz-Klausel

Alle späteren Cloud-Provider-Plugins (deepseek, openai, anthropic, openrouter) folgen diesem Stack by default. Provider-spezifische Anforderung, die vom Stack abweicht, verlangt Amendment-Commit per `feedback_adr_amendment_convention` (eigener Commit, anhängen-nicht-überschreiben).

## Consequences

**Positiv:**
- Einheitlicher Stack über alle Cloud-Provider-Plugins → Test-Harness, Error-Mapping, Tracing-Pattern sind wiederverwendbar.
- Pure-Rust-TLS (`rustls`) eliminiert System-OpenSSL/SChannel-Build-Variance zwischen Dev-Maschinen und CI.
- OS-Truststore-Root-Source unterstützt Corporate-CA-Szenarien ohne User-Workaround. Native-Roots is chosen over bundled Mozilla-Roots so Institute-Users behind Corporate-CA/MITM-Proxy can operate without ADR-Amendment. Binary-Size-Cost (~50–100KB) is accepted tradeoff; per-machine root-set-variance is intentional (reflects deployment reality).
- `default-features = false` + minimale Feature-Liste hält Plugin-Binary-Surface klein.
- Per-Plugin-Instance-Client-Reuse eliminiert redundante DNS/TLS-Roundtrips pro Call.

**Negativ:**
- `reqwest` + `rustls` + `hyper`-Stack sind gemeinsam mehrere MB Binary-Footprint. Phase-1-Shell (`klarvo-voice-shell`) compiliert diese Deps pro aktiviertem `plugin-<provider>`-Feature ein. Acceptabel gegen die Alternative (manuelles HTTP/TLS).
- `wiremock` ist tokio-bound → Plugin-Tests müssen `#[tokio::test]` sein. Konsistent zum Async-SttProvider-Contract, kein echter Nachteil.
- `rustls-tls-native-roots` liest OS-Cert-Store beim Client-Start → leicht höhere Startup-Latenz (~1-5 ms auf Windows, mess-abhängig). Vernachlässigbar gegen Pipeline-Init-Kosten.

**Mitigations:**
- Binary-Footprint: Cloud-Plugins stehen hinter Cargo-Features; User, die nur Offline-STT wollen, compilieren sie nicht mit.
- Startup-Latenz: Client-Konstruktion passiert einmal beim Plugin-Register, nicht pro Call — no-op für Steady-State-Load.
- Plugin-Lifetime vs Client-Pool-Reuse: If Epic-2-Pipeline-Lifecycle-Decision re-instantiates Plugin-structs per Dictation-Session, Connection-Pool-Reuse is lost per-Session. Mitigation: Plugin-Registry caches Plugin-Instances across Sessions. This is the recommended Epic-2-Mechanism but not binding at ADR-0005-Level — ADR-0005 only mandates the Plugin-holds-Client pattern, not Plugin-Lifetime-Semantics.

## Referenzen

- `output/planning-artifacts/architecture.md` Zeile 85 (v1-Dep-Inventory), Zeile 1269 (HTTPS-BYOK-Statement)
- `output/planning-artifacts/prd.md` FR10 (Cloud-Provider-Plugins)
- `output/planning-artifacts/epics.md` Story 1A.6 (SttProvider-Trait-Signatur), Story 1B.4 (erste Konkret-Impl)
- `memory/project_api_key_os_keystore_mvp.md`
- `memory/project_no_remote_telemetry.md`
- `memory/project_market_positioning.md` (Institute-Zielgruppe → Corporate-CA-Trust)
- `memory/feedback_adr_amendment_convention.md`
- ADR-0004 §Decision 2 (`SecretString`-Integration-Pattern)

## Forward-References

- **Epic 1B Story 1B.4** — erste Impl dieses Stacks; AC verankert `per ADR-0005`-Reference in HTTPS-Wire-Up + HTTP-Mock-Harness.
- **Epic 2 FR29** — Pipeline-Level-Retry-Orchestration konsumiert 1B.4-Error-Surface.
- **Epic 2 Pipeline-Lifecycle** — Plugin-Instance-Caching via Plugin-Registry empfohlen, um Client-Pool-Reuse über Dictation-Sessions hinweg zu erhalten (siehe Consequences/Mitigations). Nicht binding auf ADR-0005-Level — Entscheidung liegt bei Epic-2.
- **Epic 6** — Observability/Logging-Pattern innerhalb Plugins (strukturierte `tracing`-Spans, kein Remote-Sink).
- **Phase 2** — deepseek/openai/anthropic/openrouter-Plugins re-usen Stack.
- **Phase 3+** — Potentielle WebSocket-Divergenz (Realtime-STT) → Amendment.

## Next Action

1. Scope-Lock-Approve von Andy (Divergenz-Fokus: Sub-Decisions #2 `rustls-tls-native-roots`, #3 `default-features`-Hygiene, #4 Client-Lifetime-Pattern).
2. Commit dieses ADR als separaten Commit `feat: ADR-0005 HTTPS client + HTTP-mock stack for cloud-provider-plugins` (getrennt von Epic-1B-Story-Work per `feedback_commit_hygiene`).
3. Status auf `Accepted` flippen im Approve-Commit.
4. Story 1B.4 AC-Writing referenziert ADR-0005 in HTTPS-Wire-Up + HTTP-Mock-Test-Harness-ACs.
