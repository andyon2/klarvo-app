import { useState, useCallback, useEffect } from "react";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import "./styles.css";
import type { RecordingState, CleanupStyle, HotkeyMode, AppSettings, AppProfile, HistoryEntry, UsageSummary } from "./types";
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
  addHistoryEntry,
  onStateChanged,
  setLanguage as syncLanguage,
  setCleanupStyle as syncCleanupStyle,
  listAudioDevices,
  getHistory,
  deleteHistoryEntry,
  clearHistory,
  searchHistory,
  getUsageStats,
  getProfiles,
  saveProfiles,
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
    <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60 flex-shrink min-w-0">
      {STYLE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          title={opt.description}
          className={[
            "px-1.5 py-1 rounded-md text-[11px] font-medium transition-all duration-100 whitespace-nowrap",
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

// --- Provider Priority List (Drag & Drop) ------------------------------------

function SortableProviderItem({ id, label, active }: { id: string; label: string; active: boolean }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };
  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      className={[
        "flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs cursor-grab active:cursor-grabbing select-none",
        "bg-[#111113] border",
        active ? "border-emerald-500/30 text-zinc-200" : "border-zinc-800/40 text-zinc-500",
      ].join(" ")}
    >
      <svg viewBox="0 0 16 16" className="w-3 h-3 text-zinc-600 flex-shrink-0" fill="currentColor">
        <circle cx="5" cy="4" r="1.2" /><circle cx="11" cy="4" r="1.2" />
        <circle cx="5" cy="8" r="1.2" /><circle cx="11" cy="8" r="1.2" />
        <circle cx="5" cy="12" r="1.2" /><circle cx="11" cy="12" r="1.2" />
      </svg>
      <span className="flex-1">{label}</span>
      <span className={["w-1.5 h-1.5 rounded-full flex-shrink-0", active ? "bg-emerald-400" : "bg-zinc-700"].join(" ")} />
    </div>
  );
}

function ProviderPriorityList({
  items, onChange, keyStatus, labels,
}: {
  items: string[];
  onChange: (items: string[]) => void;
  keyStatus: Record<string, boolean>;
  labels: Record<string, string>;
}) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      const oldIdx = items.indexOf(active.id as string);
      const newIdx = items.indexOf(over.id as string);
      onChange(arrayMove(items, oldIdx, newIdx));
    }
  }

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={items} strategy={verticalListSortingStrategy}>
        <div className="flex flex-col gap-1">
          {items.map((id) => (
            <SortableProviderItem
              key={id}
              id={id}
              label={labels[id] ?? id}
              active={!!keyStatus[id]}
            />
          ))}
        </div>
      </SortableContext>
    </DndContext>
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
  onSave: (groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle, hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null, sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean, openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[]) => Promise<void>;
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
  const [localSttModel, setLocalSttModel] = useState(loadedSettings?.sttModel ?? "whisper-large-v3-turbo");
  const [localCustomPrompt, setLocalCustomPrompt] = useState(loadedSettings?.customPrompt ?? "");
  const [localAutostart, setLocalAutostart] = useState(loadedSettings?.autostart ?? false);
  const [localWhisperMode, setLocalWhisperMode] = useState(loadedSettings?.whisperMode ?? false);
  const [openaiKey, setOpenaiKey] = useState("");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [localSttPriority, setLocalSttPriority] = useState<string[]>(loadedSettings?.sttPriority ?? ["groq", "openai"]);
  const [localLlmPriority, setLocalLlmPriority] = useState<string[]>(loadedSettings?.llmPriority ?? ["deepseek", "openai", "anthropic", "groq"]);
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [newTerm, setNewTerm] = useState("");

  // Load profiles on mount.
  useEffect(() => { getProfiles().then(setProfiles).catch(console.error); }, []);

  useEffect(() => { setLocalLang(language); }, [language]);
  useEffect(() => { setLocalStyle(cleanupStyle); }, [cleanupStyle]);
  useEffect(() => { setLocalHotkey(hotkey); }, [hotkey]);
  useEffect(() => { setLocalHotkeyMode(hotkeyMode); }, [hotkeyMode]);
  useEffect(() => { setLocalAudioDevice(audioDevice); }, [audioDevice]);
  useEffect(() => {
    if (loadedSettings) {
      setLocalSttModel(loadedSettings.sttModel);
      setLocalCustomPrompt(loadedSettings.customPrompt);
      setLocalAutostart(loadedSettings.autostart);
      setLocalWhisperMode(loadedSettings.whisperMode);
      setLocalSttPriority(loadedSettings.sttPriority);
      setLocalLlmPriority(loadedSettings.llmPriority);
    }
  }, [loadedSettings]);

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
      await onSave(groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey.trim(), anthropicKey.trim(), localSttPriority, localLlmPriority);
      setGroqKey("");
      setDeepseekKey("");
      setOpenaiKey("");
      setAnthropicKey("");
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey, anthropicKey, localSttPriority, localLlmPriority, onSave]);

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
  const openaiOk = !!loadedSettings?.openaiApiKeyMasked;
  const anthropicOk = !!loadedSettings?.anthropicApiKeyMasked;

  // Shared input classes
  const inputCls = "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors";
  const labelCls = "text-xs text-zinc-300";
  const sectionTitleCls = "text-[10px] font-semibold text-zinc-400 uppercase tracking-widest";

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col max-h-[calc(100vh-120px)]">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40 flex-shrink-0">
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
      <div className="overflow-y-auto flex-1 min-h-0 p-4 flex flex-col gap-5">

        {/* --- Voice & Recording --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Voice & Recording</span>

          {/* Microphone */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Microphone</span>
            <select
              value={localAudioDevice ?? ""}
              onChange={(e) => handleAudioDeviceChange(e.target.value || null)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 max-w-[180px] truncate focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
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
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              <option value="">Auto (DE + EN)</option>
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

          {/* STT Model */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>STT Model</span>
            <select
              value={localSttModel}
              onChange={(e) => setLocalSttModel(e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 max-w-[200px] truncate focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              <option value="whisper-large-v3-turbo">Large V3 Turbo ($0.04/h)</option>
              <option value="whisper-large-v3">Large V3 ($0.111/h)</option>
              <option value="distil-whisper-large-v3-en">Distil V3 EN ($0.02/h)</option>
            </select>
          </div>
        </div>

        {/* --- Hotkey --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Hotkey</span>

          <div className="flex flex-col gap-1.5">
            <span className="text-xs text-zinc-300">Shortcut</span>
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
          <p className="text-[11px] text-zinc-500">
            {localHotkeyMode === "hold" ? "Hold to record, release to process" : "Press once to start, press again to stop"}
          </p>
        </div>

        {/* --- Custom Prompt --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Custom Prompt</span>
          <textarea
            value={localCustomPrompt}
            onChange={(e) => setLocalCustomPrompt(e.target.value)}
            placeholder="Extra instructions for the LLM, e.g. 'Always use formal German' or 'Keep technical terms in English'"
            rows={3}
            className={`${inputCls} resize-none`}
          />
          <p className="text-[11px] text-zinc-500">Appended to the system prompt during cleanup.</p>
        </div>

        {/* --- General --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>General</span>
          <label className="flex items-center justify-between gap-3 cursor-pointer">
            <span className={labelCls}>Launch on startup</span>
            <button
              type="button"
              role="switch"
              aria-checked={localAutostart}
              onClick={() => setLocalAutostart(!localAutostart)}
              className={[
                "relative w-9 h-5 rounded-full transition-colors duration-200",
                localAutostart ? "bg-emerald-500/40" : "bg-zinc-700",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                  localAutostart ? "translate-x-4" : "",
                ].join(" ")}
              />
            </button>
          </label>

          <label className="flex items-center justify-between gap-3 cursor-pointer">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Whisper mode</span>
              <span className="text-[10px] text-zinc-500">Amplifies mic input for quiet dictation</span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={localWhisperMode}
              onClick={() => setLocalWhisperMode(!localWhisperMode)}
              className={[
                "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
                localWhisperMode ? "bg-emerald-500/40" : "bg-zinc-700",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                  localWhisperMode ? "translate-x-4" : "",
                ].join(" ")}
              />
            </button>
          </label>

          <div className="flex flex-col gap-0.5">
            <span className={labelCls}>Command mode</span>
            <span className="text-[10px] text-zinc-500">Select text, hold Ctrl+Shift+E, speak your edit. The selected text will be rewritten.</span>
          </div>
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

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={labelCls}>OpenAI</span>
              <StatusDot active={openaiOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={openaiOk ? loadedSettings!.openaiApiKeyMasked : "sk-..."}
              value={openaiKey}
              onChange={(e) => setOpenaiKey(e.target.value)}
              className={inputCls}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={labelCls}>Anthropic</span>
              <StatusDot active={anthropicOk} />
              <span className="text-[10px] text-zinc-500">(LLM only)</span>
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={anthropicOk ? loadedSettings!.anthropicApiKeyMasked : "sk-ant-..."}
              value={anthropicKey}
              onChange={(e) => setAnthropicKey(e.target.value)}
              className={inputCls}
            />
          </div>
        </div>

        {/* --- Provider Priority --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Provider Priority</span>
          <p className="text-[10px] text-zinc-500">Drag to reorder. First provider with a configured key is used. If it fails, the next one is tried.</p>

          <div className="flex flex-col gap-2">
            <span className={labelCls}>Speech-to-Text</span>
            <ProviderPriorityList
              items={localSttPriority}
              onChange={setLocalSttPriority}
              keyStatus={{ groq: groqOk, openai: openaiOk }}
              labels={{ groq: "Groq Whisper", openai: "OpenAI Whisper" }}
            />
          </div>

          <div className="flex flex-col gap-2">
            <span className={labelCls}>Text Cleanup (LLM)</span>
            <ProviderPriorityList
              items={localLlmPriority}
              onChange={setLocalLlmPriority}
              keyStatus={{ deepseek: deepseekOk, openai: openaiOk, anthropic: anthropicOk, groq: groqOk }}
              labels={{ deepseek: "DeepSeek", openai: "OpenAI", anthropic: "Anthropic", groq: "Groq (Llama)" }}
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
            <p className="text-xs text-zinc-500 italic">No terms yet.</p>
          )}
        </div>

        {/* --- App Profiles --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>App Profiles</span>
          <p className="text-[11px] text-zinc-500">Override style/language per app. Matches window title substring.</p>

          {profiles.map((p, i) => (
            <div key={i} className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <input
                  type="text"
                  placeholder="Profile name"
                  value={p.name}
                  onChange={(e) => {
                    const next = [...profiles];
                    next[i] = { ...next[i], name: e.target.value };
                    setProfiles(next);
                  }}
                  className={`flex-1 ${inputCls}`}
                />
                <button
                  onClick={() => {
                    const next = profiles.filter((_, j) => j !== i);
                    setProfiles(next);
                    saveProfiles(next).catch(console.error);
                  }}
                  className="text-zinc-500 hover:text-red-400 transition-colors p-1"
                >
                  <CloseIcon />
                </button>
              </div>
              <input
                type="text"
                placeholder="Window title pattern, e.g. 'Slack' or 'Visual Studio'"
                value={p.appPattern}
                onChange={(e) => {
                  const next = [...profiles];
                  next[i] = { ...next[i], appPattern: e.target.value };
                  setProfiles(next);
                }}
                className={inputCls}
              />
              <div className="flex gap-2">
                <select
                  value={p.cleanupStyle}
                  onChange={(e) => {
                    const next = [...profiles];
                    next[i] = { ...next[i], cleanupStyle: e.target.value as CleanupStyle };
                    setProfiles(next);
                  }}
                  className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 cursor-pointer"
                >
                  {STYLE_OPTIONS.map((opt) => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
                </select>
                <select
                  value={p.language}
                  onChange={(e) => {
                    const next = [...profiles];
                    next[i] = { ...next[i], language: e.target.value };
                    setProfiles(next);
                  }}
                  className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 cursor-pointer"
                >
                  <option value="">Auto</option>
                  <option value="de">DE</option>
                  <option value="en">EN</option>
                </select>
              </div>
              <input
                type="text"
                placeholder="Custom prompt for this app (optional)"
                value={p.customPrompt}
                onChange={(e) => {
                  const next = [...profiles];
                  next[i] = { ...next[i], customPrompt: e.target.value };
                  setProfiles(next);
                }}
                className={inputCls}
              />
            </div>
          ))}

          <div className="flex gap-2">
            <button
              onClick={() => setProfiles([...profiles, { name: "", appPattern: "", cleanupStyle: "polished", language: "", customPrompt: "" }])}
              className="px-3 py-2 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 transition-colors"
            >
              + Add Profile
            </button>
            {profiles.length > 0 && (
              <button
                onClick={() => saveProfiles(profiles).then(() => setSaveMsg("Profiles saved")).catch((e) => setSaveMsg(String(e)))}
                className="px-3 py-2 rounded-lg text-xs font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-colors"
              >
                Save Profiles
              </button>
            )}
          </div>
        </div>

      </div>

      {/* Save button -- sticky footer, always visible */}
      <div className="px-4 py-3 border-t border-zinc-800/40">
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

// --- Stats helpers -----------------------------------------------------------

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds.toFixed(0)}s`;
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (mins < 60) return `${mins}m ${secs}s`;
  const hrs = Math.floor(mins / 60);
  const remainMins = mins % 60;
  return `${hrs}h ${remainMins}m`;
}

function StatCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3">
      <p className="text-[10px] text-zinc-500 uppercase tracking-wide">{label}</p>
      <p className="text-lg font-semibold text-zinc-200 mt-0.5">
        {value}
        {sub && <span className="text-[10px] text-zinc-500 font-normal ml-1">{sub}</span>}
      </p>
    </div>
  );
}

// --- Main App ----------------------------------------------------------------

export default function App() {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [currentStyle, setCurrentStyle] = useState<CleanupStyle>("polished");
  const [resultText, setResultText] = useState<string | null>(null);
  const [rawText, setRawText] = useState<string | null>(null);
  const [showRawText, setShowRawText] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [usageStats, setUsageStats] = useState<UsageSummary | null>(null);
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historySearch, setHistorySearch] = useState("");
  const [expandedHistoryRaw, setExpandedHistoryRaw] = useState<Set<number>>(new Set());
  const [language, setLanguage] = useState("");
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
      if (p.rawText !== undefined) setRawText(p.rawText);
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
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[],
  ) => {
    await saveSettings(groqKey, deepseekKey, lang, style, hotkey, hotkeyMode, audioDevice, sttModel, customPrompt, autostart, whisperMode, openaiKey, anthropicKey, sttPriority, llmPriority);
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
        const transcript = await transcribeAudio(language);
        setRawText(transcript);
        setRecordingState("cleaning");
        const cleanedText = await cleanupText(transcript, currentStyle);
        setResultText(cleanedText);
        setRecordingState("done");
        // Save to history (fire-and-forget)
        addHistoryEntry(cleanedText, transcript, currentStyle, language).catch(console.error);
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      setResultText(null);
      setRawText(null);
      setShowRawText(false);
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

  const toggleSettings = useCallback(() => {
    setShowSettings((prev) => !prev);
    setShowHistory(false);
    setShowStats(false);
  }, []);

  const toggleHistory = useCallback(() => {
    setShowHistory((prev) => {
      if (!prev) {
        getHistory(50).then(setHistoryEntries).catch(console.error);
      }
      return !prev;
    });
    setShowSettings(false);
    setShowStats(false);
  }, []);

  const toggleStats = useCallback(() => {
    setShowStats((prev) => {
      if (!prev) {
        getUsageStats().then(setUsageStats).catch(console.error);
      }
      return !prev;
    });
    setShowSettings(false);
    setShowHistory(false);
  }, []);

  const handleHistorySearch = useCallback(async (query: string) => {
    setHistorySearch(query);
    if (query.trim()) {
      const results = await searchHistory(query);
      setHistoryEntries(results);
    } else {
      const entries = await getHistory(50);
      setHistoryEntries(entries);
    }
  }, []);

  const handleDeleteHistoryEntry = useCallback(async (id: number) => {
    await deleteHistoryEntry(id);
    setHistoryEntries((prev) => prev.filter((e) => e.id !== id));
  }, []);

  const handleClearHistory = useCallback(async () => {
    await clearHistory();
    setHistoryEntries([]);
  }, []);

  const handleCopyHistoryText = useCallback((text: string) => {
    navigator.clipboard.writeText(text).catch(console.error);
  }, []);

  const hotkeyDisplay = formatHotkeyDisplay(loadedSettings?.hotkey ?? "ctrl+shift+d");

  return (
    <main
      className="h-screen bg-[#0a0a0c] text-zinc-100 flex flex-col select-none overflow-hidden"
      style={{ fontFamily: "'Inter', system-ui, -apple-system, sans-serif" }}
    >
      {/* ── Header ── */}
      <div className="flex items-center justify-between flex-wrap gap-2 px-4 pt-3.5 pb-2 flex-shrink-0">
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

          {/* History toggle */}
          <button
            aria-label="Toggle history"
            aria-expanded={showHistory}
            onClick={toggleHistory}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showHistory
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z" />
            </svg>
          </button>

          {/* Stats toggle */}
          <button
            aria-label="Toggle stats"
            aria-expanded={showStats}
            onClick={toggleStats}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showStats
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
              <path d="M5 9.2h3V19H5V9.2zM10.6 5h2.8v14h-2.8V5zm5.6 8H19v6h-2.8v-6z" />
            </svg>
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
          showSettings ? "max-h-[calc(100vh-100px)] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
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

      {/* ── History Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showHistory ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showHistory && (
          <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
              <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">History</span>
              <div className="flex items-center gap-2">
                {historyEntries.length > 0 && (
                  <button
                    onClick={handleClearHistory}
                    className="text-[10px] text-zinc-500 hover:text-red-400 transition-colors"
                  >
                    Clear All
                  </button>
                )}
                <button
                  onClick={() => setShowHistory(false)}
                  className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
                >
                  <CloseIcon />
                </button>
              </div>
            </div>

            <div className="px-4 pt-3">
              <input
                type="text"
                placeholder="Search..."
                value={historySearch}
                onChange={(e) => handleHistorySearch(e.target.value)}
                className="w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
              />
            </div>

            <div className="overflow-y-auto max-h-[calc(100vh-250px)] p-4 flex flex-col gap-2">
              {historyEntries.length === 0 ? (
                <p className="text-xs text-zinc-500 italic text-center py-4">No dictations yet.</p>
              ) : (
                historyEntries.map((entry) => (
                  <div
                    key={entry.id}
                    className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors"
                  >
                    <p className="text-xs text-zinc-300 whitespace-pre-wrap line-clamp-3">{entry.text}</p>
                    {entry.rawText && entry.rawText !== entry.text && (
                      <div className="mt-1.5">
                        <button
                          onClick={() => setExpandedHistoryRaw((prev) => {
                            const next = new Set(prev);
                            next.has(entry.id) ? next.delete(entry.id) : next.add(entry.id);
                            return next;
                          })}
                          className="text-[10px] text-zinc-600 hover:text-zinc-400 transition-colors"
                        >
                          {expandedHistoryRaw.has(entry.id) ? "Hide original" : "Show original"}
                        </button>
                        {expandedHistoryRaw.has(entry.id) && (
                          <div className="mt-1 relative group/raw">
                            <p className="text-[11px] text-zinc-500 whitespace-pre-wrap bg-[#0c0c0e] rounded-lg px-2.5 py-1.5 border border-zinc-800/40">
                              {entry.rawText}
                            </p>
                            <button
                              onClick={() => navigator.clipboard.writeText(entry.rawText!)}
                              className="absolute top-1 right-1 text-[10px] text-zinc-600 hover:text-zinc-300 opacity-0 group-hover/raw:opacity-100 transition-opacity"
                            >
                              Copy
                            </button>
                          </div>
                        )}
                      </div>
                    )}
                    <div className="flex items-center justify-between mt-2">
                      <span className="text-[10px] text-zinc-500">
                        {new Date(entry.createdAt + "Z").toLocaleString()}
                        {entry.style !== "polished" && ` · ${entry.style}`}
                      </span>
                      <div className="flex gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={() => handleCopyHistoryText(entry.text)}
                          className="text-[10px] text-zinc-500 hover:text-emerald-400 transition-colors"
                        >
                          Copy
                        </button>
                        <button
                          onClick={() => handleDeleteHistoryEntry(entry.id)}
                          className="text-[10px] text-zinc-500 hover:text-red-400 transition-colors"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* ── Stats Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showStats ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showStats && usageStats && (
          <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
              <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">Statistics & Costs</span>
              <button
                onClick={() => setShowStats(false)}
                className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
              >
                <CloseIcon />
              </button>
            </div>

            <div className="p-4 grid grid-cols-2 gap-3">
              {/* Today */}
              <StatCard label="Today" value={`${usageStats.dictationsToday}`} sub="dictations" />
              <StatCard label="Cost Today" value={formatCost(usageStats.costTodayUsd)} sub="USD" />

              {/* All time */}
              <StatCard label="Total Dictations" value={`${usageStats.totalDictations}`} />
              <StatCard label="Total Words" value={usageStats.totalWords.toLocaleString()} />
              <StatCard label="Audio Recorded" value={formatDuration(usageStats.totalAudioSeconds)} />
              <StatCard label="Total Cost" value={formatCost(usageStats.totalCostUsd)} sub="USD" />

              {/* Cost breakdown */}
              <StatCard label="STT (Groq)" value={formatCost(usageStats.totalSttCostUsd)} sub="USD" />
              <StatCard label="LLM (DeepSeek)" value={formatCost(usageStats.totalLlmCostUsd)} sub="USD" />
            </div>
          </div>
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
              : "text-zinc-500",
          ].join(" ")}>
            {errorMessage && recordingState === "error"
              ? errorMessage
              : STATUS_LABELS[recordingState]}
          </p>
        </div>

        {/* Result */}
        {resultText !== null && (
          <div className="w-full max-w-xs flex flex-col gap-1.5">
            <textarea
              readOnly
              value={resultText}
              rows={3}
              className="w-full bg-[#111113] border border-zinc-800/60 rounded-xl px-3.5 py-2.5 text-sm text-zinc-200 resize-none focus:outline-none focus:border-emerald-500/30 transition-colors"
            />
            {rawText && rawText !== resultText && (
              <div>
                <button
                  onClick={() => setShowRawText((v) => !v)}
                  className="text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors"
                >
                  {showRawText ? "Hide original" : "Show original"}
                </button>
                {showRawText && (
                  <div className="mt-1 relative group">
                    <textarea
                      readOnly
                      value={rawText}
                      rows={2}
                      className="w-full bg-[#0c0c0e] border border-zinc-800/40 rounded-lg px-3 py-2 text-xs text-zinc-400 resize-none focus:outline-none"
                    />
                    <button
                      onClick={() => navigator.clipboard.writeText(rawText)}
                      className="absolute top-1.5 right-1.5 text-[10px] text-zinc-600 hover:text-zinc-300 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      Copy
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── Footer ── */}
      <div className="flex items-center justify-center px-4 py-3 flex-shrink-0">
        <span className="text-[11px] font-mono text-zinc-500">{hotkeyDisplay}</span>
      </div>
    </main>
  );
}
