# API-Provider -- Dikta

## Groq -- Speech-to-Text (Whisper API)

### Ueberblick
Groq hostet OpenAI Whisper-Modelle auf ihrer schnellen LPU-Inferenz-Hardware. Extrem niedrige Latenz fuer STT.

### Setup
- API-Key: https://console.groq.com/keys
- Base-URL: `https://api.groq.com/openai/v1`
- Header: `Authorization: Bearer {api_key}`

### Relevanter Endpoint

**POST /audio/transcriptions**
```
Content-Type: multipart/form-data

file: [audio-datei, max 25MB]
model: "whisper-large-v3-turbo"  (schnellstes Modell)
language: "de" | "en"  (optional, verbessert Genauigkeit)
response_format: "json" | "text"
```

Response (json):
```json
{
  "text": "Der transkribierte Text..."
}
```

### Modelle
- `whisper-large-v3-turbo` -- Empfohlen: schnellstes, gute Qualitaet
- `whisper-large-v3` -- Etwas langsamer, marginal besser

### Limits & Kosten
- TODO: Aktuelle Pricing recherchieren mit /research-api
- Free Tier vorhanden (Rate Limits beachten)

### Gotchas
- Audio muss als Datei geschickt werden (nicht als Stream)
- Max 25MB pro Request
- Unterstuetzte Formate: mp3, mp4, mpeg, mpga, m4a, wav, webm

---

## DeepSeek -- Text-Cleanup (Chat API)

### Ueberblick
DeepSeek bietet guenstige LLM-Inferenz. Wir nutzen es fuer die Text-Bereinigung nach der Transkription.

### Setup
- API-Key: https://platform.deepseek.com/api_keys
- Base-URL: `https://api.deepseek.com/v1`
- Header: `Authorization: Bearer {api_key}`

### Relevanter Endpoint

**POST /chat/completions**
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

Response:
```json
{
  "choices": [
    {
      "message": {
        "content": "Der bereinigte Text..."
      }
    }
  ]
}
```

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

### Limits & Kosten
- TODO: Aktuelle Pricing recherchieren mit /research-api
- Deutlich guenstiger als OpenAI/Anthropic

### Gotchas
- Latenz hoeher als Groq (kein LPU), aber fuer Cleanup akzeptabel
- Temperature niedrig halten (0.2-0.4) -- wir wollen treue Bereinigung, keine Kreativitaet
