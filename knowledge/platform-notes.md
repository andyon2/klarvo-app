# Plattform-Notizen -- Dikta

Hier werden plattformspezifische Quirks, Workarounds und Lessons Learned dokumentiert.

## Windows

### Text-Paste via SendInput
- Windows nutzt `SendInput` API fuer Ctrl+V Simulation
- Reihenfolge: Text in Clipboard -> SendInput(Ctrl Down, V Down, V Up, Ctrl Up)
- **Terminal-Erkennung noetig:** PowerShell, Windows Terminal, Git Bash nutzen Ctrl+Shift+V
- OpenWhispr hat einen nativen C-Binary dafuer (`windows-fast-paste`) -- als Referenz nutzen
- Alternative: `clipboard` Crate + `enigo` Crate fuer Keyboard-Simulation

### Globaler Hotkey
- `RegisterHotKey` Win32 API in einem separaten Thread
- Braucht ein unsichtbares Window fuer die Message-Loop
- Alternative: `global-hotkey` Crate (cross-platform, nutzt Tauri intern auch)

### GPU-Erkennung (Akku-Modus)
- `windows::System::Power::PowerManager` fuer Akku-Status
- Oder einfacher: `SYSTEM_POWER_STATUS` via Win32 API

## Android

### InputMethodService (IME)
- Der IME ist ein Service, kein Activity -- hat eigenen Lifecycle
- `onCreateInputView()` erstellt die Keyboard-UI
- `currentInputConnection.commitText(text, 1)` fuegt Text ein
- User muss Dikta manuell als System-Keyboard aktivieren (Settings -> Language & Input)
- Seit Android 11: `InputMethodManager.showInputMethodPicker()` kann Wechsel-Dialog zeigen

### Permissions
- `RECORD_AUDIO`: Muss zur Laufzeit angefragt werden (Android 6+)
- `POST_NOTIFICATIONS`: Muss zur Laufzeit angefragt werden (Android 13+)
- `FOREGROUND_SERVICE`: Manifest-Declaration reicht
- `FOREGROUND_SERVICE_MICROPHONE`: Noetig ab Android 14 fuer Mikrofon in Foreground Services

### Battery / Background
- Android killt Background-Services aggressiv (besonders Samsung, Xiaomi)
- ForegroundService mit Notification waehrend Aufnahme ist Pflicht
- Kein permanenter Service wenn nicht aufgenommen wird
- Doze-Mode: Kein Netzwerk moeglich -> STT/LLM-Calls scheitern -> Fallback benoetigt oder Doze-Whitelist

### Tauri v2 auf Android -- Recherche-Ergebnisse (2026-03-07)

#### Prerequisites (genaue Versionen)
- **JDK 17** (empfohlen, Temurin Distribution in CI-Workflows)
- **NDK**: kein exaktes "offizielles" Minimum, NDK r26d/27.x funktionieren laut Community.
  DeepWiki-Quelle nennt NDK 29.0.13846066 + SDK Platform 36 (moeglicherweise aus
  tauri-cli-Quellcode). Sicher ist: immer NDK via SDK Manager installieren, nicht manuell.
- **SDK**: Minimum API 24 (Android 7.0) fuer die App; Build-Tools 34.0.0 empfohlen.
- **Rust targets**: aarch64-linux-android, armv7-linux-androideabi, i686-linux-android, x86_64-linux-android
- **Rust-Version**: >= 1.77.2 (Tauri-Pflicht), >= 1.82 noetig fuer cpal AAudio-Backend

#### Environment Variables
```bash
export JAVA_HOME="$HOME/android-studio/jbr"  # oder wo Android Studio JBR liegt
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)"  # auto-detect
```
WICHTIG: In ~/.bashrc oder ~/.zshrc schreiben, sonst gehen sie nach Session-Ende verloren!

#### Projekt-Setup
```bash
npx tauri android init   # generiert src-tauri/gen/android/
```
Generierte Struktur:
```
src-tauri/gen/android/
  app/
    src/main/
      java/{com.dikta.voice}/
        MainActivity.kt
        generated/
      res/
      AndroidManifest.xml    # INTERNET + WAKE_LOCK vorbelegt, weitere manuell adden
    build.gradle
    tauri.properties
  build.gradle
  gradle/
```

#### invoke() auf Android -- identisch zu Desktop
Das Frontend-invoke()-API ist 1:1 gleich auf Android. Tauri marshallt automatisch:
```javascript
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('my_command', { arg: 'value' });
```
Der Rust-Backend-Code mit `#[tauri::command]` laeuft unveraendert.
ACHTUNG: Mobile braucht `#[tauri::mobile_entry_point]` statt des Desktop-Patterns.

#### Native Kotlin-Plugins (Tauri Plugin Bridge)
Kotlin-Klasse muss `@TauriPlugin` annotiert sein und `Plugin(activity)` extenden:
```kotlin
@TauriPlugin
class ExamplePlugin(private val activity: Activity): Plugin(activity) {
  @Command
  fun doSomething(invoke: Invoke) {
    val args = invoke.parseArgs(MyArgs::class.java)
    val ret = JSObject()
    ret.put("result", "value")
    invoke.resolve(ret)
  }
}
```
Rust ruft Kotlin an via `run_mobile_plugin()`:
```rust
self.0.run_mobile_plugin("doSomething", payload).map_err(Into::into)
```
Plugin-Erstellung: `tauri plugin new --android my-plugin` -- KEINE Bindestriche im Namen!

#### IME + Tauri Activity -- Koexistenz moeglich
Android erlaubt InputMethodService und Activity im selben APK. Standard-IME-Architektur
hat bereits eine Settings-Activity. Tauri-App ist die Activity, IME ist ein separater Service.
RISIKO: Der IME-Service hat KEINEN Zugriff auf Tauri-Laufzeit/WebView. Der IME muss also
entweder (a) Kotlin-nativ die REST-APIs direkt aufrufen oder (b) via Android IPC
(Broadcast/Messenger/AIDL) mit der Tauri-Activity kommunizieren.
Option (b) ist komplex. Option (a) ist einfacher: Kotlin IME macht HTTP-Calls direkt.

#### Audio-Aufnahme in Tauri Android -- KRITISCHE EINSCHRAENKUNGEN
- `navigator.mediaDevices.getUserMedia()` aus dem WebView funktioniert, aber:
  - Braucht `<uses-permission android:name="android.permission.RECORD_AUDIO" />`
  - UND `<uses-permission android:name="android.permission.MODIFY_AUDIO_SETTINGS" />`
  - Ohne MODIFY_AUDIO_SETTINGS: "not-allowed" Fehler trotz RECORD_AUDIO! (Bug #10846, gefixt 2024)
  - WebView-Permissions werden NICHT persistent gespeichert -- erneute Abfrage bei App-Neustart
- cpal auf Android: Nutzt AAudio-Backend, minAPI 26 (Android 8+), Rust >= 1.82
  - cpal-crate kompiliert fuer Android, aber aus IME-Service heraus kaum nutzbar
  - Empfehlung fuer IME: Android AudioRecord API direkt in Kotlin

#### Plugin-Kompatibilitaet auf Android
- tauri-plugin-opener: NUR Desktop (kein Android/iOS)
- tauri-plugin-global-shortcut: NUR Desktop (kein Android/iOS)
- tauri-plugin-updater: Desktop-only (kein Android, kein iOS)
- tauri-plugin-http: Android unterstuetzt
- tauri-plugin-fs: Android unterstuetzt
- tauri-plugin-sql: Android unterstuetzt
- tauri-plugin-notification: Android unterstuetzt
KONSEQUENZ: tray-icon und global-shortcut in Cargo.toml muessen mit #[cfg(not(target_os="android"))]
bedingt kompiliert werden, sonst Build-Fehler!

#### Build & Test Workflow
- Emulator: `npx tauri android dev` startet Emulator automatisch wenn kein Device angeschlossen
- Physisches Geraet: USB Debug aktivieren, dann `npx tauri android dev [DEVICE-ID]`
- Devtools: chrome://inspect/#devices im Chrome-Browser auf dem Entwicklungsrechner
- AAB fuer Play Store: `npx tauri android build -- --aab`
- APK fuer Sideload: `npx tauri android build -- --apk`
- versionCode: Automatisch aus semantic version berechnet: major*1M + minor*1K + patch

#### WSL2-Spezifisches Setup
Fuer WSL2 (wie bei Dikta!) braucht man:
```bash
# Windows-IP ermitteln:
export WSL_HOST=$(tail -1 /etc/resolv.conf | cut -d' ' -f2)
# ADB-Verbindung zum Windows-ADB-Server:
export ADB_SERVER_SOCKET=tcp:$WSL_HOST:5037
```
Alternativ: `adb start-server` auf Windows-Seite, dann ADB-Befehle von WSL aus.
Bekanntes Problem: Bei physischen Geraeten schlaegt Reinstall fehl wegen falschem adb-Flag
(-s statt -r) -- Issue #9067, Stand 2024 noch offen.

### Floating Bubble Overlay (implementiert 2026-03-08)

#### Architektur-Entscheidung: Direkte Kotlin-Implementierung (kein Tauri-Bridge)
Der `DiktaOverlayService` ist komplett unabhaengig von der Tauri-Runtime. Er liest
`config.json` direkt aus `context.filesDir` (identischer Pfad den Tauri auch schreibt).
HTTP-Calls laufen via `java.net.HttpURLConnection` ohne OkHttp oder andere Abhaengigkeiten.

#### SYSTEM_ALERT_WINDOW Permission
- Muss in `onResume()` geprueft werden (nicht nur `onCreate()`!) -- User kann Permission
  nach dem ersten Start in den Settings wieder entziehen.
- Bei fehlendem Permission: `Settings.ACTION_MANAGE_OVERLAY_PERMISSION` oeffnen.
- TYPE_APPLICATION_OVERLAY (API 26+) ist der korrekte Window-Type fuer Overlays.
  Der alte TYPE_PHONE/TYPE_SYSTEM_OVERLAY-Typ ist seit Android 8 deprecated/blockiert.

#### ForegroundService mit Mikrofon
- `android:foregroundServiceType="microphone"` im Manifest noetig (Android 10+).
- `startForeground()` muss `FOREGROUND_SERVICE_TYPE_MICROPHONE` als dritten Parameter
  erhalten (Android 10+ = API 29+). Ohne: Crash mit `MissingForegroundServiceTypeException`.
- `FOREGROUND_SERVICE_MICROPHONE` Permission ist ab Android 14 (API 34) Pflicht.

#### AudioRecord direkt aus Service
- `AudioRecord` funktioniert problemlos aus einem `Service` heraus (nicht nur Activity).
- Minmal-Puffergroesse via `AudioRecord.getMinBufferSize()` abfragen, dann mindestens
  8192 Bytes nehmen (manche Geraete liefern sehr kleine Werte).
- PCM Short-Array (16-bit) direkt sammeln, dann erst beim Stop in WAV konvertieren.

#### Touch-Handling im WindowManager
- `FLAG_NOT_FOCUSABLE` noetig, damit Tastatur-Events nicht abgefangen werden.
- Drag vs. Tap: 10dp Schwelle (in Pixel umrechnen!). `event.rawX/rawY` statt `x/y`
  verwenden, sonst gibt es Drift beim Drag (rawX ist Bildschirmkoordinate).
- `windowManager.updateViewLayout()` fuer Echtzeit-Drag-Updates.

#### Kotlin-Warnung: unused variable in `when`-Block
Der Kotlin-Compiler warnt bei unbenutzten lokalen Variablen auch in Canvas-Zeichencode.
Einfach weglassen wenn nicht benoetigt (z.B. `micH` in `drawMicIcon`).

#### coroutines-android Dependency
Im Build wurde `kotlinx-coroutines-android:1.8.0` hinzugefuegt. Die Implementierung
nutzt aktuell `Thread {}` direkt (einfacher fuer Foreground-Service-Kontext).
Die Coroutines-Dependency schadet nicht, kann spater fuer `lifecycleScope.launch` genutzt
werden wenn ein LifecycleOwner verfuegbar ist.

#### Opacity-Slider im Long-Press-Menue (implementiert 2026-03-08)
- `bubbleView.alpha` (Android View-Property, 0.0..1.0) steuert die Transparenz.
- Der Wert wird als Integer 5..100 in SharedPreferences gespeichert (Key: `bubble_opacity`).
- SeekBar-Trick: `max = 95`, `progress = opacity - 5` ergibt Anzeige 5..100 ohne Float-Arithmetik.
- Live-Preview via `onProgressChanged(fromUser=true)` -- nur im IDLE-State anwenden,
  damit Recording/Processing immer alpha=1.0f haben (Status muss sichtbar bleiben).
- Persistenz erst in `onStopTrackingTouch` (Finger gehoben), nicht bei jedem Frame-Update.
- Der SeekBar laeuft in einem WindowManager-Overlay ohne Activity-Context -- das funktioniert,
  weil SeekBar nur den Service-Context benoetigt (kein Theme-Resolving ueber Activity noetig).
- `FLAG_NOT_FOCUSABLE` auf dem Overlay-Window wuerde Slider-Touch-Events blockieren.
  Das Menu-Window selbst hat `FLAG_NOT_FOCUSABLE`, aber Touch-Events auf dem SeekBar
  kommen trotzdem an, weil der SeekBar seine eigenen Motion-Events verarbeitet
  (kein Fokus noetig fuer Slider-Drag -- nur fuer Tastatur-Input).

## Beide Plattformen

### Audio-Formate
- Groq akzeptiert: mp3, wav, webm, m4a (max 25MB)
- whisper.cpp erwartet: 16kHz 16-bit mono WAV
- -> Audio immer als 16kHz mono WAV aufnehmen (funktioniert fuer beide)
- Fuer Groq: WAV direkt schicken (unter 25MB bei normaler Diktat-Laenge)

### Latenz-Budget
- Ziel: <2s von Sprach-Ende bis Text erscheint
- Audio-Stop: ~50ms
- Upload zu Groq: ~200-500ms (abhaengig von Dateigroesse/Verbindung)
- Groq STT: ~500-1000ms (sehr schnell)
- DeepSeek Cleanup: ~500-1500ms
- Paste: ~50ms
- **Gesamt: ~1.3-3s** -- akzeptabel, aber eng. Optimierungspotenzial: Streaming, paralleles Processing
