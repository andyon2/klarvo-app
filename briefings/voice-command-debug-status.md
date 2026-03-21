# Voice Command Mode — Debug-Status (2026-03-21)

## Was implementiert ist (alles kompiliert, 427 Tests gruen)

### Neue Dateien
- `src-tauri/src/voice_command/mod.rs` — VoiceCommandEngine + recognize_command (40 Tests)
- `src-tauri/src/voice_command/dispatch.rs` — Pipeline-Integration, Groq-Anbindung, Debounce
- `src-tauri/src/stt/hallucination.rs` — Refactored Blocklist (18 Tests)
- `src-tauri/src/commands/voice_command.rs` — toggle_voice_command_mode, get_voice_command_active

### Geaenderte Dateien
- `src-tauri/src/audio/mod.rs` — Monitor-Modus (start_monitor/stop_monitor/query_input_format)
- `src-tauri/src/lib.rs` — AppState.voice_command_active, Auto-Start im Setup, invoke_handler
- `src-tauri/src/config/mod.rs` — voice_command_enabled Feld
- `src-tauri/src/pipeline.rs` — Hallucination-Import refactored
- `src-tauri/src/llm/mod.rs` — Punctuation-Commands in allen 3 Cleanup-Prompts
- `src-tauri/src/license/mod.rs` — Legacy DIKTA-Keys akzeptieren (Prefix + HMAC)
- `src/components/SettingsPanel.tsx` — Voice Command Toggle + License-Input fuer DIKTA
- `src/tauri-commands.ts` — toggleVoiceCommandMode, getVoiceCommandActive
- `src/types.ts` — voiceCommandEnabled
- `android/kotlin-src/.../VoxlitApi.kt` — Punctuation-Commands in Kotlin-Prompts

## Offene Bugs (naechste Session)

### Bug 1: Phantom-Auto-Start
Config sagt `voiceCommandEnabled: false`, aber `[voice_command] Device format: 48000 Hz, 2 ch`
erscheint beim App-Start. Irgendwo wird `start_voice_command_monitor` aufgerufen obwohl die
Bedingung `vc_enabled == true` nicht erfuellt sein sollte.

**Debug-Ansatz:** eprintln in den Auto-Start-Block (lib.rs Z.790-799) einbauen:
`eprintln!("[setup] vc_enabled={vc_enabled}");` VOR dem if-Check.

### Bug 2: Toggle-Desync
UI-State (localVoiceCommandEnabled) und Backend-State (voice_command_active AtomicBool)
laufen auseinander. Ursachen:
- Auto-Start setzt Backend auf active=true, UI zeigt false (liest Config)
- Wenn Toggle klickt: Frontend denkt "aus" → versucht start → Backend sagt "already active" → Fehler → catch setzt UI auf false

**Fix-Ansatz:** Frontend soll beim Mount `get_voice_command_active()` aufrufen und den
Toggle-State daraus ableiten, nicht aus der Config.

### Bug 3: Build-Sync
`cargo build --release` (direkt) erzeugt Binary ohne Frontend-Assets. Danach erkennt
`tauri build` die Binary als aktuell und ueberspringt Neukompilierung.

**Fix:** Nie `cargo build` direkt aufrufen. Immer `sync-and-build.ps1`. Wenn Binary
gelockt: App schliessen, warten, dann bauen. Fingerprints loeschen wenn noetig:
`Remove-Item 'D:\Apps\voxlit\src-tauri\target\release\.fingerprint\voxlit-*' -Recurse -Force`

### Bug 4: Groq-Pfad nie getestet
Der Umbau von lokalem Whisper auf Groq API ist implementiert aber noch nie erfolgreich
end-to-end gelaufen. Erwartet nach Bug 1-3 Fix:
```
[voice_command] Using Groq Whisper for command recognition
[voice_command] SnippetReady: XXXX samples
[voice_command] Groq completed in 0.Xs
[voice_command] Whisper text: "Voxlit start"
[voice_command] Recognised command: StartDictation
```

## Debug-Reihenfolge
1. Bug 1 fixen (Phantom-Auto-Start) — sonst ist Bug 2 nicht testbar
2. Bug 2 fixen (Toggle-Desync) — Frontend-State aus Runtime, nicht Config
3. Sauberen Build machen (Bug 3 beachten)
4. Groq-Pfad end-to-end testen (Bug 4)
5. Wenn alles laeuft: eprintln durch log:: ersetzen

## Alle eprintln-Stellen (muessen nach Debug entfernt werden)
- `dispatch.rs`: Device format, Using Groq, SnippetReady, Processing snippet,
  Groq completed, Whisper text, No command recognised, Groq error, Tokio error
- Keine eprintln in mod.rs (nur Tests) oder anderen Dateien
