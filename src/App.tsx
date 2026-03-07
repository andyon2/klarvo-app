import { useState, useCallback, useEffect, useRef } from "react";
import "./styles.css";
import type { RecordingState, CleanupStyle, HotkeyMode, AppSettings } from "./types";
import { STATUS_LABELS, STYLE_OPTIONS } from "./types";
import {
  startRecording,
  stopRecording,
  transcribeAudio,
  cleanupText,
  getSettings,
  saveSettings,
  getDictionaryTerms,
  addDictionaryTerm,
  removeDictionaryTerm,
  onStateChanged,
  setLanguage as syncLanguage,
  setCleanupStyle as syncCleanupStyle,
  listAudioDevices,
} from "./tauri-commands";

// --- Icons -------------------------------------------------------------------

function MicIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <rect x="5" y="5" width="14" height="14" rx="2" />
    </svg>
  );
}

function SpinnerIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white animate-spin"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
    >
      <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
      <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-4 h-4"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.01 7.01 0 0 0-1.62-.94l-.36-2.54A.484.484 0 0 0 14 2h-4a.484.484 0 0 0-.48.41l-.36 2.54a7.07 7.07 0 0 0-1.62.94l-2.39-.96a.49.49 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.27.41.49.41h4c.22 0 .43-.17.47-.41l.36-2.54a7.07 7.07 0 0 0 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
    </svg>
  );
}

// --- Sub-components ----------------------------------------------------------

interface RecordButtonProps {
  recordingState: RecordingState;
  onClick: () => void;
}

function RecordButton({ recordingState, onClick }: RecordButtonProps) {
  const isRecording = recordingState === "recording";
  const isBusy =
    recordingState === "transcribing" || recordingState === "cleaning";

  return (
    <button
      aria-label={
        isRecording
          ? "Stop recording"
          : isBusy
          ? "Processing audio"
          : "Start recording"
      }
      disabled={isBusy}
      onClick={onClick}
      className={[
        "relative flex items-center justify-center",
        "w-20 h-20 rounded-full",
        "transition-all duration-150",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
        "disabled:cursor-not-allowed disabled:opacity-60",
        isRecording
          ? "bg-red-600 hover:bg-red-500 shadow-[0_0_24px_rgba(239,68,68,0.5)]"
          : isBusy
          ? "bg-amber-500 shadow-[0_0_20px_rgba(245,158,11,0.4)]"
          : "bg-blue-600 hover:bg-blue-500 shadow-[0_0_16px_rgba(59,130,246,0.35)]",
      ].join(" ")}
    >
      {/* Pulse ring -- only while recording */}
      {isRecording && (
        <span className="absolute inset-0 rounded-full bg-red-500 opacity-40 animate-ping" />
      )}

      {isBusy ? (
        <SpinnerIcon />
      ) : isRecording ? (
        <StopIcon />
      ) : (
        <MicIcon />
      )}
    </button>
  );
}

interface StylePickerProps {
  value: CleanupStyle;
  onChange: (style: CleanupStyle) => void;
  disabled: boolean;
}

function StylePicker({ value, onChange, disabled }: StylePickerProps) {
  return (
    <div
      role="radiogroup"
      aria-label="Cleanup style"
      className="flex gap-1 bg-zinc-800 rounded-lg p-1"
    >
      {STYLE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          role="radio"
          aria-checked={value === opt.value}
          aria-label={`${opt.label}: ${opt.description}`}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          title={opt.description}
          className={[
            "flex-1 px-3 py-1.5 rounded-md text-xs font-medium",
            "transition-colors duration-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
            "disabled:cursor-not-allowed disabled:opacity-50",
            value === opt.value
              ? "bg-zinc-600 text-white"
              : "text-zinc-400 hover:text-zinc-200",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

// Formats a Tauri shortcut string for display: "ctrl+shift+d" -> "Ctrl+Shift+D".
function formatHotkeyDisplay(hotkey: string): string {
  return hotkey
    .split("+")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("+");
}

interface StatusBarProps {
  recordingState: RecordingState;
  errorMessage: string | null;
  hotkey: string;
  hotkeyMode: HotkeyMode;
}

function StatusBar({ recordingState, errorMessage, hotkey, hotkeyMode }: StatusBarProps) {
  const isError = recordingState === "error";
  const label =
    errorMessage && isError ? errorMessage : STATUS_LABELS[recordingState];
  const hotkeyDisplay = formatHotkeyDisplay(hotkey);

  return (
    <div className="flex items-center gap-2">
      <div
        aria-live="polite"
        aria-atomic="true"
        className={[
          "text-xs font-mono",
          isError ? "text-red-400" : "text-zinc-500",
        ].join(" ")}
      >
        {label}
      </div>
      {/* Hotkey hint -- always visible, low contrast so it doesn't distract */}
      <span
        aria-label={`Global hotkey: ${hotkeyDisplay} (${hotkeyMode})`}
        title={hotkeyMode === "hold" ? "Hold to record" : "Press to toggle recording"}
        className="text-xs font-mono text-zinc-700 select-none"
      >
        · {hotkeyDisplay} ({hotkeyMode})
      </span>
    </div>
  );
}

// Dot indicator for API key status -- green if the masked key is non-empty.
function StatusDot({ configured }: { configured: boolean }) {
  return (
    <span
      aria-label={configured ? "Configured" : "Not configured"}
      className={[
        "inline-block w-2 h-2 rounded-full flex-shrink-0",
        configured ? "bg-green-500" : "bg-red-500",
      ].join(" ")}
    />
  );
}

// Single dictionary term displayed as a removable chip/tag.
interface DictionaryTagProps {
  term: string;
  onRemove: (term: string) => void;
}

function DictionaryTag({ term, onRemove }: DictionaryTagProps) {
  return (
    <span className="inline-flex items-center bg-zinc-700 text-zinc-200 px-2 py-0.5 rounded-full text-xs">
      {term}
      <button
        aria-label={`Remove ${term}`}
        onClick={() => onRemove(term)}
        className={[
          "text-zinc-400 hover:text-red-400 ml-1",
          "focus:outline-none focus-visible:ring-1 focus-visible:ring-red-400/50 rounded-full",
          "transition-colors duration-100",
        ].join(" ")}
      >
        &times;
      </button>
    </span>
  );
}

// Captures a key combination on click and converts it to Tauri shortcut format.
// Requires at least one modifier key (ctrl/shift/alt/super) for a valid global shortcut.
interface ShortcutRecorderProps {
  value: string;
  onChange: (shortcut: string) => void;
}

function ShortcutRecorder({ value, onChange }: ShortcutRecorderProps) {
  const [listening, setListening] = useState(false);

  const handleClick = useCallback(() => setListening(true), []);

  useEffect(() => {
    if (!listening) return;

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Wait for a non-modifier key to finalize the combo.
      if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("ctrl");
      if (e.shiftKey) parts.push("shift");
      if (e.altKey) parts.push("alt");
      if (e.metaKey) parts.push("super");

      // Require at least one modifier -- bare keys can't be global shortcuts.
      if (parts.length === 0) return;

      // Map JS KeyboardEvent.code to Tauri shortcut key name.
      // Tauri uses lowercase for letters, but specific names for special keys.
      const KEY_MAP: Record<string, string> = {
        " ": "space", "Enter": "enter", "Escape": "escape", "Tab": "tab",
        "Backspace": "backspace", "Delete": "delete", "Insert": "insert",
        "Home": "home", "End": "end", "PageUp": "pageup", "PageDown": "pagedown",
        "ArrowUp": "up", "ArrowDown": "down", "ArrowLeft": "left", "ArrowRight": "right",
      };
      let key = KEY_MAP[e.key] ?? e.key.toLowerCase();
      // F-keys: JS gives "F1" etc, Tauri wants lowercase "f1"
      if (/^F\d+$/.test(e.key)) key = e.key.toLowerCase();
      parts.push(key);

      onChange(parts.join("+"));
      setListening(false);
    };

    document.addEventListener("keydown", handler, true);
    return () => document.removeEventListener("keydown", handler, true);
  }, [listening, onChange]);

  return (
    <button
      type="button"
      onClick={handleClick}
      aria-label={listening ? "Press a key combination" : `Current shortcut: ${value || "none"}`}
      className={[
        "w-full bg-zinc-800 border rounded-lg px-3 py-1.5 text-xs text-left",
        listening
          ? "border-blue-500 text-blue-400 animate-pulse"
          : "border-zinc-700 text-zinc-100 hover:border-zinc-500",
        "focus:outline-none transition-colors duration-100",
      ].join(" ")}
    >
      {listening ? "Press shortcut..." : value || "Click to set shortcut"}
    </button>
  );
}

interface SettingsPanelProps {
  onClose: () => void;
  language: string;
  cleanupStyle: CleanupStyle;
  hotkey: string;
  hotkeyMode: HotkeyMode;
  loadedSettings: AppSettings | null;
  dictionary: string[];
  onSave: (groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle, hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
}

function SettingsPanel({
  onClose,
  language,
  cleanupStyle,
  hotkey,
  hotkeyMode,
  loadedSettings,
  dictionary,
  onSave,
  onLanguageChange,
  onStyleChange,
  onAddTerm,
  onRemoveTerm,
}: SettingsPanelProps) {
  const [groqKey, setGroqKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [localLang, setLocalLang] = useState(language);
  const [localStyle, setLocalStyle] = useState<CleanupStyle>(cleanupStyle);
  const [localHotkey, setLocalHotkey] = useState(hotkey);
  const [localHotkeyMode, setLocalHotkeyMode] = useState<HotkeyMode>(hotkeyMode);
  const [localAudioDevice, setLocalAudioDevice] = useState<string | null>(loadedSettings?.audioDevice ?? null);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveDone, setSaveDone] = useState(false);
  const [newTerm, setNewTerm] = useState("");
  const [termError, setTermError] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Sync local copies when parent state changes (e.g. initial load finishes).
  useEffect(() => { setLocalLang(language); }, [language]);
  useEffect(() => { setLocalStyle(cleanupStyle); }, [cleanupStyle]);
  useEffect(() => { setLocalHotkey(hotkey); }, [hotkey]);
  useEffect(() => { setLocalHotkeyMode(hotkeyMode); }, [hotkeyMode]);
  useEffect(() => { setLocalAudioDevice(loadedSettings?.audioDevice ?? null); }, [loadedSettings?.audioDevice]);

  // Fetch available audio devices on mount.
  useEffect(() => {
    listAudioDevices().then(setAudioDevices).catch(console.error);
  }, []);

  // Close on Escape.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  const handleLangChange = useCallback((lang: string) => {
    setLocalLang(lang);
    onLanguageChange(lang);
  }, [onLanguageChange]);

  const handleStyleChange = useCallback((style: CleanupStyle) => {
    setLocalStyle(style);
    onStyleChange(style);
  }, [onStyleChange]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveError(null);
    setSaveDone(false);
    try {
      await onSave(groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice);
      // Clear key inputs after successful save -- masked placeholder will re-appear.
      setGroqKey("");
      setDeepseekKey("");
      setSaveDone(true);
      setTimeout(() => setSaveDone(false), 1500);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, onSave]);

  const handleAddTerm = useCallback(async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    setTermError(null);
    try {
      await onAddTerm(trimmed);
      setNewTerm("");
    } catch (err) {
      setTermError(err instanceof Error ? err.message : String(err));
    }
  }, [newTerm, onAddTerm]);

  const handleTermKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") handleAddTerm();
  }, [handleAddTerm]);

  // Derive whether keys are configured from the masked value.
  const groqConfigured = !!loadedSettings?.groqApiKeyMasked;
  const deepseekConfigured = !!loadedSettings?.deepseekApiKeyMasked;

  // Masked key as placeholder: show "****abcd" when configured, otherwise the usual hint.
  const groqPlaceholder = groqConfigured ? loadedSettings!.groqApiKeyMasked : "gsk_...";
  const deepseekPlaceholder = deepseekConfigured ? loadedSettings!.deepseekApiKeyMasked : "sk-...";

  return (
    <div
      ref={panelRef}
      className={[
        "w-full bg-zinc-900 border border-zinc-700 rounded-xl",
        "p-4 flex flex-col gap-3",
        "transition-all duration-200",
      ].join(" ")}
      role="region"
      aria-label="Settings"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
          Settings
        </span>
        <button
          aria-label="Close settings"
          onClick={onClose}
          className={[
            "text-zinc-500 hover:text-zinc-200",
            "transition-colors duration-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
            "rounded p-0.5",
          ].join(" ")}
        >
          <svg
            aria-hidden="true"
            className="w-3.5 h-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Scrollable body */}
      <div className="flex flex-col gap-3 overflow-y-auto max-h-[calc(100vh-120px)] pr-0.5">

        {/* Groq API Key */}
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-1.5">
            <label htmlFor="groq-key" className="text-xs text-zinc-400 flex-1">
              Groq API Key
            </label>
            <StatusDot configured={groqConfigured} />
          </div>
          <input
            id="groq-key"
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={groqPlaceholder}
            value={groqKey}
            onChange={(e) => setGroqKey(e.target.value)}
            className={[
              "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
              "px-3 py-1.5 text-xs text-zinc-100",
              "placeholder:text-zinc-600",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
              "transition-shadow duration-100",
            ].join(" ")}
          />
        </div>

        {/* DeepSeek API Key */}
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-1.5">
            <label htmlFor="deepseek-key" className="text-xs text-zinc-400 flex-1">
              DeepSeek API Key
            </label>
            <StatusDot configured={deepseekConfigured} />
          </div>
          <input
            id="deepseek-key"
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={deepseekPlaceholder}
            value={deepseekKey}
            onChange={(e) => setDeepseekKey(e.target.value)}
            className={[
              "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
              "px-3 py-1.5 text-xs text-zinc-100",
              "placeholder:text-zinc-600",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
              "transition-shadow duration-100",
            ].join(" ")}
          />
        </div>

        {/* Language */}
        <div className="flex items-center gap-3">
          <label htmlFor="language-select" className="text-xs text-zinc-400 flex-1">
            Language
          </label>
          <select
            id="language-select"
            value={localLang}
            onChange={(e) => handleLangChange(e.target.value)}
            className={[
              "bg-zinc-800 border border-zinc-700 rounded-lg",
              "px-2 py-1.5 text-xs text-zinc-100",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
              "transition-shadow duration-100 cursor-pointer",
            ].join(" ")}
          >
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </div>

        {/* Audio Device */}
        <div className="flex items-center gap-3">
          <label htmlFor="audio-device-select" className="text-xs text-zinc-400 flex-1">
            Microphone
          </label>
          <select
            id="audio-device-select"
            value={localAudioDevice ?? ""}
            onChange={(e) => setLocalAudioDevice(e.target.value || null)}
            className={[
              "bg-zinc-800 border border-zinc-700 rounded-lg",
              "px-2 py-1.5 text-xs text-zinc-100 max-w-[200px] truncate",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
              "transition-shadow duration-100 cursor-pointer",
            ].join(" ")}
          >
            <option value="">System Default</option>
            {audioDevices.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        </div>

        {/* Cleanup Style */}
        <div className="flex items-center gap-3">
          <span className="text-xs text-zinc-400 flex-1">Style</span>
          <div
            role="radiogroup"
            aria-label="Default cleanup style"
            className="flex gap-1 bg-zinc-800 rounded-lg p-0.5"
          >
            {STYLE_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                role="radio"
                aria-checked={localStyle === opt.value}
                aria-label={`${opt.label}: ${opt.description}`}
                onClick={() => handleStyleChange(opt.value)}
                title={opt.description}
                className={[
                  "px-2 py-1 rounded-md text-xs font-medium",
                  "transition-colors duration-100",
                  "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
                  localStyle === opt.value
                    ? "bg-zinc-600 text-white"
                    : "text-zinc-400 hover:text-zinc-200",
                ].join(" ")}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        {/* Hotkey */}
        <div className="flex flex-col gap-2">
          <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            Hotkey
          </span>

          {/* Shortcut recorder */}
          <div className="flex flex-col gap-1">
            <label className="text-xs text-zinc-400">Shortcut</label>
            <ShortcutRecorder value={localHotkey} onChange={setLocalHotkey} />
          </div>

          {/* Mode toggle */}
          <div className="flex flex-col gap-1">
            <label className="text-xs text-zinc-400">Mode</label>
            <div
              role="radiogroup"
              aria-label="Hotkey activation mode"
              className="flex gap-1 bg-zinc-800 rounded-lg p-0.5"
            >
              {(["hold", "toggle"] as HotkeyMode[]).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  role="radio"
                  aria-checked={localHotkeyMode === mode}
                  onClick={() => setLocalHotkeyMode(mode)}
                  title={
                    mode === "hold"
                      ? "Hold to record, release to process"
                      : "Press to start, press again to stop"
                  }
                  className={[
                    "flex-1 px-2 py-1 rounded-md text-xs font-medium capitalize",
                    "transition-colors duration-100",
                    "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
                    localHotkeyMode === mode
                      ? "bg-zinc-600 text-white"
                      : "text-zinc-400 hover:text-zinc-200",
                  ].join(" ")}
                >
                  {mode}
                </button>
              ))}
            </div>
            <p className="text-xs text-zinc-600 italic">
              {localHotkeyMode === "hold"
                ? "Hold to record, release to process"
                : "Press to start, press again to stop"}
            </p>
          </div>
        </div>

        {/* Save button + feedback */}
        {saveError && (
          <p className="text-xs text-red-400 break-all">{saveError}</p>
        )}
        <button
          onClick={handleSave}
          disabled={saving}
          className={[
            "w-full py-1.5 rounded-lg text-xs font-semibold",
            saveDone
              ? "bg-green-600 text-white"
              : "bg-blue-600 hover:bg-blue-500 text-white",
            "disabled:opacity-50 disabled:cursor-not-allowed",
            "transition-colors duration-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
          ].join(" ")}
        >
          {saving ? "Saving..." : saveDone ? "Saved" : "Save"}
        </button>

        {/* Divider */}
        <div className="border-t border-zinc-800" />

        {/* Dictionary section */}
        <div className="flex flex-col gap-2">
          <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            Dictionary
          </span>

          {/* Add term input */}
          <div className="flex gap-1.5">
            <input
              type="text"
              aria-label="New dictionary term"
              placeholder="Add word or phrase..."
              value={newTerm}
              onChange={(e) => setNewTerm(e.target.value)}
              onKeyDown={handleTermKeyDown}
              className={[
                "flex-1 bg-zinc-800 border border-zinc-700 rounded-lg",
                "px-3 py-1.5 text-xs text-zinc-100",
                "placeholder:text-zinc-600",
                "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
                "transition-shadow duration-100",
              ].join(" ")}
            />
            <button
              onClick={handleAddTerm}
              disabled={!newTerm.trim()}
              aria-label="Add term"
              className={[
                "px-3 py-1.5 rounded-lg text-xs font-semibold",
                "bg-zinc-700 hover:bg-zinc-600 text-zinc-200",
                "disabled:opacity-40 disabled:cursor-not-allowed",
                "transition-colors duration-100",
                "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
              ].join(" ")}
            >
              Add
            </button>
          </div>

          {termError && (
            <p className="text-xs text-red-400 break-all">{termError}</p>
          )}

          {/* Term chips */}
          {dictionary.length > 0 ? (
            <div className="flex flex-wrap gap-1.5">
              {dictionary.map((term) => (
                <DictionaryTag key={term} term={term} onRemove={onRemoveTerm} />
              ))}
            </div>
          ) : (
            <p className="text-xs text-zinc-600 italic">No terms yet.</p>
          )}
        </div>
      </div>
    </div>
  );
}

// --- Main component ----------------------------------------------------------

export default function App() {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [currentStyle, setCurrentStyle] = useState<CleanupStyle>("polished");
  const [resultText, setResultText] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [language, setLanguage] = useState("de");
  const [loadedSettings, setLoadedSettings] = useState<AppSettings | null>(null);
  const [dictionary, setDictionary] = useState<string[]>([]);

  const isBusy =
    recordingState === "transcribing" || recordingState === "cleaning";
  const isRecording = recordingState === "recording";

  // Load persisted settings and dictionary on mount.
  useEffect(() => {
    getSettings()
      .then((settings) => {
        setLoadedSettings(settings);
        setLanguage(settings.language);
        setCurrentStyle(settings.cleanupStyle);
        // Sync backend pipeline with persisted values.
        syncLanguage(settings.language).catch(console.error);
        syncCleanupStyle(settings.cleanupStyle).catch(console.error);
      })
      .catch((err) => console.error("[dikta] get_settings error:", err));

    getDictionaryTerms()
      .then(setDictionary)
      .catch((err) => console.error("[dikta] get_dictionary_terms error:", err));
  }, []);

  // Subscribe to backend pipeline events (hotkey-triggered flow).
  // The backend owns the entire pipeline when the hotkey fires; we just mirror
  // whatever state it reports. This runs once on mount.
  useEffect(() => {
    const unlistenPromise = onStateChanged((payload) => {
      setRecordingState(payload.state as RecordingState);
      if (payload.text !== undefined) setResultText(payload.text);
      if (payload.error !== undefined) setErrorMessage(payload.error);
    });

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  // Keep the backend in sync whenever the user changes the cleanup style.
  // The hotkey pipeline reads these values from backend state directly.
  const handleStyleChange = useCallback((style: CleanupStyle) => {
    setCurrentStyle(style);
    syncCleanupStyle(style).catch((err) =>
      console.error("[dikta] set_cleanup_style error:", err)
    );
  }, []);

  // Keep the backend in sync whenever the user changes the language.
  const handleLanguageChange = useCallback((lang: string) => {
    setLanguage(lang);
    syncLanguage(lang).catch((err) =>
      console.error("[dikta] set_language error:", err)
    );
  }, []);

  // Persist all settings, re-register the hotkey at OS level, and refresh local state.
  const handleSaveSettings = useCallback(async (
    groqKey: string,
    deepseekKey: string,
    lang: string,
    style: CleanupStyle,
    hotkey: string,
    hotkeyMode: HotkeyMode,
    audioDevice: string | null
  ) => {
    await saveSettings(groqKey, deepseekKey, lang, style, hotkey, hotkeyMode, audioDevice);
    // save_settings already re-registers the hotkey in the backend, no need to call setHotkey.
    // Refresh loaded settings so masked key placeholders and hotkey display update.
    const updated = await getSettings();
    setLoadedSettings(updated);
  }, []);

  // Dictionary mutations -- optimistic local update, backend is source of truth.
  const handleAddTerm = useCallback(async (term: string) => {
    await addDictionaryTerm(term);
    setDictionary((prev) => (prev.includes(term) ? prev : [...prev, term]));
  }, []);

  const handleRemoveTerm = useCallback(async (term: string) => {
    await removeDictionaryTerm(term);
    setDictionary((prev) => prev.filter((t) => t !== term));
  }, []);

  const handleRecordToggle = useCallback(async () => {
    if (isRecording) {
      // --- Stop -> transcribe -> cleanup flow ---
      try {
        setRecordingState("transcribing");
        await stopRecording();

        const rawText = await transcribeAudio(language);

        setRecordingState("cleaning");
        const cleanedText = await cleanupText(rawText, currentStyle);

        setResultText(cleanedText);
        setRecordingState("done");
      } catch (err) {
        console.error("[dikta] stop/process error:", err);
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      // --- Start flow ---
      setResultText(null);
      setErrorMessage(null);
      try {
        await startRecording();
        setRecordingState("recording");
      } catch (err) {
        console.error("[dikta] start_recording error:", err);
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    }
  }, [isRecording, currentStyle, language]);

  // Allow clicking again after done/error to reset.
  const handleReset = useCallback(() => {
    if (recordingState === "done" || recordingState === "error") {
      setRecordingState("idle");
      setErrorMessage(null);
    }
  }, [recordingState]);

  const toggleSettings = useCallback(() => {
    setShowSettings((prev) => !prev);
  }, []);

  return (
    <main
      className="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col items-center justify-between p-4 select-none"
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      {/* Header */}
      <div className="w-full flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold tracking-widest text-zinc-500 uppercase">
            Dikta
          </span>
          <button
            aria-label="Toggle settings"
            aria-expanded={showSettings}
            onClick={toggleSettings}
            className={[
              "p-1 rounded-md",
              "transition-colors duration-100",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
              showSettings
                ? "text-zinc-200 bg-zinc-800"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800",
            ].join(" ")}
          >
            <GearIcon />
          </button>
        </div>
        <StylePicker
          value={currentStyle}
          onChange={handleStyleChange}
          disabled={isBusy || isRecording}
        />
      </div>

      {/* Settings panel -- inline, no modal */}
      <div
        className={[
          "w-full overflow-hidden transition-all duration-200",
          showSettings ? "max-h-[600px] opacity-100 mt-3" : "max-h-0 opacity-0",
        ].join(" ")}
        aria-hidden={!showSettings}
      >
        {showSettings && (
          <SettingsPanel
            onClose={() => setShowSettings(false)}
            language={language}
            cleanupStyle={currentStyle}
            hotkey={loadedSettings?.hotkey ?? "ctrl+shift+d"}
            hotkeyMode={loadedSettings?.hotkeyMode ?? "hold"}
            loadedSettings={loadedSettings}
            dictionary={dictionary}
            onSave={handleSaveSettings}
            onLanguageChange={handleLanguageChange}
            onStyleChange={handleStyleChange}
            onAddTerm={handleAddTerm}
            onRemoveTerm={handleRemoveTerm}
          />
        )}
      </div>

      {/* Center section -- record button */}
      <div className="flex flex-col items-center gap-6 flex-1 justify-center">
        <RecordButton
          recordingState={
            recordingState === "done" || recordingState === "error"
              ? "idle"
              : recordingState
          }
          onClick={
            recordingState === "done" || recordingState === "error"
              ? handleReset
              : handleRecordToggle
          }
        />

        {/* Result textarea */}
        {resultText !== null && (
          <div className="w-full max-w-xs">
            <label
              htmlFor="result-text"
              className="block text-xs text-zinc-500 mb-1"
            >
              Result
            </label>
            <textarea
              id="result-text"
              readOnly
              value={resultText}
              rows={4}
              className={[
                "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
                "px-3 py-2 text-sm text-zinc-100 resize-none",
                "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
                "placeholder:text-zinc-600",
              ].join(" ")}
            />
          </div>
        )}
      </div>

      {/* Footer -- status */}
      <StatusBar
        recordingState={recordingState}
        errorMessage={errorMessage}
        hotkey={loadedSettings?.hotkey ?? "ctrl+shift+d"}
        hotkeyMode={loadedSettings?.hotkeyMode ?? "hold"}
      />
    </main>
  );
}
