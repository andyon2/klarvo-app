===============================================================================
  DIKTA v0.4.0 -- Voice Dictation with AI Cleanup
  Eine freie Alternative zu Wispr Flow
===============================================================================

SCHNELLSTART
------------
1. Dikta starten (Desktop-Shortcut oder Startmenue)
2. Beim ersten Start: Onboarding-Wizard fuehrt durch die Einrichtung
3. Mindestens einen API-Key eintragen (Empfehlung: Groq -- kostenlos!)
   -> https://console.groq.com (Account erstellen, API Key generieren)
4. Hotkey druecken (Standard: Ctrl+Shift+D), sprechen, loslassen
5. Der bereinigte Text wird automatisch ins aktive Fenster eingefuegt

BEDIENUNG
---------
  Ctrl+Shift+D (halten)   Aufnahme starten, loslassen = transkribieren
  Ctrl+Shift+D (Toggle)   Einmal druecken = Start, nochmal = Stopp
  Ctrl+Shift+E            Command Mode: Text markieren, Sprachbefehl geben

  Der Hotkey und der Modus (Hold/Toggle) sind in den Settings aenderbar.

FEATURES
--------
  - Sprache-zu-Text in jedem Textfeld (systemweit)
  - KI-Textbereinigung (Fuellwoerter, Grammatik, Interpunktion)
  - 3 Cleanup-Stile: Polished / Clean / Raw
  - Live-Uebersetzung in 13 Sprachen
  - Multi-Format-Output: Email, Aufzaehlung, Zusammenfassung
  - Command Mode: Markierten Text per Sprache umschreiben
  - Whisper Mode: Verstaerkung fuer leises Diktieren
  - Voice Notes: Sprachnotizen speichern statt einfuegen
  - Text Snippets: Textbausteine per Klick einfuegen
  - App Profiles: Einstellungen pro Anwendung
  - History mit Volltextsuche
  - Nutzungsstatistiken und Kostentracking
  - Webhook/API-Export nach jedem Diktat
  - Advanced Settings fuer Power-User (26 Optionen)

API-KEYS
--------
Dikta braucht API-Keys fuer die Sprach- und Texterkennung.
Alle Keys bleiben lokal auf deinem Rechner (in %APPDATA%\com.dikta.voice).

  Empfohlene Kombination (kostenlos/guenstig):
    STT:  Groq Whisper   -> https://console.groq.com
    LLM:  DeepSeek       -> https://platform.deepseek.com

  Weitere unterstuetzte Provider:
    STT:  OpenAI Whisper  -> https://platform.openai.com
    LLM:  OpenAI GPT      -> https://platform.openai.com
          Anthropic Claude -> https://console.anthropic.com
          Groq Llama       -> https://console.groq.com

  Die Provider-Prioritaet ist per Drag & Drop in den Settings aenderbar.
  Der erste Provider mit gueltigem Key wird verwendet.

SYSTEM-TRAY
-----------
Dikta minimiert sich in den System-Tray (Infobereich der Taskleiste).
  - Linksklick: Hauptfenster oeffnen
  - Rechtsklick: Menue mit Settings und Beenden

DATEIEN
-------
  Einstellungen:  %APPDATA%\com.dikta.voice\config.json
  Woerterbuch:    %APPDATA%\com.dikta.voice\dictionary.json
  History (DB):   %APPDATA%\com.dikta.voice\history.db

PROBLEMLOESUNG
--------------
  - Kein Sound? Mikrofon-Berechtigung in Windows-Einstellungen pruefen
  - API-Fehler? Key in Settings pruefen, ggf. neuen Key generieren
  - Zu kurze Aufnahmen? Mindestdauer ist 500ms (aenderbar in Advanced Settings)
  - Text wird nicht eingefuegt? "Auto Paste" in Advanced Settings pruefen

KONTAKT
-------
  GitHub: https://github.com/andyon2/dikta

===============================================================================
