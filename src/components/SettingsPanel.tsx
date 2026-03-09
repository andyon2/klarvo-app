import { useState, useEffect, useCallback, useRef } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check } from "@tauri-apps/plugin-updater";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { AppSettings, CleanupStyle, HotkeyMode, AppProfile, ParsedLicenseStatus } from "../types";
import { STYLE_OPTIONS } from "../types";
import { getProfiles, saveProfiles, syncHistory } from "../tauri-commands";
import { isDesktop, isMobile } from "../platform";
import { CloseIcon, LockIcon } from "./icons";
import { StatusDot, DictionaryTag, INPUT_CLS, LABEL_CLS, SECTION_TITLE_CLS, INPUT_CLS_M, LABEL_CLS_M } from "./ui";
import { MobileTextarea } from "./MobileTextarea";

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
      <span className="text-[11px] font-semibold text-zinc-400 uppercase tracking-widest">Updates</span>
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
        <span className="text-[11px] text-zinc-500">v0.4.0</span>
      </div>
      {errorMsg && <p className="text-[11px] text-red-400">{errorMsg}</p>}
    </div>
  );
}

// --- License Section ---------------------------------------------------------

// Auto-formats a license key input: uppercase, inserts dashes after every 4 chars
// in the payload section (after "DIKTA-").
function formatLicenseKeyInput(raw: string): string {
  // Strip everything that is not alphanumeric.
  const stripped = raw.replace(/[^a-zA-Z0-9]/g, "").toUpperCase();
  // The key format is DIKTA-XXXX-XXXX-XXXX-XXXX.
  // The prefix "DIKTA" is 5 chars, then groups of 4 separated by dashes.
  if (stripped.length === 0) return "";
  const prefix = "DIKTA";
  if (!stripped.startsWith(prefix)) {
    // Let the user type freely if they haven't matched the prefix yet.
    // Still uppercase, no dashes until prefix is complete.
    if (stripped.length <= prefix.length) return stripped;
    // Prefix matched now.
  }
  const body = stripped.startsWith(prefix) ? stripped.slice(prefix.length) : stripped;
  const chunks: string[] = [];
  for (let i = 0; i < body.length && i < 16; i += 4) {
    chunks.push(body.slice(i, i + 4));
  }
  const formatted = prefix + (chunks.length > 0 ? "-" + chunks.join("-") : "");
  return formatted;
}

function formatGraceDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
}

const LOCKED_FEATURES = [
  "All Providers",
  "Cleanup Styles",
  "Command Mode",
  "Snippets",
  "Profiles",
  "Voice Notes",
  "Sync",
  "Offline Mode",
  "Analytics",
];

interface LicenseSectionProps {
  licenseStatus: ParsedLicenseStatus;
  onValidate: (key: string) => Promise<string | null>;
  onRemove: () => Promise<void>;
  licenseLoading: boolean;
}

function LicenseSection({ licenseStatus, onValidate, onRemove, licenseLoading }: LicenseSectionProps) {
  const [keyInput, setKeyInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const confirmTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleKeyChange = useCallback((raw: string) => {
    setKeyInput(formatLicenseKeyInput(raw));
    setError(null);
  }, []);

  const handleActivate = useCallback(async () => {
    const trimmed = keyInput.trim();
    if (!trimmed) return;
    setError(null);
    const err = await onValidate(trimmed);
    if (err) {
      setError(err);
    } else {
      setKeyInput("");
    }
  }, [keyInput, onValidate]);

  const handleRemoveClick = useCallback(() => {
    if (!confirmRemove) {
      setConfirmRemove(true);
      confirmTimerRef.current = setTimeout(() => setConfirmRemove(false), 4000);
      return;
    }
    if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current);
    setConfirmRemove(false);
    onRemove();
  }, [confirmRemove, onRemove]);

  // Cleanup timer on unmount.
  useEffect(() => {
    return () => {
      if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current);
    };
  }, []);

  const isLicensed = licenseStatus.type === "licensed";
  const isGrace = licenseStatus.type === "grace_period";
  const isUnlicensed = licenseStatus.type === "unlicensed";

  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
      {/* Status badge */}
      <div className="flex items-center gap-2">
        {isLicensed && (
          <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-green-500/20 text-green-400">
            <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M20 6L9 17l-5-5" />
            </svg>
            Licensed
          </span>
        )}
        {isGrace && (
          <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-yellow-500/20 text-yellow-400">
            Grace Period
          </span>
        )}
        {isUnlicensed && (
          <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-zinc-700 text-zinc-400">
            Free Tier
          </span>
        )}
      </div>

      {/* Licensed state */}
      {isLicensed && (
        <>
          <p className={isMobile ? "text-sm text-zinc-300" : "text-xs text-zinc-300"}>All features unlocked.</p>
          <button
            onClick={handleRemoveClick}
            disabled={licenseLoading}
            className={[
              "self-start transition-colors disabled:opacity-40",
              isMobile ? "text-sm" : "text-[11px]",
              confirmRemove ? "text-red-400 hover:text-red-300" : "text-zinc-500 hover:text-zinc-300",
            ].join(" ")}
          >
            {confirmRemove ? "Click again to confirm removal" : "Remove License"}
          </button>
        </>
      )}

      {/* Grace period state */}
      {isGrace && (
        <>
          {licenseStatus.graceUntil && (
            <p className={isMobile ? "text-sm text-yellow-400/80" : "text-xs text-yellow-400/80"}>
              License expires on {formatGraceDate(licenseStatus.graceUntil)}
            </p>
          )}
          <p className={isMobile ? "text-sm text-zinc-400" : "text-[11px] text-zinc-400"}>
            Re-validate your license to continue using all features.
          </p>
          <LicenseKeyInput
            value={keyInput}
            onChange={handleKeyChange}
            onActivate={handleActivate}
            loading={licenseLoading}
            error={error}
          />
        </>
      )}

      {/* Unlicensed state */}
      {isUnlicensed && (
        <>
          <LicenseKeyInput
            value={keyInput}
            onChange={handleKeyChange}
            onActivate={handleActivate}
            loading={licenseLoading}
            error={error}
          />
          <div className="flex flex-wrap gap-1.5 mt-0.5">
            {LOCKED_FEATURES.map((f) => (
              <span key={f} className="rounded-full px-2 py-0.5 text-[11px] font-medium bg-zinc-700 text-zinc-400">
                {f}
              </span>
            ))}
          </div>
          <button
            onClick={() => openUrl("https://dikta.app")}
            className={[
              "self-start transition-colors",
              isMobile ? "text-sm" : "text-[11px]",
              "text-zinc-400 hover:text-zinc-200 underline underline-offset-2",
            ].join(" ")}
          >
            Get a license at dikta.app
          </button>
        </>
      )}
    </div>
  );
}

function LicenseKeyInput({
  value, onChange, onActivate, loading, error,
}: {
  value: string;
  onChange: (v: string) => void;
  onActivate: () => void;
  loading: boolean;
  error: string | null;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex gap-2">
        <input
          type="text"
          spellCheck={false}
          autoComplete="off"
          placeholder="DIKTA-XXXX-XXXX-XXXX-XXXX"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !loading && onActivate()}
          maxLength={24} // DIKTA(5) + 4 dashes + 16 chars
          className={[
            "flex-1 font-mono tracking-widest",
            isMobile ? INPUT_CLS_M : INPUT_CLS,
          ].join(" ")}
        />
        <button
          onClick={onActivate}
          disabled={loading || !value.trim()}
          className={[
            "rounded-lg font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400",
            "hover:bg-emerald-500/15 disabled:opacity-40 disabled:cursor-not-allowed transition-colors",
            isMobile ? "px-4 py-2.5 text-sm" : "px-3 py-2 text-xs",
          ].join(" ")}
        >
          {loading ? "..." : "Activate"}
        </button>
      </div>
      {error && (
        <p className={["text-red-400", isMobile ? "text-sm" : "text-xs"].join(" ")}>
          {error}
        </p>
      )}
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
  licenseStatus: ParsedLicenseStatus;
  licenseLoading: boolean;
  onValidateLicense: (key: string) => Promise<string | null>;
  onRemoveLicense: () => Promise<void>;
  onSave: (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null,
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string, sttPriority: string[], llmPriority: string[],
    outputLanguage: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
    bubbleSize?: number | null, bubbleOpacity?: number | null,
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
  licenseStatus, licenseLoading, onValidateLicense, onRemoveLicense,
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
  const [localBubbleSize, setLocalBubbleSize] = useState(loadedSettings?.bubbleSize ?? 1.0);
  const [localBubbleOpacity, setLocalBubbleOpacity] = useState(loadedSettings?.bubbleOpacity ?? 0.85);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [newTerm, setNewTerm] = useState("");
  const [appVersion, setAppVersion] = useState<string>("");
  // Accordion: only one section open at a time. First section open by default.
  const [openSections, setOpenSections] = useState<Record<string, boolean>>({
    voiceRecording: true,
  });

  const toggleSection = useCallback((key: string) => {
    setOpenSections((prev) => {
      const wasOpen = prev[key];
      return wasOpen ? {} : { [key]: true };
    });
  }, []);

  const sectionBtnCls = "flex items-center gap-2 w-full py-2 text-left";

  // Load profiles on mount.
  useEffect(() => { getProfiles().then(setProfiles).catch(console.error); }, []);

  // Load app version on mount.
  useEffect(() => { getVersion().then(setAppVersion).catch(() => setAppVersion("0.4.1")); }, []);

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
      setLocalBubbleSize(loadedSettings.bubbleSize ?? 1.0);
      setLocalBubbleOpacity(loadedSettings.bubbleOpacity ?? 0.85);
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
        localBubbleSize, localBubbleOpacity,
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

  // Feature gate: user has an active paid license (licensed or valid grace period).
  const isPaid =
    licenseStatus.type === "licensed" ||
    (licenseStatus.type === "grace_period" &&
      licenseStatus.graceUntil !== undefined &&
      licenseStatus.graceUntil > Date.now() / 1000);

  // On Android the system nav bar (~48 px) overlaps the WebView bottom edge.
  // env(safe-area-inset-bottom) is unreliable in Android WebView so we use a
  // fixed 48 px deduction on mobile to keep the sticky Save footer visible.
  const panelMaxH = isMobile ? "max-h-[calc(100vh-168px)]" : "max-h-[calc(100vh-120px)]";

  return (
    <div className={`w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col ${panelMaxH}`}>
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
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("voiceRecording")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.voiceRecording ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Voice & Recording</span>
          </button>
          {openSections.voiceRecording && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
                  {STYLE_OPTIONS.map((opt) => {
                    const locked = !isPaid && opt.value !== "polished";
                    return (
                      <button
                        key={opt.value}
                        onClick={() => { if (!locked) handleStyleChange(opt.value); }}
                        title={locked ? "Requires Dikta License" : opt.description}
                        disabled={locked}
                        className={[
                          isMobile ? "flex-1 px-3 py-2 rounded-md text-sm font-medium transition-all duration-100" : "px-2 py-1 rounded-md text-xs font-medium transition-all duration-100",
                          locked ? "opacity-50 cursor-not-allowed" : "",
                          localStyle === opt.value
                            ? "bg-emerald-500/15 text-emerald-400"
                            : "text-zinc-500 hover:text-zinc-300",
                        ].join(" ")}
                      >
                        <span className="flex items-center gap-1 justify-center">
                          {opt.label}
                          {locked && <LockIcon className="w-2.5 h-2.5 text-zinc-600" />}
                        </span>
                      </button>
                    );
                  })}
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
          )}
        </div>

        {/* --- Hotkey -- desktop only (no global hotkeys on Android) --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("hotkey")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.hotkey ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Hotkey</span>
            </button>
            {openSections.hotkey && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
          </div>
        )}

        {/* --- Cleanup Instructions --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("customPrompt")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.customPrompt ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
              Cleanup Instructions
              {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
            </span>
          </button>
          {openSections.customPrompt && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              <MobileTextarea
                label="Cleanup Instructions"
                hint="Appended to the system prompt during LLM cleanup."
                value={localCustomPrompt}
                onChange={isPaid ? setLocalCustomPrompt : () => {}}
                placeholder={isPaid ? "Extra instructions for the LLM, e.g. 'Always use formal German' or 'Keep technical terms in English'" : "Requires Dikta License"}
                rows={3}
                className={`${INPUT_CLS_M} resize-none${!isPaid ? " opacity-50 cursor-not-allowed" : ""}`}
                disabled={!isPaid}
              />
              {/* Preset buttons -- one click replaces the entire custom prompt */}
              <div className="flex items-center gap-2 flex-wrap">
                <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Presets:</span>
                {([
                  { label: "Formal", prompt: "Always use formal language. Avoid colloquialisms and slang." },
                  { label: "Technical", prompt: "Keep technical terms in English. Use precise, professional language." },
                  { label: "Casual", prompt: "Keep it casual and conversational. Use natural, relaxed language." },
                ] as const).map(({ label, prompt }) => (
                  <button
                    key={label}
                    type="button"
                    onClick={() => setLocalCustomPrompt(prompt)}
                    className={[
                      "border rounded-lg font-medium transition-colors",
                      "bg-transparent border-zinc-700/60 text-zinc-400",
                      "hover:border-zinc-500 hover:text-zinc-200",
                      isMobile ? "px-4 min-h-[44px] text-sm" : "px-3 py-1.5 text-xs",
                    ].join(" ")}
                  >
                    {label}
                  </button>
                ))}
                <button
                  type="button"
                  onClick={() => setLocalCustomPrompt("")}
                  className={[
                    "transition-colors",
                    "text-zinc-600 hover:text-zinc-400",
                    isMobile ? "px-3 min-h-[44px] text-sm" : "px-2 py-1.5 text-xs",
                  ].join(" ")}
                >
                  Clear
                </button>
              </div>
              <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Appended to the system prompt during LLM cleanup.</p>
            </div>
          )}
        </div>

        {/* --- General -- desktop only features --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("general")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.general ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">General</span>
            </button>
            {openSections.general && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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

                <label className={`flex items-center justify-between gap-3 ${isPaid ? "cursor-pointer" : "cursor-not-allowed"}`}>
                  <div className={`flex flex-col gap-0.5 ${!isPaid ? "opacity-50" : ""}`}>
                    <span className="flex items-center gap-1.5">
                      <span className={LABEL_CLS_M}>Whisper mode</span>
                      {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
                    </span>
                    <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Amplifies mic input for quiet dictation</span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={localWhisperMode}
                    disabled={!isPaid}
                    onClick={() => { if (isPaid) setLocalWhisperMode(!localWhisperMode); }}
                    className={[
                      "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
                      !isPaid ? "opacity-50 cursor-not-allowed" : "",
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Select text, hold Ctrl+Shift+E, speak your edit. The selected text will be rewritten.</span>
                </div>
              </div>
            )}
          </div>
        )}

        {/* --- Webhook -- desktop only --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("webhook")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.webhook ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Webhook</span>
            </button>
            {openSections.webhook && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
                <div className={!isPaid ? "opacity-50" : ""}>
                  <input
                    type="url"
                    placeholder={isPaid ? "https://example.com/webhook" : "Requires Dikta License"}
                    value={localWebhookUrl}
                    disabled={!isPaid}
                    onChange={(e) => setLocalWebhookUrl(e.target.value)}
                    className={`${INPUT_CLS_M}${!isPaid ? " cursor-not-allowed" : ""}`}
                  />
                </div>
                <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>HTTP POST after each dictation. Leave empty to disable.</p>
              </div>
            )}
          </div>
        )}

        {/* --- Sync --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("sync")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.sync ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Cross-Device Sync</span>
          </button>
          {openSections.sync && (
            <div className={`flex flex-col gap-3 pl-4 pb-3 pt-1${!isPaid ? " opacity-50" : ""}`}>
              <div className="flex flex-col gap-1.5">
                <span className="flex items-center gap-1.5">
                  <span className={LABEL_CLS_M}>Turso URL</span>
                  {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
                </span>
                <input
                  type="text"
                  placeholder={isPaid ? "libsql://your-db.turso.io" : "Requires Dikta License"}
                  value={localTursoUrl}
                  disabled={!isPaid}
                  onChange={(e) => setLocalTursoUrl(e.target.value)}
                  className={`${INPUT_CLS_M}${!isPaid ? " cursor-not-allowed" : ""}`}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS_M}>Turso Token</span>
                <input
                  type="password"
                  autoComplete="off"
                  placeholder={isPaid ? (loadedSettings?.tursoTokenMasked || "Auth token") : "Requires Dikta License"}
                  value={tursoToken}
                  disabled={!isPaid}
                  onChange={(e) => setTursoToken(e.target.value)}
                  className={`${INPUT_CLS_M}${!isPaid ? " cursor-not-allowed" : ""}`}
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
                disabled={syncing || !localTursoUrl || !isPaid}
                className={`px-3 py-1.5 text-sm bg-zinc-700 text-white rounded hover:bg-zinc-600 disabled:opacity-40 transition-colors ${isMobile ? "py-2.5 text-base" : ""}${!isPaid ? " cursor-not-allowed" : ""}`}
              >
                {syncing ? "Syncing..." : "Sync Now"}
              </button>
              {syncMsg && <p className={isMobile ? "text-xs text-zinc-400" : "text-[11px] text-zinc-400"}>{syncMsg}</p>}
              <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>Sync dictation history across devices via Turso. Leave empty to disable.</p>
            </div>
          )}
        </div>

        {/* --- API Keys --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("apiKeys")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.apiKeys ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">API Keys</span>
          </button>
          {openSections.apiKeys && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>(LLM only)</span>
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
          )}
        </div>

        {/* --- Provider Priority --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("providerPriority")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.providerPriority ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Provider Priority</span>
          </button>
          {openSections.providerPriority && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              <p className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>
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
          )}
        </div>

        {/* --- Dictionary --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("dictionary")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.dictionary ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Dictionary</span>
          </button>
          {openSections.dictionary && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
          )}
        </div>

        {/* --- Updates -- desktop only (Tauri updater not available on sideloaded APKs) --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("updates")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.updates ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Updates</span>
            </button>
            {openSections.updates && (
              <div className="pl-4 pb-3 pt-1">
                <UpdateChecker />
              </div>
            )}
          </div>
        )}

        {/* --- App Profiles --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("appProfiles")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.appProfiles ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">App Profiles</span>
          </button>
          {openSections.appProfiles && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
          )}
        </div>

        {/* --- License --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("license")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.license ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">License</span>
          </button>
          {openSections.license && (
            <LicenseSection
              licenseStatus={licenseStatus}
              onValidate={onValidateLicense}
              onRemove={onRemoveLicense}
              licenseLoading={licenseLoading}
            />
          )}
        </div>

        {/* --- About --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("about")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.about ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">About</span>
          </button>
          {openSections.about && (
            <div className="flex flex-col gap-2 pl-4 pb-3 pt-1">
              <p className="text-xs font-medium text-zinc-300">
                Dikta{appVersion ? ` v${appVersion}` : ""}
              </p>
              <p className="text-[11px] text-zinc-500">Voice dictation you own.</p>
              <p className="text-[11px] text-zinc-500">by Andreas Nolte</p>
              <div className="flex items-center gap-2 mt-0.5">
                <button
                  onClick={() => openUrl("https://github.com/andyon2/dikta")}
                  className="text-[11px] text-zinc-400 hover:text-zinc-200 underline underline-offset-2 transition-colors"
                >
                  GitHub
                </button>
                <span className="text-[11px] text-zinc-600">·</span>
                <span className="text-[11px] text-zinc-500">MIT License</span>
              </div>
            </div>
          )}
        </div>

      </div>

      {/* Save button -- sticky footer, always visible.
          On Android the nav bar (Back/Home/Recent) overlaps the WebView bottom.
          mobile-safe-bottom adds a fixed 56 px padding (env() is unreliable in
          Android WebView and returns 0). The parent panel max-h also accounts for
          the ~48 px nav bar so this footer is never clipped by the container. */}
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
