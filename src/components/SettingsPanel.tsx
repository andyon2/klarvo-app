import { useState, useEffect, useCallback } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { AppSettings, CleanupStyle, HotkeyMode, AppProfile } from "../types";
import { STYLE_OPTIONS } from "../types";
import { getProfiles, saveProfiles, syncHistory } from "../tauri-commands";
import { isDesktop, isMobile } from "../platform";
import { CloseIcon } from "./icons";
import { StatusDot, DictionaryTag, INPUT_CLS, LABEL_CLS, SECTION_TITLE_CLS, INPUT_CLS_M, LABEL_CLS_M, SECTION_TITLE_CLS_M } from "./ui";

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

// --- Drag-and-drop provider priority -----------------------------------------

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

/** Mobile-only row with Up/Down buttons instead of drag handle. */
function MobileProviderItem({
  label, active, onUp, onDown, isFirst, isLast,
}: {
  label: string; active: boolean; onUp: () => void; onDown: () => void; isFirst: boolean; isLast: boolean;
}) {
  return (
    <div className={[
      "flex items-center gap-2 px-3 py-2.5 rounded-lg text-sm",
      "bg-[#111113] border",
      active ? "border-emerald-500/30 text-zinc-200" : "border-zinc-800/40 text-zinc-500",
    ].join(" ")}>
      <span className="flex-1">{label}</span>
      <span className={["w-2 h-2 rounded-full flex-shrink-0", active ? "bg-emerald-400" : "bg-zinc-700"].join(" ")} />
      <button
        onClick={onUp}
        disabled={isFirst}
        aria-label="Move up"
        className="min-w-[36px] min-h-[36px] flex items-center justify-center rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-700/50 disabled:opacity-20 disabled:cursor-not-allowed transition-colors"
      >
        <svg viewBox="0 0 24 24" className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
          <path d="M18 15l-6-6-6 6" />
        </svg>
      </button>
      <button
        onClick={onDown}
        disabled={isLast}
        aria-label="Move down"
        className="min-w-[36px] min-h-[36px] flex items-center justify-center rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-700/50 disabled:opacity-20 disabled:cursor-not-allowed transition-colors"
      >
        <svg viewBox="0 0 24 24" className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>
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

  function moveItem(index: number, direction: -1 | 1) {
    const newIdx = index + direction;
    if (newIdx < 0 || newIdx >= items.length) return;
    onChange(arrayMove(items, index, newIdx));
  }

  // On mobile, show Up/Down buttons instead of drag handles.
  if (isMobile) {
    return (
      <div className="flex flex-col gap-1.5">
        {items.map((id, i) => (
          <MobileProviderItem
            key={id}
            label={labels[id] ?? id}
            active={!!keyStatus[id]}
            onUp={() => moveItem(i, -1)}
            onDown={() => moveItem(i, 1)}
            isFirst={i === 0}
            isLast={i === items.length - 1}
          />
        ))}
      </div>
    );
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

// --- Output language options --------------------------------------------------

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

// --- SettingsPanel -----------------------------------------------------------

export interface SettingsPanelProps {
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
  onSave: (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null,
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[],
    outputLanguage: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
  ) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  onAudioDeviceChange: (d: string | null) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
  onOutputLanguageChange: (lang: string) => void;
}

export function SettingsPanel({
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
  const [localTursoUrl, setLocalTursoUrl] = useState(loadedSettings?.tursoUrl ?? "");
  const [tursoToken, setTursoToken] = useState("");
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
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
      setLocalTursoUrl(loadedSettings.tursoUrl ?? "");
    }
  }, [loadedSettings]);

  // Close on Escape.
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
      await onSave(
        groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        openaiKey.trim(), anthropicKey.trim(), localSttPriority, localLlmPriority,
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), tursoToken.trim(),
      );
      setGroqKey("");
      setDeepseekKey("");
      setOpenaiKey("");
      setAnthropicKey("");
      setTursoToken("");
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [
    groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey, anthropicKey,
    localSttPriority, localLlmPriority, localOutputLanguage, localWebhookUrl, localTursoUrl, tursoToken, onSave,
  ]);

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

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col max-h-[calc(100vh-120px)]">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40 flex-shrink-0">
        <span className={SECTION_TITLE_CLS}>Settings</span>
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
          <span className={SECTION_TITLE_CLS_M}>Voice & Recording</span>

          {/* Microphone -- desktop only (Android uses its own mic via MediaRecorder) */}
          {isDesktop && (
            <div className="flex items-center justify-between gap-3">
              <span className={LABEL_CLS_M}>Microphone</span>
              <select
                value={localAudioDevice ?? ""}
                onChange={(e) => handleAudioDeviceChange(e.target.value || null)}
                className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 max-w-[180px] truncate focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer"
              >
                <option value="">System Default</option>
                {audioDevices.map((n) => <option key={n} value={n}>{n}</option>)}
              </select>
            </div>
          )}

          {/* Language */}
          <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
            <span className={LABEL_CLS_M}>Language</span>
            <select
              value={localLang}
              onChange={(e) => handleLangChange(e.target.value)}
              className={`bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
            >
              <option value="">Auto (DE + EN)</option>
              <option value="de">Deutsch</option>
              <option value="en">English</option>
            </select>
          </div>

          {/* Output language (translation) */}
          <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
            <span className={LABEL_CLS_M}>Translate to</span>
            <select
              value={localOutputLanguage}
              onChange={(e) => handleOutputLanguageChange(e.target.value)}
              className={`bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
            >
              {OUTPUT_LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>{l.label}</option>
              ))}
            </select>
          </div>

          {/* Cleanup style */}
          <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
            <span className={LABEL_CLS_M}>Cleanup Style</span>
            <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60">
              {STYLE_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  onClick={() => handleStyleChange(opt.value)}
                  title={opt.description}
                  className={[
                    isMobile ? "flex-1 px-3 py-2 rounded-md text-sm font-medium transition-all duration-100" : "px-2 py-1 rounded-md text-xs font-medium transition-all duration-100",
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
          <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
            <span className={LABEL_CLS_M}>STT Model</span>
            <select
              value={localSttModel}
              onChange={(e) => setLocalSttModel(e.target.value)}
              className={`bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 truncate focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer ${isMobile ? "w-full" : "max-w-[200px]"}`}
            >
              <option value="whisper-large-v3-turbo">Large V3 Turbo ($0.04/h)</option>
              <option value="whisper-large-v3">Large V3 ($0.111/h)</option>
              <option value="distil-whisper-large-v3-en">Distil V3 EN ($0.02/h)</option>
            </select>
          </div>
        </div>

        {/* --- Hotkey -- desktop only (no global hotkeys on Android) --- */}
        {isDesktop && (
          <div className="flex flex-col gap-3">
            <span className={SECTION_TITLE_CLS}>Hotkey</span>

            <div className="flex flex-col gap-1.5">
              <span className="text-xs text-zinc-300">Shortcut</span>
              <ShortcutRecorder value={localHotkey} onChange={handleHotkeyChange} />
            </div>

            <div className="flex items-center justify-between gap-3">
              <span className={LABEL_CLS}>Mode</span>
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
        )}

        {/* --- Custom Prompt --- */}
        <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS_M}>Custom Prompt</span>
          <textarea
            value={localCustomPrompt}
            onChange={(e) => setLocalCustomPrompt(e.target.value)}
            placeholder="Extra instructions for the LLM, e.g. 'Always use formal German' or 'Keep technical terms in English'"
            rows={3}
            className={`${INPUT_CLS_M} resize-none`}
          />
          <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Appended to the system prompt during cleanup.</p>
        </div>

        {/* --- General -- desktop only features --- */}
        {isDesktop && <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS_M}>General</span>
          <label className="flex items-center justify-between gap-3 cursor-pointer">
            <span className={LABEL_CLS_M}>Launch on startup</span>
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
              <span className={LABEL_CLS_M}>Whisper mode</span>
              <span className={isMobile ? "text-xs text-zinc-500" : "text-[10px] text-zinc-500"}>Amplifies mic input for quiet dictation</span>
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
            <span className={LABEL_CLS_M}>Command mode</span>
            <span className={isMobile ? "text-xs text-zinc-500" : "text-[10px] text-zinc-500"}>Select text, hold Ctrl+Shift+E, speak your edit. The selected text will be rewritten.</span>
          </div>
        </div>}

        {/* --- Webhook -- desktop only --- */}
        {isDesktop && (
          <div className="flex flex-col gap-3">
            <span className={SECTION_TITLE_CLS_M}>Webhook</span>
            <input
              type="url"
              placeholder="https://example.com/webhook"
              value={localWebhookUrl}
              onChange={(e) => setLocalWebhookUrl(e.target.value)}
              className={INPUT_CLS_M}
            />
            <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>HTTP POST after each dictation. Leave empty to disable.</p>
          </div>
        )}

        {/* --- Sync --- */}
        <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS_M}>Cross-Device Sync</span>
          <div className="flex flex-col gap-1.5">
            <span className={LABEL_CLS_M}>Turso URL</span>
            <input
              type="text"
              placeholder="libsql://your-db.turso.io"
              value={localTursoUrl}
              onChange={(e) => setLocalTursoUrl(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <span className={LABEL_CLS_M}>Turso Token</span>
            <input
              type="password"
              autoComplete="off"
              placeholder={loadedSettings?.tursoTokenMasked || "Auth token"}
              value={tursoToken}
              onChange={(e) => setTursoToken(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>
          {loadedSettings?.deviceId && (
            <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Device: {loadedSettings.deviceId.slice(0, 8)}...</p>
          )}
          <button
            onClick={async () => {
              setSyncing(true);
              setSyncMsg(null);
              try {
                const [pushed, pulled] = await syncHistory();
                setSyncMsg(`Synced: ${pushed} pushed, ${pulled} pulled`);
              } catch (e: unknown) {
                setSyncMsg(`Error: ${String(e).slice(0, 80)}`);
              } finally {
                setSyncing(false);
              }
            }}
            disabled={syncing || !localTursoUrl}
            className={`px-3 py-1.5 text-sm bg-zinc-700 text-white rounded hover:bg-zinc-600 disabled:opacity-40 transition-colors ${isMobile ? "py-2.5 text-base" : ""}`}
          >
            {syncing ? "Syncing..." : "Sync Now"}
          </button>
          {syncMsg && <p className={isMobile ? "text-xs text-zinc-400" : "text-[11px] text-zinc-400"}>{syncMsg}</p>}
          <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Sync dictation history across devices via Turso. Leave empty to disable.</p>
        </div>

        {/* --- API Keys --- */}
        <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS_M}>API Keys</span>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={LABEL_CLS_M}>Groq</span>
              <StatusDot active={groqOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={groqOk ? loadedSettings!.groqApiKeyMasked : "gsk_..."}
              value={groqKey}
              onChange={(e) => setGroqKey(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={LABEL_CLS_M}>DeepSeek</span>
              <StatusDot active={deepseekOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={deepseekOk ? loadedSettings!.deepseekApiKeyMasked : "sk-..."}
              value={deepseekKey}
              onChange={(e) => setDeepseekKey(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={LABEL_CLS_M}>OpenAI</span>
              <StatusDot active={openaiOk} />
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={openaiOk ? loadedSettings!.openaiApiKeyMasked : "sk-..."}
              value={openaiKey}
              onChange={(e) => setOpenaiKey(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center gap-2">
              <span className={LABEL_CLS_M}>Anthropic</span>
              <StatusDot active={anthropicOk} />
              <span className={isMobile ? "text-xs text-zinc-500" : "text-[10px] text-zinc-500"}>(LLM only)</span>
            </div>
            <input
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={anthropicOk ? loadedSettings!.anthropicApiKeyMasked : "sk-ant-..."}
              value={anthropicKey}
              onChange={(e) => setAnthropicKey(e.target.value)}
              className={INPUT_CLS_M}
            />
          </div>
        </div>

        {/* --- Provider Priority --- */}
        <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS_M}>Provider Priority</span>
          <p className={isMobile ? "text-xs text-zinc-500" : "text-[10px] text-zinc-500"}>
            {isMobile ? "Use arrows to reorder." : "Drag to reorder."} First provider with a configured key is used. If it fails, the next one is tried.
          </p>

          <div className="flex flex-col gap-2">
            <span className={LABEL_CLS_M}>Speech-to-Text</span>
            <ProviderPriorityList
              items={localSttPriority}
              onChange={setLocalSttPriority}
              keyStatus={{ groq: groqOk, openai: openaiOk }}
              labels={{ groq: "Groq Whisper", openai: "OpenAI Whisper" }}
            />
          </div>

          <div className="flex flex-col gap-2">
            <span className={LABEL_CLS_M}>Text Cleanup (LLM)</span>
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
          <span className={SECTION_TITLE_CLS_M}>Dictionary</span>

          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Add word or phrase..."
              value={newTerm}
              onChange={(e) => setNewTerm(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddTerm()}
              className={`flex-1 ${INPUT_CLS_M}`}
            />
            <button
              onClick={handleAddTerm}
              disabled={!newTerm.trim()}
              className={`px-3 rounded-lg font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 disabled:opacity-30 disabled:cursor-not-allowed transition-colors ${isMobile ? "py-2.5 text-sm min-w-[56px]" : "py-2 text-xs"}`}
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

        {/* --- Updates -- desktop only (Tauri updater not available on sideloaded APKs) --- */}
        {isDesktop && <UpdateChecker />}

        {/* --- App Profiles --- */}
        <div className="flex flex-col gap-3">
          <span className={SECTION_TITLE_CLS}>App Profiles</span>
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
                  className={`flex-1 ${INPUT_CLS}`}
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
                className={INPUT_CLS}
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
                className={INPUT_CLS}
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

      {/* Save button -- sticky footer, always visible.
          On Android the navigation bar (Back/Home/Recent) overlaps the bottom of the WebView.
          mobile-safe-bottom uses max(24px, env(safe-area-inset-bottom)) for correct clearance
          even on devices with tall gesture bars (e.g. Xiaomi HyperOS ~48px). */}
      <div className={`px-4 py-3 border-t border-zinc-800/40 ${isMobile ? "mobile-safe-bottom" : ""}`}>
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
