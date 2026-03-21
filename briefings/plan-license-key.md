# License-Key-System — Feature-Briefing

## Ziel

Open-Core-Modell: Free-Tier (Basis-Diktat) vs. Paid-Tier (EUR 29 Einmalkauf, alle Features).
License Key wird offline via HMAC-SHA256 validiert. Kein externer Server noetig fuer v1.

## Architektur

### Key-Format
`VOXLIT-XXXX-XXXX-XXXX-XXXX` (Base32-kodiert, 16 Bytes Payload + 8 Bytes HMAC-Truncated)

### Validierung
- HMAC-SHA256 mit im Binary eingebettetem Secret
- Secret als Kombination aus zwei Konstanten (erschwert String-Suche)
- Online-Validierung via Lemon Squeezy API spaeter als optionaler Bonus

### LicenseStatus Enum
```rust
pub enum LicenseStatus {
    Unlicensed,              // kein Key, Free-Tier
    GracePeriod { until: u64 }, // Key validiert, aber online-Check ausstaendig
    Licensed,                // vollstaendig validiert
}
```

### Offline-Toleranz
- Nach erfolgreicher Erstvalidierung: Status + Timestamp in config.json gecacht
- Cached License gilt 30 Tage ohne erneute Online-Pruefung
- Bei Ablauf: 48h Gnadenfrist, dann Downgrade auf Free-Tier (kein Datenverlust)

### Early-Adopter-Migration
- Bei App-Start: Wenn `license_key` Feld in Config FEHLT (= Bestandsnutzer vor License-System)
- → Setze automatisch 60 Tage Grace Period
- → Nach 60 Tagen: Downgrade auf Free-Tier mit Hinweis in UI

## Feature-Gating

### Free-Tier
- Kern-Diktat (Hotkey → Sprechen → Text)
- Ein STT-Provider (Groq Whisper)
- Ein LLM-Provider (DeepSeek)
- Ein Cleanup-Stil (Polished)
- Basis-Settings
- Limitierte History (letzte 50 Eintraege)

### Paid-Tier (EUR 29)
- Alle STT/LLM-Provider (OpenAI, Anthropic, Groq LLM, etc.)
- Alle Cleanup-Stile (Verbatim, Chat) + Custom Prompts
- Command Mode
- Text Snippets
- App Profiles
- Unbegrenzte History + Volltextsuche
- Voice Notes
- Cross-Device Sync (Turso)
- Offline-Modus (whisper.cpp, wenn implementiert)
- Whisper Mode (leises Diktieren)
- Filler-Word-Analyse + Kostentracking

### Feature-Gate-Pattern (Rust)
```rust
// Macro statt manueller Checks in jedem Command
macro_rules! require_license {
    ($state:expr, $feature:expr) => {
        let status = $state.license_status.lock().map_err(|_| "lock error")?;
        if !license::is_feature_allowed(&status, $feature) {
            return Err(format!("feature_requires_license:{:?}", $feature).into());
        }
    };
}
```

### LicensedFeature Enum
```rust
pub enum LicensedFeature {
    AlternativeProviders,   // nicht-Groq STT, nicht-DeepSeek LLM
    AllCleanupStyles,       // Verbatim, Chat
    CustomPrompts,          // Custom Cleanup Instructions
    CommandMode,
    Snippets,
    AppProfiles,
    UnlimitedHistory,       // > 50 Eintraege + Volltextsuche
    VoiceNotes,
    Sync,                   // Turso Cross-Device
    OfflineMode,            // whisper.cpp
    WhisperMode,
    FillerAnalysis,
    CostTracking,
}
```

## Config-Aenderungen

`config.json` erhaelt neue Felder (alle mit `#[serde(default)]`):
```json
{
  "license_key": "",
  "license_validated_at": 0
}
```

## Tasks

### Task 1: Rust — `license` Modul (Kern-Validierung)
Agent: rust-core
Neue Dateien: `src-tauri/src/license/mod.rs`, `src-tauri/src/commands/license.rs`
Geaenderte Dateien: `src-tauri/src/lib.rs`, `src-tauri/src/config/mod.rs`, `Cargo.toml`

### Task 2: Rust — Feature-Gates in bestehenden Commands
Agent: rust-core
Geaenderte Dateien: commands/recording.rs, commands/history.rs, commands/misc.rs, commands/settings.rs, pipeline.rs, hotkey/mod.rs

### Task 3: Frontend — License-Sektion in Settings
Agent: ui-dev
Geaenderte Dateien: SettingsPanel.tsx, tauri-commands.ts, types.ts, useSettings.ts

### Task 4: Frontend — Paid-Feature-Lockout UI
Agent: ui-dev
Geaenderte Dateien: SettingsPanel.tsx, AdvancedSettingsPanel.tsx, App.tsx

### Task 5: Android — License-Lesen + Feature-Gating
Agent: android-platform
Geaenderte Dateien: VoxlitApi.kt, VoxlitOverlayService.kt

## Reihenfolge
Task 1 zuerst → Task 2 + Task 3 parallel → Task 4 → Task 5
