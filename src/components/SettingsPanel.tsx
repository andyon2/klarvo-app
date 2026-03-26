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
import { getProfiles, saveProfiles, syncHistory, getAdvancedSettings, saveAdvancedSettings, toggleVoiceCommandMode, getVoiceCommandActive, validateApiKey, clearApiKey } from "../tauri-commands";
import type { AdvancedSettings } from "../types";
import { isDesktop, isMobile } from "../platform";
import { CloseIcon, LockIcon } from "./icons";
import { StatusDot, DictionaryTag, INPUT_CLS, LABEL_CLS, SECTION_TITLE_CLS, INPUT_CLS_M, LABEL_CLS_M } from "./ui";
import { MobileTextarea } from "./MobileTextarea";
import { WhisperModelManager } from "./WhisperModelManager";

// --- Shortcut Recorder -------------------------------------------------------

function ShortcutRecorder({ value, onChange }: { value: string; onChange: (s: string) => void }) {
  const [listening, setListening] = useState(false);
  // Track which modifier keys are currently held. This is needed for the
  // keyup-based Alt fallback: when Alt is the last key released, e.altKey is
  // already false in the keyup event, so we cannot derive the modifier state
  // from the event alone.
  const heldModifiers = useRef<Set<string>>(new Set());
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cancel = useCallback(() => {
    heldModifiers.current.clear();
    setListening(false);
  }, []);

  // Pause/resume the global hotkey while the recorder is listening,
  // so pressing the current shortcut doesn't trigger the pipeline.
  useEffect(() => {
    if (listening) {
      import("@tauri-apps/api/core")
        .then(({ invoke: inv }) => inv("set_hotkey_paused", { paused: true }))
        .catch(console.error);
    }
    return () => {
      if (listening) {
        import("@tauri-apps/api/core")
          .then(({ invoke: inv }) => inv("set_hotkey_paused", { paused: false }))
          .catch(console.error);
      }
    };
  }, [listening]);

  // Auto-cancel after 5 seconds so the button never stays stuck.
  useEffect(() => {
    if (!listening) return;
    timeoutRef.current = setTimeout(cancel, 5000);
    return () => {
      if (timeoutRef.current !== null) clearTimeout(timeoutRef.current);
    };
  }, [listening, cancel]);

  useEffect(() => {
    if (!listening) return;

    const KEY_MAP: Record<string, string> = {
      " ": "space", Enter: "enter", Tab: "tab",
      Backspace: "backspace", Delete: "delete", Insert: "insert",
      Home: "home", End: "end", PageUp: "pageup", PageDown: "pagedown",
      ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
    };
    const MODIFIERS = new Set(["Control", "Shift", "Alt", "Meta"]);

    const buildParts = (ctrl: boolean, shift: boolean, alt: boolean, meta: boolean, rawKey: string): string | null => {
      const parts: string[] = [];
      if (ctrl) parts.push("ctrl");
      if (shift) parts.push("shift");
      if (alt) parts.push("alt");
      if (meta) parts.push("super");
      if (parts.length === 0) return null;
      let key = KEY_MAP[rawKey] ?? rawKey.toLowerCase();
      if (/^F\d+$/.test(rawKey)) key = rawKey.toLowerCase();
      parts.push(key);
      return parts.join("+");
    };

    const keydownHandler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Escape cancels listening regardless of modifiers.
      if (e.key === "Escape") {
        cancel();
        return;
      }

      // Track held modifiers for the keyup fallback.
      if (MODIFIERS.has(e.key)) {
        heldModifiers.current.add(e.key);
        return;
      }

      // Normal path: at least one modifier must be held.
      const combo = buildParts(e.ctrlKey, e.shiftKey, e.altKey, e.metaKey, e.key);
      if (combo === null) return;

      onChange(combo);
      cancel();
    };

    // keyup fallback for Alt-based shortcuts on Windows. WebView2 sometimes
    // swallows keydown events that include Alt before JS sees them (the browser
    // treats Alt as "focus the menu bar"). The keyup event is more reliably
    // delivered. We only use this path when the keydown handler did NOT already
    // commit a combo (i.e. listening is still true when keyup fires).
    //
    // Caveat: on keyup the modifier flags already reflect the *released* state,
    // so e.altKey is false when Alt itself is being released. We therefore fall
    // back to heldModifiers to reconstruct the held set at the moment the
    // non-modifier key was pressed.
    const keyupHandler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (MODIFIERS.has(e.key)) {
        heldModifiers.current.delete(e.key);
        return;
      }

      // Only engage if this is an Alt-combo that keydown might have missed.
      if (!heldModifiers.current.has("Alt")) return;

      const ctrl = heldModifiers.current.has("Control") || e.ctrlKey;
      const shift = heldModifiers.current.has("Shift") || e.shiftKey;
      const alt = true; // we know Alt is/was held
      const meta = heldModifiers.current.has("Meta") || e.metaKey;

      const combo = buildParts(ctrl, shift, alt, meta, e.key);
      if (combo === null) return;

      onChange(combo);
      cancel();
    };

    document.addEventListener("keydown", keydownHandler, true);
    document.addEventListener("keyup", keyupHandler, true);
    return () => {
      document.removeEventListener("keydown", keydownHandler, true);
      document.removeEventListener("keyup", keyupHandler, true);
      heldModifiers.current.clear();
    };
  }, [listening, onChange, cancel]);

  return (
    <button
      type="button"
      onClick={() => setListening(true)}
      onBlur={cancel}
      className={[
        "w-full bg-klarvo-bg border rounded-lg px-3 py-2 text-sm text-left font-mono",
        listening
          ? "border-klarvo-primary/50 text-klarvo-primary animate-pulse"
          : "border-klarvo-border/50 text-klarvo-text hover:border-klarvo-border-active",
        "focus:outline-none transition-all duration-150",
      ].join(" ")}
    >
      {listening ? "Press shortcut... (Esc to cancel)" : value || "Click to set"}
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
  const [appVersion, setAppVersion] = useState<string>("…");

  useEffect(() => {
    getAppVersion().then(setAppVersion);
  }, []);

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
      <span className="text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest">Updates</span>
      <div className="flex items-center gap-2">
        {status === "available" ? (
          <button
            onClick={handleInstall}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-primary/10 border border-klarvo-primary/20 text-klarvo-primary hover:bg-klarvo-primary/15 transition-colors"
          >
            Install v{updateVersion}
          </button>
        ) : (
          <button
            onClick={handleCheck}
            disabled={status === "checking" || status === "downloading"}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {status === "checking" ? "Checking..." : status === "downloading" ? "Downloading..." : status === "upToDate" ? "Up to date" : "Check for updates"}
          </button>
        )}
        <span className="text-[11px] text-klarvo-dim">v{appVersion}</span>
      </div>
      {errorMsg && <p className="text-[11px] text-klarvo-danger">{errorMsg}</p>}
    </div>
  );
}

// --- License Section ---------------------------------------------------------

// Auto-formats a license key input: uppercase, inserts dashes after every 4 chars
// in the payload section (after "KLARVO-").
function formatLicenseKeyInput(raw: string): string {
  // Strip everything that is not alphanumeric.
  const stripped = raw.replace(/[^a-zA-Z0-9]/g, "").toUpperCase();
  // The key format is KLARVO-XXXX-XXXX-XXXX-XXXX (or legacy VOXLIT-/DIKTA-XXXX-...).
  if (stripped.length === 0) return "";

  // Detect prefix: "KLARVO" (6 chars), legacy "VOXLIT" (6 chars) or legacy "DIKTA" (5 chars).
  let prefix = "";
  let prefixLen = 0;
  if (stripped.startsWith("KLARVO")) {
    prefix = "KLARVO";
    prefixLen = 6;
  } else if (stripped.startsWith("VOXLIT")) {
    prefix = "VOXLIT";
    prefixLen = 6;
  } else if (stripped.startsWith("DIKTA")) {
    prefix = "DIKTA";
    prefixLen = 5;
  } else {
    // Let the user type freely until a prefix is matched.
    if (stripped.length <= 6) return stripped;
  }

  const body = prefixLen > 0 ? stripped.slice(prefixLen) : stripped;
  const chunks: string[] = [];
  for (let i = 0; i < body.length && i < 16; i += 4) {
    chunks.push(body.slice(i, i + 4));
  }
  const formatted = (prefix || stripped.slice(0, 6)) + (chunks.length > 0 ? "-" + chunks.join("-") : "");
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
          <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-elevated text-klarvo-muted">
            Free Tier
          </span>
        )}
      </div>

      {/* Licensed state */}
      {isLicensed && (
        <>
          <p className={isMobile ? "text-sm text-klarvo-muted" : "text-xs text-klarvo-muted"}>All features unlocked.</p>
          <button
            onClick={handleRemoveClick}
            disabled={licenseLoading}
            className={[
              "self-start transition-colors disabled:opacity-40",
              isMobile ? "text-sm" : "text-[11px]",
              confirmRemove ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning",
            ].join(" ")}
          >
            {confirmRemove ? "Click again to confirm removal" : "Remove License"}
          </button>
        </>
      )}

      {/* Trial state */}
      {isTrial && (
        <>
          <p className={isMobile ? "text-sm text-klarvo-muted" : "text-xs text-klarvo-muted"}>All features unlocked during trial.</p>
          <button
            onClick={handleRemoveClick}
            disabled={licenseLoading}
            className={[
              "self-start transition-colors disabled:opacity-40",
              isMobile ? "text-sm" : "text-[11px]",
              confirmRemove ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning",
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
          <p className={isMobile ? "text-sm text-klarvo-muted" : "text-[11px] text-klarvo-muted"}>
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
              <span key={f} className="rounded-full px-2 py-0.5 text-[11px] font-medium bg-klarvo-elevated text-klarvo-muted">
                {f}
              </span>
            ))}
          </div>
          <button
            onClick={() => openUrl("https://klarvo.app")}
            className={[
              "self-start transition-colors",
              isMobile ? "text-sm" : "text-[11px]",
              "text-klarvo-muted hover:text-klarvo-text underline underline-offset-2",
            ].join(" ")}
          >
            Get a license at klarvo.app
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
          placeholder="KLARVO-XXXX-XXXX-XXXX-XXXX"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !loading && onActivate()}
          maxLength={26} // KLARVO(6) + 4 dashes + 16 chars = 26
          className={[
            "flex-1 font-mono tracking-widest",
            isMobile ? INPUT_CLS_M : INPUT_CLS,
          ].join(" ")}
        />
        <button
          onClick={onActivate}
          disabled={loading || !value.trim()}
          className={[
            "rounded-lg font-medium bg-klarvo-primary/10 border border-klarvo-primary/20 text-klarvo-primary",
            "hover:bg-klarvo-primary/15 disabled:opacity-40 disabled:cursor-not-allowed transition-colors",
            isMobile ? "px-4 py-2.5 text-sm" : "px-3 py-2 text-xs",
          ].join(" ")}
        >
          {loading ? "..." : "Activate"}
        </button>
      </div>
      {error && (
        <p className={["text-klarvo-danger", isMobile ? "text-sm" : "text-xs"].join(" ")}>
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
  hotkeySlot2: string;
  hotkeyModeSlot2: HotkeyMode;
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
    openaiKey: string, anthropicKey: string, openrouterKey: string,
    outputLanguage: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
    bubbleSize?: number | null, bubbleOpacity?: number | null,
    localWhisperModel?: string | null, localWhisperGpu?: boolean | null,
    sttProvider?: string | null, llmProvider?: string | null,
    insertAndSendSlot1?: boolean | null, autostopSilenceSecs?: number | null,
    autoModeSilenceSecs?: number | null,
    hotkeySlot2?: string | null, hotkeyModeSlot2?: HotkeyMode | null,
    insertAndSendSlot2?: boolean | null,
    bubbleTapMode?: string | null, bubbleTapAutoSend?: boolean | null,
    bubbleTapSilenceSecs?: number | null, bubbleLongPressMode?: string | null,
    bubbleLongPressAutoSend?: boolean | null, bubbleLongPressSilenceSecs?: number | null,
  ) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  onAudioDeviceChange: (d: string | null) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
  onOutputLanguageChange: (lang: string) => void;
  /** Called when user clicks "Setup-Assistent erneut starten". */
  onRestartOnboarding?: () => void;
}

export function SettingsPanel({
  onClose, loadedSettings, language, cleanupStyle, hotkey, hotkeyMode,
  hotkeySlot2, hotkeyModeSlot2,
  audioDevice, audioDevices, dictionary, outputLanguage,
  licenseStatus, licenseLoading, onValidateLicense, onRemoveLicense,
  onSave, onLanguageChange, onStyleChange, onHotkeyChange, onHotkeyModeChange,
  onAudioDeviceChange, onAddTerm, onRemoveTerm, onOutputLanguageChange,
  onRestartOnboarding,
}: SettingsPanelProps) {
  const [groqKey, setGroqKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [localLang, setLocalLang] = useState(language);
  const [localStyle, setLocalStyle] = useState(cleanupStyle);
  const [localHotkey, setLocalHotkey] = useState(hotkey);
  const [localHotkeyMode, setLocalHotkeyMode] = useState(hotkeyMode);
  const [localHotkeySlot2, setLocalHotkeySlot2] = useState(hotkeySlot2);
  const [localHotkeyModeSlot2, setLocalHotkeyModeSlot2] = useState(hotkeyModeSlot2);
  const [localAudioDevice, setLocalAudioDevice] = useState(audioDevice);
  const [localSttModel, setLocalSttModel] = useState(loadedSettings?.sttModel ?? "whisper-large-v3-turbo");
  const [localCustomPrompt, setLocalCustomPrompt] = useState(loadedSettings?.customPrompt ?? "");
  const [localAutostart, setLocalAutostart] = useState(loadedSettings?.autostart ?? false);
  const [localWhisperMode, setLocalWhisperMode] = useState(loadedSettings?.whisperMode ?? false);
  const [openaiKey, setOpenaiKey] = useState("");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [openrouterKey, setOpenrouterKey] = useState("");
  // API key validation errors: null = no error, string = error message.
  const [apiKeyErrors, setApiKeyErrors] = useState<Record<string, string | null>>({});
  // Which keys are currently being validated (spinner state).
  const [apiKeyValidating, setApiKeyValidating] = useState<Record<string, boolean>>({});
  // Confirm-remove state per provider. When truthy, shows "Remove?" confirmation.
  const [apiKeyConfirmRemove, setApiKeyConfirmRemove] = useState<Record<string, boolean>>({});
  const apiKeyConfirmTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
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
  const [localInsertAndSendSlot1, setLocalInsertAndSendSlot1] = useState(loadedSettings?.insertAndSendSlot1 ?? false);
  const [localInsertAndSendSlot2, setLocalInsertAndSendSlot2] = useState(loadedSettings?.insertAndSendSlot2 ?? false);
  const [localSilenceSecs, setLocalSilenceSecs] = useState(() => {
    const mode = loadedSettings?.hotkeyMode ?? "hold";
    if (mode === "auto") return loadedSettings?.autoModeSilenceSecs ?? 2.0;
    return loadedSettings?.autostopSilenceSecs ?? 2.0;
  });
  const [bubbleTab, setBubbleTab] = useState<0 | 1>(0);
  const [localBubbleTapMode, setLocalBubbleTapMode] = useState<HotkeyMode>((loadedSettings?.bubbleTapMode ?? "toggle") as HotkeyMode);
  const [localBubbleTapAutoSend, setLocalBubbleTapAutoSend] = useState(loadedSettings?.bubbleTapAutoSend ?? false);
  const [localBubbleTapSilenceSecs, setLocalBubbleTapSilenceSecs] = useState(loadedSettings?.bubbleTapSilenceSecs ?? 2.0);
  const [localBubbleLongPressMode, setLocalBubbleLongPressMode] = useState<HotkeyMode>((loadedSettings?.bubbleLongPressMode ?? "hold") as HotkeyMode);
  const [localBubbleLongPressAutoSend, setLocalBubbleLongPressAutoSend] = useState(loadedSettings?.bubbleLongPressAutoSend ?? false);
  const [localBubbleLongPressSilenceSecs, setLocalBubbleLongPressSilenceSecs] = useState(loadedSettings?.bubbleLongPressSilenceSecs ?? 2.0);
  // Voice Command Mode: reflects what the backend monitor is currently doing.
  // Toggling this calls toggle_voice_command_mode and syncs the backend directly.
  const [localVoiceCommandEnabled, setLocalVoiceCommandEnabled] = useState(loadedSettings?.voiceCommandEnabled ?? false);
  // Silence threshold: lives in AdvancedSettings, loaded separately on mount.
  const [localSilenceThreshold, setLocalSilenceThreshold] = useState(0.005);
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
  const [openSections, setOpenSections] = useState<Record<string, boolean>>({});
  // Active tab inside the combined Hotkey section: 0 = Hotkey 1, 1 = Hotkey 2
  const [hotkeyTab, setHotkeyTab] = useState<0 | 1>(0);

  const toggleSection = useCallback((key: string) => {
    setOpenSections((prev) => {
      const wasOpen = prev[key];
      return wasOpen ? {} : { [key]: true };
    });
  }, []);

  const sectionBtnCls = "flex items-center gap-2 w-full py-2 text-left";

  // Load profiles on mount.
  useEffect(() => { getProfiles().then(setProfiles).catch(console.error); }, []);

  // Load advanced settings on mount to initialise localSilenceThreshold.
  const [advancedSettings, setAdvancedSettings] = useState<AdvancedSettings | null>(null);
  useEffect(() => {
    getAdvancedSettings()
      .then((adv) => {
        setAdvancedSettings(adv);
        setLocalSilenceThreshold(adv.silenceThreshold);
      })
      .catch(console.error);
  }, []);

  // Sync Voice Command toggle with the actual backend runtime state on mount,
  // and keep it in sync via events (e.g. when monitor stops itself via "Klarvo off").
  useEffect(() => {
    if (!isDesktop) return;

    getVoiceCommandActive()
      .then(setLocalVoiceCommandEnabled)
      .catch(console.error);

    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<{ active: boolean }>("klarvo://voice-command-state-changed", (event) => {
        setLocalVoiceCommandEnabled(event.payload.active);
      }).then((fn) => { unlisten = fn; }).catch(console.error);
    });

    return () => { unlisten?.(); };
  }, []);

  // Load app version on mount.
  useEffect(() => { getAppVersion().then(setAppVersion).catch(() => setAppVersion("0.4.1")); }, []);

  useEffect(() => { setLocalLang(language); }, [language]);
  useEffect(() => { setLocalStyle(cleanupStyle); }, [cleanupStyle]);
  useEffect(() => { setLocalHotkey(hotkey); }, [hotkey]);
  useEffect(() => { setLocalHotkeyMode(hotkeyMode); }, [hotkeyMode]);
  useEffect(() => { setLocalHotkeySlot2(hotkeySlot2); }, [hotkeySlot2]);
  useEffect(() => { setLocalHotkeyModeSlot2(hotkeyModeSlot2); }, [hotkeyModeSlot2]);
  useEffect(() => { setLocalAudioDevice(audioDevice); }, [audioDevice]);
  useEffect(() => {
    if (loadedSettings) {
      setLocalSttModel(loadedSettings.sttModel);
      setLocalCustomPrompt(loadedSettings.customPrompt);
      setLocalAutostart(loadedSettings.autostart);
      setLocalWhisperMode(loadedSettings.whisperMode);
      setLocalSttProvider(loadedSettings.sttProvider ?? "groq");
      const llmProv = loadedSettings.llmProvider ?? "deepseek";
      const llmKeyMap: Record<string, string | undefined> = {
        deepseek: loadedSettings.deepseekApiKeyMasked,
        openai: loadedSettings.openaiApiKeyMasked,
        anthropic: loadedSettings.anthropicApiKeyMasked,
        groq: loadedSettings.groqApiKeyMasked,
        openrouter: loadedSettings.openrouterApiKeyMasked,
      };
      if (!llmKeyMap[llmProv]) {
        const fallback = ["deepseek", "openai", "groq", "anthropic", "openrouter"].find(p => llmKeyMap[p]);
        setLocalLlmProvider(fallback ?? llmProv);
      } else {
        setLocalLlmProvider(llmProv);
      }
      setLocalOutputLanguage(loadedSettings.outputLanguage ?? "");
      setLocalWebhookUrl(loadedSettings.webhookUrl ?? "");
      setLocalTursoUrl(loadedSettings.tursoUrl ?? "");
      setLocalBubbleSize(loadedSettings.bubbleSize ?? 1.0);
      setLocalBubbleOpacity(loadedSettings.bubbleOpacity ?? 0.85);
      setLocalWhisperModel(loadedSettings.localWhisperModel ?? "small");
      setLocalWhisperGpu(loadedSettings.localWhisperGpu ?? true);
      setLocalInsertAndSendSlot1(loadedSettings.insertAndSendSlot1 ?? false);
      setLocalInsertAndSendSlot2(loadedSettings.insertAndSendSlot2 ?? false);
      const mode = loadedSettings.hotkeyMode ?? "hold";
      if (mode === "auto") {
        setLocalSilenceSecs(loadedSettings.autoModeSilenceSecs ?? 2.0);
      } else {
        setLocalSilenceSecs(loadedSettings.autostopSilenceSecs ?? 2.0);
      }
      setLocalHotkeySlot2(loadedSettings.hotkeySlot2 ?? "");
      setLocalHotkeyModeSlot2(loadedSettings.hotkeyModeSlot2 ?? "hold");
      setLocalBubbleTapMode((loadedSettings.bubbleTapMode ?? "toggle") as HotkeyMode);
      setLocalBubbleTapAutoSend(loadedSettings.bubbleTapAutoSend ?? false);
      setLocalBubbleTapSilenceSecs(loadedSettings.bubbleTapSilenceSecs ?? 2.0);
      setLocalBubbleLongPressMode((loadedSettings.bubbleLongPressMode ?? "hold") as HotkeyMode);
      setLocalBubbleLongPressAutoSend(loadedSettings.bubbleLongPressAutoSend ?? false);
      setLocalBubbleLongPressSilenceSecs(loadedSettings.bubbleLongPressSilenceSecs ?? 2.0);
      setLocalVoiceCommandEnabled(loadedSettings.voiceCommandEnabled ?? false);
    }
  }, [loadedSettings]);

  // Auto-switch STT provider when the currently selected provider has no API key
  // but another cloud provider does. Prevents the user from accidentally saving
  // a provider config that will produce a 401 on first use.
  // Does NOT touch "local" provider -- that needs no cloud key.
  useEffect(() => {
    if (!loadedSettings) return;
    if (localSttProvider === "local") return;

    const groqAvailable = !!loadedSettings.groqApiKeyMasked;
    const openaiAvailable = !!loadedSettings.openaiApiKeyMasked;

    if (localSttProvider === "groq" && !groqAvailable && openaiAvailable) {
      setLocalSttProvider("openai");
      setLocalSttModel("whisper-1");
    } else if (localSttProvider === "openai" && !openaiAvailable && groqAvailable) {
      setLocalSttProvider("groq");
      setLocalSttModel("whisper-large-v3-turbo");
    }
    // If neither provider has a key: leave state as-is -- both are equally broken.
  }, [loadedSettings, localSttProvider, localSttModel]);

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
      localInsertAndSendSlot1 !== (loadedSettings.insertAndSendSlot1 ?? false) ||
      localInsertAndSendSlot2 !== (loadedSettings.insertAndSendSlot2 ?? false) ||
      ((localHotkeyMode === "autostop" || localHotkeyModeSlot2 === "autostop") && localSilenceSecs !== (loadedSettings.autostopSilenceSecs ?? 2.0)) ||
      ((localHotkeyMode === "auto" || localHotkeyModeSlot2 === "auto") && localSilenceSecs !== (loadedSettings.autoModeSilenceSecs ?? 2.0)) ||
      localHotkeySlot2 !== (loadedSettings.hotkeySlot2 ?? "") ||
      localHotkeyModeSlot2 !== (loadedSettings.hotkeyModeSlot2 ?? "hold") ||
      groqKey.trim() !== "" ||
      deepseekKey.trim() !== "" ||
      openaiKey.trim() !== "" ||
      anthropicKey.trim() !== "" ||
      tursoToken.trim() !== "" ||
      (!isDesktop && (
        localBubbleTapMode !== (loadedSettings.bubbleTapMode ?? "toggle") ||
        localBubbleTapAutoSend !== (loadedSettings.bubbleTapAutoSend ?? false) ||
        localBubbleTapSilenceSecs !== (loadedSettings.bubbleTapSilenceSecs ?? 2.0) ||
        localBubbleLongPressMode !== (loadedSettings.bubbleLongPressMode ?? "hold") ||
        localBubbleLongPressAutoSend !== (loadedSettings.bubbleLongPressAutoSend ?? false) ||
        localBubbleLongPressSilenceSecs !== (loadedSettings.bubbleLongPressSilenceSecs ?? 2.0)
      ));
    setIsDirty(dirty);
  }, [
    loadedSettings, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, localSttProvider,
    localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl, localBubbleSize,
    localBubbleOpacity, localWhisperModel, localWhisperGpu,
    localInsertAndSendSlot1, localInsertAndSendSlot2, localSilenceSecs, localHotkeySlot2, localHotkeyModeSlot2,
    localBubbleTapMode, localBubbleTapAutoSend, localBubbleTapSilenceSecs,
    localBubbleLongPressMode, localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
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
    // Validate any newly entered API keys before persisting.
    // Keys that are empty (no change) are skipped.
    const keysToValidate: Array<{ provider: string; key: string }> = [
      { provider: "groq", key: groqKey.trim() },
      { provider: "deepseek", key: deepseekKey.trim() },
      { provider: "openai", key: openaiKey.trim() },
      { provider: "anthropic", key: anthropicKey.trim() },
      { provider: "openrouter", key: openrouterKey.trim() },
    ].filter((e) => e.key !== "");

    if (keysToValidate.length > 0) {
      // Mark all pending keys as validating.
      const validatingState: Record<string, boolean> = {};
      keysToValidate.forEach(({ provider }) => { validatingState[provider] = true; });
      setApiKeyValidating(validatingState);

      const errors: Record<string, string | null> = {};
      await Promise.all(
        keysToValidate.map(async ({ provider, key }) => {
          try {
            const valid = await validateApiKey(provider, key);
            errors[provider] = valid ? null : "Invalid API key";
          } catch {
            // Network error: treat as invalid so the user can retry.
            errors[provider] = "Validation failed — check your network";
          }
        }),
      );

      setApiKeyValidating({});
      setApiKeyErrors((prev) => ({ ...prev, ...errors }));

      const hasErrors = Object.values(errors).some((e) => e !== null);
      if (hasErrors) return; // Abort save — errors shown inline.
    }

    setSaving(true);
    if (!opts?.silent) setSaveMsg(null);
    try {
      const autostopSecs = localHotkeyMode === "autostop" ? localSilenceSecs : null;
      const autoModeSecs = localHotkeyMode === "auto" ? localSilenceSecs : null;
      await onSave(
        groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        openaiKey.trim(), anthropicKey.trim(), openrouterKey.trim(),
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), tursoToken.trim(),
        localBubbleSize, localBubbleOpacity,
        localWhisperModel, localWhisperGpu,
        localSttProvider, localLlmProvider,
        localInsertAndSendSlot1, autostopSecs, autoModeSecs,
        localHotkeySlot2, localHotkeyModeSlot2,
        localInsertAndSendSlot2,
        localBubbleTapMode, localBubbleTapAutoSend,
        localBubbleTapSilenceSecs, localBubbleLongPressMode,
        localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
      );
      // Save silence threshold into AdvancedSettings when it has changed.
      if (advancedSettings !== null && advancedSettings.silenceThreshold !== localSilenceThreshold) {
        const updatedAdv: AdvancedSettings = { ...advancedSettings, silenceThreshold: localSilenceThreshold };
        await saveAdvancedSettings(updatedAdv);
        setAdvancedSettings(updatedAdv);
      }
      setGroqKey("");
      setDeepseekKey("");
      setOpenaiKey("");
      setAnthropicKey("");
      setTursoToken("");
      // Clear validation errors after a successful save.
      setApiKeyErrors({});
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
    localBubbleSize, localBubbleOpacity, localWhisperModel, localWhisperGpu,
    localInsertAndSendSlot1, localInsertAndSendSlot2, localSilenceSecs, localHotkeySlot2, localHotkeyModeSlot2,
    localBubbleTapMode, localBubbleTapAutoSend, localBubbleTapSilenceSecs,
    localBubbleLongPressMode, localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
    advancedSettings, localSilenceThreshold,
    openrouterKey,
    onSave,
  ]);

  const handleSave = useCallback(async () => {
    await saveCurrentSettings();
  }, [saveCurrentSettings]);

  // Handles the two-step confirm-then-remove flow for API keys.
  // First click: shows "Remove?" confirmation for 4 s.
  // Second click within 4 s: calls clearApiKey and reloads settings.
  const handleApiKeyRemoveClick = useCallback((provider: string) => {
    if (!apiKeyConfirmRemove[provider]) {
      setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: true }));
      apiKeyConfirmTimers.current[provider] = setTimeout(() => {
        setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: false }));
      }, 4000);
      return;
    }
    // Second click: execute removal.
    if (apiKeyConfirmTimers.current[provider]) clearTimeout(apiKeyConfirmTimers.current[provider]);
    setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: false }));
    clearApiKey(provider)
      .then(() => saveCurrentSettings({ silent: true }))
      .catch((err) => console.error("clearApiKey failed:", err));
  }, [apiKeyConfirmRemove, saveCurrentSettings]);

  // Clear confirm timers on unmount.
  useEffect(() => {
    const timers = apiKeyConfirmTimers.current;
    return () => { Object.values(timers).forEach(clearTimeout); };
  }, []);

  // Called from LicenseSection after successful activation: persist immediately
  // so the user never has to click Save Settings for license changes.
  // We do NOT want to mark this as a dirty operation -- use silent mode and
  // pass empty strings for API keys (backend ignores them when empty).
  const handleLicenseAutoSave = useCallback(async () => {
    setSaving(true);
    try {
      const autostopSecs = localHotkeyMode === "autostop" ? localSilenceSecs : null;
      const autoModeSecs = localHotkeyMode === "auto" ? localSilenceSecs : null;
      await onSave(
        "", "", localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        "", "", "",
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), "",
        localBubbleSize, localBubbleOpacity,
        localWhisperModel, localWhisperGpu,
        localSttProvider, localLlmProvider,
        localInsertAndSendSlot1, autostopSecs, autoModeSecs,
        localHotkeySlot2, localHotkeyModeSlot2,
        localInsertAndSendSlot2,
        localBubbleTapMode, localBubbleTapAutoSend,
        localBubbleTapSilenceSecs, localBubbleLongPressMode,
        localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
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
    localBubbleSize, localBubbleOpacity, localWhisperModel, localWhisperGpu,
    localInsertAndSendSlot1, localInsertAndSendSlot2, localSilenceSecs, localHotkeySlot2, localHotkeyModeSlot2,
    localBubbleTapMode, localBubbleTapAutoSend, localBubbleTapSilenceSecs,
    localBubbleLongPressMode, localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
    onSave,
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
  const openrouterOk = !!loadedSettings?.openrouterApiKeyMasked;

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
    <div className={`w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col ${panelMaxH}`}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-klarvo-border/40 flex-shrink-0">
        <span className={SECTION_TITLE_CLS}>Settings</span>
        <button
          aria-label="Close settings"
          onClick={onClose}
          className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Scrollable body */}
      <div className="overflow-y-auto flex-1 min-h-0 p-4 flex flex-col gap-5">

        {/* --- Voice & Recording --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("voiceRecording")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.voiceRecording ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">Voice & Recording</span>
          </button>
          {openSections.voiceRecording && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">

              {/* Cloud / Offline toggle -- same visual style as StylePicker */}
              <div className="flex flex-col gap-2">
                <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Speech Recognition</span>
                <div className="flex flex-col gap-2 pl-0">
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 w-fit">
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
                        ? "bg-klarvo-primary/15 text-klarvo-primary"
                        : "text-klarvo-dim hover:text-klarvo-muted",
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
                        ? "bg-klarvo-primary/15 text-klarvo-primary"
                        : "text-klarvo-dim hover:text-klarvo-muted",
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
                        className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
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
                    <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-klarvo-surface/30 border border-klarvo-border/30">
                      <svg className="w-3.5 h-3.5 text-klarvo-muted mt-0.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" />
                      </svg>
                      <p className="text-[11px] text-klarvo-muted leading-relaxed">
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
                  <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Text Cleanup</span>

                  <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
                    <span className={LABEL_CLS_M}>Provider</span>
                    <select
                      value={localLlmProvider}
                      onChange={(e) => setLocalLlmProvider(e.target.value)}
                      className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
                    >
                      <option value="deepseek" disabled={!deepseekOk}>DeepSeek{!deepseekOk ? " (no key)" : ""}</option>
                      <option value="openai" disabled={!openaiOk}>OpenAI{!openaiOk ? " (no key)" : ""}</option>
                      <option value="groq" disabled={!groqOk}>Groq (Llama){!groqOk ? " (no key)" : ""}</option>
                      <option value="openrouter" disabled={!openrouterOk}>OpenRouter{!openrouterOk ? " (no key)" : ""}</option>
                    </select>
                  </div>

                  <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
                    <span className={LABEL_CLS_M}>Style</span>
                    <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                      {STYLE_OPTIONS.map((opt) => (
                        <button
                          key={opt.value}
                          onClick={() => handleStyleChange(opt.value)}
                          title={opt.description}
                          className={[
                            isMobile ? "flex-1 px-3 py-2 rounded-md text-sm font-medium transition-all duration-100" : "px-2 py-1 rounded-md text-xs font-medium transition-all duration-100",
                            localStyle === opt.value
                              ? "bg-klarvo-primary/15 text-klarvo-primary"
                              : "text-klarvo-dim hover:text-klarvo-muted",
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
                  className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
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
                  className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
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
                    className="bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text max-w-[180px] truncate focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer"
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
              <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.hotkey ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">Hotkey</span>
            </button>
            {openSections.hotkey && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
                {/* Tab bar */}
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 self-start">
                  <button
                    onClick={() => setHotkeyTab(0)}
                    className={[
                      "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      hotkeyTab === 0 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    Hotkey 1
                  </button>
                  <button
                    onClick={() => setHotkeyTab(1)}
                    className={[
                      "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      hotkeyTab === 1 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    Hotkey 2
                  </button>
                </div>

                {/* Tab 1: Hotkey 1 */}
                {hotkeyTab === 0 && (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <span className="text-xs text-klarvo-muted">Shortcut</span>
                      <ShortcutRecorder value={localHotkey} onChange={handleHotkeyChange} />
                    </div>

                    <div className="flex flex-col gap-1.5">
                      <span className={LABEL_CLS}>Mode</span>
                      <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                        {([
                          { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                          { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                          { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                          { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                        ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                          <button
                            key={value}
                            onClick={() => {
                              handleHotkeyModeChange(value);
                              // When switching modes, load the appropriate silence default from persisted settings
                              if (value === "auto") {
                                setLocalSilenceSecs(loadedSettings?.autoModeSilenceSecs ?? 2.0);
                              } else if (value === "autostop") {
                                setLocalSilenceSecs(loadedSettings?.autostopSilenceSecs ?? 2.0);
                              }
                            }}
                            title={tooltip}
                            className={[
                              "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                              localHotkeyMode === value
                                ? "bg-klarvo-primary/15 text-klarvo-primary"
                                : "text-klarvo-dim hover:text-klarvo-muted",
                            ].join(" ")}
                          >
                            {label}
                          </button>
                        ))}
                      </div>
                    </div>
                    <p className="text-[11px] text-klarvo-muted">
                      {localHotkeyMode === "hold" && "Hold to record, release to process"}
                      {localHotkeyMode === "toggle" && "Press once to start, press again to stop"}
                      {localHotkeyMode === "autostop" && "Press to start, stops automatically on silence"}
                      {localHotkeyMode === "auto" && "Continuous — restarts after each silence gap"}
                    </p>

                    {(localHotkeyMode === "autostop" || localHotkeyMode === "auto") && (
                      <>
                        <div className="flex flex-col gap-1.5">
                          <div className="flex items-center justify-between">
                            <span className={LABEL_CLS}>Silence Duration</span>
                            <span className="text-xs font-mono text-klarvo-primary">{localSilenceSecs.toFixed(1)}s</span>
                          </div>
                          <input
                            type="range"
                            min={1.0}
                            max={4.0}
                            step={0.1}
                            value={localSilenceSecs}
                            onChange={(e) => setLocalSilenceSecs(parseFloat(e.target.value))}
                            className="w-full accent-klarvo-primary"
                          />
                          <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                        </div>

                      </>
                    )}

                    {/* Insert & Send -- per-slot option for Hotkey 1 */}
                    <div className="flex items-center justify-between gap-3 pt-1 border-t border-klarvo-border/40">
                      <div className="flex flex-col gap-0.5">
                        <span className={LABEL_CLS}>Insert &amp; Send</span>
                        <span className="text-[11px] text-klarvo-muted">Send Enter after pasting (useful for chat apps)</span>
                      </div>
                      <button
                        role="switch"
                        aria-checked={localInsertAndSendSlot1}
                        onClick={() => setLocalInsertAndSendSlot1((v) => !v)}
                        className={[
                          "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
                          localInsertAndSendSlot1 ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
                        ].join(" ")}
                      >
                        <span
                          className={[
                            "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                            localInsertAndSendSlot1 ? "translate-x-4" : "",
                          ].join(" ")}
                        />
                      </button>
                    </div>
                  </>
                )}

                {/* Tab 2: Hotkey 2 */}
                {hotkeyTab === 1 && (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <div className="flex items-center justify-between">
                        <span className="text-xs text-klarvo-muted">Shortcut</span>
                        {localHotkeySlot2 && (
                          <button
                            type="button"
                            onClick={() => setLocalHotkeySlot2("")}
                            className="text-[11px] text-klarvo-dim hover:text-klarvo-muted transition-colors"
                          >
                            Clear
                          </button>
                        )}
                      </div>
                      {localHotkeySlot2 ? (
                        <ShortcutRecorder value={localHotkeySlot2} onChange={setLocalHotkeySlot2} />
                      ) : (
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-klarvo-dim italic">Not set</span>
                          <ShortcutRecorder value="" onChange={setLocalHotkeySlot2} />
                        </div>
                      )}
                    </div>

                    {localHotkeySlot2 && (
                      <div className="flex flex-col gap-1.5">
                        <span className={LABEL_CLS}>Mode</span>
                        <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                          {([
                            { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                            { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                            { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                            { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                          ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                            <button
                              key={value}
                              onClick={() => setLocalHotkeyModeSlot2(value)}
                              title={tooltip}
                              className={[
                                "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                                localHotkeyModeSlot2 === value
                                  ? "bg-klarvo-primary/15 text-klarvo-primary"
                                  : "text-klarvo-dim hover:text-klarvo-muted",
                              ].join(" ")}
                            >
                              {label}
                            </button>
                          ))}
                        </div>
                        <p className="text-[11px] text-klarvo-dim">
                          {localHotkeyModeSlot2 === "hold" && "Hold to record, release to process"}
                          {localHotkeyModeSlot2 === "toggle" && "Press once to start, press again to stop"}
                          {localHotkeyModeSlot2 === "autostop" && "Press to start, stops automatically on silence"}
                          {localHotkeyModeSlot2 === "auto" && "Continuous — restarts after each silence gap"}
                        </p>

                        {(localHotkeyModeSlot2 === "autostop" || localHotkeyModeSlot2 === "auto") && (
                          <>
                            <div className="flex flex-col gap-1.5 mt-1">
                              <div className="flex items-center justify-between">
                                <span className={LABEL_CLS}>Silence Duration</span>
                                <span className="text-xs font-mono text-klarvo-primary">{localSilenceSecs.toFixed(1)}s</span>
                              </div>
                              <input
                                type="range"
                                min={1.0}
                                max={4.0}
                                step={0.1}
                                value={localSilenceSecs}
                                onChange={(e) => setLocalSilenceSecs(parseFloat(e.target.value))}
                                className="w-full accent-klarvo-primary"
                              />
                              <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                            </div>

                          </>
                        )}
                      </div>
                    )}

                    {/* Insert & Send -- per-slot option for Hotkey 2 */}
                    <div className="flex items-center justify-between gap-3 pt-1 border-t border-klarvo-border/40">
                      <div className="flex flex-col gap-0.5">
                        <span className={LABEL_CLS}>Insert &amp; Send</span>
                        <span className="text-[11px] text-klarvo-muted">Send Enter after pasting (useful for chat apps)</span>
                      </div>
                      <button
                        role="switch"
                        aria-checked={localInsertAndSendSlot2}
                        onClick={() => setLocalInsertAndSendSlot2((v) => !v)}
                        className={[
                          "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
                          localInsertAndSendSlot2 ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
                        ].join(" ")}
                      >
                        <span
                          className={[
                            "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                            localInsertAndSendSlot2 ? "translate-x-4" : "",
                          ].join(" ")}
                        />
                      </button>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>
        )}

        {/* --- Voice Command Mode -- desktop only, parked until SAPI implementation --- */}
        {false && isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("voiceCommand")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.voiceCommand ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className={`flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide ${isPaid ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
                Voice Command Mode
                {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
              </span>
            </button>
            {openSections.voiceCommand && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
                {/* Voice command requires a Groq API key for real-time STT */}
                {!groqOk && (
                  <p className="text-[11px] text-klarvo-warning/80">
                    Requires a Groq API key (Settings &rarr; API Keys)
                  </p>
                )}

                {/* Toggle row */}
                <label className={`flex items-center justify-between gap-3 ${isPaid ? "cursor-pointer" : "cursor-not-allowed"}`}>
                  <div className={`flex flex-col gap-0.5 ${!isPaid ? "opacity-50" : ""}`}>
                    <span className="flex items-center gap-1.5">
                      <span className={LABEL_CLS}>Voice Command Mode</span>
                      {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
                    </span>
                    <span className="text-[11px] text-klarvo-dim">
                      Activate dictation by saying &ldquo;Klarvo toggle&rdquo; &mdash; no hotkey needed
                    </span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={localVoiceCommandEnabled}
                    disabled={!isPaid && !localVoiceCommandEnabled}
                    onClick={async () => {
                      if (!isPaid && !localVoiceCommandEnabled) return;
                      try {
                        const newState = await toggleVoiceCommandMode();
                        setLocalVoiceCommandEnabled(newState);
                      } catch (e) {
                        // If toggle failed, force UI to "off" so user isn't stuck.
                        console.error("[voice_command] toggle failed:", e);
                        setLocalVoiceCommandEnabled(false);
                      }
                    }}
                    className={[
                      "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
                      (!isPaid && !localVoiceCommandEnabled) ? "opacity-50 cursor-not-allowed" : "",
                      localVoiceCommandEnabled ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
                    ].join(" ")}
                  >
                    <span
                      className={[
                        "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
                        localVoiceCommandEnabled ? "translate-x-4" : "",
                      ].join(" ")}
                    />
                  </button>
                </label>

                {/* Running indicator */}
                {localVoiceCommandEnabled && (
                  <p className="text-[11px] text-klarvo-primary/80">
                    Listening for &ldquo;Klarvo&rdquo; commands&hellip;
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {/* --- Bubble Controls -- mobile only --- */}
        {!isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("bubble")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.bubble ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">Bubble Controls</span>
            </button>
            {openSections.bubble && (
              <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
                {/* Tab bar: Tap / Long Press */}
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 self-start">
                  <button
                    onClick={() => setBubbleTab(0)}
                    className={[
                      "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      bubbleTab === 0 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    Tap
                  </button>
                  <button
                    onClick={() => setBubbleTab(1)}
                    className={[
                      "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      bubbleTab === 1 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    Long Press
                  </button>
                </div>

                {/* Tab 0: Tap */}
                {bubbleTab === 0 && (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <span className={LABEL_CLS}>Mode</span>
                      <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                        {([
                          { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                          { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                          { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                          { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                        ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                          <button
                            key={value}
                            onClick={() => setLocalBubbleTapMode(value)}
                            title={tooltip}
                            className={[
                              "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                              localBubbleTapMode === value
                                ? "bg-klarvo-primary/15 text-klarvo-primary"
                                : "text-klarvo-dim hover:text-klarvo-muted",
                            ].join(" ")}
                          >
                            {label}
                          </button>
                        ))}
                      </div>
                    </div>
                    <p className="text-[11px] text-klarvo-dim">
                      {localBubbleTapMode === "hold" && "Hold to record, release to process"}
                      {localBubbleTapMode === "toggle" && "Press once to start, press again to stop"}
                      {localBubbleTapMode === "autostop" && "Press to start, stops automatically on silence"}
                      {localBubbleTapMode === "auto" && "Continuous — restarts after each silence gap"}
                    </p>

                    {(localBubbleTapMode === "autostop" || localBubbleTapMode === "auto") && (
                      <div className="flex flex-col gap-1.5">
                        <div className="flex items-center justify-between">
                          <span className={LABEL_CLS}>Silence Duration</span>
                          <span className="text-xs font-mono text-klarvo-primary">{localBubbleTapSilenceSecs.toFixed(1)}s</span>
                        </div>
                        <input
                          type="range"
                          min={1.0}
                          max={4.0}
                          step={0.1}
                          value={localBubbleTapSilenceSecs}
                          onChange={(e) => setLocalBubbleTapSilenceSecs(parseFloat(e.target.value))}
                          className="w-full accent-klarvo-primary"
                        />
                        <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                      </div>
                    )}

                    {/* Insert & Send hidden on Android — Enter key rarely works in mobile apps */}
                  </>
                )}

                {/* Tab 1: Long Press */}
                {bubbleTab === 1 && (
                  <>
                    <div className="flex flex-col gap-1.5">
                      <span className={LABEL_CLS}>Mode</span>
                      <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                        {([
                          { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                          { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                          { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                          { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                        ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                          <button
                            key={value}
                            onClick={() => setLocalBubbleLongPressMode(value)}
                            title={tooltip}
                            className={[
                              "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                              localBubbleLongPressMode === value
                                ? "bg-klarvo-primary/15 text-klarvo-primary"
                                : "text-klarvo-dim hover:text-klarvo-muted",
                            ].join(" ")}
                          >
                            {label}
                          </button>
                        ))}
                      </div>
                    </div>
                    <p className="text-[11px] text-klarvo-dim">
                      {localBubbleLongPressMode === "hold" && "Hold to record, release to process"}
                      {localBubbleLongPressMode === "toggle" && "Press once to start, press again to stop"}
                      {localBubbleLongPressMode === "autostop" && "Press to start, stops automatically on silence"}
                      {localBubbleLongPressMode === "auto" && "Continuous — restarts after each silence gap"}
                    </p>

                    {(localBubbleLongPressMode === "autostop" || localBubbleLongPressMode === "auto") && (
                      <div className="flex flex-col gap-1.5">
                        <div className="flex items-center justify-between">
                          <span className={LABEL_CLS}>Silence Duration</span>
                          <span className="text-xs font-mono text-klarvo-primary">{localBubbleLongPressSilenceSecs.toFixed(1)}s</span>
                        </div>
                        <input
                          type="range"
                          min={1.0}
                          max={4.0}
                          step={0.1}
                          value={localBubbleLongPressSilenceSecs}
                          onChange={(e) => setLocalBubbleLongPressSilenceSecs(parseFloat(e.target.value))}
                          className="w-full accent-klarvo-primary"
                        />
                        <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                      </div>
                    )}

                    {/* Insert & Send hidden on Android — Enter key rarely works in mobile apps */}
                  </>
                )}
              </div>
            )}
          </div>
        )}

        {/* --- Cleanup Instructions -- hidden when offline STT mode is active --- */}
        {localSttProvider !== "local" && <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("customPrompt")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.customPrompt ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className={`flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide ${isPaid ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
              Cleanup Instructions
              {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
            </span>
          </button>
          {openSections.customPrompt && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              <MobileTextarea
                label="Cleanup Instructions"
                hint="Appended to the system prompt during LLM cleanup."
                value={localCustomPrompt}
                onChange={isPaid ? setLocalCustomPrompt : () => {}}
                placeholder={isPaid ? "Extra instructions for the LLM, e.g. 'Always use formal German' or 'Keep technical terms in English'" : "Requires Klarvo License"}
                rows={3}
                className={`${INPUT_CLS_M} resize-none${!isPaid ? " opacity-50 cursor-not-allowed" : ""}`}
                disabled={!isPaid}
              />
              {/* Preset buttons -- one click replaces the entire custom prompt */}
              <div className="flex items-center gap-2 flex-wrap">
                <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Presets:</span>
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
                      "bg-transparent border-klarvo-border/60 text-klarvo-muted",
                      "hover:border-klarvo-border-active hover:text-klarvo-text",
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
                    "text-klarvo-dim hover:text-klarvo-muted",
                    isMobile ? "px-3 min-h-[44px] text-sm" : "px-2 py-1.5 text-xs",
                  ].join(" ")}
                >
                  Clear
                </button>
              </div>
              <p className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Appended to the system prompt during LLM cleanup.</p>
            </div>
          )}
        </div>}

        {/* --- General -- desktop only features --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("general")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.general ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">General</span>
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
                      localAutostart ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
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
                      {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
                    </span>
                    <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Amplifies mic input for quiet dictation</span>
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
                      localWhisperMode ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
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
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Select text, hold Ctrl+Shift+E, speak your edit. The selected text will be rewritten.</span>
                </div>
              </div>
            )}
          </div>
        )}


        {/* --- Sync --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("sync")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.sync ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className={`flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide ${isPaid ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
              Cross-Device Sync
              {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
            </span>
          </button>
          {openSections.sync && (
            <div className={`flex flex-col gap-3 pl-4 pb-3 pt-1${!isPaid ? " opacity-50" : ""}`}>
              <div className="flex flex-col gap-1.5">
                <span className="flex items-center gap-1.5">
                  <span className={LABEL_CLS_M}>Turso URL</span>
                  {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
                </span>
                <input
                  type="text"
                  placeholder={isPaid ? "libsql://your-db.turso.io" : "Requires Klarvo License"}
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
                  placeholder={isPaid ? (loadedSettings?.tursoTokenMasked || "Auth token") : "Requires Klarvo License"}
                  value={tursoToken}
                  disabled={!isPaid}
                  onChange={(e) => setTursoToken(e.target.value)}
                  className={`${INPUT_CLS_M}${!isPaid ? " cursor-not-allowed" : ""}`}
                />
              </div>
              {loadedSettings?.deviceId && (
                <p className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Device: {loadedSettings.deviceId.slice(0, 8)}...</p>
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
                className={`px-3 py-1.5 text-sm bg-klarvo-elevated text-white rounded hover:bg-klarvo-border-active disabled:opacity-40 transition-colors ${isMobile ? "py-2.5 text-base" : ""}${!isPaid ? " cursor-not-allowed" : ""}`}
              >
                {syncing ? "Syncing..." : "Sync Now"}
              </button>
              {syncMsg && <p className={isMobile ? "text-xs text-klarvo-muted" : "text-[11px] text-klarvo-muted"}>{syncMsg}</p>}
              <p className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Sync dictation history across devices via Turso. Leave empty to disable.</p>
            </div>
          )}
        </div>

        {/* --- API Keys --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("apiKeys")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.apiKeys ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">API Keys</span>
          </button>
          {openSections.apiKeys && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-2">
                  <span className={LABEL_CLS_M}>Groq</span>
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Speech + Cleanup)</span>
                  <StatusDot active={groqOk} />
                </div>
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={groqOk ? loadedSettings!.groqApiKeyMasked : "gsk_..."}
                  value={groqKey}
                  onChange={(e) => { setGroqKey(e.target.value); setApiKeyErrors((p) => ({ ...p, groq: null })); }}
                  className={INPUT_CLS_M}
                />
                {apiKeyValidating["groq"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
                {apiKeyErrors["groq"] && <span className="text-[11px] text-klarvo-warning">{apiKeyErrors["groq"]}</span>}
                {groqOk && (
                  <button type="button" onClick={() => handleApiKeyRemoveClick("groq")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["groq"] ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning"}`}>
                    {apiKeyConfirmRemove["groq"] ? "Click again to confirm removal" : "Remove Key"}
                  </button>
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-2">
                  <span className={LABEL_CLS_M}>DeepSeek</span>
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
                  <StatusDot active={deepseekOk} />
                </div>
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={deepseekOk ? loadedSettings!.deepseekApiKeyMasked : "sk-..."}
                  value={deepseekKey}
                  onChange={(e) => { setDeepseekKey(e.target.value); setApiKeyErrors((p) => ({ ...p, deepseek: null })); }}
                  className={INPUT_CLS_M}
                />
                {apiKeyValidating["deepseek"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
                {apiKeyErrors["deepseek"] && <span className="text-[11px] text-klarvo-warning">{apiKeyErrors["deepseek"]}</span>}
                {deepseekOk && (
                  <button type="button" onClick={() => handleApiKeyRemoveClick("deepseek")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["deepseek"] ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning"}`}>
                    {apiKeyConfirmRemove["deepseek"] ? "Click again to confirm removal" : "Remove Key"}
                  </button>
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-2">
                  <span className={LABEL_CLS_M}>OpenAI</span>
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Speech + Cleanup)</span>
                  <StatusDot active={openaiOk} />
                </div>
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={openaiOk ? loadedSettings!.openaiApiKeyMasked : "sk-..."}
                  value={openaiKey}
                  onChange={(e) => { setOpenaiKey(e.target.value); setApiKeyErrors((p) => ({ ...p, openai: null })); }}
                  className={INPUT_CLS_M}
                />
                {apiKeyValidating["openai"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
                {apiKeyErrors["openai"] && <span className="text-[11px] text-klarvo-warning">{apiKeyErrors["openai"]}</span>}
                {openaiOk && (
                  <button type="button" onClick={() => handleApiKeyRemoveClick("openai")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["openai"] ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning"}`}>
                    {apiKeyConfirmRemove["openai"] ? "Click again to confirm removal" : "Remove Key"}
                  </button>
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-2">
                  <span className={LABEL_CLS_M}>Anthropic</span>
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
                  <StatusDot active={anthropicOk} />
                </div>
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={anthropicOk ? loadedSettings!.anthropicApiKeyMasked : "sk-ant-..."}
                  value={anthropicKey}
                  onChange={(e) => { setAnthropicKey(e.target.value); setApiKeyErrors((p) => ({ ...p, anthropic: null })); }}
                  className={INPUT_CLS_M}
                />
                {apiKeyValidating["anthropic"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
                {apiKeyErrors["anthropic"] && <span className="text-[11px] text-klarvo-warning">{apiKeyErrors["anthropic"]}</span>}
                {anthropicOk && (
                  <button type="button" onClick={() => handleApiKeyRemoveClick("anthropic")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["anthropic"] ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning"}`}>
                    {apiKeyConfirmRemove["anthropic"] ? "Click again to confirm removal" : "Remove Key"}
                  </button>
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <div className="flex items-center gap-2">
                  <span className={LABEL_CLS_M}>OpenRouter</span>
                  <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
                  <StatusDot active={openrouterOk} />
                </div>
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={openrouterOk ? loadedSettings!.openrouterApiKeyMasked : "sk-or-..."}
                  value={openrouterKey}
                  onChange={(e) => { setOpenrouterKey(e.target.value); setApiKeyErrors((p) => ({ ...p, openrouter: null })); }}
                  className={INPUT_CLS_M}
                />
                {apiKeyValidating["openrouter"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
                {apiKeyErrors["openrouter"] && <span className="text-[11px] text-klarvo-warning">{apiKeyErrors["openrouter"]}</span>}
                {openrouterOk && (
                  <button type="button" onClick={() => handleApiKeyRemoveClick("openrouter")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["openrouter"] ? "text-klarvo-danger hover:text-red-300" : "text-klarvo-warning/80 hover:text-klarvo-warning"}`}>
                    {apiKeyConfirmRemove["openrouter"] ? "Click again to confirm removal" : "Remove Key"}
                  </button>
                )}
              </div>
            </div>
          )}
        </div>

        {/* --- Dictionary --- */}
        <div className="flex flex-col gap-1">
          <button onClick={() => toggleSection("dictionary")} className={sectionBtnCls}>
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.dictionary ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="flex items-center gap-1.5 text-sm font-semibold text-klarvo-primary uppercase tracking-wide">
              Dictionary
              <span className={`text-[10px] font-normal normal-case tracking-normal ${!isPaid && dictionary.length >= 20 ? "text-klarvo-warning/80" : "text-klarvo-primary"}`}>
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
                  className={`px-3 rounded-lg font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 disabled:opacity-30 disabled:cursor-not-allowed transition-colors ${isMobile ? "py-2.5 text-sm min-w-[56px]" : "py-2 text-xs"}`}
                >
                  Add
                </button>
              </div>

              {!isPaid && dictionary.length >= 20 && (
                <p className="text-[11px] text-klarvo-warning/80">
                  Free limit reached (20 terms). Upgrade for unlimited.
                </p>
              )}

              {dictionary.length > 0 ? (
                <div className="flex flex-wrap gap-1.5">
                  {dictionary.map((t) => <DictionaryTag key={t} term={t} onRemove={onRemoveTerm} />)}
                </div>
              ) : (
                <p className="text-xs text-klarvo-dim italic">No terms yet.</p>
              )}
            </div>
          )}
        </div>

        {/* --- Updates -- desktop only (Tauri updater not available on sideloaded APKs) --- */}
        {isDesktop && (
          <div className="flex flex-col gap-1">
            <button onClick={() => toggleSection("updates")} className={sectionBtnCls}>
              <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.updates ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">Updates</span>
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
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.appProfiles ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className={`flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide ${isPaid ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
              App Profiles
              {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
            </span>
          </button>
          {openSections.appProfiles && (
            <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
              {!isPaid ? (
                // Free-tier paygate: show lock message, no profile editing allowed.
                <div className="flex flex-col gap-2">
                  <div className="flex items-center gap-2 text-klarvo-dim">
                    <LockIcon className="w-3.5 h-3.5 text-klarvo-dim flex-shrink-0" />
                    <p className="text-xs">App Profiles require a Klarvo license.</p>
                  </div>
                  <p className="text-[11px] text-klarvo-dim">Override style and language per app based on window title.</p>
                </div>
              ) : (
                <>
                  <p className="text-[11px] text-klarvo-dim">Override style/language per app. Matches window title substring.</p>

                  {profiles.map((p, i) => (
                    <div key={i} className="bg-klarvo-bg border border-klarvo-border/60 rounded-xl p-3 flex flex-col gap-2">
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
                          className="text-klarvo-dim hover:text-klarvo-danger transition-colors p-1"
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
                          className="bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 cursor-pointer"
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
                          className="bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 cursor-pointer"
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
                      className="px-3 py-2 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 transition-colors"
                    >
                      + Add Profile
                    </button>
                    {profiles.length > 0 && (
                      <button
                        onClick={() => saveProfiles(profiles).then(() => setSaveMsg("Profiles saved")).catch((e) => setSaveMsg(String(e)))}
                        className="px-3 py-2 rounded-lg text-xs font-medium bg-klarvo-primary/10 border border-klarvo-primary/20 text-klarvo-primary hover:bg-klarvo-primary/15 transition-colors"
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
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.license ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">License</span>
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
            <svg className={`w-4 h-4 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSections.about ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M9 18l6-6-6-6" />
            </svg>
            <span className="text-sm font-semibold text-klarvo-primary uppercase tracking-wide">About</span>
          </button>
          {openSections.about && (
            <div className="flex flex-col gap-2 pl-4 pb-3 pt-1">
              <p className="text-xs font-medium text-klarvo-muted">
                Klarvo{appVersion ? ` v${appVersion}` : ""}
              </p>
              <p className="text-[11px] text-klarvo-dim">Voice dictation you own.</p>
              <p className="text-[11px] text-klarvo-dim">by Andreas Nolte</p>
              <div className="flex items-center gap-2 mt-0.5">
                <button
                  onClick={() => openUrl("https://github.com/andyon2/klarvo")}
                  className="text-[11px] text-klarvo-muted hover:text-klarvo-text underline underline-offset-2 transition-colors"
                >
                  GitHub
                </button>
                <span className="text-[11px] text-klarvo-dim">·</span>
                <span className="text-[11px] text-klarvo-dim">MIT License</span>
              </div>
              {onRestartOnboarding && (
                <button
                  onClick={onRestartOnboarding}
                  className="mt-2 text-[11px] text-klarvo-dim hover:text-klarvo-muted underline underline-offset-2 transition-colors text-left"
                >
                  Setup assistant restart
                </button>
              )}
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
        <div className={`px-4 py-3 border-t border-klarvo-border/40 ${isMobile ? "mobile-safe-bottom" : ""}`}>
          <button
            onClick={handleSave}
            disabled={saving}
            className={[
              "w-full py-2.5 rounded-xl text-sm font-medium transition-all duration-150 border",
              saveMsg === "Saved"
                ? "bg-klarvo-primary/15 border-klarvo-primary/30 text-klarvo-primary"
                : saveMsg && saveMsg !== "Saved"
                ? "bg-klarvo-danger/10 border-klarvo-danger/20 text-klarvo-danger"
                : "bg-klarvo-primary/10 border-klarvo-primary/30 text-klarvo-primary hover:bg-klarvo-primary/15 hover:border-klarvo-primary/40 animate-pulse",
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
