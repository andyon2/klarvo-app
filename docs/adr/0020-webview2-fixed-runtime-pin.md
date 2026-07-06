# ADR-0020: WebView2 auf eine gepinnte Fixed-Version-Runtime festnageln (Overlay-Occlusion-Regression)

**Status:** Accepted
**Date:** 2026-06-26

## Context

Die transparenten, always-on-top Desktop-Overlays — die „Pille" (`bar`, 200×38) und die Live-Preview (`preview`, 545×818) — **verschwinden, sobald eine andere App ihren Bildschirm-Bereich verdeckt** (also genau dann, wenn man in die App diktiert, in die man tippt). Symptom über Wochen: „Pille kommt kurz nach Neustart, ist nach ein paar Minuten weg, Neustart hilft nur kurz."

Der Bug wurde mehrfach für gefixt gehalten und kam jedes Mal zurück. Die Untersuchung am 2026-06-26 hat die Wurzel **gemessen** statt geraten:

- **Es ist kein Klarvo-Code-Bug.** Der vermeintliche Fix (`--disable-features=CalculateNativeWinOcclusion`, commit `2294008`) ist im Code und **live in allen Renderern** (Cmdline verifiziert) — der Bug tritt trotzdem auf. Der 06-21-„Fix" war ein transient-paint False-Positive (frisch gestartete Renderer malen bis zur ersten echten Verdeckung).
- **Die Overlays rendern, werden aber nicht präsentiert.** `PrintWindow(PW_RENDERFULLCONTENT)` liefert den vollen Pillen-Inhalt (Selbst-Render OK, `alpha=255`, nicht DWM-cloaked), während am selben Bildschirm-Ort die Vordergrund-App steht. Die Occlusion-Sperre sitzt **im WebView2-Compositor** (er liefert die Swapchain nicht an DWM, solange er das Fenster für verdeckt hält) — von der Fenster-Ebene aus nicht aufzubrechen (RedrawWindow / Topmost-Re-Assert / 1px-Nudge alle wirkungslos gemessen).
- **Mobile-Cockpit ist ausgeschlossen.** Dessen 1,5-s-`SetWindowPos(HWND_TOPMOST)`-Timer war ein plausibler Verdächtiger (Z-Order-Churn → Occlusion-Recalc); A/B-Messung mit beendetem Cockpit-Prozess → Pille weiter unsichtbar. Nicht der Auslöser.
- **Es ist eine Regression in der WebView2-Evergreen-Runtime.** A/B gemessen + von Andi am Bildschirm bestätigt: Klarvo via `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` auf Runtime **`149.0.4022.62`** gepinnt → Pille `screenTeal≈300` **sichtbar** über der Vordergrund-App; auf der installierten Evergreen **`.80`** in *jeder* Messung `0`. Die Regression liegt zwischen `.62` und `.69/.80`. Weil die Evergreen-Runtime sich selbst aktualisiert, ist „funktioniert tagelang, nach Reboot/Restart kaputt" genau erklärt: die laufende App behält ihre Runtime bis zum Neustart, zieht dann die neu-installierte, regressierte.

Die Evergreen-Runtime ist also eine **bewegliche Variable außerhalb unserer Kontrolle**, die das Verhalten eines Kern-Features (Overlay-Sichtbarkeit) bricht. Solange wir gegen Evergreen laufen, kann jeder MS-Patch das Feature erneut zerstören — der Grund, warum „gefixt" nie hielt.

## Decision

**Klarvo liefert eine feste WebView2-Runtime mit und nagelt sich darauf fest**, statt die auto-aktualisierende Evergreen-Runtime zu benutzen.

1. **Code-Pin (`src-tauri/src/lib.rs`, ganz oben in `pub fn run()`, vor `tauri::Builder::default()`):** Auf Windows wird `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` auf einen **exe-relativen** Ordner `webview2-runtime\` gesetzt, falls dort `msedgewebview2.exe` liegt. Maschinen-agnostisch (kein hardcoded Pfad), Fallback auf Evergreen falls die Runtime fehlt. Muss vor jeder Webview-Erzeugung laufen. Eine Observability-Zeile in `setup()` loggt die aktive Runtime (`[webview2] runtime: …`) in Klarvo.log.

2. **Gepinnte Version = `149.0.4022.62`** — die letzte Build vor der Regression. (Lag bereits auf der Maschine unter `EdgeWebView\Application\`; kein Download nötig.)

3. **Durability gegen „Fix verschwindet heimlich"** (die Fehlerklasse, die diesen Bug wochenlang am Leben hielt): Eine Master-Kopie der Runtime liegt **außerhalb des Build-Baums** (`D:\apps\klarvo-webview2-runtime`), und `scripts/sync-and-build.ps1` heilt den Build-Baum bei jedem Build daraus selbst (kopiert die Runtime nach `target\release\webview2-runtime`, falls ein voller `cargo clean` sie gelöscht hat) — und **warnt laut**, wenn beide fehlen, statt still auf Evergreen zurückzufallen.

## Consequences

- **Positiv:** Die Overlay-Sichtbarkeit ist nicht mehr von MS-Evergreen-Updates abhängig. Der Fix überlebt Neustart und normale Rebuilds (der WSL→D:-Sync schließt `target` aus; das Build-Script heilt nach `cargo clean`). Verifiziert: neuer Build **ohne** Env-Var gestartet → alle WebView2-Prozesse laufen aus `…\webview2-runtime`, Log bestätigt den Pin.
- **Preis:** Die gepinnte Runtime bekommt **keine automatischen (Sicherheits-)Updates** mehr. Bewusst akzeptiert — bei einem lokalen BYOK-Overlay-Tool ist die Overlay-Funktion wichtiger als Auto-Patching; Runtime-Bumps werden künftig **bewusst** gemacht (neue Version testen → Master-Kopie ersetzen → Build).
- **Größe:** +~180 MB Runtime neben der exe (bzw. im Installer).
- **Offen / Follow-up (siehe `docs/backlog.md`):**
  - Für die **distribuierte** App (NSIS/MSI, nicht nur der lokale Dev-Build): auf Tauris natives `bundle.windows.webviewInstallMode: { type: "fixedRuntime", path: … }` umstellen, damit der Installer die Runtime mitbringt — sauberer als die exe-relative Kopie für Endnutzer.
  - **Re-Verifikation über Zeit:** Andis „vorerst grün" ist noch kein Dauer-Beleg; bei Wiederauftreten zuerst Klarvo.log-`[webview2] runtime`-Zeile prüfen (Pin aktiv?), dann ob die Master-Kopie noch existiert.

Verwandt: Memory `project_webview2_overlay_backgrounding` (volle Mess-Saga + Probe-Skripte), `reference_dev_setup_topology` (Build via `sync-and-build.ps1`; robocopy braucht vollen Pfad aus WSL), `reference_hyperos_overlay_quirks` (Android-Overlay-Occlusion ist ein *anderes* Thema).
