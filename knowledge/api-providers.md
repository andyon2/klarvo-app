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
