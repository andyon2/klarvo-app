import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { check } from "@tauri-apps/plugin-updater";
import "./styles.css";
import type { RecordingState, CleanupStyle, HotkeyMode, AppSettings, AppProfile, HistoryEntry, UsageSummary, AdvancedSettings } from "./types";
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
  setOutputLanguage as syncOutputLanguage,
  reformatText,
  getFillerStats,
  getNotes,
  saveNote,
  getSnippets,
  saveSnippets,
  pasteSnippet,
  getAdvancedSettings,
  saveAdvancedSettings,
  type TextSnippet,
} from "./tauri-commands";
import Onboarding from "./Onboarding";

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

function GlobeIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function MailIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="4" width="20" height="16" rx="2" />
      <path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
    </svg>
  );
}

function ListIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="8" y1="6" x2="21" y2="6" />
      <line x1="8" y1="12" x2="21" y2="12" />
      <line x1="8" y1="18" x2="21" y2="18" />
      <line x1="3" y1="6" x2="3.01" y2="6" />
      <line x1="3" y1="12" x2="3.01" y2="12" />
      <line x1="3" y1="18" x2="3.01" y2="18" />
    </svg>
  );
}

function SummaryIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14,2 14,8 20,8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
      <line x1="10" y1="9" x2="8" y2="9" />
    </svg>
  );
}

function NoteIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M3 18h12v-2H3v2zM3 6v2h18V6H3zm0 7h18v-2H3v2z" />
    </svg>
  );
}

function SnippetIcon({ className = "w-3.5 h-3.5" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
      <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
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

// --- Output Language Picker --------------------------------------------------

const OUTPUT_LANGUAGES = [
  { code: "", label: "No translation" },
  { code: "en", label: "English" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
  { code: "es", label: "Español" },
  { code: "it", label: "Italiano" },
  { code: "pt", label: "Português" },
  { code: "nl", label: "Nederlands" },
  { code: "pl", label: "Polski" },
  { code: "ru", label: "Русский" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "中文" },
  { code: "ko", label: "한국어" },
];

function OutputLanguagePicker({ value, onChange, disabled }: { value: string; onChange: (lang: string) => void; disabled: boolean }) {
  const isActive = value !== "";
  const activeLabel = OUTPUT_LANGUAGES.find((l) => l.code === value)?.label ?? "";
  // Short badge label: take first token or 2-char code
  const badgeLabel = value.toUpperCase();

  return (
    <div className="relative flex items-center gap-1">
      {isActive && (
        <span className="text-[10px] font-semibold text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded px-1.5 py-0.5 leading-none pointer-events-none select-none">
          {`→ ${badgeLabel}`}
        </span>
      )}
      <div className="relative flex items-center">
        <GlobeIcon className={`w-3.5 h-3.5 absolute left-2 pointer-events-none ${isActive ? "text-emerald-400" : "text-zinc-500"}`} />
        <select
          disabled={disabled}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          title={isActive ? `Translate to ${activeLabel}` : "No translation"}
          aria-label="Output language"
          className={[
            "bg-[#111113] border rounded-lg pl-7 pr-2 py-1 text-[11px] appearance-none cursor-pointer",
            "focus:outline-none transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
            isActive
              ? "border-emerald-500/30 text-emerald-400"
              : "border-zinc-800/60 text-zinc-500 hover:text-zinc-300 hover:border-zinc-700/60",
          ].join(" ")}
        >
          {OUTPUT_LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>{l.label}</option>
          ))}
        </select>
      </div>
    </div>
  );
}

// --- Reformat Buttons --------------------------------------------------------

interface ReformatButtonsProps {
  text: string;
  originalText: string;
  onResult: (text: string) => void;
}

function ReformatButtons({ text, originalText, onResult }: ReformatButtonsProps) {
  const [loading, setLoading] = useState<string | null>(null);
  const isReformatted = text !== originalText;

  const FORMATS = [
    { id: "email", label: "Email", Icon: MailIcon },
    { id: "bullets", label: "Bullets", Icon: ListIcon },
    { id: "summary", label: "Summary", Icon: SummaryIcon },
  ] as const;

  const handleReformat = async (format: string) => {
    if (loading) return;
    setLoading(format);
    try {
      const result = await reformatText(originalText, format);
      onResult(result);
      navigator.clipboard.writeText(result).catch(console.error);
    } catch (err) {
      console.error("reformat_text failed:", err);
    } finally {
      setLoading(null);
    }
  };

  const handleReset = () => {
    onResult(originalText);
    navigator.clipboard.writeText(originalText).catch(console.error);
  };

  return (
    <div className="flex items-center gap-1.5">
      {isReformatted && (
        <button
          onClick={handleReset}
          title="Reset to original"
          className="flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium border bg-zinc-800/60 border-zinc-700/60 text-zinc-300 hover:text-zinc-100 transition-all duration-100"
        >
          <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
            <path d="M3 3v5h5" />
          </svg>
          Reset
        </button>
      )}
      {FORMATS.map(({ id, label, Icon }) => (
        <button
          key={id}
          onClick={() => handleReformat(id)}
          disabled={loading !== null}
          title={`Reformat as ${label}`}
          className={[
            "flex items-center gap-1 px-2 py-1 rounded-lg text-[10px] font-medium border",
            "transition-all duration-100 disabled:opacity-50 disabled:cursor-not-allowed",
            loading === id
              ? "bg-amber-500/10 border-amber-500/20 text-amber-400"
              : "bg-[#111113] border-zinc-800/60 text-zinc-400 hover:text-zinc-200 hover:border-zinc-700/60",
          ].join(" ")}
        >
          {loading === id ? (
            <SpinnerIcon className="w-3 h-3" />
          ) : (
            <Icon className="w-3 h-3" />
          )}
          {label}
        </button>
      ))}
    </div>
  );
}

// --- Filler Stats Chart ------------------------------------------------------

interface FillerEntry {
  word: string;
  count: number;
}

function FillerStatsChart({ entries }: { entries: FillerEntry[] }) {
  if (entries.length === 0) {
    return <p className="text-xs text-zinc-500 italic">No filler words tracked yet.</p>;
  }

  const max = entries[0].count;

  return (
    <div className="flex flex-col gap-1.5">
      {entries.slice(0, 10).map(({ word, count }) => (
        <div key={word} className="flex items-center gap-2">
          <span className="text-[11px] text-zinc-400 w-16 shrink-0 font-mono truncate">{word}</span>
          <div className="flex-1 bg-zinc-800/60 rounded-full h-1.5 overflow-hidden">
            <div
              className="h-full bg-emerald-500/50 rounded-full transition-all duration-300"
              style={{ width: `${Math.round((count / max) * 100)}%` }}
            />
          </div>
          <span className="text-[10px] text-zinc-500 w-6 text-right shrink-0">{count}</span>
        </div>
      ))}
    </div>
  );
}

/** Renders text with search query highlighted and context around first match. */
function HighlightedText({ text, query, className }: { text: string; query: string; className?: string }) {
  if (!query.trim()) {
    return <p className={`${className} line-clamp-3`}>{text}</p>;
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const firstIdx = lowerText.indexOf(lowerQuery);

  let displayText = text;
  let prefix = "";
  let suffix = "";
  if (firstIdx > 60) {
    const start = text.lastIndexOf(" ", firstIdx - 20);
    displayText = text.slice(start > 0 ? start : firstIdx - 40);
    prefix = "…";
  }
  if (displayText.length > 200) {
    const end = displayText.indexOf(" ", 180);
    displayText = displayText.slice(0, end > 0 ? end : 200);
    suffix = "…";
  }

  const parts: { text: string; highlight: boolean }[] = [];
  const lowerDisplay = displayText.toLowerCase();
  let cursor = 0;
  let matchIdx = lowerDisplay.indexOf(lowerQuery, cursor);
  while (matchIdx !== -1) {
    if (matchIdx > cursor) {
      parts.push({ text: displayText.slice(cursor, matchIdx), highlight: false });
    }
    parts.push({ text: displayText.slice(matchIdx, matchIdx + query.length), highlight: true });
    cursor = matchIdx + query.length;
    matchIdx = lowerDisplay.indexOf(lowerQuery, cursor);
  }
  if (cursor < displayText.length) {
    parts.push({ text: displayText.slice(cursor), highlight: false });
  }

  return (
    <p className={className}>
      {prefix}{parts.map((p, i) =>
        p.highlight
          ? <mark key={i} className="bg-emerald-500/30 text-emerald-300 rounded-sm px-0.5">{p.text}</mark>
          : <span key={i}>{p.text}</span>
      )}{suffix}
    </p>
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

// --- Update Checker ----------------------------------------------------------

function UpdateChecker() {
  const [status, setStatus] = useState<"idle" | "checking" | "available" | "downloading" | "upToDate" | "error">("idle");
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleCheck = useCallback(async () => {
    setStatus("checking");
    setErrorMsg(null);
    try {
      const update = await check();
      if (update) {
        setUpdateVersion(update.version);
        setStatus("available");
      } else {
        setStatus("upToDate");
        setTimeout(() => setStatus("idle"), 3000);
      }
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, []);

  const handleInstall = useCallback(async () => {
    setStatus("downloading");
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
      }
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, []);

  return (
    <div className="flex flex-col gap-2">
      <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-widest">Updates</span>
      <div className="flex items-center gap-2">
        {status === "available" ? (
          <button
            onClick={handleInstall}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-colors"
          >
            Install v{updateVersion}
          </button>
        ) : (
          <button
            onClick={handleCheck}
            disabled={status === "checking" || status === "downloading"}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {status === "checking" ? "Checking..." : status === "downloading" ? "Downloading..." : status === "upToDate" ? "Up to date" : "Check for updates"}
          </button>
        )}
        <span className="text-[10px] text-zinc-500">v0.4.0</span>
      </div>
      {errorMsg && <p className="text-[10px] text-red-400">{errorMsg}</p>}
    </div>
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
  outputLanguage: string;
  onSave: (groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle, hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null, sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean, openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[], outputLanguage: string, webhookUrl: string) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  onAudioDeviceChange: (d: string | null) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
  onOutputLanguageChange: (lang: string) => void;
}

function SettingsPanel({
  onClose, loadedSettings, language, cleanupStyle, hotkey, hotkeyMode,
  audioDevice, audioDevices, dictionary, outputLanguage,
  onSave, onLanguageChange, onStyleChange, onHotkeyChange, onHotkeyModeChange,
  onAudioDeviceChange, onAddTerm, onRemoveTerm, onOutputLanguageChange,
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
  const [localOutputLanguage, setLocalOutputLanguage] = useState(outputLanguage);
  useEffect(() => { setLocalOutputLanguage(outputLanguage); }, [outputLanguage]);
  const [localWebhookUrl, setLocalWebhookUrl] = useState(loadedSettings?.webhookUrl ?? "");
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
      setLocalOutputLanguage(loadedSettings.outputLanguage ?? "");
      setLocalWebhookUrl(loadedSettings.webhookUrl ?? "");
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

  const handleOutputLanguageChange = useCallback((lang: string) => {
    setLocalOutputLanguage(lang);
    onOutputLanguageChange(lang);
  }, [onOutputLanguageChange]);

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
      await onSave(groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey.trim(), anthropicKey.trim(), localSttPriority, localLlmPriority, localOutputLanguage, localWebhookUrl.trim());
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
  }, [groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey, anthropicKey, localSttPriority, localLlmPriority, localOutputLanguage, localWebhookUrl, onSave]);

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

          {/* Output language (translation) */}
          <div className="flex items-center justify-between gap-3">
            <span className={labelCls}>Translate to</span>
            <select
              value={localOutputLanguage}
              onChange={(e) => handleOutputLanguageChange(e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              {OUTPUT_LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>{l.label}</option>
              ))}
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

        {/* --- Webhook --- */}
        <div className="flex flex-col gap-3">
          <span className={sectionTitleCls}>Webhook</span>
          <input
            type="url"
            placeholder="https://example.com/webhook"
            value={localWebhookUrl}
            onChange={(e) => setLocalWebhookUrl(e.target.value)}
            className={inputCls}
          />
          <p className="text-[11px] text-zinc-500">HTTP POST after each dictation. Leave empty to disable.</p>
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

        {/* --- Updates --- */}
        <UpdateChecker />

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

// --- Snippets Quick-Access Panel ----------------------------------------------

function SnippetsPanel({
  snippets, onUpdate, onClose,
}: {
  snippets: TextSnippet[];
  onUpdate: (s: TextSnippet[]) => void;
  onClose: () => void;
}) {
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const inputCls = "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors";

  const handlePaste = async (content: string) => {
    try {
      await pasteSnippet(content);
    } catch (err) {
      console.error("paste_snippet failed:", err);
    }
  };

  const handleSave = async () => {
    try {
      const clean = snippets.filter((s) => s.name.trim() || s.content.trim());
      await saveSnippets(clean);
      onUpdate(clean);
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(String(err));
    }
  };

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">Text Snippets</span>
        <button
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      <div className="overflow-y-auto max-h-[400px] p-4 flex flex-col gap-2">
        <p className="text-[10px] text-zinc-500">Click "Paste" to insert a snippet into the active window.</p>

        {snippets.length === 0 ? (
          <p className="text-xs text-zinc-500 italic text-center py-4">No snippets yet. Add your first one below.</p>
        ) : (
          snippets.map((s, i) => (
            <div key={i} className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors">
              <div className="flex items-center justify-between gap-2 mb-2">
                <input
                  type="text"
                  placeholder="Name"
                  value={s.name}
                  onChange={(e) => {
                    const next = [...snippets];
                    next[i] = { ...next[i], name: e.target.value };
                    onUpdate(next);
                  }}
                  className={`flex-1 ${inputCls}`}
                />
                <button
                  onClick={() => onUpdate(snippets.filter((_, j) => j !== i))}
                  className="text-zinc-500 hover:text-red-400 transition-colors p-1"
                >
                  <CloseIcon />
                </button>
              </div>
              <textarea
                placeholder="Content to paste..."
                value={s.content}
                onChange={(e) => {
                  const next = [...snippets];
                  next[i] = { ...next[i], content: e.target.value };
                  onUpdate(next);
                }}
                rows={2}
                className={`${inputCls} resize-none`}
              />
              <div className="flex justify-end mt-2">
                <button
                  onClick={() => handlePaste(s.content)}
                  disabled={!s.content.trim()}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-[10px] font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-all disabled:opacity-30 disabled:cursor-not-allowed"
                >
                  Paste
                </button>
              </div>
            </div>
          ))
        )}

        <div className="flex gap-2 pt-1">
          <button
            onClick={() => onUpdate([...snippets, { name: "", content: "" }])}
            className="px-3 py-2 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 transition-colors"
          >
            + Add Snippet
          </button>
          {snippets.length > 0 && (
            <button
              onClick={handleSave}
              className="px-3 py-2 rounded-lg text-xs font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-colors"
            >
              {saveMsg ?? "Save"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// --- Voice Notes Panel -------------------------------------------------------

function VoiceNotesPanel({
  notes, onRefresh, onClose,
}: {
  notes: HistoryEntry[];
  onRefresh: () => void;
  onClose: () => void;
}) {
  const [noteState, setNoteState] = useState<"idle" | "recording" | "processing">("idle");
  const [noteError, setNoteError] = useState<string | null>(null);

  const handleRecordNote = useCallback(async () => {
    if (noteState === "recording") {
      // Stop and save as note
      setNoteState("processing");
      try {
        await stopRecording();
        const transcript = await transcribeAudio("");
        const cleaned = await cleanupText(transcript, "polished");
        await saveNote(cleaned, transcript, "polished");
        onRefresh();
        setNoteState("idle");
      } catch (err) {
        setNoteError(err instanceof Error ? err.message : String(err));
        setNoteState("idle");
      }
    } else {
      setNoteError(null);
      try {
        await startRecording();
        setNoteState("recording");
      } catch (err) {
        setNoteError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [noteState, onRefresh]);

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">Voice Notes</span>
        <button
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Record note button */}
      <div className="px-4 pt-3 flex items-center gap-3">
        <button
          onClick={handleRecordNote}
          disabled={noteState === "processing"}
          className={[
            "flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-medium border transition-all duration-150",
            noteState === "recording"
              ? "bg-red-500/15 border-red-500/30 text-red-400"
              : noteState === "processing"
              ? "bg-amber-500/10 border-amber-500/20 text-amber-400 opacity-60 cursor-not-allowed"
              : "bg-emerald-500/10 border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15",
          ].join(" ")}
        >
          {noteState === "recording" ? (
            <><StopIcon className="w-3.5 h-3.5" /> Stop & Save</>
          ) : noteState === "processing" ? (
            <><SpinnerIcon className="w-3.5 h-3.5" /> Processing...</>
          ) : (
            <><MicIcon className="w-3.5 h-3.5" /> Record Note</>
          )}
        </button>
        {noteError && <span className="text-[10px] text-red-400">{noteError}</span>}
        <p className="text-[10px] text-zinc-500 ml-auto">Notes are saved, not pasted.</p>
      </div>

      {/* Notes list */}
      <div className="overflow-y-auto max-h-[300px] p-4 flex flex-col gap-2">
        {notes.length === 0 ? (
          <p className="text-xs text-zinc-500 italic text-center py-4">No voice notes yet. Record your first one!</p>
        ) : (
          notes.map((note) => (
            <div
              key={note.id}
              className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors"
            >
              <p className="text-xs text-zinc-300 whitespace-pre-wrap line-clamp-3">{note.text}</p>
              <div className="flex items-center justify-between mt-2">
                <span className="text-[10px] text-zinc-500">
                  {new Date(note.createdAt + "Z").toLocaleString()}
                </span>
                <button
                  onClick={() => navigator.clipboard.writeText(note.text).catch(console.error)}
                  className="text-[10px] text-zinc-500 hover:text-emerald-400 opacity-0 group-hover:opacity-100 transition-all"
                >
                  Copy
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// --- Advanced Settings Panel -------------------------------------------------

const ADVANCED_DEFAULTS: AdvancedSettings = {
  sttPromptDe: "",
  sttPromptEn: "",
  sttPromptAuto: "",
  sttTemperature: 0,
  llmSystemPromptPolished: "",
  llmSystemPromptVerbatim: "",
  llmSystemPromptChat: "",
  llmCommandModePrompt: "",
  llmTemperature: 0.3,
  llmMaxTokens: 1024,
  llmModelDeepseek: "deepseek-chat",
  llmModelOpenai: "gpt-4o-mini",
  llmModelAnthropic: "claude-haiku-4-5-20251001",
  llmModelGroq: "llama-3.3-70b-versatile",
  chunkThreshold: 400,
  chunkTargetSize: 300,
  silenceThreshold: 0.005,
  whisperModeThreshold: 0.001,
  minRecordingMs: 500,
  whisperModeGain: 3.0,
  autoPaste: true,
  pasteDelayMs: 80,
  autoCapitalize: false,
  webhookHeaders: "",
  webhookTimeoutSecs: 10,
  logLevel: "info",
};

function AccordionSection({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="border border-zinc-800/60 rounded-xl overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-3 py-2.5 bg-[#111113] hover:bg-zinc-800/40 transition-colors text-left"
      >
        <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-widest">{title}</span>
        <span
          className={[
            "text-zinc-500 text-xs transition-transform duration-150 select-none",
            open ? "rotate-90" : "",
          ].join(" ")}
        >
          ▸
        </span>
      </button>
      {open && (
        <div className="px-3 py-3 flex flex-col gap-3 bg-[#0e0e11]">
          {children}
        </div>
      )}
    </div>
  );
}

function AdvancedSettingsPanel({ onClose }: { onClose: () => void }) {
  const [settings, setSettings] = useState<AdvancedSettings>(ADVANCED_DEFAULTS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  const inputCls = "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors";
  const labelCls = "text-xs text-zinc-300";
  const hintCls = "text-[10px] text-zinc-500 leading-relaxed";
  const numberInputCls = `${inputCls} w-28`;

  useEffect(() => {
    getAdvancedSettings()
      .then((s) => { setSettings(s); setLoaded(true); })
      .catch((err) => {
        // Backend may not have the command yet in dev; fall back to defaults.
        console.warn("get_advanced_settings failed, using defaults:", err);
        setLoaded(true);
      });
  }, []);

  // Close on Escape.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const set = useCallback(<K extends keyof AdvancedSettings>(key: K, value: AdvancedSettings[K]) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveMsg(null);
    try {
      await saveAdvancedSettings(settings);
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [settings]);

  const handleReset = useCallback(() => {
    setSettings(ADVANCED_DEFAULTS);
    setSaveMsg(null);
  }, []);

  if (!loaded) {
    return (
      <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl p-6 text-center">
        <SpinnerIcon className="w-5 h-5 text-zinc-500 mx-auto" />
      </div>
    );
  }

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col max-h-[calc(100vh-120px)]">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40 flex-shrink-0">
        <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-widest">Advanced Settings</span>
        <button
          aria-label="Close advanced settings"
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Scrollable body */}
      <div className="overflow-y-auto flex-1 min-h-0 p-4 flex flex-col gap-3">

        {/* --- Speech-to-Text --- */}
        <AccordionSection title="Speech-to-Text (STT)" defaultOpen={true}>
          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>STT Prompt (German)</span>
            <textarea
              value={settings.sttPromptDe}
              onChange={(e) => set("sttPromptDe", e.target.value)}
              placeholder="Context prompt sent with German transcriptions, e.g. 'Fachbegriffe: XYZ'"
              rows={2}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Injected as context when language is set to German.</span>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>STT Prompt (English)</span>
            <textarea
              value={settings.sttPromptEn}
              onChange={(e) => set("sttPromptEn", e.target.value)}
              placeholder="Context prompt for English transcriptions"
              rows={2}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Injected as context when language is set to English.</span>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>STT Prompt (Auto-detect)</span>
            <textarea
              value={settings.sttPromptAuto}
              onChange={(e) => set("sttPromptAuto", e.target.value)}
              placeholder="Context prompt for auto-detect mode"
              rows={2}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Used when language is set to Auto (DE + EN).</span>
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>STT Temperature</span>
              <span className={hintCls}>0.0 = deterministic, 1.0 = more creative. Default: 0.0</span>
            </div>
            <input
              type="number"
              min={0}
              max={1}
              step={0.1}
              value={settings.sttTemperature}
              onChange={(e) => set("sttTemperature", parseFloat(e.target.value) || 0)}
              className={numberInputCls}
            />
          </div>
        </AccordionSection>

        {/* --- LLM Cleanup --- */}
        <AccordionSection title="LLM Cleanup">
          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>System Prompt: Polished</span>
            <textarea
              value={settings.llmSystemPromptPolished}
              onChange={(e) => set("llmSystemPromptPolished", e.target.value)}
              placeholder="Leave empty for built-in default"
              rows={3}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Overrides the built-in system prompt for Polished mode.</span>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>System Prompt: Verbatim</span>
            <textarea
              value={settings.llmSystemPromptVerbatim}
              onChange={(e) => set("llmSystemPromptVerbatim", e.target.value)}
              placeholder="Leave empty for built-in default"
              rows={3}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Overrides the built-in system prompt for Verbatim (Clean) mode.</span>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>System Prompt: Chat</span>
            <textarea
              value={settings.llmSystemPromptChat}
              onChange={(e) => set("llmSystemPromptChat", e.target.value)}
              placeholder="Leave empty for built-in default"
              rows={3}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>Overrides the built-in system prompt for Chat mode.</span>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>Command Mode Prompt</span>
            <textarea
              value={settings.llmCommandModePrompt}
              onChange={(e) => set("llmCommandModePrompt", e.target.value)}
              placeholder="Leave empty for built-in default"
              rows={3}
              className={`${inputCls} resize-none`}
            />
            <span className={hintCls}>System prompt used when rewriting selected text via Command Mode (Ctrl+Shift+E).</span>
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>LLM Temperature</span>
              <span className={hintCls}>0.0 – 2.0. Lower = more focused, higher = more varied.</span>
            </div>
            <input
              type="number"
              min={0}
              max={2}
              step={0.1}
              value={settings.llmTemperature}
              onChange={(e) => set("llmTemperature", parseFloat(e.target.value) || 0)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Max Tokens</span>
              <span className={hintCls}>Maximum output tokens per LLM request.</span>
            </div>
            <input
              type="number"
              min={64}
              max={8192}
              step={1}
              value={settings.llmMaxTokens}
              onChange={(e) => set("llmMaxTokens", parseInt(e.target.value, 10) || 1024)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Model: DeepSeek</span>
              <span className={hintCls}>Model ID sent to the DeepSeek API.</span>
            </div>
            <input
              type="text"
              placeholder="deepseek-chat"
              value={settings.llmModelDeepseek}
              onChange={(e) => set("llmModelDeepseek", e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors w-44"
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Model: OpenAI</span>
              <span className={hintCls}>Model ID sent to the OpenAI API.</span>
            </div>
            <input
              type="text"
              placeholder="gpt-4o-mini"
              value={settings.llmModelOpenai}
              onChange={(e) => set("llmModelOpenai", e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors w-44"
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Model: Anthropic</span>
              <span className={hintCls}>Model ID sent to the Anthropic API.</span>
            </div>
            <input
              type="text"
              placeholder="claude-haiku-4-5-20251001"
              value={settings.llmModelAnthropic}
              onChange={(e) => set("llmModelAnthropic", e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors w-44"
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Model: Groq</span>
              <span className={hintCls}>Model ID sent to the Groq LLM API.</span>
            </div>
            <input
              type="text"
              placeholder="llama-3.3-70b-versatile"
              value={settings.llmModelGroq}
              onChange={(e) => set("llmModelGroq", e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors w-44"
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Chunk Threshold</span>
              <span className={hintCls}>Word count above which text is split into parallel chunks.</span>
            </div>
            <input
              type="number"
              min={50}
              step={1}
              value={settings.chunkThreshold}
              onChange={(e) => set("chunkThreshold", parseInt(e.target.value, 10) || 400)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Chunk Target Size</span>
              <span className={hintCls}>Target word count per chunk when splitting long texts.</span>
            </div>
            <input
              type="number"
              min={50}
              step={1}
              value={settings.chunkTargetSize}
              onChange={(e) => set("chunkTargetSize", parseInt(e.target.value, 10) || 300)}
              className={numberInputCls}
            />
          </div>
        </AccordionSection>

        {/* --- Audio --- */}
        <AccordionSection title="Audio">
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Silence Threshold</span>
              <span className={hintCls}>RMS level below which audio is considered silence. Lower = more sensitive (0.0 – 0.1).</span>
            </div>
            <input
              type="number"
              min={0}
              max={0.1}
              step={0.001}
              value={settings.silenceThreshold}
              onChange={(e) => set("silenceThreshold", parseFloat(e.target.value) || 0)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Whisper Mode Threshold</span>
              <span className={hintCls}>Silence threshold used specifically in Whisper Mode (should be lower than the normal threshold).</span>
            </div>
            <input
              type="number"
              min={0}
              max={0.1}
              step={0.001}
              value={settings.whisperModeThreshold}
              onChange={(e) => set("whisperModeThreshold", parseFloat(e.target.value) || 0)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Min Recording Duration (ms)</span>
              <span className={hintCls}>Recordings shorter than this are discarded (avoids accidental triggers).</span>
            </div>
            <input
              type="number"
              min={0}
              step={50}
              value={settings.minRecordingMs}
              onChange={(e) => set("minRecordingMs", parseInt(e.target.value, 10) || 500)}
              className={numberInputCls}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Whisper Mode Gain</span>
              <span className={hintCls}>Amplification multiplier applied in Whisper Mode. Higher = louder mic input.</span>
            </div>
            <input
              type="number"
              min={1}
              max={20}
              step={0.5}
              value={settings.whisperModeGain}
              onChange={(e) => set("whisperModeGain", parseFloat(e.target.value) || 1)}
              className={numberInputCls}
            />
          </div>
        </AccordionSection>

        {/* --- Paste & Behavior --- */}
        <AccordionSection title="Paste & Behavior">
          <label className="flex items-center justify-between gap-3 cursor-pointer">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Auto-Paste</span>
              <span className={hintCls}>Automatically paste the result into the active window after cleanup.</span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={settings.autoPaste}
              onClick={() => set("autoPaste", !settings.autoPaste)}
              className={[
                "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
                settings.autoPaste ? "bg-emerald-500/40" : "bg-zinc-700",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                  settings.autoPaste ? "translate-x-4" : "",
                ].join(" ")}
              />
            </button>
          </label>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Paste Delay (ms)</span>
              <span className={hintCls}>Milliseconds to wait between focusing the target window and sending the paste keystroke.</span>
            </div>
            <input
              type="number"
              min={0}
              max={2000}
              step={10}
              value={settings.pasteDelayMs}
              onChange={(e) => set("pasteDelayMs", parseInt(e.target.value, 10) || 0)}
              className={numberInputCls}
            />
          </div>

          <label className="flex items-center justify-between gap-3 cursor-pointer">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Auto-Capitalize</span>
              <span className={hintCls}>Automatically capitalize the first letter of every dictation result.</span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={settings.autoCapitalize}
              onClick={() => set("autoCapitalize", !settings.autoCapitalize)}
              className={[
                "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
                settings.autoCapitalize ? "bg-emerald-500/40" : "bg-zinc-700",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                  settings.autoCapitalize ? "translate-x-4" : "",
                ].join(" ")}
              />
            </button>
          </label>
        </AccordionSection>

        {/* --- Webhook --- */}
        <AccordionSection title="Webhook">
          <div className="flex flex-col gap-1.5">
            <span className={labelCls}>Custom Headers (JSON)</span>
            <textarea
              value={settings.webhookHeaders}
              onChange={(e) => set("webhookHeaders", e.target.value)}
              placeholder={'{"Authorization": "Bearer ..."}'}
              rows={3}
              className={`${inputCls} resize-none font-mono`}
            />
            <span className={hintCls}>Additional HTTP headers sent with each webhook request. Must be valid JSON.</span>
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Timeout (seconds)</span>
              <span className={hintCls}>Maximum time to wait for a webhook response before giving up.</span>
            </div>
            <input
              type="number"
              min={1}
              max={120}
              step={1}
              value={settings.webhookTimeoutSecs}
              onChange={(e) => set("webhookTimeoutSecs", parseInt(e.target.value, 10) || 10)}
              className={numberInputCls}
            />
          </div>
        </AccordionSection>

        {/* --- System --- */}
        <AccordionSection title="System">
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5">
              <span className={labelCls}>Log Level</span>
              <span className={hintCls}>Verbosity of backend logs. Use "debug" when troubleshooting.</span>
            </div>
            <select
              value={settings.logLevel}
              onChange={(e) => set("logLevel", e.target.value)}
              className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-2 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
            >
              <option value="debug">debug</option>
              <option value="info">info</option>
              <option value="warn">warn</option>
              <option value="error">error</option>
            </select>
          </div>
        </AccordionSection>

      </div>

      {/* Sticky footer: Save + Reset */}
      <div className="px-4 py-3 border-t border-zinc-800/40 flex gap-2">
        <button
          onClick={handleReset}
          className="px-4 py-2.5 rounded-xl text-sm font-medium border bg-[#111113] border-zinc-700/60 text-zinc-400 hover:text-zinc-200 hover:border-zinc-600 transition-all duration-150 flex-shrink-0"
        >
          Reset to Defaults
        </button>
        <button
          onClick={handleSave}
          disabled={saving}
          className={[
            "flex-1 py-2.5 rounded-xl text-sm font-medium transition-all duration-150 border",
            saveMsg === "Saved"
              ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
              : saveMsg && saveMsg !== "Saved"
              ? "bg-red-500/10 border-red-500/20 text-red-400"
              : "bg-emerald-500/10 border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 hover:border-emerald-500/30",
            "disabled:opacity-50 disabled:cursor-not-allowed",
          ].join(" ")}
        >
          {saving ? "Saving..." : saveMsg ?? "Save"}
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
  const [originalResultText, setOriginalResultText] = useState<string | null>(null);
  const [rawText, setRawText] = useState<string | null>(null);
  const [showRawText, setShowRawText] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [usageStats, setUsageStats] = useState<UsageSummary | null>(null);
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historySearch, setHistorySearch] = useState("");
  const [historyAppSearch, setHistoryAppSearch] = useState("");
  const [expandedHistoryRaw, setExpandedHistoryRaw] = useState<Set<number>>(new Set());
  const [language, setLanguage] = useState("");
  const [loadedSettings, setLoadedSettings] = useState<AppSettings | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [dictionary, setDictionary] = useState<string[]>([]);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [localHotkey, setLocalHotkey] = useState("ctrl+shift+d");
  const [localHotkeyMode, setLocalHotkeyMode] = useState<HotkeyMode>("hold");
  const [localAudioDevice, setLocalAudioDevice] = useState<string | null>(null);
  const [outputLanguage, setOutputLanguage] = useState("");
  const [fillerStats, setFillerStats] = useState<{word: string; count: number}[]>([]);
  const [showFillerStats, setShowFillerStats] = useState(false);
  const [showNotes, setShowNotes] = useState(false);
  const [notes, setNotes] = useState<HistoryEntry[]>([]);
  const [showSnippets, setShowSnippets] = useState(false);
  const [snippetsList, setSnippetsList] = useState<TextSnippet[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const toggleAdvanced = useCallback(() => {
    setShowAdvanced((prev) => !prev);
    setShowSettings(false);
    setShowHistory(false);
    setShowStats(false);
    setShowNotes(false);
    setShowSnippets(false);
  }, []);

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
      setOutputLanguage(s.outputLanguage || "");
      syncLanguage(s.language).catch(console.error);
      syncCleanupStyle(s.cleanupStyle).catch(console.error);
    }).catch(console.error);

    // Show onboarding wizard if no API keys are configured yet.
    // isFirstRun() is implemented on the backend side (see src-tauri/src/lib.rs).
    invoke<boolean>("is_first_run").then((firstRun) => {
      if (firstRun) setShowOnboarding(true);
    }).catch(console.error);

    getDictionaryTerms().then(setDictionary).catch(console.error);
    listAudioDevices().then(setAudioDevices).catch(console.error);
  }, []);

  // Subscribe to backend pipeline events.
  useEffect(() => {
    const unlisten = onStateChanged((p) => {
      setRecordingState(p.state as RecordingState);
      if (p.text !== undefined) { setResultText(p.text); setOriginalResultText(p.text); }
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

  const handleOutputLanguageChange = useCallback((lang: string) => {
    setOutputLanguage(lang);
    syncOutputLanguage(lang).catch(console.error);
  }, []);

  const handleSaveSettings = useCallback(async (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null,
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[],
    outputLang: string, webhookUrl: string,
  ) => {
    await saveSettings(groqKey, deepseekKey, lang, style, hotkey, hotkeyMode, audioDevice, sttModel, customPrompt, autostart, whisperMode, openaiKey, anthropicKey, sttPriority, llmPriority, outputLang, webhookUrl);
    const updated = await getSettings();
    setLoadedSettings(updated);
    setLanguage(updated.language);
    setCurrentStyle(updated.cleanupStyle);
    setLocalHotkey(updated.hotkey);
    setLocalHotkeyMode(updated.hotkeyMode);
    setLocalAudioDevice(updated.audioDevice);
    setOutputLanguage(updated.outputLanguage || "");
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
        setOriginalResultText(cleanedText);
        setRecordingState("done");
        // Save to history (fire-and-forget)
        addHistoryEntry(cleanedText, transcript, currentStyle, language).catch(console.error);
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      setResultText(null);
      setOriginalResultText(null);
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
    setShowNotes(false);
    setShowSnippets(false);
    setShowAdvanced(false);
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
    setShowNotes(false);
    setShowSnippets(false);
    setShowAdvanced(false);
  }, []);

  const toggleStats = useCallback(() => {
    setShowStats((prev) => {
      if (!prev) {
        getUsageStats().then(setUsageStats).catch(console.error);
        getFillerStats().then(setFillerStats).catch(console.error);
      }
      return !prev;
    });
    setShowSettings(false);
    setShowHistory(false);
    setShowNotes(false);
    setShowSnippets(false);
    setShowAdvanced(false);
  }, []);

  const toggleNotes = useCallback(() => {
    setShowNotes((prev) => {
      if (!prev) {
        getNotes(50).then(setNotes).catch(console.error);
      }
      return !prev;
    });
    setShowSettings(false);
    setShowHistory(false);
    setShowStats(false);
    setShowSnippets(false);
    setShowAdvanced(false);
  }, []);

  const toggleSnippets = useCallback(() => {
    setShowSnippets((prev) => {
      if (!prev) {
        getSnippets().then(setSnippetsList).catch(console.error);
      }
      return !prev;
    });
    setShowSettings(false);
    setShowHistory(false);
    setShowStats(false);
    setShowNotes(false);
    setShowAdvanced(false);
  }, []);

  const handleHistorySearch = useCallback(async (textQ: string, appQ: string) => {
    setHistorySearch(textQ);
    setHistoryAppSearch(appQ);
    if (textQ.trim() || appQ.trim()) {
      const results = await searchHistory(textQ.trim() || undefined, appQ.trim() || undefined);
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

  const handleOnboardingComplete = useCallback(async (updated: AppSettings) => {
    setLoadedSettings(updated);
    setLanguage(updated.language);
    setCurrentStyle(updated.cleanupStyle);
    setLocalHotkey(updated.hotkey);
    setLocalHotkeyMode(updated.hotkeyMode);
    setLocalAudioDevice(updated.audioDevice);
    syncLanguage(updated.language).catch(console.error);
    syncCleanupStyle(updated.cleanupStyle).catch(console.error);
    setShowOnboarding(false);
  }, []);

  if (showOnboarding) {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

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

          {/* Notes toggle */}
          <button
            aria-label="Toggle voice notes"
            aria-expanded={showNotes}
            onClick={toggleNotes}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showNotes
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <NoteIcon className="w-4 h-4" />
          </button>

          {/* Snippets toggle */}
          <button
            aria-label="Toggle snippets"
            aria-expanded={showSnippets}
            onClick={toggleSnippets}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showSnippets
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <SnippetIcon className="w-4 h-4" />
          </button>
          <button
            title="Advanced settings"
            aria-label="Toggle advanced settings"
            aria-expanded={showAdvanced}
            onClick={toggleAdvanced}
            className={[
              "p-1.5 rounded-lg transition-all duration-150",
              showAdvanced
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
            </svg>
          </button>
        </div>

        {/* Style picker + output language in header */}
        <div className="flex items-center gap-1.5">
          <StylePicker
            value={currentStyle}
            onChange={handleStyleChange}
            disabled={isBusy || isRecording}
          />
          <OutputLanguagePicker
            value={outputLanguage}
            onChange={handleOutputLanguageChange}
            disabled={isBusy || isRecording}
          />
        </div>
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
            outputLanguage={outputLanguage}
            onOutputLanguageChange={handleOutputLanguageChange}
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

            <div className="px-4 pt-3 flex gap-2">
              <input
                type="text"
                placeholder="Search text..."
                value={historySearch}
                onChange={(e) => handleHistorySearch(e.target.value, historyAppSearch)}
                className="flex-1 bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
              />
              <input
                type="text"
                placeholder="App..."
                value={historyAppSearch}
                onChange={(e) => handleHistorySearch(historySearch, e.target.value)}
                className="w-24 bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
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
                    <HighlightedText text={entry.text} query={historySearch} className="text-xs text-zinc-300 whitespace-pre-wrap" />
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
                        {entry.appName && (
                          <span className="ml-1 px-1.5 py-0.5 bg-zinc-800/60 rounded text-[9px] text-zinc-400">{entry.appName}</span>
                        )}
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

            {/* Filler words (collapsible) */}
            {fillerStats.length > 0 && (
              <div className="px-4 pb-4">
                <button
                  onClick={() => setShowFillerStats((v) => !v)}
                  className="flex items-center gap-1.5 text-[10px] font-semibold text-zinc-500 uppercase tracking-widest hover:text-zinc-300 transition-colors w-full text-left"
                >
                  <span className={`transition-transform duration-150 ${showFillerStats ? "rotate-90" : ""}`}>▸</span>
                  Top Filler Words
                </button>
                {showFillerStats && (
                  <div className="mt-2">
                    <FillerStatsChart entries={fillerStats} />
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── Voice Notes Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showNotes ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showNotes && (
          <VoiceNotesPanel
            notes={notes}
            onRefresh={() => getNotes(50).then(setNotes).catch(console.error)}
            onClose={() => setShowNotes(false)}
          />
        )}
      </div>

      {/* ── Snippets Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showSnippets ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showSnippets && (
          <SnippetsPanel
            snippets={snippetsList}
            onUpdate={setSnippetsList}
            onClose={() => setShowSnippets(false)}
          />
        )}
      </div>

      {/* ── Advanced Settings Panel (toggleable) ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          showAdvanced ? "max-h-[calc(100vh-100px)] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {showAdvanced && (
          <AdvancedSettingsPanel onClose={() => setShowAdvanced(false)} />
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
            {/* Reformat buttons */}
            {recordingState === "done" && (
              <ReformatButtons text={resultText} originalText={originalResultText ?? resultText} onResult={(t) => setResultText(t)} />
            )}
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
