# API-Provider -- Dikta

## Groq -- Speech-to-Text (Whisper API) -- Recherchiert 2026-03-06

### Ueberblick
Groq hostet OpenAI Whisper-Modelle auf ihrer LPU-Inferenz-Hardware. 216-228x Echtzeit-Geschwindigkeit, sehr niedrige Latenz. Wir nutzen es als primaeren STT-Provider.

### Setup / Authentication
- API-Key: https://console.groq.com/keys
- Base-URL: `https://api.groq.com/openai/v1`
- Header: `Authorization: Bearer {GROQ_API_KEY}`
- OpenAI-kompatible API (gleiche Struktur wie openai.audio.transcriptions)

### Relevante Endpoints

**POST /audio/transcriptions** -- Transkription (unser Haupt-Usecase)
```
Content-Type: multipart/form-data

file          required  Audio-Datei (max 25MB Free, 100MB Dev)
model         required  "whisper-large-v3-turbo" | "whisper-large-v3"
language      optional  ISO-639-1, z.B. "de" oder "en" -- verbessert Genauigkeit
prompt        optional  Kontext-Hinweis fuer das Modell (max 224 Tokens),
                        z.B. Fachbegriffe oder Namen die korrekt transkribiert werden sollen
response_format optional "json" (default) | "text" | "verbose_json"
temperature   optional  0.0-1.0 (default 0, empfohlen beibehalten)
timestamp_granularities[] optional ["word", "segment"] -- nur bei verbose_json
```

Response (`json`):
```json
{ "text": "Der transkribierte Text..." }
```

Response (`verbose_json`) -- zusaetzlich:
```json
{
  "text": "...",
  "segments": [
    {
      "start": 0.0, "end": 2.5,
      "text": "...",
      "avg_logprob": -0.15,      // Konfidenz: naeher an 0 = besser
      "no_speech_prob": 0.01,    // niedrig = sicher Sprache (nicht Stille)
      "compression_ratio": 1.2   // Ausreisser deuten auf Qualitaetsprobleme hin
    }
  ],
  "words": [{ "word": "...", "start": 0.0, "end": 0.4 }]
}
```

**POST /audio/translations** -- Uebersetzt Audio direkt nach Englisch (nur whisper-large-v3)

### Modelle

| Modell | Preis/Stunde | Geschwindigkeit | Fehlerrate | Uebersetzung |
|--------|-------------|----------------|-----------|-------------|
| `whisper-large-v3-turbo` | $0.04 | 228x Echtzeit | ~12% | Nein |
| `whisper-large-v3` | $0.111 | 217x Echtzeit | ~10.3% | Ja |

**Empfehlung fuer Dikta:** `whisper-large-v3-turbo` -- 3x guenstiger, minimal schlechtere Genauigkeit, voellig ausreichend fuer Diktat-Usecase.

### Code-Beispiel (Rust)

```rust
use reqwest::multipart;

pub async fn transcribe(
    api_key: &str,
    audio_bytes: Vec<u8>,
    language: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-large-v3-turbo")
        .text("language", language.to_string())
        .text("response_format", "json");

    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let text = json["text"].as_str().unwrap_or("").to_string();
    Ok(text)
}
```

Cargo.toml benoetigt: `reqwest = { features = ["multipart"] }`, `serde_json`, `anyhow`

Audio-Optimierung vor dem Upload (reduziert Latenz und Dateigroesse):
```
ffmpeg -i input.wav -ar 16000 -ac 1 -c:a flac output.flac
```
Groq downsampled sowieso auf 16kHz Mono -- besser selbst konvertieren.

### Limits & Kosten

**Rate Limits (Free Tier):**
| Modell | RPM | RPD | Sekunden/Stunde | Sekunden/Tag |
|--------|-----|-----|----------------|-------------|
| whisper-large-v3-turbo | 20 | 2.000 | 7.200 | 28.800 |
| whisper-large-v3 | 20 | 2.000 | 7.200 | 28.800 |

(RPM = Requests per Minute, RPD = Requests per Day)

**Pricing (On-Demand):**
- `whisper-large-v3-turbo`: **$0.04 / Stunde** Audio
- `whisper-large-v3`: **$0.111 / Stunde** Audio
- Mindest-Abrechnung: **10 Sekunden** pro Request
- Hochrechnung fuer Diktat: 1 Minute Diktat/Tag = ~$0.0007/Tag mit turbo -- praktisch kostenlos

### Gotchas

- **Mindest-Abrechnung 10s:** Kurze Aufnahmen (< 10s) werden trotzdem als 10s berechnet. Lohnt sich also, etwas groessere Chunks zu senden.
- **Dateiformat:** WAV oder FLAC empfohlen fuer beste Latenz. Groq konvertiert intern auf 16kHz Mono.
- **Max-Groesse:** 25MB (Free), 100MB (Dev-Plan). Bei langen Aufnahmen Chunking implementieren.
- **`prompt`-Parameter nutzen:** Dictionary-Eintraege des Nutzers als Prompt mitschicken verbessert Transkription von Fachwoertern und Namen erheblich (max 224 Tokens).
- **Kein Streaming:** Audio muss komplett hochgeladen werden -- kein Live-Streaming des Audios.
- **`verbose_json` fuer Qualitaetspruefung:** `no_speech_prob > 0.5` bedeutet Stille erkannt -- kein LLM-Cleanup benoetigt.
- **`whisper-large-v3` fuer Translation:** Wenn wir mal Uebersetzung brauchen, nur v3 (nicht turbo) unterstuetzt `/audio/translations`.
- **Unterstuetzte Formate:** flac, mp3, mp4, mpeg, mpga, m4a, ogg, wav, webm

---

## DeepSeek -- Text-Cleanup (Chat API) -- Recherchiert 2026-03-06

### Ueberblick
DeepSeek bietet sehr guenstige LLM-Inferenz (deutlich unter OpenAI/Anthropic). Wir nutzen `deepseek-chat` (DeepSeek-V3.2, Non-Thinking-Mode) fuer die Text-Bereinigung nach der Transkription. OpenAI-kompatibles API-Format.

### Setup / Authentication
- API-Key: https://platform.deepseek.com/api_keys
- Base-URL: `https://api.deepseek.com/v1`
- Header: `Authorization: Bearer {DEEPSEEK_API_KEY}`
- Neues Konto: 5 Mio. Token gratis (gueltig 30 Tage) -- ca. $8.40 Wert

### Relevanter Endpoint

**POST /chat/completions**

Request:
```json
{
  "model": "deepseek-chat",
  "messages": [
    {
      "role": "system",
      "content": "[Style-abhaengiger System-Prompt]"
    },
    {
      "role": "user",
      "content": "[Roher Transkript-Text]"
    }
  ],
  "temperature": 0.3,
  "max_tokens": 2048
}
```

Wichtige optionale Parameter:
| Parameter | Default | Fuer uns relevant? |
|-----------|---------|-------------------|
| `temperature` | 1 | Ja -- auf 0.2-0.4 setzen (treue Bereinigung) |
| `max_tokens` | 4096 max | Ja -- 2048 reicht fuer Diktat-Output |
| `stream` | false | Ja -- fuer spaeteres Streaming-Feature |
| `response_format` | `{"type":"text"}` | Nein (text reicht) |
| `thinking` | - | Nein -- deepseek-chat hat kein Thinking |

Response:
```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Der bereinigte Text..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 120,
    "completion_tokens": 95,
    "total_tokens": 215,
    "prompt_cache_hit_tokens": 80,
    "prompt_cache_miss_tokens": 40
  }
}
```

`finish_reason` Werte: `stop` (normal), `length` (max_tokens erreicht), `content_filter`, `insufficient_system_resource`

### System-Prompts pro Stil

**Polished:**
```
You are a text cleanup assistant. The user will give you raw speech-to-text output. Clean it up:
- Remove filler words (um, uh, like, you know / äh, ähm, also)
- Remove false starts and self-corrections (keep only the final version)
- Fix grammar and punctuation
- Format professionally (proper capitalization, paragraphs where appropriate)
- Preserve the speaker's meaning exactly -- do not add or change content
- Language: respond in the same language as the input

The user's custom dictionary terms (preserve these exactly): {dictionary_terms}
```

**Verbatim:**
```
You are a text cleanup assistant. The user will give you raw speech-to-text output. Minimal cleanup only:
- Add punctuation and capitalization
- Fix obvious transcription errors
- Keep filler words and speech patterns intact
- Language: respond in the same language as the input

The user's custom dictionary terms (preserve these exactly): {dictionary_terms}
```

**Chat:**
```
You are a text cleanup assistant. The user will give you raw speech-to-text output. Make it chat-ready:
- Remove all filler words
- Make it concise and casual
- Keep it short -- this is for messaging apps
- Emojis are okay if they fit naturally
- Language: respond in the same language as the input
```

### Code-Beispiel (Rust)

```rust
use reqwest::Client;
use serde_json::{json, Value};

pub async fn cleanup_text(
    api_key: &str,
    raw_text: &str,
    system_prompt: &str,
) -> anyhow::Result<String> {
    let client = Client::new();

    let body = json!({
        "model": "deepseek-chat",
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user",   "content": raw_text }
        ],
        "temperature": 0.3,
        "max_tokens": 2048
    });

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    let json: Value = response.json().await?;
    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(text)
}
```

Cargo.toml benoetigt: `reqwest = { features = ["json"] }`, `serde_json`, `anyhow`

### Limits & Kosten

**Pricing (deepseek-chat = DeepSeek-V3.2, Stand 2026-03-06):**
| Token-Typ | Preis pro 1M Token |
|-----------|-------------------|
| Input (Cache Hit) | **$0.028** |
| Input (Cache Miss) | **$0.28** |
| Output | **$0.42** |

- Context Window: **128K Token**
- Max Output: **8K Token** (default 4K)
- Prompt-Caching: automatisch serverseitig -- bei wiederholten System-Prompts greift Cache-Preis

**Hochrechnung fuer Diktat-Usecase:**
- Typischer Cleanup-Request: ~150 Input-Token (System-Prompt + Transkript) + ~100 Output-Token
- Pro Request: ~$0.000042 bei Cache Miss -- praktisch kostenlos
- 1.000 Requests/Monat: ~$0.04

**Rate Limits:**
- DeepSeek setzt offiziell **keine Rate Limits** -- kein RPM/RPD-Cap dokumentiert
- Bei Hochlast: Server kann Verbindung nach 10 Minuten Wartezeit schliessen

### Gotchas
- **Latenz hoeher als Groq** (kein LPU-Chip), aber fuer nachgelagerten Text-Cleanup (Nutzer tippt nicht live) akzeptabel (~1-3s typisch).
- **Temperature niedrig halten (0.2-0.4)** -- wir wollen treue Bereinigung, keine Kreativitaet.
- **Cache-Hit-Preis nutzen:** System-Prompts moeglichst stabil halten (nicht dynamisch variieren) -- dann greift der 10x guenstigere Cache-Hit-Preis bei Folge-Requests.
- **`usage.prompt_cache_hit_tokens` tracken** fuer Kostenmonitoring: zeigt wie viele Token gecacht wurden.
- **`finish_reason: "length"` abfangen:** Wenn Output bei max_tokens abbricht, ist Text unvollstaendig -- Fehler an UI melden oder max_tokens erhoehen.
- **Kein Streaming noetig fuer MVP**, aber `"stream": true` ist trivial nachzuruesten fuer spaeteres Live-Typing-Feature.
- **deepseek-reasoner nicht verwenden:** Denkt laut nach (reasoning_tokens), ist teurer und langsamer -- fuer Text-Cleanup sinnlos.

---

## whisper-rs (Offline STT Fallback) -- Recherchiert 2026-03-10

### Ueberblick
whisper-rs bietet Rust-Bindings fuer whisper.cpp (C++ Port von OpenAI Whisper). Wir nutzen es als **Offline-Fallback** wenn kein Internet verfuegbar ist oder der Nutzer Cloud-APIs deaktiviert hat. Audio muss als 32-bit Float, 16kHz, Mono vorliegen.

- **Crate:** `whisper-rs` v0.15.1 (Stand Sept 2025) -- aktiv gepflegt
- **GitHub (archiviert):** https://github.com/tazz4843/whisper-rs (read-only seit Juli 2025)
- **Aktive Entwicklung:** https://codeberg.org/tazz4843/whisper-rs
- **Sys-Crate:** `whisper-rs-sys` v0.14.1 (separat, wird automatisch gezogen)
- **Lizenz:** Unlicense (Public Domain)

### Setup / Cargo.toml

```toml
# Basisintegration (CPU only):
whisper-rs = "0.15.1"

# Mit NVIDIA CUDA GPU-Unterstuetzung:
whisper-rs = { version = "0.15.1", features = ["cuda"] }

# Mit AMD ROCm/hipBLAS (Linux only):
whisper-rs = { version = "0.15.1", features = ["hipblas"] }

# Mit Apple Metal:
whisper-rs = { version = "0.15.1", features = ["metal"] }

# Mit Vulkan (cross-platform GPU):
whisper-rs = { version = "0.15.1", features = ["vulkan"] }

# Mit OpenBLAS CPU-Optimierung:
whisper-rs = { version = "0.15.1", features = ["openblas"] }

# Mit Logging-Integration:
whisper-rs = { version = "0.15.1", features = ["cuda", "log_backend"] }
```

GPU-Features setzen intern automatisch ein Hidden-GPU-Flag -- kein manuelles Eingreifen noetig.

### Build-Anforderungen

**Linux (einfachster Weg):** Laeuft out-of-the-box, kein extra Setup.

**Windows (WSL2 oder nativ):**
- CMake muss im PATH sein
- Fuer CUDA: NVIDIA CUDA Toolkit installiert (`nvcc` im PATH)
- Build-Command whisper.cpp intern: `cmake -B build -DGGML_CUDA=1 && cmake --build build -j --config Release`
- Bei Binding-Problemen: `WHISPER_DONT_GENERATE_BINDINGS=1` env-var setzen (nutzt prebuilt bindings)

**Fuer unseren WSL2-Build:** Da wir Windows-Binaries aus WSL2 bauen, braucht der Windows-Target CMake auf der Windows-Seite (nicht WSL). Der bestehende Tauri-Build-Workflow (`sync-and-build.ps1`) muss CMake kennen.

### Relevante API-Oberflaeche

**WhisperContext** -- Modell laden:
```rust
use whisper_rs::{WhisperContext, WhisperContextParameters};

let ctx = WhisperContext::new_with_params(
    "/path/to/ggml-base.bin",
    WhisperContextParameters::default(),
).expect("failed to load model");
```

**WhisperState** -- Inferenz-State (aus Context erzeugen):
```rust
let mut state = ctx.create_state().expect("failed to create state");
```

**FullParams** -- Transkriptions-Konfiguration:
```rust
use whisper_rs::{FullParams, SamplingStrategy};

let mut params = FullParams::new(SamplingStrategy::Greedy { n_past: 0 });
params.set_language(Some("de"));       // "de", "en", "auto", None = auto-detect
params.set_n_threads(4);               // CPU-Threads
params.set_translate(false);           // true = in Englisch uebersetzen
params.set_print_special(false);       // Sondertoken unterdruecken
params.set_print_progress(false);      // kein stdout-Spam
params.set_print_realtime(false);
params.set_print_timestamps(false);
params.set_no_timestamps(true);        // bei reinem Text-Output empfohlen
params.set_initial_prompt("Fachbegriff1 Fachbegriff2"); // STT-Konditionierung
params.set_single_segment(false);      // true fuer kurze Clips
params.set_suppress_blank(true);
```

**Transkription ausfuehren + Segmente lesen:**
```rust
// Audio: Vec<f32>, 16kHz Mono 32-bit float
state.full(params, &audio_data[..]).expect("failed to run model");

let num_segments = state.full_n_segments().expect("failed to get segment count");
let mut transcript = String::new();
for i in 0..num_segments {
    let text = state.full_get_segment_text(i).expect("failed to get segment text");
    transcript.push_str(&text);
}
```

**Audio-Konvertierungs-Helpers (im Crate enthalten):**
```rust
use whisper_rs::{convert_integer_to_float_audio, convert_stereo_to_mono_audio};

// 16-bit PCM -> 32-bit float (fuer cpal-Output):
let float_audio = convert_integer_to_float_audio(&pcm_i16);

// Stereo -> Mono:
let mono = convert_stereo_to_mono_audio(&stereo_floats).unwrap();
```

### Code-Beispiel (Rust -- vollstaendig)

```rust
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
    convert_integer_to_float_audio,
};

pub fn transcribe_offline(
    model_path: &str,
    pcm_i16: &[i16],   // 16kHz Mono aus cpal
    language: &str,
) -> anyhow::Result<String> {
    // Modell laden (einmalig, dann cachen!)
    let ctx = WhisperContext::new_with_params(
        model_path,
        WhisperContextParameters::default(),
    )?;

    let mut state = ctx.create_state()?;

    // i16 PCM -> f32 konvertieren
    let audio_f32 = convert_integer_to_float_audio(pcm_i16);

    // Parameter
    let mut params = FullParams::new(SamplingStrategy::Greedy { n_past: 0 });
    params.set_language(Some(language));
    params.set_n_threads(4);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_no_timestamps(true);
    params.set_suppress_blank(true);

    // Transkription
    state.full(params, &audio_f32)?;

    let n = state.full_n_segments()?;
    let mut result = String::new();
    for i in 0..n {
        result.push_str(&state.full_get_segment_text(i)?);
    }

    Ok(result.trim().to_string())
}
```

**Wichtig:** `WhisperContext` ist teuer zu laden (~100-200ms). In einer `Arc<Mutex<Option<WhisperContext>>>` halten und wiederverwenden.

### GGML-Modelle -- Download-URLs & Groessen

Modelle von HuggingFace: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/`

| Modell | Datei | Disk | RAM | Encode-Zeit* |
|--------|-------|------|-----|-------------|
| tiny | `ggml-tiny.bin` | 77.7 MB | ~273 MB | ~420ms (Ryzen 9 3900X, 8T) |
| tiny.en | `ggml-tiny.en.bin` | 77.7 MB | ~273 MB | schneller als multilingual |
| tiny-q5_1 | `ggml-tiny-q5_1.bin` | 32.2 MB | ~150 MB | aehnlich tiny |
| base | `ggml-base.bin` | 148 MB | ~388 MB | ~1.5-2x tiny |
| base.en | `ggml-base.en.bin` | 148 MB | ~388 MB | -- |
| base-q5_1 | `ggml-base-q5_1.bin` | 59.7 MB | ~200 MB | -- |
| small | `ggml-small.bin` | 488 MB | ~852 MB | ~5-8x tiny |
| small-q5_1 | `ggml-small-q5_1.bin` | 190 MB | ~450 MB | -- |
| medium | `ggml-medium.bin` | 1.53 GB | ~2.1 GB | ~19s (Ryzen 5 4500U) |

*Encode-Zeit = Verarbeitungszeit fuer ~30s Audio-Chunk auf CPU, 8 Threads

Direktdownload-URL-Muster:
```
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin
```

**Empfehlung fuer Dikta-Offline-Modus:**
- **`ggml-base.bin`** (148 MB): Bestes Qualitaets-/Groessen-Verhaeltnis fuer Diktat. Laeuft auf jedem Rechner.
- **`ggml-tiny-q5_1.bin`** (32 MB): Minimale Groesse, niedrigste RAM-Last, fuer schwache Hardware.
- **`ggml-small.bin`** nicht empfohlen: Zu gross fuer integrierten Download, Nutzer kann selbst waehlen.

### Performance-Benchmarks (whisper.cpp, CPU vs GPU)

**CPU-Benchmarks (Encode-Zeit fuer 30s Audio, whisper.cpp intern):**

| Hardware | Modell | Threads | Encode (ms) | Real-Time-Faktor |
|----------|--------|---------|-------------|-----------------|
| MacBook M1 Pro | tiny | 8 | 102 | ~294x |
| MacBook M1 Pro | base | 8 | 220 | ~136x |
| MacBook M1 Pro | small | 8 | 685 | ~44x |
| MacBook M1 Pro | medium | 8 | 1.928 | ~16x |
| Ryzen 9 5950X | tiny | 8 | 197 | ~152x |
| Ryzen 9 3900X | tiny | 8 | 422 | ~71x |
| i7-11800H (WSL2) | tiny | 8 | 620 | ~48x |
| i7-4790K | tiny.en | 4 | 808 | ~37x |
| Ryzen 5 4500U | medium.en | 6 | 19.673 | ~1.5x (grenzwertig!) |

**GPU (CUDA) -- whisper.cpp Benchmarks:**

| Hardware | Modell | Encode (ms) | Speedup vs CPU |
|----------|--------|-------------|---------------|
| RTX 3090 | tiny.en | ~50ms | ~6-8x vs i9 CPU |
| RTX 3090 | base.en | ~80ms | ~8-10x vs CPU |
| RTX 4080 | small | ~200ms | ~10-15x vs i7 CPU |

**Praktische Faustregeln fuer Dikta:**
- **tiny auf moderner CPU (i7/Ryzen 5+):** ~600-800ms pro 30s Clip -- fuer Diktat-Usecase absolut ausreichend (Nutzer spricht selten laenger als 30s am Stueck)
- **base auf moderner CPU:** ~1-2s fuer 30s Audio -- noch akzeptabel
- **GPU aktiviert (CUDA):** 5-10x schneller als CPU, tiny wird unter 100ms
- **Wichtig:** Bei unserem Use-Case (Diktat-Chunks, typisch 5-15s) skalieren die Zeiten linear mit Audio-Laenge -- tiny.bin auf einem i7 macht 15s Audio in ~300-400ms

### Limits & Kosten
- **Kosten:** Einmalig Modell herunterladen (148 MB fuer base), danach 100% kostenlos, kein Internet noetig.
- **RAM:** base belegt ~388 MB RAM waehrend Inferenz -- kein Problem auf modernen Rechnern.
- **Keine Rate Limits:** Lokale Inferenz, unbegrenzte Anfragen.
- **Modell-Kompatibilitaet:** GGML-Format ist an whisper.cpp gebunden. Nicht kompatibel mit HuggingFace Transformers.

### Gotchas
- **WhisperContext NICHT per Request neu laden.** Laden kostet 100-200ms + RAM-Allokation. Einmalig laden, in `Arc<Mutex<_>>` halten, State (`create_state()`) pro Request neu erzeugen.
- **Audio muss exakt 16kHz Mono f32 sein.** cpal liefert oft i16 -- `convert_integer_to_float_audio()` nutzen. Bei Stereo: `convert_stereo_to_mono_audio()` vorher.
- **Windows-Build mit CUDA:** `CUDA_PATH` env-var muss gesetzt sein, CMake muss im PATH liegen. Ohne CUDA einfach `cuda`-Feature weglassen -- dann reine CPU-Version.
- **`set_language(Some("de"))` explizit setzen!** Default ist English-Erkennung. Bei deutschem Input ohne Language-Flag koennen Fehler entstehen.
- **`set_initial_prompt` fuer Dictionary:** Fachbegriffe/Namen aus dem Dikta-Dictionary als initial_prompt mitgeben verbessert Transkription (wie bei Groq-API). Max ~200 Token.
- **Bindings-Generierung schlaegt fehl?** `WHISPER_DONT_GENERATE_BINDINGS=1` als env-var setzen, dann werden die gecachten Bindings aus dem Crate genutzt.
- **Repo auf Codeberg migriert:** Seit Juli 2025 ist https://github.com/tazz4843/whisper-rs archiviert. Issues/PRs nur noch auf Codeberg: https://codeberg.org/tazz4843/whisper-rs
- **Android:** whisper-rs funktioniert technisch als Rust-Crate auch im Android-Build (NDK), aber der GGML-Build fuer aarch64 braucht extra cmake-Konfiguration. Fuer MVP besser: Groq API auf Android, whisper-rs nur Desktop.
