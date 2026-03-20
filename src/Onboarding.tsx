/**
 * Onboarding wizard.
 *
 * Full state-machine wizard that guides new users through setup.
 * Persists progress via setOnboardingState() after every step transition.
 *
 * Flow (desktop cloud):
 *   0 Welcome → 1 Mode → 2 Language → 3 STT Key → 4 LLM Key → 5 Test → 6 Done
 *
 * Flow (desktop offline):
 *   0 Welcome → 1 Mode → 2 Language → 3 Model Download → 4 LLM Key → 5 Test → 6 Done
 *
 * Flow (android cloud):
 *   0 Welcome → 1 Mode → 1a Overlay Perm → 1b Mic Perm → 1c Accessibility Perm
 *   → 1d Battery Perm → 2 Language → 3 STT Key → 4 LLM Key → 5 Test → 6 Done
 */
import { useState, useCallback, useEffect, useRef } from "react";
import { isMobile, isDesktop } from "./platform";
import type { OnboardingState } from "./types";
import {
  setOnboardingState,
  validateApiKey,
  saveSettings,
  getSettings,
  downloadWhisperModel,
  getWhisperModels,
  onModelDownloadProgress,
  onModelDownloadComplete,
  onModelDownloadError,
  startRecording,
  stopRecording,
  transcribeAudio,
  cleanupText,
} from "./tauri-commands";
import { startBrowserRecording, stopBrowserRecording } from "./media-recorder";
import type { AppSettings } from "./types";

// ---------------------------------------------------------------------------
// External URL helper
// ---------------------------------------------------------------------------

async function openExternalUrl(url: string): Promise<void> {
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

// ---------------------------------------------------------------------------
// Wizard step constants — logical step IDs
// ---------------------------------------------------------------------------

type WizardMode = "cloud" | "offline" | "";

// We compute a flat ordered list of step IDs at runtime based on mode + platform.
type StepId =
  | "welcome"
  | "mode"
  | "perm-overlay"
  | "perm-mic"
  | "perm-accessibility"
  | "perm-battery"
  | "language"
  | "stt-key"
  | "model-download"
  | "llm-key"
  | "test-dictation"
  | "done";

function buildStepList(mode: WizardMode): StepId[] {
  const base: StepId[] = ["welcome", "mode"];
  if (isMobile) {
    base.push("perm-overlay", "perm-mic", "perm-accessibility", "perm-battery");
  }
  base.push("language");
  if (mode === "offline" && isDesktop) {
    base.push("model-download", "llm-key");
  } else {
    // cloud or not yet chosen — we always show cloud steps
    base.push("stt-key", "llm-key");
  }
  base.push("test-dictation", "done");
  return base;
}

// ---------------------------------------------------------------------------
// Shared icon primitives
// ---------------------------------------------------------------------------

function MicIconLarge() {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className="w-full h-full">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function MicIconSm({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-5 h-5"} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function CloudIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-6 h-6"} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
    </svg>
  );
}

function ShieldIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-6 h-6"} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  );
}

function KeyIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-4 h-4"} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="15" r="4" />
      <path d="m21 3-9.4 9.4M15 9l2 2" />
    </svg>
  );
}

function ExternalLinkIcon() {
  return (
    <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" y1="14" x2="21" y2="3" />
    </svg>
  );
}

function CheckCircleIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-5 h-5"} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
      <polyline points="22 4 12 14.01 9 11.01" />
    </svg>
  );
}

function XCircleIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-5 h-5"} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}

function SpinnerIcon({ className }: { className?: string }) {
  return (
    <svg className={`${className ?? "w-4 h-4"} animate-spin`} viewBox="0 0 24 24" fill="none">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
  );
}

function StopIcon({ className }: { className?: string }) {
  return (
    <svg className={className ?? "w-8 h-8"} viewBox="0 0 24 24" fill="currentColor">
      <rect x="6" y="6" width="12" height="12" rx="1" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Step progress indicator
// ---------------------------------------------------------------------------

function StepDots({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex items-center gap-1.5">
      {Array.from({ length: total }, (_, i) => (
        <span
          key={i}
          className={[
            "rounded-full transition-all duration-300",
            i === current
              ? "w-4 h-1.5 bg-emerald-400"
              : i < current
              ? "w-1.5 h-1.5 bg-emerald-500/40"
              : "w-1.5 h-1.5 bg-zinc-700",
          ].join(" ")}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared button styles
// ---------------------------------------------------------------------------

const BTN_PRIMARY = [
  "w-full rounded-xl py-2.5 px-6 text-sm font-medium",
  "bg-emerald-500/15 border border-emerald-500/30 text-emerald-400",
  "hover:bg-emerald-500/20 hover:border-emerald-500/40",
  "disabled:opacity-40 disabled:cursor-not-allowed",
  "transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40",
].join(" ");

// ---------------------------------------------------------------------------
// API key input with live validation
// ---------------------------------------------------------------------------

type ValidationState = "idle" | "loading" | "valid" | "invalid";

interface ApiKeyFieldProps {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  provider: string;
  costHint?: string;
  magicLinkUrl?: string;
  magicLinkLabel?: string;
  validationState: ValidationState;
  validationError: string;
  onValidate: () => void;
}

function ApiKeyField({
  label,
  value,
  onChange,
  placeholder,
  provider,
  costHint,
  magicLinkUrl,
  magicLinkLabel,
  validationState,
  validationError,
  onValidate,
}: ApiKeyFieldProps) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <label className="text-xs text-zinc-400 font-medium">{label}</label>
        {magicLinkUrl && (
          <button
            type="button"
            onClick={() => openExternalUrl(magicLinkUrl).catch(console.error)}
            className="flex items-center gap-1 text-xs text-emerald-400 hover:text-emerald-300 transition-colors"
          >
            {magicLinkLabel ?? "Key erstellen"}
            <ExternalLinkIcon />
          </button>
        )}
      </div>
      <div className="flex gap-2">
        <input
          type="password"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder ?? "sk-..."}
          autoComplete="off"
          spellCheck={false}
          className={[
            "flex-1 bg-[#111113] border rounded-lg px-3 py-2",
            "text-sm text-zinc-200 font-mono placeholder:text-zinc-600",
            "focus:outline-none focus:border-emerald-500/40 focus:ring-1 focus:ring-emerald-500/20",
            "transition-colors duration-150",
            validationState === "valid"
              ? "border-emerald-500/50"
              : validationState === "invalid"
              ? "border-red-500/50"
              : "border-zinc-800/60",
          ].join(" ")}
        />
        {value.trim().length > 0 && (
          <button
            type="button"
            onClick={onValidate}
            disabled={validationState === "loading"}
            className="px-3 py-2 rounded-lg bg-zinc-800/80 border border-zinc-700/60 text-xs text-zinc-300 hover:text-zinc-100 hover:border-zinc-600 transition-all disabled:opacity-50 flex-shrink-0"
          >
            {validationState === "loading" ? (
              <SpinnerIcon className="w-4 h-4" />
            ) : (
              "Prüfen"
            )}
          </button>
        )}
      </div>
      {validationState === "valid" && (
        <div className="flex items-center gap-1.5 text-xs text-emerald-400">
          <CheckCircleIcon className="w-3.5 h-3.5" />
          <span>Key funktioniert</span>
        </div>
      )}
      {validationState === "invalid" && (
        <div className="flex items-center gap-1.5 text-xs text-red-400">
          <XCircleIcon className="w-3.5 h-3.5" />
          <span>{validationError || `Ungültiger ${provider}-Key`}</span>
        </div>
      )}
      {costHint && validationState === "idle" && (
        <p className="text-[11px] text-zinc-600">{costHint}</p>
      )}
    </div>
  );
}

function useKeyValidation(provider: string, key: string) {
  const [state, setState] = useState<ValidationState>("idle");
  const [error, setError] = useState("");

  const validate = useCallback(async () => {
    if (!key.trim()) return;
    setState("loading");
    setError("");
    try {
      const ok = await validateApiKey(provider, key.trim());
      setState(ok ? "valid" : "invalid");
      if (!ok) setError(`Key abgelehnt — ungültig oder abgelaufen`);
    } catch (err) {
      setState("invalid");
      setError(err instanceof Error ? err.message : "Netzwerkfehler");
    }
  }, [provider, key]);

  // Reset state when key changes
  useEffect(() => {
    setState("idle");
    setError("");
  }, [key]);

  return { state, error, validate };
}

// ---------------------------------------------------------------------------
// Step 0: Welcome
// ---------------------------------------------------------------------------

function StepWelcome({ onNext, onSkip }: { onNext: () => void; onSkip: () => void }) {
  return (
    <div className="flex flex-col items-center text-center gap-8">
      {/* Skip link */}
      <button
        onClick={onSkip}
        className="self-end text-xs text-zinc-600 hover:text-zinc-400 transition-colors"
      >
        Ich kenn mich aus →
      </button>

      {/* Animated mic with pulse ring */}
      <div className="relative flex items-center justify-center">
        <span className="absolute w-24 h-24 rounded-full bg-emerald-500/10 animate-ping" style={{ animationDuration: "2s" }} />
        <span className="absolute w-20 h-20 rounded-full bg-emerald-500/10 animate-ping" style={{ animationDuration: "2s", animationDelay: "0.5s" }} />
        <div className="relative w-16 h-16 rounded-2xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.18)]">
          <div className="w-8 h-8">
            <MicIconLarge />
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold text-zinc-100 tracking-tight">
          Sprich. Dikta tippt.
        </h1>
        <p className="text-sm text-zinc-400 leading-relaxed max-w-xs">
          Freies Sprachdiktat mit KI-Bereinigung. Dikta transkribiert und poliert deinen Text — und fügt ihn direkt ein, wo du gerade schreibst.
        </p>
      </div>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Loslegen
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1: Cloud / Offline mode selection
// ---------------------------------------------------------------------------

function StepMode({ selected, onSelect, onNext }: {
  selected: WizardMode;
  onSelect: (m: WizardMode) => void;
  onNext: () => void;
}) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Wie willst du Dikta nutzen?</h2>
        <p className="text-sm text-zinc-400">Beide Varianten sind kostengünstig — du entscheidest.</p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Cloud card */}
        <button
          type="button"
          onClick={() => onSelect("cloud")}
          className={[
            "flex flex-col gap-3 p-4 rounded-xl border text-left transition-all duration-150",
            selected === "cloud"
              ? "border-emerald-500/50 bg-emerald-500/8"
              : "border-zinc-800/60 bg-[#111113] hover:border-zinc-700/60",
          ].join(" ")}
        >
          <div className={`flex items-center gap-2 ${selected === "cloud" ? "text-emerald-400" : "text-zinc-400"}`}>
            <CloudIcon className="w-5 h-5" />
            <span className="text-sm font-semibold text-zinc-200">Cloud</span>
            {selected === "cloud" && (
              <span className="ml-auto text-[10px] font-medium text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded-full px-2 py-0.5">
                empfohlen
              </span>
            )}
            {selected !== "cloud" && (
              <span className="ml-auto text-[10px] font-medium text-zinc-500 bg-zinc-800/60 border border-zinc-700/40 rounded-full px-2 py-0.5">
                empfohlen
              </span>
            )}
          </div>
          <ul className="flex flex-col gap-1">
            {["Beste Qualität", "API-Key benötigt", "Groq kostenlos (mit Limit)"].map((b) => (
              <li key={b} className="text-[11px] text-zinc-500 flex items-start gap-1.5">
                <span className="text-emerald-500/60 mt-0.5">•</span>
                {b}
              </li>
            ))}
          </ul>
        </button>

        {/* Offline card */}
        {isDesktop ? (
          <button
            type="button"
            onClick={() => onSelect("offline")}
            className={[
              "flex flex-col gap-3 p-4 rounded-xl border text-left transition-all duration-150",
              selected === "offline"
                ? "border-emerald-500/50 bg-emerald-500/8"
                : "border-zinc-800/60 bg-[#111113] hover:border-zinc-700/60",
            ].join(" ")}
          >
            <div className={`flex items-center gap-2 ${selected === "offline" ? "text-emerald-400" : "text-zinc-400"}`}>
              <ShieldIcon className="w-5 h-5" />
              <span className="text-sm font-semibold text-zinc-200">Offline</span>
            </div>
            <ul className="flex flex-col gap-1">
              {["Läuft ohne Internet", "Privacy-First", "488 MB Download"].map((b) => (
                <li key={b} className="text-[11px] text-zinc-500 flex items-start gap-1.5">
                  <span className="text-zinc-600 mt-0.5">•</span>
                  {b}
                </li>
              ))}
            </ul>
          </button>
        ) : (
          <div className="flex flex-col gap-3 p-4 rounded-xl border border-zinc-800/40 bg-[#0e0e10] opacity-50 cursor-not-allowed">
            <div className="flex items-center gap-2 text-zinc-600">
              <ShieldIcon className="w-5 h-5" />
              <span className="text-sm font-semibold text-zinc-500">Offline</span>
            </div>
            <p className="text-[11px] text-zinc-600">Nicht verfügbar auf Android</p>
          </div>
        )}
      </div>

      <p className="text-[11px] text-zinc-600 text-center">Du kannst jederzeit in den Einstellungen wechseln.</p>

      <button
        onClick={onNext}
        disabled={!selected}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Android permission steps (info cards)
// ---------------------------------------------------------------------------

interface PermissionStepProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  settingsHint: string;
  onNext: () => void;
}

function PermissionStep({ icon, title, description, settingsHint, onNext }: PermissionStepProps) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col items-center text-center gap-4">
        <div className="w-14 h-14 rounded-2xl bg-zinc-800/80 border border-zinc-700/60 flex items-center justify-center text-zinc-400">
          {icon}
        </div>
        <div className="flex flex-col gap-1.5">
          <h2 className="text-xl font-semibold text-zinc-100">{title}</h2>
          <p className="text-sm text-zinc-400 leading-relaxed max-w-xs">{description}</p>
        </div>
      </div>

      <div className="rounded-xl bg-zinc-800/40 border border-zinc-700/40 px-4 py-3">
        <p className="text-xs text-zinc-500 leading-relaxed">{settingsHint}</p>
      </div>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Ich habe es erteilt — Weiter
      </button>
    </div>
  );
}

function OverlayIcon() {
  return (
    <svg className="w-7 h-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <rect x="8" y="8" width="8" height="8" rx="1" />
    </svg>
  );
}

function AccessibilityIcon() {
  return (
    <svg className="w-7 h-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="4" r="2" />
      <path d="M5 9h14M12 9v8M8 17h8" />
    </svg>
  );
}

function BatteryIcon() {
  return (
    <svg className="w-7 h-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="7" width="16" height="10" rx="2" />
      <path d="M22 11v2" />
      <path d="M7 11h4" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Step 2: Language selection
// ---------------------------------------------------------------------------

function StepLanguage({ language, onLanguageChange, onNext }: {
  language: string;
  onLanguageChange: (l: string) => void;
  onNext: () => void;
}) {
  // Detect system locale on mount if no language chosen yet
  useEffect(() => {
    if (!language) {
      const locale = navigator.language?.split("-")[0]?.toLowerCase() ?? "de";
      const supported = ["de", "en"];
      onLanguageChange(supported.includes(locale) ? locale : "de");
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Welche Sprache sprichst du?</h2>
        <p className="text-sm text-zinc-400">Du kannst das jederzeit in den Einstellungen ändern.</p>
      </div>

      <div className="flex flex-col gap-2">
        {[
          { value: "de", label: "Deutsch" },
          { value: "en", label: "English" },
          { value: "", label: "Auto-detect" },
        ].map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => onLanguageChange(opt.value)}
            className={[
              "flex items-center justify-between px-4 py-3 rounded-xl border text-sm font-medium transition-all duration-150",
              language === opt.value
                ? "border-emerald-500/50 bg-emerald-500/8 text-emerald-400"
                : "border-zinc-800/60 bg-[#111113] text-zinc-400 hover:border-zinc-700/60 hover:text-zinc-300",
            ].join(" ")}
          >
            {opt.label}
            {language === opt.value && (
              <CheckCircleIcon className="w-4 h-4 text-emerald-400" />
            )}
          </button>
        ))}
      </div>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3a: STT key (cloud path)
// ---------------------------------------------------------------------------

function StepSttKey({ onNext }: { onNext: (groqKey: string) => void }) {
  const [groqKey, setGroqKey] = useState("");
  const [othersOpen, setOthersOpen] = useState(false);
  const groqValidation = useKeyValidation("groq", groqKey);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Spracherkennung einrichten</h2>
        <p className="text-sm text-zinc-400">Dikta nutzt Groq Whisper zum Transkribieren — schnell, mit kostenlosem Free-Tier.</p>
      </div>

      {/* Groq highlighted block */}
      <div className="flex flex-col gap-4 rounded-xl bg-emerald-500/5 border border-emerald-500/20 p-4">
        <div className="flex items-center gap-2">
          <span className="text-emerald-400">
            <KeyIcon className="w-4 h-4" />
          </span>
          <span className="text-sm font-semibold text-zinc-200">Groq</span>
          <span className="text-[10px] font-medium text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded-full px-2 py-0.5">
            empfohlen — kostenloses Free-Tier
          </span>
        </div>
        <ApiKeyField
          label="Groq API Key"
          value={groqKey}
          onChange={setGroqKey}
          placeholder="gsk_..."
          provider="groq"
          costHint="Groq Whisper ist kostenlos nutzbar. Bei intensiver Nutzung kann ein kurzes Limit greifen."
          magicLinkUrl="https://console.groq.com"
          magicLinkLabel="Kostenlosen Key holen"
          validationState={groqValidation.state}
          validationError={groqValidation.error}
          onValidate={groqValidation.validate}
        />
      </div>

      {/* Collapsible other providers */}
      <div className="flex flex-col gap-2">
        <button
          type="button"
          onClick={() => setOthersOpen((v) => !v)}
          className="flex items-center justify-between w-full py-1 text-xs text-zinc-500 hover:text-zinc-300 transition-colors focus:outline-none"
        >
          <span>Andere Provider (OpenAI)</span>
          <svg
            className={`w-3.5 h-3.5 transition-transform duration-200 ${othersOpen ? "rotate-180" : ""}`}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
        {othersOpen && (
          <div className="rounded-xl bg-[#111113] border border-zinc-800/60 p-4 text-xs text-zinc-500">
            OpenAI-Validierung wird in einer kommenden Version unterstützt. Du kannst den Key in den Einstellungen hinterlegen.
          </div>
        )}
      </div>

      <button
        onClick={() => onNext(groqKey.trim())}
        disabled={!groqKey.trim()}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 4a: LLM key (both paths)
// ---------------------------------------------------------------------------

function StepLlmKey({ onNext, onSkip }: { onNext: (deepseekKey: string) => void; onSkip: () => void }) {
  const [deepseekKey, setDeepseekKey] = useState("");
  const [openrouterKey, setOpenrouterKey] = useState("");
  const [useOpenRouter, setUseOpenRouter] = useState(false);
  const dsValidation = useKeyValidation("deepseek", deepseekKey);
  const orValidation = useKeyValidation("openrouter", openrouterKey);

  const activeKey = useOpenRouter ? openrouterKey.trim() : deepseekKey.trim();

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Text-Bereinigung (optional)</h2>
        <p className="text-sm text-zinc-400 leading-relaxed">
          Ein KI-Modell bereinigt rohen Transkript-Text. Optional — ohne Key wird der unbearbeitete Text eingefügt.
        </p>
      </div>

      {/* Tab switcher */}
      <div className="flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60">
        <button
          type="button"
          onClick={() => setUseOpenRouter(false)}
          className={[
            "flex-1 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-100",
            !useOpenRouter ? "bg-emerald-500/15 text-emerald-400" : "text-zinc-500 hover:text-zinc-300",
          ].join(" ")}
        >
          DeepSeek
        </button>
        <button
          type="button"
          onClick={() => setUseOpenRouter(true)}
          className={[
            "flex-1 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-100",
            useOpenRouter ? "bg-emerald-500/15 text-emerald-400" : "text-zinc-500 hover:text-zinc-300",
          ].join(" ")}
        >
          OpenRouter
        </button>
      </div>

      {!useOpenRouter ? (
        <div className="rounded-xl bg-emerald-500/5 border border-emerald-500/20 p-4">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-emerald-400"><KeyIcon className="w-4 h-4" /></span>
            <span className="text-sm font-semibold text-zinc-200">DeepSeek</span>
            <span className="text-[10px] font-medium text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded-full px-2 py-0.5">empfohlen</span>
          </div>
          <ApiKeyField
            label="DeepSeek API Key"
            value={deepseekKey}
            onChange={setDeepseekKey}
            placeholder="sk-..."
            provider="deepseek"
            costHint="~$0.0001–0.0003 pro Diktat. Sehr günstig."
            magicLinkUrl="https://platform.deepseek.com/api_keys"
            magicLinkLabel="Key erstellen"
            validationState={dsValidation.state}
            validationError={dsValidation.error}
            onValidate={dsValidation.validate}
          />
        </div>
      ) : (
        <div className="rounded-xl bg-[#111113] border border-zinc-800/60 p-4">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-zinc-400"><KeyIcon className="w-4 h-4" /></span>
            <span className="text-sm font-semibold text-zinc-200">OpenRouter</span>
          </div>
          <ApiKeyField
            label="OpenRouter API Key"
            value={openrouterKey}
            onChange={setOpenrouterKey}
            placeholder="sk-or-..."
            provider="openrouter"
            costHint="Zugang zu vielen LLM-Modellen über einen einzigen Key."
            magicLinkUrl="https://openrouter.ai/keys"
            magicLinkLabel="Key erstellen"
            validationState={orValidation.state}
            validationError={orValidation.error}
            onValidate={orValidation.validate}
          />
        </div>
      )}

      <button
        onClick={() => onNext(activeKey)}
        disabled={!activeKey}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>

      <button
        onClick={onSkip}
        className="text-sm text-zinc-500 hover:text-zinc-400 transition-colors text-center"
      >
        Überspringen — rohen Text nutzen
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3b: Whisper model download (offline desktop path)
// ---------------------------------------------------------------------------

function StepModelDownload({ onNext }: { onNext: () => void }) {
  const [downloadState, setDownloadState] = useState<"idle" | "downloading" | "done" | "error">("idle");
  const [progress, setProgress] = useState(0); // 0–1
  const [errorMsg, setErrorMsg] = useState("");
  const [llmKey, setLlmKey] = useState("");
  const llmValidation = useKeyValidation("deepseek", llmKey);
  const unlistenRefs = useRef<(() => void)[]>([]);

  // Check if model already downloaded
  useEffect(() => {
    getWhisperModels()
      .then((models) => {
        const small = models.find((m) => m.id === "small");
        if (small?.status === "downloaded") setDownloadState("done");
      })
      .catch(console.error);

    return () => {
      unlistenRefs.current.forEach((fn) => fn());
    };
  }, []);

  const startDownload = useCallback(async () => {
    setDownloadState("downloading");
    setProgress(0);
    setErrorMsg("");

    const unlistenProgress = await onModelDownloadProgress((p) => {
      if (p.modelId === "small" && p.totalBytes > 0) {
        setProgress(p.bytesReceived / p.totalBytes);
      }
    });
    const unlistenComplete = await onModelDownloadComplete((p) => {
      if (p.modelId === "small") setDownloadState("done");
    });
    const unlistenError = await onModelDownloadError((p) => {
      if (p.modelId === "small") {
        setDownloadState("error");
        setErrorMsg(p.error);
      }
    });

    unlistenRefs.current = [unlistenProgress, unlistenComplete, unlistenError];

    try {
      await downloadWhisperModel("small");
    } catch (err) {
      setDownloadState("error");
      setErrorMsg(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const progressPct = Math.round(progress * 100);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Offline-Modell herunterladen</h2>
        <p className="text-sm text-zinc-400">Einmaliger Download — danach laeuft Dikta ohne Internet.</p>
      </div>

      {/* Model card */}
      <div className="rounded-xl bg-[#111113] border border-zinc-800/60 p-4 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-semibold text-zinc-200">Whisper small</p>
            <p className="text-xs text-zinc-500">488 MB — gute Qualität, schnell</p>
          </div>
          {downloadState === "done" && (
            <CheckCircleIcon className="w-5 h-5 text-emerald-400" />
          )}
        </div>

        {downloadState === "downloading" && (
          <div className="flex flex-col gap-1.5">
            <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden">
              <div
                className="h-full bg-emerald-500/70 rounded-full transition-all duration-300"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <p className="text-[11px] text-zinc-500">{progressPct}%</p>
          </div>
        )}

        {downloadState === "error" && (
          <p className="text-xs text-red-400">{errorMsg}</p>
        )}

        {downloadState === "idle" && (
          <button
            onClick={startDownload}
            className="w-full rounded-lg py-2 text-sm font-medium bg-emerald-500/15 border border-emerald-500/30 text-emerald-400 hover:bg-emerald-500/20 transition-all"
          >
            Jetzt herunterladen
          </button>
        )}
        {downloadState === "error" && (
          <button
            onClick={startDownload}
            className="w-full rounded-lg py-2 text-sm font-medium bg-zinc-800/60 border border-zinc-700/60 text-zinc-300 hover:bg-zinc-800 transition-all"
          >
            Erneut versuchen
          </button>
        )}
      </div>

      {/* LLM key input while waiting */}
      {downloadState === "downloading" || downloadState === "done" ? (
        <div className="flex flex-col gap-2">
          <p className="text-xs font-semibold text-zinc-500 uppercase tracking-wide">Während du wartest: LLM-Cleanup (optional)</p>
          <ApiKeyField
            label="DeepSeek API Key"
            value={llmKey}
            onChange={setLlmKey}
            placeholder="sk-..."
            provider="deepseek"
            costHint="~$0.0001–0.0003 pro Diktat"
            magicLinkUrl="https://platform.deepseek.com/api_keys"
            magicLinkLabel="Key erstellen"
            validationState={llmValidation.state}
            validationError={llmValidation.error}
            onValidate={llmValidation.validate}
          />
        </div>
      ) : null}

      <button
        onClick={onNext}
        disabled={downloadState !== "done"}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 5: Test dictation
// ---------------------------------------------------------------------------

type TestState = "idle" | "recording" | "transcribing" | "cleaning" | "done" | "error";

function StepTestDictation({ language, cleanupStyle, onNext }: {
  language: string;
  cleanupStyle: string;
  onNext: () => void;
}) {
  const [testState, setTestState] = useState<TestState>("idle");
  const [resultText, setResultText] = useState("");
  const [errorMsg, setErrorMsg] = useState("");
  const [hasDone, setHasDone] = useState(false);

  const handleRecord = useCallback(async () => {
    if (testState === "done" || testState === "error") {
      setTestState("idle");
      setResultText("");
      setErrorMsg("");
      return;
    }

    if (testState === "recording") {
      // Stop recording
      try {
        setTestState("transcribing");
        let transcript: string;
        if (isMobile) {
          const wavBytes = await stopBrowserRecording();
          const { transcribeAudioBytes: tab } = await import("./tauri-commands");
          transcript = await tab(Array.from(wavBytes), language);
        } else {
          await stopRecording();
          transcript = await transcribeAudio(language);
        }
        setTestState("cleaning");
        const cleaned = await cleanupText(transcript, cleanupStyle as "polished" | "verbatim" | "chat");
        setResultText(cleaned);
        setTestState("done");
        setHasDone(true);
      } catch (err) {
        setErrorMsg(err instanceof Error ? err.message : String(err));
        setTestState("error");
      }
      return;
    }

    // Start recording
    setResultText("");
    setErrorMsg("");
    try {
      if (isMobile) {
        await startBrowserRecording();
      } else {
        await startRecording();
      }
      setTestState("recording");
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setTestState("error");
    }
  }, [testState, language, cleanupStyle]);

  const isRecording = testState === "recording";
  const isBusy = testState === "transcribing" || testState === "cleaning";

  const statusText: Record<TestState, string> = {
    idle: "Drücke den Button um dein erstes Diktat zu starten",
    recording: "Aufnahme laeuft... Drücke erneut um zu stoppen",
    transcribing: "Transkribiere...",
    cleaning: "Bereinige Text...",
    done: "Fertig! So wird dein Text eingefügt.",
    error: errorMsg,
  };

  // Android: show info screen instead of record button
  if (isMobile) {
    return (
      <div className="flex flex-col gap-6">
        <div className="flex flex-col gap-1">
          <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Probiere es aus!</h2>
          <p className="text-sm text-zinc-400">Das Diktat läuft über die schwebende Blase.</p>
        </div>

        <div className="rounded-xl bg-[#111113] border border-zinc-800/60 p-5 flex flex-col items-center gap-4 text-center">
          <div className="w-14 h-14 rounded-2xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400">
            <MicIconSm className="w-7 h-7" />
          </div>
          <div className="flex flex-col gap-1.5">
            <p className="text-sm font-semibold text-zinc-200">Tippe auf die schwebende Blase</p>
            <p className="text-xs text-zinc-500 leading-relaxed max-w-[220px]">
              Die Blase erscheint über anderen Apps und startet das Diktat mit einem Tipp.
            </p>
          </div>
        </div>

        <button onClick={onNext} className={BTN_PRIMARY}>
          Weiter
        </button>
        <button onClick={onNext} className="text-sm text-zinc-500 hover:text-zinc-400 transition-colors text-center">
          Später ausprobieren
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-zinc-100 tracking-tight">Probiere es aus!</h2>
        <p className="text-sm text-zinc-400">Starte eine Aufnahme und sprich etwas.</p>
      </div>

      <div className="flex flex-col items-center gap-4">
        {/* Record button */}
        <button
          onClick={handleRecord}
          disabled={isBusy}
          className={[
            "w-20 h-20 rounded-full relative flex items-center justify-center",
            "transition-all duration-200 focus:outline-none",
            "disabled:cursor-not-allowed disabled:opacity-60",
            isRecording
              ? "bg-red-500/20 text-red-400 shadow-[0_0_40px_rgba(239,68,68,0.3)]"
              : isBusy
              ? "bg-amber-500/15 text-amber-400"
              : "bg-emerald-500/15 text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.2)] hover:bg-emerald-500/20",
          ].join(" ")}
        >
          <span className={[
            "absolute inset-0 rounded-full border-2 transition-colors",
            isRecording ? "border-red-500/40" : isBusy ? "border-amber-500/30" : "border-emerald-500/25",
          ].join(" ")} />
          {isRecording && (
            <span className="absolute inset-0 rounded-full border-2 border-red-400 opacity-40 animate-ping" />
          )}
          {isBusy ? (
            <SpinnerIcon className="w-8 h-8" />
          ) : isRecording ? (
            <StopIcon className="w-8 h-8" />
          ) : (
            <MicIconSm className="w-8 h-8" />
          )}
        </button>

        <p className={[
          "text-xs font-medium text-center max-w-xs",
          testState === "error" ? "text-red-400" : testState === "done" ? "text-emerald-400" : isBusy ? "text-amber-400" : "text-zinc-500",
        ].join(" ")}>
          {statusText[testState]}
        </p>

        {resultText && (
          <textarea
            readOnly
            value={resultText}
            rows={3}
            className="w-full bg-[#111113] border border-zinc-800/60 rounded-xl px-3.5 py-2.5 text-sm text-zinc-200 resize-none focus:outline-none"
          />
        )}
      </div>

      {isDesktop && (
        <div className="rounded-xl bg-zinc-800/30 border border-zinc-700/30 px-4 py-3">
          <p className="text-xs text-zinc-500">
            Im Alltag: Drücke <kbd className="inline-flex items-center px-1.5 py-0.5 rounded bg-zinc-700 border border-zinc-600 text-[11px] font-mono text-zinc-300">Ctrl+Shift+D</kbd> zum Diktieren — Dikta fügt den Text direkt ein.
          </p>
        </div>
      )}

      <button onClick={onNext} disabled={!hasDone} className={BTN_PRIMARY}>
        Weiter
      </button>
      <button onClick={onNext} className="text-sm text-zinc-500 hover:text-zinc-400 transition-colors text-center">
        Später ausprobieren
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 6: Done
// ---------------------------------------------------------------------------

function StepDone({ mode, language, hasLlm, onFinish }: {
  mode: WizardMode;
  language: string;
  hasLlm: boolean;
  onFinish: () => void;
}) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col items-center text-center gap-5">
        {/* Animated checkmark */}
        <div className="w-16 h-16 rounded-full bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.18)]">
          <svg className="w-8 h-8" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ strokeDasharray: 40, strokeDashoffset: 0 }}>
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </div>
        <div className="flex flex-col gap-1.5">
          <h2 className="text-2xl font-bold text-zinc-100">Du bist startklar!</h2>
          <p className="text-sm text-zinc-400">Alles eingerichtet. Zeit zu diktieren.</p>
        </div>
      </div>

      {/* Summary */}
      <div className="rounded-xl bg-[#111113] border border-zinc-800/60 p-4 flex flex-col gap-2.5">
        <SummaryRow label="Modus" value={mode === "offline" ? "Offline (Whisper small)" : "Cloud (Groq Whisper)"} />
        <SummaryRow label="Sprache" value={language === "de" ? "Deutsch" : language === "en" ? "English" : "Auto-detect"} />
        <SummaryRow label="LLM-Cleanup" value={hasLlm ? "Aktiv" : "Inaktiv (roher Text)"} positive={hasLlm} />
      </div>

      <button onClick={onFinish} className={BTN_PRIMARY}>
        Dikta starten
      </button>
    </div>
  );
}

function SummaryRow({ label, value, positive }: { label: string; value: string; positive?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-xs text-zinc-500">{label}</span>
      <span className={`text-xs font-medium ${positive === false ? "text-zinc-600" : positive === true ? "text-emerald-400" : "text-zinc-300"}`}>
        {value}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Onboarding component
// ---------------------------------------------------------------------------

export interface OnboardingProps {
  onComplete: (settings: AppSettings) => void;
  initialState?: OnboardingState;
}

export default function Onboarding({ onComplete, initialState }: OnboardingProps) {
  const [mode, setMode] = useState<WizardMode>((initialState?.mode as WizardMode) ?? "");
  const [language, setLanguage] = useState(initialState?.language ?? "");
  const [collectedGroqKey, setCollectedGroqKey] = useState("");
  const [collectedDeepseekKey, setCollectedDeepseekKey] = useState("");
  const [hasLlm, setHasLlm] = useState(false);

  // Visible step index in the *current* step list
  const stepList = buildStepList(mode);
  const [stepIndex, setStepIndex] = useState(() => {
    if (!initialState?.currentStep) return 0;
    // Try to find the matching step index; fallback to 0
    return Math.min(initialState.currentStep, stepList.length - 1);
  });

  // Transition animation
  const [visible, setVisible] = useState(true);

  // Persist state on every step change
  const persist = useCallback(
    async (overrides: Partial<OnboardingState> = {}) => {
      const state: OnboardingState = {
        completed: false,
        skipped: false,
        currentStep: stepIndex,
        mode,
        language,
        ...overrides,
      };
      await setOnboardingState(state).catch(console.error);
    },
    [stepIndex, mode, language],
  );

  const advance = useCallback(
    (override?: Partial<OnboardingState>) => {
      setVisible(false);
      setTimeout(() => {
        setStepIndex((i) => {
          const next = i + 1;
          persist({ currentStep: next, ...override }).catch(console.error);
          return next;
        });
        setVisible(true);
      }, 120);
    },
    [persist],
  );

  const handleSkip = useCallback(async () => {
    await setOnboardingState({
      completed: true,
      skipped: true,
      currentStep: stepIndex,
      mode,
      language,
    }).catch(console.error);
    // We still need to close the wizard — save minimal settings then call onComplete
    try {
      await saveSettings("", "", language || "de", "polished");
      const updated = await getSettings();
      onComplete(updated);
    } catch {
      onComplete({} as AppSettings);
    }
  }, [stepIndex, mode, language, onComplete]);

  const handleModeSelect = useCallback(
    (m: WizardMode) => {
      setMode(m);
      persist({ mode: m }).catch(console.error);
    },
    [persist],
  );

  const handleLanguageChange = useCallback(
    (l: string) => {
      setLanguage(l);
      persist({ language: l }).catch(console.error);
    },
    [persist],
  );

  const handleSttKeyNext = useCallback(
    (key: string) => {
      setCollectedGroqKey(key);
      advance();
    },
    [advance],
  );

  const handleLlmKeyNext = useCallback(
    (key: string) => {
      setCollectedDeepseekKey(key);
      setHasLlm(!!key);
      advance();
    },
    [advance],
  );

  const handleLlmKeySkip = useCallback(() => {
    setHasLlm(false);
    advance();
  }, [advance]);

  const handleFinish = useCallback(async () => {
    try {
      await saveSettings(
        collectedGroqKey,
        collectedDeepseekKey,
        language || "de",
        "polished",
        "ctrl+shift+d",
        "hold",
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        mode === "offline" ? "local" : "groq",
        collectedDeepseekKey ? "deepseek" : null,
      );
      await setOnboardingState({
        completed: true,
        skipped: false,
        currentStep: stepList.length - 1,
        mode,
        language,
      });
      const updated = await getSettings();
      onComplete(updated);
    } catch (err) {
      console.error("Failed to save onboarding settings:", err);
      onComplete({} as AppSettings);
    }
  }, [collectedGroqKey, collectedDeepseekKey, language, mode, onComplete, stepList.length]);

  // Rebuild step list when mode changes (so index stays valid)
  const newStepList = buildStepList(mode);
  const clampedIndex = Math.min(stepIndex, newStepList.length - 1);
  const effectiveStepList = newStepList;
  const effectiveStepId = effectiveStepList[clampedIndex] as StepId;

  const totalSteps = effectiveStepList.length;

  return (
    <div
      className="min-h-screen bg-[#09090b] flex flex-col items-center justify-center px-6 py-8"
      style={{
        fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
        ...(isMobile ? { paddingBottom: "env(safe-area-inset-bottom, 40px)" } : {}),
      }}
    >
      <div
        className={[
          "w-full max-w-sm flex flex-col gap-6",
          "transition-all duration-150",
          visible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-1",
        ].join(" ")}
      >
        {/* Header row: step dots + skip */}
        <div className="flex items-center justify-between">
          <StepDots current={clampedIndex} total={totalSteps} />
          {effectiveStepId !== "welcome" && effectiveStepId !== "done" && (
            <button
              onClick={handleSkip}
              className="text-xs text-zinc-600 hover:text-zinc-400 transition-colors"
            >
              Überspringen
            </button>
          )}
        </div>

        {/* Step content */}
        {effectiveStepId === "welcome" && (
          <StepWelcome onNext={() => advance()} onSkip={handleSkip} />
        )}

        {effectiveStepId === "mode" && (
          <StepMode
            selected={mode}
            onSelect={handleModeSelect}
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "perm-overlay" && (
          <PermissionStep
            icon={<OverlayIcon />}
            title="Overlay-Berechtigung"
            description="Dikta braucht Overlay-Berechtigung um über anderen Apps zu erscheinen und den Diktat-Button anzuzeigen."
            settingsHint="Gehe zu: Einstellungen → Apps → Dikta → Spezielle App-Zugriffe → Ueber anderen Apps anzeigen → Aktivieren"
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "perm-mic" && (
          <PermissionStep
            icon={<MicIconSm className="w-7 h-7" />}
            title="Mikrofon-Berechtigung"
            description="Dikta braucht Zugriff auf dein Mikrofon um Sprache aufzunehmen."
            settingsHint="Gehe zu: Einstellungen → Apps → Dikta → Berechtigungen → Mikrofon → Erlauben"
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "perm-accessibility" && (
          <PermissionStep
            icon={<AccessibilityIcon />}
            title="Bedienungshilfen"
            description="Dikta nutzt Bedienungshilfen um Text direkt in das aktive Textfeld einzufügen — ohne Zwischenablage."
            settingsHint="Gehe zu: Einstellungen → Bedienungshilfen → Heruntergeladene Apps → Dikta → Aktivieren"
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "perm-battery" && (
          <PermissionStep
            icon={<BatteryIcon />}
            title="Akku-Optimierung"
            description="Verhindert, dass Android Dikta im Hintergrund stoppt und den Diktat-Button unsichtbar macht."
            settingsHint="Gehe zu: Einstellungen → Akku → Akku-Optimierung → Dikta → Nicht optimieren"
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "language" && (
          <StepLanguage
            language={language}
            onLanguageChange={handleLanguageChange}
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "stt-key" && (
          <StepSttKey onNext={handleSttKeyNext} />
        )}

        {effectiveStepId === "model-download" && (
          <StepModelDownload onNext={() => advance()} />
        )}

        {effectiveStepId === "llm-key" && (
          <StepLlmKey onNext={handleLlmKeyNext} onSkip={handleLlmKeySkip} />
        )}

        {effectiveStepId === "test-dictation" && (
          <StepTestDictation
            language={language || "de"}
            cleanupStyle={hasLlm ? "polished" : "verbatim"}
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "done" && (
          <StepDone
            mode={mode || "cloud"}
            language={language}
            hasLlm={hasLlm}
            onFinish={handleFinish}
          />
        )}
      </div>
    </div>
  );
}
