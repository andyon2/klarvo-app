import { useState, useEffect, useCallback, useRef } from "react";
import { isPreviewMode } from "../tauri-commands";

// Plugin imports are loaded dynamically so that this module can be evaluated
// in a plain browser (preview mode) without crashing on missing Tauri globals.

/** Returns the app version string. Falls back to a hardcoded string in preview mode. */
async function getAppVersion(): Promise<string> {
  if (isPreviewMode) return "0.4.1-preview";
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return getVersion();
  } catch {
    return "0.4.1";
  }
}

/** Opens a URL in the system browser. Falls back to window.open in preview mode. */
async function openUrl(url: string): Promise<void> {
  try {
    const { openUrl: tauriOpenUrl } = await import("@tauri-apps/plugin-opener");
    await tauriOpenUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

/** Checks for app updates. Returns null in preview mode. */
async function checkForUpdate(): Promise<{ version: string; downloadAndInstall: () => Promise<void> } | null> {
  if (isPreviewMode) return null;
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    return check();
  } catch {
    return null;
  }
}
import type { AppSettings, CleanupStyle, HotkeyMode, AppProfile, ParsedLicenseStatus } from "../types";
import { STYLE_OPTIONS } from "../types";
import { getProfiles, saveProfiles, syncHistory } from "../tauri-commands";
import { isDesktop, isMobile } from "../platform";
import { CloseIcon, LockIcon } from "./icons";
import { StatusDot, DictionaryTag, INPUT_CLS, LABEL_CLS, SECTION_TITLE_CLS, INPUT_CLS_M, LABEL_CLS_M } from "./ui";
import { MobileTextarea } from "./MobileTextarea";
import { WhisperModelManager } from "./WhisperModelManager";

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

// --- Cloud STT models ---------------------------------------------------------

const CLOUD_STT_MODELS = [
  { value: "whisper-large-v3-turbo", label: "Groq — Large V3 Turbo", price: "~$0.0007/min", provider: "groq" },
  { value: "whisper-large-v3", label: "Groq — Large V3", price: "~$0.002/min", provider: "groq" },
  { value: "whisper-1", label: "OpenAI — Whisper 1", price: "~$0.006/min", provider: "openai" },
];

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
      const update = await checkForUpdate();
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
      const update = await checkForUpdate();
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
  "Offline HD Models",
  "Command Mode",
  "Snippets",
  "Unlimited Dictionary",
  "Voice Notes",
  "Live Transcription",
  "Cleanup Instructions",
  "Cross-Device Sync",
  "Advanced Statistics",
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
  const isTrial = licenseStatus.type === "trial";
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
        {isTrial && (
          <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-blue-500/15 text-blue-400">
            {isPreviewMode
              ? "Trial — Preview Mode"
              : `Trial${licenseStatus.trialUntil ? ` — expires ${formatGraceDate(licenseStatus.trialUntil)}` : ""}`}
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

      {/* Trial state */}
      {isTrial && (
        <>
          <p className={isMobile ? "text-sm text-zinc-300" : "text-xs text-zinc-300"}>All features unlocked during trial.</p>
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
          maxLength={25} // DIKTA(5) + 4 dashes + 16 chars = 25
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
    openaiKey: string, anthropicKey: string,
    outputLanguage: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
    bubbleSize?: number | null, bubbleOpacity?: number | null,
    localWhisperModel?: string | null, localWhisperGpu?: boolean | null,
    sttProvider?: string | null, llmProvider?: string | null,
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
  const [localSttProvider, setLocalSttProvider] = useState<string>(loadedSettings?.sttProvider ?? "groq");
  const [localLlmProvider, setLocalLlmProvider] = useState<string>(loadedSettings?.llmProvider ?? "deepseek");
  const [localOutputLanguage, setLocalOutputLanguage] = useState(outputLanguage);
  useEffect(() => { setLocalOutputLanguage(outputLanguage); }, [outputLanguage]);
  const [localWebhookUrl, setLocalWebhookUrl] = useState(loadedSettings?.webhookUrl ?? "");
  const [localTursoUrl, setLocalTursoUrl] = useState(loadedSettings?.tursoUrl ?? "");
  const [tursoToken, setTursoToken] = useState("");
  const [localBubbleSize, setLocalBubbleSize] = useState(loadedSettings?.bubbleSize ?? 1.0);
  const [localBubbleOpacity, setLocalBubbleOpacity] = useState(loadedSettings?.bubbleOpacity ?? 0.85);
  const [localWhisperModel, setLocalWhisperModel] = useState(loadedSettings?.localWhisperModel ?? "small");
  const [localWhisperGpu, setLocalWhisperGpu] = useState(loadedSettings?.localWhisperGpu ?? true);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [newTerm, setNewTerm] = useState("");
  const [appVersion, setAppVersion] = useState<string>("");
  // isDirty: true when any local state differs from the persisted loadedSettings.
  // License activation must NOT set this flag (it auto-saves immediately).
  const [isDirty, setIsDirty] = useState(false);
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
  useEffect(() => { getAppVersion().then(setAppVersion).catch(() => setAppVersion("0.4.1")); }, []);

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
      setLocalSttProvider(loadedSettings.sttProvider ?? "groq");
      setLocalLlmProvider(loadedSettings.llmProvider ?? "deepseek");
      setLocalOutputLanguage(loadedSettings.outputLanguage ?? "");
      setLocalWebhookUrl(loadedSettings.webhookUrl ?? "");
      setLocalTursoUrl(loadedSettings.tursoUrl ?? "");
      setLocalBubbleSize(loadedSettings.bubbleSize ?? 1.0);
      setLocalBubbleOpacity(loadedSettings.bubbleOpacity ?? 0.85);
      setLocalWhisperModel(loadedSettings.localWhisperModel ?? "small");
      setLocalWhisperGpu(loadedSettings.localWhisperGpu ?? true);
    }
  }, [loadedSettings]);

  // Track dirty state: compare local values against the last saved settings.
  // API key fields: any non-empty input counts as dirty (new key to save).
  // License activation is excluded -- it triggers auto-save and must not set dirty.
  useEffect(() => {
    if (!loadedSettings) return;
    const dirty =
      localLang !== (loadedSettings.language ?? "") ||
      localStyle !== (loadedSettings.cleanupStyle ?? "polished") ||
      localHotkey !== (loadedSettings.hotkey ?? "") ||
      localHotkeyMode !== (loadedSettings.hotkeyMode ?? "hold") ||
      localAudioDevice !== (loadedSettings.audioDevice ?? null) ||
      localSttModel !== (loadedSettings.sttModel ?? "whisper-large-v3-turbo") ||
      localCustomPrompt !== (loadedSettings.customPrompt ?? "") ||
      localAutostart !== (loadedSettings.autostart ?? false) ||
      localWhisperMode !== (loadedSettings.whisperMode ?? false) ||
      localSttProvider !== (loadedSettings.sttProvider ?? "groq") ||
      localLlmProvider !== (loadedSettings.llmProvider ?? "deepseek") ||
      localOutputLanguage !== (loadedSettings.outputLanguage ?? "") ||
      localWebhookUrl !== (loadedSettings.webhookUrl ?? "") ||
      localTursoUrl !== (loadedSettings.tursoUrl ?? "") ||
      localBubbleSize !== (loadedSettings.bubbleSize ?? 1.0) ||
      localBubbleOpacity !== (loadedSettings.bubbleOpacity ?? 0.85) ||
      localWhisperModel !== (loadedSettings.localWhisperModel ?? "small") ||
      localWhisperGpu !== (loadedSettings.localWhisperGpu ?? true) ||
      groqKey.trim() !== "" ||
      deepseekKey.trim() !== "" ||
      openaiKey.trim() !== "" ||
      anthropicKey.trim() !== "" ||
      tursoToken.trim() !== "";
    setIsDirty(dirty);
  }, [
    loadedSettings, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, localSttProvider,
    localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl, localBubbleSize,
    localBubbleOpacity, localWhisperModel, localWhisperGpu,
    groqKey, deepseekKey, openaiKey, anthropicKey, tursoToken,
  ]);

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

  // Internal helper: calls onSave with all current values. Used by both the
  // explicit Save button and the auto-save after license activation.
  const saveCurrentSettings = useCallback(async (opts?: { silent?: boolean }) => {
    setSaving(true);
    if (!opts?.silent) setSaveMsg(null);
    try {
      await onSave(
        groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        openaiKey.trim(), anthropicKey.trim(),
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), tursoToken.trim(),
        localBubbleSize, localBubbleOpacity,
        localWhisperModel, localWhisperGpu,
        localSttProvider, localLlmProvider,
      );
      setGroqKey("");
      setDeepseekKey("");
      setOpenaiKey("");
      setAnthropicKey("");
      setTursoToken("");
      if (!opts?.silent) {
        setSaveMsg("Saved");
        setTimeout(() => setSaveMsg(null), 2000);
      }
    } catch (err) {
      if (!opts?.silent) setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [
    groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey, anthropicKey,
    localSttProvider, localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl, tursoToken,
    localBubbleSize, localBubbleOpacity, localWhisperModel, localWhisperGpu, onSave,
  ]);

  const handleSave = useCallback(async () => {
    await saveCurrentSettings();
  }, [saveCurrentSettings]);

  // Called from LicenseSection after successful activation: persist immediately
  // so the user never has to click Save Settings for license changes.
  // We do NOT want to mark this as a dirty operation -- use silent mode and
  // pass empty strings for API keys (backend ignores them when empty).
  const handleLicenseAutoSave = useCallback(async () => {
    setSaving(true);
    try {
      await onSave(
        "", "", localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        "", "",
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), "",
        localBubbleSize, localBubbleOpacity,
        localWhisperModel, localWhisperGpu,
        localSttProvider, localLlmProvider,
      );
    } catch (err) {
      console.error("License auto-save failed:", err);
    } finally {
      setSaving(false);
    }
  }, [
    localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
    localSttProvider, localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl,
    localBubbleSize, localBubbleOpacity, localWhisperModel, localWhisperGpu, onSave,
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

  // Feature gate: user has an active paid license (licensed, active trial, or valid grace period).
  const isPaid =
    licenseStatus.type === "licensed" ||
    (licenseStatus.type === "trial" &&
      licenseStatus.trialUntil !== undefined &&
      licenseStatus.trialUntil > Date.now() / 1000) ||
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

              {/* Cloud / Offline toggle -- same visual style as StylePicker */}
              <div className="flex flex-col gap-2">
                <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wide">Speech Recognition</span>
                <div className="flex flex-col gap-2 pl-0">
                <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60 w-fit">
                  <button
                    type="button"
                    onClick={() => {
                      if (localSttProvider === "local") {
                        setLocalSttProvider("groq");
                      }
                    }}
                    className={[
                      "px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-100",
                      localSttProvider !== "local"
                        ? "bg-emerald-500/15 text-emerald-400"
                        : "text-zinc-500 hover:text-zinc-300",
                    ].join(" ")}
                  >
                    Cloud
                  </button>
                  <button
                    type="button"
                    onClick={() => setLocalSttProvider("local")}
                    className={[
                      "px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-100",
                      localSttProvider === "local"
                        ? "bg-emerald-500/15 text-emerald-400"
                        : "text-zinc-500 hover:text-zinc-300",
                    ].join(" ")}
                  >
                    Offline
                  </button>
                </div>

                {/* Cloud mode: model picker */}
                {localSttProvider !== "local" && (
                  <div className="flex flex-col gap-2 mt-1">
                    <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
                      <span className={LABEL_CLS_M}>Model</span>
                      <select
                        value={localSttModel}
                        onChange={(e) => {
                          const model = e.target.value;
                          setLocalSttModel(model);
                          // Sync provider to match the selected model's API.
                          if (model === "whisper-1") {
                            setLocalSttProvider("openai");
                          } else {
                            setLocalSttProvider("groq");
                          }
                        }}
                        className={`bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
                      >
                        {CLOUD_STT_MODELS.filter((m) => {
                          if (m.provider === "groq") return groqOk;
                          if (m.provider === "openai") return openaiOk;
                          return true;
                        }).map((m) => (
                          <option key={m.value} value={m.value}>
                            {m.label} ({m.price})
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                )}

                {/* Offline mode: WhisperModelManager */}
                {localSttProvider === "local" && isDesktop && (
                  <div className="flex flex-col gap-3 mt-1">
                    <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-zinc-800/30 border border-zinc-700/30">
                      <svg className="w-3.5 h-3.5 text-zinc-400 mt-0.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" />
                      </svg>
                      <p className="text-[11px] text-zinc-400 leading-relaxed">
                        Speech is transcribed locally. Text cleanup is skipped (no internet needed).
                      </p>
                    </div>
                    <WhisperModelManager
                      selectedModel={localWhisperModel}
                      gpuEnabled={localWhisperGpu}
                      onModelChange={setLocalWhisperModel}
                      onGpuChange={setLocalWhisperGpu}
                      isPaid={isPaid}
                    />
                  </div>
                )}
                </div>
              </div>

              {/* Text Cleanup -- only in Cloud mode */}
              {localSttProvider !== "local" && (
                <div className="flex flex-col gap-2.5">
                  <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wide">Text Cleanup</span>

                  <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
                    <span className={LABEL_CLS_M}>Provider</span>
                    <select
                      value={localLlmProvider}
                      onChange={(e) => setLocalLlmProvider(e.target.value)}
                      className={`bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
                    >
                      <option value="deepseek" disabled={!deepseekOk}>DeepSeek{!deepseekOk ? " (no key)" : ""}</option>
                      <option value="openai" disabled={!openaiOk}>OpenAI{!openaiOk ? " (no key)" : ""}</option>
                      <option value="anthropic" disabled={!anthropicOk}>Anthropic{!anthropicOk ? " (no key)" : ""}</option>
                      <option value="groq" disabled={!groqOk}>Groq (Llama){!groqOk ? " (no key)" : ""}</option>
                    </select>
                  </div>

                  <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
                    <span className={LABEL_CLS_M}>Style</span>
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

        {/* --- Cleanup Instructions -- hidden when offline STT mode is active --- */}
        {localSttProvider !== "local" && <div className="flex flex-col gap-1">
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
        </div>}

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


        {/* --- Sync --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("sync")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.sync ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
              Cross-Device Sync
              {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
            </span>
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>(Speech + Cleanup)</span>
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>(Cleanup)</span>
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>(Speech + Cleanup)</span>
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
                  <span className={isMobile ? "text-xs text-zinc-500" : "text-[11px] text-zinc-500"}>(Cleanup)</span>
                  <StatusDot active={anthropicOk} />
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

        {/* --- Dictionary --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("dictionary")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.dictionary ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
              Dictionary
              <span className={`text-[10px] font-normal normal-case tracking-normal ${!isPaid && dictionary.length >= 20 ? "text-amber-500/80" : "text-zinc-600"}`}>
                {!isPaid ? `${dictionary.length}/20` : `${dictionary.length}`}
              </span>
            </span>
          </button>
          {openSections.dictionary && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder="Add word or phrase..."
                  value={newTerm}
                  disabled={!isPaid && dictionary.length >= 20}
                  onChange={(e) => setNewTerm(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleAddTerm()}
                  className={`flex-1 ${INPUT_CLS_M}${(!isPaid && dictionary.length >= 20) ? " cursor-not-allowed opacity-50" : ""}`}
                />
                <button
                  onClick={handleAddTerm}
                  disabled={!newTerm.trim() || (!isPaid && dictionary.length >= 20)}
                  title={(!isPaid && dictionary.length >= 20) ? "Free limit reached (20 terms). Upgrade for unlimited." : undefined}
                  className={`px-3 rounded-lg font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 disabled:opacity-30 disabled:cursor-not-allowed transition-colors ${isMobile ? "py-2.5 text-sm min-w-[56px]" : "py-2 text-xs"}`}
                >
                  Add
                </button>
              </div>

              {!isPaid && dictionary.length >= 20 && (
                <p className="text-[11px] text-amber-500/80">
                  Free limit reached (20 terms). Upgrade for unlimited.
                </p>
              )}

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

        {/* --- App Profiles (paid feature) --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("appProfiles")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.appProfiles ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
              App Profiles
              {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
            </span>
          </button>
          {openSections.appProfiles && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              {!isPaid ? (
                // Free-tier paygate: show lock message, no profile editing allowed.
                <div className="flex flex-col gap-2">
                  <div className="flex items-center gap-2 text-zinc-500">
                    <LockIcon className="w-3.5 h-3.5 text-zinc-600 flex-shrink-0" />
                    <p className="text-xs">App Profiles require a Dikta license.</p>
                  </div>
                  <p className="text-[11px] text-zinc-600">Override style and language per app based on window title.</p>
                </div>
              ) : (
                <>
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
                </>
              )}
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
              onValidate={async (key) => {
                const err = await onValidateLicense(key);
                if (!err) {
                  // Persist immediately so the license is not lost on app restart.
                  // This must not affect isDirty -- handleLicenseAutoSave uses
                  // empty key strings (backend keeps existing keys unchanged).
                  await handleLicenseAutoSave();
                }
                return err;
              }}
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

      {/* Save button -- sticky footer, visible only when there are unsaved changes.
          On Android the nav bar (Back/Home/Recent) overlaps the WebView bottom.
          mobile-safe-bottom adds a fixed 56 px padding (env() is unreliable in
          Android WebView and returns 0). The parent panel max-h also accounts for
          the ~48 px nav bar so this footer is never clipped by the container. */}
      {(isDirty || saveMsg) && (
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
                : "bg-emerald-500/10 border-emerald-500/30 text-emerald-400 hover:bg-emerald-500/15 hover:border-emerald-500/40 animate-pulse",
              "disabled:opacity-50 disabled:cursor-not-allowed",
            ].join(" ")}
          >
            {saving ? "Saving..." : saveMsg ?? "Save Settings"}
          </button>
        </div>
      )}
    </div>
  );
}
