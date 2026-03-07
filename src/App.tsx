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

function MicIcon({ className = "w-8 h-8" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function StopIcon({ className = "w-8 h-8" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <rect x="6" y="6" width="12" height="12" rx="2" />
    </svg>
  );
}

function SpinnerIcon({ className = "w-8 h-8" }: { className?: string }) {
  return (
    <svg className={`${className} animate-spin`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="10" strokeOpacity="0.2" />
      <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
      <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.01 7.01 0 0 0-1.62-.94l-.36-2.54A.484.484 0 0 0 14 2h-4a.484.484 0 0 0-.48.41l-.36 2.54a7.07 7.07 0 0 0-1.62.94l-2.39-.96a.49.49 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.27.41.49.41h4c.22 0 .43-.17.47-.41l.36-2.54a7.07 7.07 0 0 0 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
      <path d="M18 6 6 18M6 6l12 12" />
    </svg>
  );
}

// --- Shortcut Recorder -------------------------------------------------------

function ShortcutRecorder({ value, onChange }: { value: string; onChange: (s: string) => void }) {
  const [listening, setListening] = useState(false);

  useEffect(() => {
    if (!listening) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
      const parts: string[] = [];
      if (e.ctrlKey) parts.push("ctrl");
      if (e.shiftKey) parts.push("shift");
      if (e.altKey) parts.push("alt");
      if (e.metaKey) parts.push("super");
      if (parts.length === 0) return;
      const KEY_MAP: Record<string, string> = {
        " ": "space", Enter: "enter", Escape: "escape", Tab: "tab",
        Backspace: "backspace", Delete: "delete", Insert: "insert",
        Home: "home", End: "end", PageUp: "pageup", PageDown: "pagedown",
        ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
      };
      let key = KEY_MAP[e.key] ?? e.key.toLowerCase();
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
      onClick={() => setListening(true)}
      className={[
        "w-full bg-[#111113] border rounded-lg px-3 py-2 text-sm text-left font-mono",
        listening
          ? "border-emerald-500/50 text-emerald-400 animate-pulse"
          : "border-zinc-700/50 text-zinc-200 hover:border-zinc-600",
        "focus:outline-none transition-all duration-150",
      ].join(" ")}
    >
      {listening ? "Press shortcut..." : value || "Click to set"}
    </button>
  );
}

// --- Helpers -----------------------------------------------------------------

function formatHotkeyDisplay(hotkey: string): string {
  return hotkey.split("+").map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(" + ");
}

// --- Sub-components ----------------------------------------------------------

function RecordButton({ recordingState, onClick }: { recordingState: RecordingState; onClick: () => void }) {
  const isRecording = recordingState === "recording";
  const isBusy = recordingState === "transcribing" || recordingState === "cleaning";

  return (
    <button
      aria-label={isRecording ? "Stop recording" : isBusy ? "Processing" : "Start recording"}
      disabled={isBusy}
      onClick={onClick}
      className={[
        "relative flex items-center justify-center",
        "w-24 h-24 rounded-full",
        "transition-all duration-200",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30",
        "disabled:cursor-not-allowed disabled:opacity-60",
        isRecording
          ? "bg-red-500/20 text-red-400 shadow-[0_0_40px_rgba(239,68,68,0.3)]"
          : isBusy
          ? "bg-amber-500/15 text-amber-400 shadow-[0_0_30px_rgba(245,158,11,0.2)]"
          : "bg-emerald-500/15 text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.2)] hover:shadow-[0_0_50px_rgba(16,185,129,0.3)] hover:bg-emerald-500/20",
      ].join(" ")}
    >
      {/* Outer ring */}
      <span
        className={[
          "absolute inset-0 rounded-full border-2 transition-colors duration-200",
          isRecording ? "border-red-500/40" : isBusy ? "border-amber-500/30" : "border-emerald-500/25",
        ].join(" ")}
      />

      {/* Pulse ring while recording */}
      {isRecording && (
        <span className="absolute inset-0 rounded-full border-2 border-red-400 opacity-40 animate-ping" />
      )}

      {isBusy ? (
        <SpinnerIcon className="w-9 h-9" />
      ) : isRecording ? (
        <StopIcon className="w-9 h-9" />
      ) : (
        <MicIcon className="w-9 h-9" />
      )}
    </button>
  );
}

function StylePicker({ value, onChange, disabled }: { value: CleanupStyle; onChange: (s: CleanupStyle) => void; disabled: boolean }) {
  return (
    <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60">
      {STYLE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          title={opt.description}
          className={[
            "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100",
            "disabled:cursor-not-allowed disabled:opacity-50",
            value === opt.value
              ? "bg-emerald-500/15 text-emerald-400"
              : "text-zinc-500 hover:text-zinc-300",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

function StatusDot({ active }: { active: boolean }) {
  return (
    <span className={`inline-block w-2 h-2 rounded-full flex-shrink-0 ${active ? "bg-emerald-500" : "bg-zinc-600"}`} />
  );
}

function DictionaryTag({ term, onRemove }: { term: string; onRemove: (t: string) => void }) {
  return (
    <span className="inline-flex items-center gap-1 bg-[#111113] text-zinc-300 pl-2.5 pr-1.5 py-1 rounded-full text-xs border border-zinc-800/60">
      {term}
      <button
        onClick={() => onRemove(term)}
        className="text-zinc-500 hover:text-red-400 rounded-full p-0.5 transition-colors"
      >
        <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </span>
  );
}

// --- Settings Panel ----------------------------------------------------------

interface SettingsPanelProps {
  onClose: () => void;
  loadedSettings: AppSettings | null;
  language: string;
  cleanupStyle: CleanupStyle;
  hotkey: string;
  hotkeyMode: HotkeyMode;
  audioDevice: string | null;
  audioDevices: string[];
  dictionary: string[];
  onSave: (groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle, hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  onAudioDeviceChange: (d: string | null) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
}

function SettingsPanel({
  onClose, loadedSettings, language, cleanupStyle, hotkey, hotkeyMode,
  audioDevice, audioDevices, dictionary,
  onSave, onLanguageChange, onStyleChange, onHotkeyChange, onHotkeyModeChange,
  onAudioDeviceChange, onAddTerm, onRemoveTerm,
}: SettingsPanelProps) {
  const [groqKey, setGroqKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [localLang, setLocalLang] = useState(language);
  const [localStyle, setLocalStyle] = useState(cleanupStyle);
  const [localHotkey, setLocalHotkey] = useState(hotkey);
  const [localHotkeyMode, setLocalHotkeyMode] = useState(hotkeyMode);
  const [localAudioDevice, setLocalAudioDevice] = useState(audioDevice);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [newTerm, setNewTerm] = useState("");

  useEffect(() => { setLocalLang(language); }, [language]);
  useEffect(() => { setLocalStyle(cleanupStyle); }, [cleanupStyle]);
  useEffect(() => { setLocalHotkey(hotkey); }, [hotkey]);
  useEffect(() => { setLocalHotkeyMode(hotkeyMode); }, [hotkeyMode]);
  useEffect(() => { setLocalAudioDevice(audioDevice); }, [audioDevice]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleLangChange = useCallback((lang: string) => {
    setLocalLang(lang);
    onLanguageChange(lang);
  }, [onLanguageChange]);

  const handleStyleChange = useCallback((style: CleanupStyle) => {
    setLocalStyle(style);
    onStyleChange(style);
  }, [onStyleChange]);

  const handleHotkeyChange = useCallback((h: string) => {
    setLocalHotkey(h);
    onHotkeyChange(h);
  }, [onHotkeyChange]);

  const handleHotkeyModeChange = useCallback((m: HotkeyMode) => {
    setLocalHotkeyMode(m);
    onHotkeyModeChange(m);
  }, [onHotkeyModeChange]);

  const handleAudioDeviceChange = useCallback((d: string | null) => {
    setLocalAudioDevice(d);
    onAudioDeviceChange(d);
  }, [onAudioDeviceChange]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveMsg(null);
    try {
      await onSave(groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice);
      setGroqKey("");
      setDeepseekKey("");
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice, onSave]);

  const handleAddTerm = useCallback(async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    try {
      await onAddTerm(trimmed);
      setNewTerm("");
    } catch (err) {
      console.error(err);
    }
  }, [newTerm, onAddTerm]);

  const groqOk = !!loadedSettings?.groqApiKeyMasked;
  const deepseekOk = !!loadedSettings?.deepseekApiKeyMasked;

  // Shared input classes
  const inputCls = "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-emerald-500/40 transition-colors";
  const labelCls = "text-xs text-zinc-400";
  const sectionTitleCls = "text-[10px] font-semibold text-zinc-500 uppercase tracking-widest";

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className={sectionTitleCls}>Settings</span>
        <button
          aria-label="Close settings"
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Scrollable body */}
      <div className="overflow-y-auto max-h-[calc(100vh-200px)] p-4 flex flex-col gap-5">

        {/* --- Voice & Recording --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Voice & Recording</span>

          {/* Microphone */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Microphone</span>
            <select
              value={localAudioDevice ?? ""}
              onChange={(e) => handleAudioDeviceChange(e.target.value || null)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-black max-w-[180px] truncate focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              <option value="">System Default</option>
              {audioDevices.map((n) => <option key={n} value={n}>{n}</option>)}
            </select>
          </div>

          {/* Language */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Language</span>
            <select
              value={localLang}
              onChange={(e) => handleLangChange(e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-black focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              <option value="de">Deutsch</option>
              <option value="en">English</option>
            </select>
          </div>

          {/* Cleanup style */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Cleanup Style</span>
            <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60">
              {STYLE_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => handleStyleChange(opt.value)}
                  title={opt.description}
                  className={[
                    "px-2 py-1 rounded-md text-xs font-medium transition-all duration-100",
                    localStyle === opt.value
                      ? "bg-emerald-500/15 text-emerald-400"
                      : "text-zinc-500 hover:text-zinc-300",
                  ].join(" ")}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* --- Hotkey --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Hotkey</span>

          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-zinc-500">Shortcut</span>
            <ShortcutRecorder value={localHotkey} onChange={handleHotkeyChange} />
          </div>

          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Mode</span>
            <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60">
              {(["hold", "toggle"] as HotkeyMode[]).map((mode) => (
                <button
                  key={mode}
                  onClick={() => handleHotkeyModeChange(mode)}
                  title={mode === "hold" ? "Hold to record, release to process" : "Press to start, press to stop"}
                  className={[
                    "px-2.5 py-1 rounded-md text-xs font-medium capitalize transition-all duration-100",
                    localHotkeyMode === mode
                      ? "bg-emerald-500/15 text-emerald-400"
                      : "text-zinc-500 hover:text-zinc-300",
                  ].join(" ")}
                >
                  {mode}
                </button>
              ))}
            </div>
          </div>
          <p className="text-[11px] text-zinc-600">
            {localHotkeyMode === "hold" ? "Hold to record, release to process" : "Press once to start, press again to stop"}
          </p>
        </div>

        {/* --- API Keys --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>API Keys</span>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={labelCls}>Groq</span>
              <StatusDot active={groqOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={groqOk ? loadedSettings!.groqApiKeyMasked : "gsk_..."}
              value={groqKey}
              onChange={(e) => setGroqKey(e.target.value)}
              className={inputCls}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={labelCls}>DeepSeek</span>
              <StatusDot active={deepseekOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={deepseekOk ? loadedSettings!.deepseekApiKeyMasked : "sk-..."}
              value={deepseekKey}
              onChange={(e) => setDeepseekKey(e.target.value)}
              className={inputCls}
            />
          </div>
        </div>

        {/* --- Dictionary --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Dictionary</span>

          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Add word or phrase..."
              value={newTerm}
              onChange={(e) => setNewTerm(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddTerm()}
              className={`flex-1 ${inputCls}`}
            />
            <button
              onClick={handleAddTerm}
              disabled={!newTerm.trim()}
              className="px-3 py-2 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            >
              Add
            </button>
          </div>

          {dictionary.length > 0 ? (
            <div className="flex flex-wrap gap-1.5">
              {dictionary.map((t) => <DictionaryTag key={t} term={t} onRemove={onRemoveTerm} />)}
            </div>
          ) : (
            <p className="text-xs text-zinc-600 italic">No terms yet.</p>
          )}
        </div>

        {/* --- Save --- */}
        <button
          onClick={handleSave}
          disabled={saving}
          className={[
            "w-full py-2.5 rounded-xl text-sm font-medium transition-all duration-150 border",
            saveMsg === "Saved"
              ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
              : saveMsg && saveMsg !== "Saved"
              ? "bg-red-500/10 border-red-500/20 text-red-400"
              : "bg-emerald-500/10 border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 hover:border-emerald-500/30",
            "disabled:opacity-50 disabled:cursor-not-allowed",
          ].join(" ")}
        >
          {saving ? "Saving..." : saveMsg ?? "Save Settings"}
        </button>
      </div>
    </div>
  );
}

// --- Main App ----------------------------------------------------------------

export default function App() {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [currentStyle, setCurrentStyle] = useState<CleanupStyle>("polished");
  const [resultText, setResultText] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [language, setLanguage] = useState("de");
  const [loadedSettings, setLoadedSettings] = useState<AppSettings | null>(null);
  const [dictionary, setDictionary] = useState<string[]>([]);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [localHotkey, setLocalHotkey] = useState("ctrl+shift+d");
  const [localHotkeyMode, setLocalHotkeyMode] = useState<HotkeyMode>("hold");
  const [localAudioDevice, setLocalAudioDevice] = useState<string | null>(null);

  const isBusy = recordingState === "transcribing" || recordingState === "cleaning";
  const isRecording = recordingState === "recording";

  // Load settings + dictionary + devices on mount.
  useEffect(() => {
    getSettings().then((s) => {
      setLoadedSettings(s);
      setLanguage(s.language);
      setCurrentStyle(s.cleanupStyle);
      setLocalHotkey(s.hotkey);
      setLocalHotkeyMode(s.hotkeyMode);
      setLocalAudioDevice(s.audioDevice);
      syncLanguage(s.language).catch(console.error);
      syncCleanupStyle(s.cleanupStyle).catch(console.error);
    }).catch(console.error);

    getDictionaryTerms().then(setDictionary).catch(console.error);
    listAudioDevices().then(setAudioDevices).catch(console.error);
  }, []);

  // Subscribe to backend pipeline events.
  useEffect(() => {
    const unlisten = onStateChanged((p) => {
      setRecordingState(p.state as RecordingState);
      if (p.text !== undefined) setResultText(p.text);
      if (p.error !== undefined) setErrorMessage(p.error);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // --- Handlers ---

  const handleStyleChange = useCallback((style: CleanupStyle) => {
    setCurrentStyle(style);
    syncCleanupStyle(style).catch(console.error);
  }, []);

  const handleLanguageChange = useCallback((lang: string) => {
    setLanguage(lang);
    syncLanguage(lang).catch(console.error);
  }, []);

  const handleSaveSettings = useCallback(async (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null,
  ) => {
    await saveSettings(groqKey, deepseekKey, lang, style, hotkey, hotkeyMode, audioDevice);
    const updated = await getSettings();
    setLoadedSettings(updated);
    setLanguage(updated.language);
    setCurrentStyle(updated.cleanupStyle);
    setLocalHotkey(updated.hotkey);
    setLocalHotkeyMode(updated.hotkeyMode);
    setLocalAudioDevice(updated.audioDevice);
  }, []);

  const handleAddTerm = useCallback(async (term: string) => {
    await addDictionaryTerm(term);
    setDictionary((prev) => (prev.includes(term) ? prev : [...prev, term]));
  }, []);

  const handleRemoveTerm = useCallback(async (term: string) => {
    await removeDictionaryTerm(term);
    setDictionary((prev) => prev.filter((t) => t !== term));
  }, []);

  const handleRecordToggle = useCallback(async () => {
    if (recordingState === "done" || recordingState === "error") {
      setRecordingState("idle");
      setErrorMessage(null);
      return;
    }
    if (isRecording) {
      try {
        setRecordingState("transcribing");
        await stopRecording();
        const rawText = await transcribeAudio(language);
        setRecordingState("cleaning");
        const cleanedText = await cleanupText(rawText, currentStyle);
        setResultText(cleanedText);
        setRecordingState("done");
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      setResultText(null);
      setErrorMessage(null);
      try {
        await startRecording();
        setRecordingState("recording");
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    }
  }, [recordingState, isRecording, currentStyle, language]);

  const toggleSettings = useCallback(() => setShowSettings((prev) => !prev), []);

  const hotkeyDisplay = formatHotkeyDisplay(loadedSettings?.hotkey ?? "ctrl+shift+d");

  return (
    <main
      className="h-screen bg-[#0a0a0c] text-zinc-100 flex flex-col select-none overflow-hidden"
      style={{ fontFamily: "'Inter', system-ui, -apple-system, sans-serif" }}
    >
      {/* ── Header ── */}
      <div className="flex items-center justify-between px-4 pt-3.5 pb-2 flex-shrink-0">
        <div className="flex items-center gap-2.5">
          {/* Logo */}
          <div className="w-7 h-7 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center">
            <MicIcon className="w-3.5 h-3.5 text-emerald-400" />
          </div>
          <span className="text-sm font-semibold text-zinc-300 tracking-wide">Dikta</span>

          {/* Settings toggle */}
          <button
            aria-label="Toggle settings"
            aria-expanded={showSettings}
            onClick={toggleSettings}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showSettings
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <GearIcon />
          </button>
        </div>

        {/* Style picker in header */}
        <StylePicker
          value={currentStyle}
          onChange={handleStyleChange}
          disabled={isBusy || isRecording}
        />
      </div>

      {/* ── Settings Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showSettings ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showSettings && (
          <SettingsPanel
            onClose={() => setShowSettings(false)}
            loadedSettings={loadedSettings}
            language={language}
            cleanupStyle={currentStyle}
            hotkey={localHotkey}
            hotkeyMode={localHotkeyMode}
            audioDevice={localAudioDevice}
            audioDevices={audioDevices}
            dictionary={dictionary}
            onSave={handleSaveSettings}
            onLanguageChange={handleLanguageChange}
            onStyleChange={handleStyleChange}
            onHotkeyChange={setLocalHotkey}
            onHotkeyModeChange={setLocalHotkeyMode}
            onAudioDeviceChange={setLocalAudioDevice}
            onAddTerm={handleAddTerm}
            onRemoveTerm={handleRemoveTerm}
          />
        )}
      </div>

      {/* ── Center: Record Button ── */}
      <div className="flex-1 flex flex-col items-center justify-center gap-4 px-4 min-h-0">
        <RecordButton
          recordingState={recordingState === "done" || recordingState === "error" ? "idle" : recordingState}
          onClick={handleRecordToggle}
        />

        {/* Status label */}
        <div className="text-center">
          <p className={[
            "text-xs font-medium",
            recordingState === "error" ? "text-red-400"
              : recordingState === "recording" ? "text-red-400"
              : recordingState === "done" ? "text-emerald-400"
              : isBusy ? "text-amber-400"
              : "text-zinc-600",
          ].join(" ")}>
            {errorMessage && recordingState === "error"
              ? errorMessage
              : STATUS_LABELS[recordingState]}
          </p>
        </div>

        {/* Result */}
        {resultText !== null && (
          <div className="w-full max-w-xs">
            <textarea
              readOnly
              value={resultText}
              rows={3}
              className="w-full bg-[#111113] border border-zinc-800/60 rounded-xl px-3.5 py-2.5 text-sm text-zinc-200 resize-none focus:outline-none focus:border-emerald-500/30 transition-colors"
            />
          </div>
        )}
      </div>

      {/* ── Footer ── */}
      <div className="flex items-center justify-center px-4 py-3 flex-shrink-0">
        <span className="text-[11px] font-mono text-zinc-600">{hotkeyDisplay}</span>
      </div>
    </main>
  );
}
