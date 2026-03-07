# Android Platform Research -- Tauri v2 Android Setup

Stand: 2026-03-07. Reine Recherche, kein Code geschrieben.

## 1. Prerequisites

### Pflicht-Software

| Tool | Version | Anmerkung |
|------|---------|-----------|
| JDK | 17 (Temurin empfohlen) | Nicht 11, nicht 21 -- 17 ist der Community-Konsens |
| Android Studio | Aktuell | Bringt JBR (JetBrains Runtime) mit, nutzbar als JAVA_HOME |
| Android SDK | Build-Tools 34.0.0+ | via SDK Manager |
| Android NDK | r26d oder r27.x funktionieren; CLI erwartet moeglicherweise 29.x | via SDK Manager (Side by Side) |
| Android SDK Platform | API 24 Minimum (Android 7.0) | fuer die App selbst |

### Rust Targets (via rustup)
```bash
rustup target add aarch64-linux-android    # ARM 64-bit (moderne Phones)
rustup target add armv7-linux-androideabi  # ARM 32-bit (aeltere Phones)
rustup target add i686-linux-android       # x86 32-bit (Emulator)
rustup target add x86_64-linux-android     # x86 64-bit (Emulator)
```

### Mindest-Rust-Version
- Tauri-Plugins: >= 1.77.2
- cpal AAudio-Backend (fuer Mikrofon in Rust): >= 1.82

### Environment Variables
```bash
export JAVA_HOME="$HOME/android-studio/jbr"
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)"
```
KRITISCH: In ~/.bashrc schreiben. Nur in der Session setzen reicht nicht -- tauri android
init prueft diese Variablen und schlaegt mit kryptischen Fehlern fehl wenn sie fehlen.

### WSL2-Zusatz (Diktas Dev-Umgebung!)
```bash
export WSL_HOST=$(tail -1 /etc/resolv.conf | cut -d' ' -f2)
export ADB_SERVER_SOCKET=tcp:$WSL_HOST:5037
```
ADB muss auf der Windows-Seite laufen (`adb start-server` in PowerShell).
Fuer physische Geraete via USB: usbipd-win installieren auf Windows.

---

## 2. Tauri Android Init

```bash
# Im Projektroot (wo tauri.conf.json liegt):
npx tauri android init
```

Was passiert:
- Liest identifier aus tauri.conf.json (`com.dikta.voice`)
- Generiert vollstaendiges Android Studio Projekt unter src-tauri/gen/android/
- Fuegt minimale Permissions hinzu (INTERNET, WAKE_LOCK)
- Erstellt tauri.properties fuer Gradle-Integration

### Generierte Struktur

```
src-tauri/gen/android/
  app/
    src/main/
      java/com/dikta/voice/
        MainActivity.kt          -- Einziger Einstiegspunkt fuer Tauri-Activity
        generated/               -- Auto-generiert, nicht manuell bearbeiten
      res/
        values/
          strings.xml
        drawable/
          ...
      AndroidManifest.xml        -- HIER Permissions eintragen
    build.gradle
    tauri.properties             -- Gradle-Tauri-Bridge Konfiguration
  build.gradle
  gradle/
  settings.gradle
```

### Was manuell nachbearbeitet werden muss
In gen/android/app/src/main/AndroidManifest.xml:
```xml
<!-- Pflicht fuer Dikta: -->
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.MODIFY_AUDIO_SETTINGS" />
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />

<!-- Fuer den IME-Service: -->
<service
    android:name=".VoiceInputService"
    android:permission="android.permission.BIND_INPUT_METHOD"
    android:exported="true">
    <intent-filter>
        <action android:name="android.view.InputMethod" />
    </intent-filter>
    <meta-data
        android:name="android.view.im"
        android:resource="@xml/method" />
</service>
```

---

## 3. Invoke-Commands auf Android -- Gleich wie Desktop

Das Frontend-invoke()-API ist 1:1 identisch zu Desktop. Kein Unterschied fuer React-Code:

```typescript
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('transcribe_audio', { audio: audioData });
```

Der Rust-Backend-Code mit `#[tauri::command]` laeuft unveraendert -- Tauri marshallt
automatisch ueber die JNI-Bridge.

EINZIGE Ausnahme: Das Rust main entry point braucht auf Mobile:
```rust
#[tauri::mobile_entry_point]
pub fn run() { ... }
```
statt der ueblichen `fn main()` fuer Desktop.

---

## 4. IME-Koexistenz mit Tauri Activity -- Machbar, aber mit Einschraenknungen

### Was funktioniert
- InputMethodService und Tauri-MainActivity koennen im selben APK sein.
- Das ist sogar der Standard-IME-Ansatz (IME-App hat typisch auch eine Settings-Activity).
- Der Service wird im AndroidManifest registriert, voellig unabhaengig von der Activity.

### Das fundamentale Problem
Der IME-Service hat KEINEN Zugriff auf die Tauri-WebView-Laufzeit. Tauri lebt in der
MainActivity -- der IME-Service ist ein separater Android-Prozesskontext.

Das bedeutet: `invoke()` aus dem IME heraus funktioniert NICHT.

### Loesungsoptionen

**Option A: Kotlin IME macht HTTP-Calls direkt (empfohlen fuer MVP)**
- IME nimmt Audio auf (Android AudioRecord API in Kotlin)
- IME sendet WAV direkt an Groq-API via Kotlin-HTTP (OkHttp oder Ktor)
- IME fuegt transkribierten Text via InputConnection ein
- Keine Bridge zu Tauri noetig
- Nachteil: Kotlin-Duplikation von API-Client-Code, API-Key-Verwaltung komplex

**Option B: Android IPC (Broadcast Intent / Bound Service / AIDL)**
- IME sendet Audio-Daten via Broadcast Intent oder Bound Service an Tauri-Activity
- Tauri-Activity verarbeitet (invoke -> Rust -> STT -> LLM -> Ergebnis)
- Ergebnis kommt per IPC zurueck zum IME
- Nachteil: Komplex, Latenz durch IPC, Activity muss im Hintergrund laufen

**Empfehlung: Option A fuer Phase 1**
IME in Kotlin direkt, kein Tauri-Bridge. Tauri-App bleibt fuer Settings/History-UI.
Diese Architektur-Entscheidung muss in knowledge/architecture.md dokumentiert werden.

---

## 5. Audio-Aufnahme -- Kritische Erkenntnisse

### WebView (getUserMedia) -- Problematisch
- `navigator.mediaDevices.getUserMedia()` aus dem Tauri-WebView:
  - Braucht RECORD_AUDIO UND MODIFY_AUDIO_SETTINGS im Manifest (nicht nur RECORD_AUDIO!)
  - Fehlt MODIFY_AUDIO_SETTINGS: "not-allowed" Error -- Bug wurde 2024 reported und ist jetzt dokumentiert
  - WebView-Permissions sind NICHT persistent -- User muss bei jedem App-Start neu bestaetigen
  - Format: WebM/Opus aus getUserMedia -- nicht optimal fuer Groq (WAV bevorzugt)

### Kotlin AudioRecord API -- Empfohlen fuer IME
- Direkte Android-API, volle Kontrolle ueber Format (16kHz, 16-bit, Mono WAV)
- Kein Permission-Persistenz-Problem (Runtime-Permission wird normal von Android gehandhabt)
- Aus IME-Service heraus direkt nutzbar
- Braucht `FOREGROUND_SERVICE` + `FOREGROUND_SERVICE_MICROPHONE` (Android 14+) als Service

### cpal auf Android -- Moeglich aber unpraktisch
- cpal 0.15 unterstuetzt Android via AAudio-Backend
- Minimum: Android API 26 (Android 8.0+), Rust >= 1.82
- Problem: cpal ist fuer die Tauri-Rust-Seite, NICHT fuer den IME-Service (der in Kotlin ist)
- Die bestehende cpal-Implementierung laeuft im Tauri-Rust-Backend -- auf Android haette
  die MainActivity-Seite Mikrofon-Zugriff, nicht der IME-Service

---

## 6. Bekannte Limitierungen und Gotchas

### Plugin-Inkompatibilitaeten (betrifft Dikta direkt!)
| Plugin | Android | Auswirkung auf Dikta |
|--------|---------|----------------------|
| tauri-plugin-global-shortcut | NEIN | Muss mit cfg(target_os) ausgeschlossen werden |
| tauri-plugin-updater | NEIN | Muss mit cfg(target_os) ausgeschlossen werden |
| tauri-plugin-opener | NEIN | Muss mit cfg(target_os) ausgeschlossen werden |
| tray-icon Feature | NEIN | Muss mit cfg(target_os) ausgeschlossen werden |
| windows crate | NEIN (offensichtlich) | Bereits in [target.cfg(windows)] |

Ohne diese cfg-Guards: Compile-Fehler beim Android-Build!

### Sonstige Gotchas
- **Plugin-Namen**: Keine Bindestriche! `my-plugin` funktioniert nicht, `myplugin` schon.
- **Reinstall-Bug**: Bei physischen Geraeten schlaegt Reinstall nach Rust-Aenderungen fehl
  (falsches adb-Flag). Workaround: App vorher manuell deinstallieren. (Issue #9067)
- **cpal auf Android braucht API 26+**: Unser Manifest-Minimum ist API 24. Falls wir
  cpal fuer Android-Audio nutzen wollen, muss minSdkVersion auf 26 angehoben werden.
- **OpenSSL-Compile-Fehler**: reqwest mit default-features kann OpenSSL-Probleme beim
  Android-Cross-Compile machen. Loesung: `features = ["json", "multipart", "rustls-tls"]`
  statt OpenSSL.
- **Keine automatische Play-Store-Veröffentlichung**: Tauri hat kein automatisiertes
  Publish-Tool. Erster Upload muss manuell sein.
- **versionCode**: Wird automatisch berechnet. 0.4.0 -> versionCode 4000. OK.

### WebView-Limitierungen
- Kein `<input type="file" accept="directory">` -- Verzeichnis-Picker nicht implementiert
- Systemwebview-Version variiert stark je nach Android-Version und OEM
- Einige CSS-Features fehlen in aelteren System-WebViews

---

## 7. Build und Test Workflow

### Entwicklung
```bash
# Emulator starten (automatisch wenn kein USB-Geraet):
npx tauri android dev

# Spezifisches Geraet:
npx tauri android dev [DEVICE-ID]

# Android Studio oeffnen:
npx tauri android dev --open
```

Devtools: chrome://inspect/#devices im Chrome-Browser (gleicher Rechner).
Hot-Reload: Rust-Aenderungen triggern Rebuild + Reinstall. Frontend-Aenderungen via Vite-HMR.

### Release Build
```bash
# APK fuer Sideload/Testing:
npx tauri android build -- --apk

# AAB fuer Google Play:
npx tauri android build -- --aab

# Nur ARM64 (spart Zeit):
npx tauri android build --target aarch64
```

Artefakte landen in: `src-tauri/gen/android/app/build/outputs/`

---

## 8. Strategische Einschaetzung fuer Dikta

### Was einfach ist
- Frontend-Code (React) laeuft unveraendert in Tauri-WebView
- invoke()-Calls zu Rust-Befehlen (transcribe, cleanup) laufen unveraendert
- Settings-UI und History-Anzeige sind triviale Ports

### Was hard ist
- **IME + Tauri-Bridge**: Kein direkter Weg. IME muss standalone in Kotlin.
- **Cargo.toml aufraaeumen**: global-shortcut, updater, tray-icon, windows-crate
  muessen mit cfg-Guards versehen werden, sonst kein Android-Build.
- **cpal auf Android**: Entweder API auf 26 anhebn, oder fuer Android einen anderen
  Audio-Recorder verwenden (AudioRecord in Kotlin oder ein Tauri-Plugin).
- **WSL2-Setup**: ADB-Bridging ist frickelig. Empfehlung: Direkt auf Windows-Host bauen
  oder CI/CD (GitHub Actions) fuer Android-Builds nutzen.

### Empfohlene Android-Architektur
```
Dikta Android APK:
  MainActivity (Tauri WebView)
    - Settings, API-Key-Management, History
    - invoke() -> Rust Backend (STT/LLM via Groq-API)
    - NICHT fuer Keyboard-Funktion

  VoiceInputService (Kotlin IME)
    - System-Keyboard-Registrierung
    - AudioRecord direkt in Kotlin
    - HTTP-Calls direkt an Groq (Kotlin OkHttp/Ktor)
    - Text einfuegen via InputConnection
    - Teilt nur Settings (SharedPreferences/Datei) mit MainActivity
```

### Offene Risiken
1. Wird der Android-Build mit den aktuellen Cargo.toml-Dependencies ueberhaupt kompilieren?
   (reqwest, cpal, windows-crate sind potenzielle Probleme)
2. Wie komplex ist der API-Key-Austausch zwischen IME und MainActivity?
3. WSL2 Build-Workflow -- muss getestet werden (ADB-Bridge-Konfiguration)
