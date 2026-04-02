/**
 * Onboarding wizard.
 *
 * Full state-machine wizard that guides new users through setup.
 * Persists progress via setOnboardingState() after every step transition.
 *
 * Flow (desktop cloud, expert):
 *   0 Welcome → 1 Mode+Gate → 2 STT Key Expert → 3 Language → 4 Test → 5 Done
 *
 * Flow (desktop cloud, beginner):
 *   0 Welcome → 1 Mode+Gate → 2a-c Beginner Steps → 3 Language → 4 Test → 5 Done
 *
 * Flow (desktop offline):
 *   0 Welcome → 1 Mode → 2 Model Download → 3 Language → 4 Test → 5 Done
 *
 * Flow (android cloud):
 *   0 Welcome → 1 Mode+Gate → 1a perm-all → 2 STT Key → 3 Language → 4 Test → 5 Done
 */
import React, { useState, useCallback, useEffect, useRef } from "react";
import { isMobile, isDesktop } from "./platform";
import { isPreviewMode } from "./tauri-commands";
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
type SttTrack = "expert" | "beginner" | "";

// We compute a flat ordered list of step IDs at runtime based on mode + platform.
type StepId =
  | "welcome"
  | "mode"
  | "perm-all"
  | "stt-key-expert"
  | "stt-key-beginner-1"
  | "stt-key-beginner-2"
  | "model-download"
  | "test-dictation"
  | "language"
  | "done";

function buildStepList(mode: WizardMode, track: SttTrack): StepId[] {
  const base: StepId[] = ["welcome", "mode"];

  if (mode === "offline") {
    // Offline: no gate, no key
    if (isMobile) base.push("perm-all");
    if (isDesktop) base.push("model-download");
    base.push("language", "test-dictation", "done");
    return base;
  }

  // Cloud path
  if (isMobile) base.push("perm-all");

  if (track === "expert") {
    base.push("stt-key-expert");
  } else if (track === "beginner") {
    base.push("stt-key-beginner-1", "stt-key-beginner-2");
  }
  // else: track="" = gate not answered yet, no stt-steps

  base.push("language", "test-dictation", "done");
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
              ? "w-4 h-1.5 bg-klarvo-primary"
              : i < current
              ? "w-1.5 h-1.5 bg-klarvo-primary/40"
              : "w-1.5 h-1.5 bg-klarvo-elevated",
          ].join(" ")}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Preview mode: override disabled state so all steps are clickable
// ---------------------------------------------------------------------------

/** In preview mode, buttons are never disabled — allows clicking through all steps. */
function previewOr(disabled: boolean): boolean {
  return isPreviewMode ? false : disabled;
}

// ---------------------------------------------------------------------------
// Shared button styles
// ---------------------------------------------------------------------------

const BTN_PRIMARY = [
  "w-full rounded-xl py-2.5 px-6 text-sm font-medium",
  "bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary",
  "hover:bg-klarvo-primary/20 hover:border-klarvo-primary/40",
  "disabled:opacity-40 disabled:cursor-not-allowed",
  "transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-klarvo-primary/40",
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
        <label className="text-xs text-klarvo-muted font-medium">{label}</label>
        {magicLinkUrl && (
          <button
            type="button"
            onClick={() => openExternalUrl(magicLinkUrl).catch(console.error)}
            className="flex items-center gap-1 text-xs text-amber-400 hover:text-amber-300 transition-colors"
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
            "flex-1 bg-klarvo-bg border rounded-lg px-3 py-2",
            "text-sm text-klarvo-text font-mono placeholder:text-klarvo-dim",
            "focus:outline-none focus:border-klarvo-primary/40 focus:ring-1 focus:ring-klarvo-primary/20",
            "transition-colors duration-150",
            validationState === "valid"
              ? "border-klarvo-primary/50"
              : validationState === "invalid"
              ? "border-klarvo-danger/50"
              : "border-klarvo-border/60",
          ].join(" ")}
        />
        {value.trim().length > 0 && (
          <button
            type="button"
            onClick={onValidate}
            disabled={validationState === "loading"}
            className="px-3 py-2 rounded-lg bg-klarvo-surface/80 border border-klarvo-border/60 text-xs text-klarvo-muted hover:text-klarvo-text hover:border-klarvo-border-active transition-all disabled:opacity-50 flex-shrink-0"
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
        <div className="flex items-center gap-1.5 text-xs text-klarvo-primary">
          <CheckCircleIcon className="w-3.5 h-3.5" />
          <span>Key funktioniert</span>
        </div>
      )}
      {validationState === "invalid" && (
        <div className="flex items-center gap-1.5 text-xs text-klarvo-danger">
          <XCircleIcon className="w-3.5 h-3.5" />
          <span>{validationError || `Ungültiger ${provider}-Key`}</span>
        </div>
      )}
      {costHint && validationState === "idle" && (
        <p className="text-[11px] text-klarvo-dim">{costHint}</p>
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
      if (!ok) setError("Key nicht akzeptiert — bitte prüfen ob er vollständig kopiert wurde.");
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

  // Auto-validate 800ms after the user stops typing (only if key is non-empty)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!key.trim()) return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      validate();
    }, 800);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps

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
        className="self-end text-xs text-klarvo-dim hover:text-klarvo-muted transition-colors"
      >
        Ich kenn mich aus →
      </button>

      {/* Animated mic with pulse ring */}
      <div className="relative flex items-center justify-center">
        <span className="absolute w-24 h-24 rounded-full bg-klarvo-primary/10 animate-ping" style={{ animationDuration: "2s" }} />
        <span className="absolute w-20 h-20 rounded-full bg-klarvo-primary/10 animate-ping" style={{ animationDuration: "2s", animationDelay: "0.5s" }} />
        <div className="relative w-16 h-16 rounded-2xl bg-klarvo-primary/15 border border-klarvo-primary/30 flex items-center justify-center text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.18)]">
          <div className="w-8 h-8">
            <MicIconLarge />
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold text-klarvo-text tracking-tight">
          Sprich. Klarvo tippt.
        </h1>
        <p className="text-sm text-klarvo-muted leading-relaxed max-w-xs">
          Freies Sprachdiktat mit KI-Bereinigung. Klarvo transkribiert und bereinigt deinen Text — und fügt ihn direkt ein, wo du gerade schreibst.
        </p>
      </div>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Loslegen
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1: Cloud / Offline mode selection + Gate (cloud path)
// ---------------------------------------------------------------------------

function StepMode({ selected, onSelect, track, onTrackSelect, onNext }: {
  selected: WizardMode;
  onSelect: (m: WizardMode) => void;
  track: SttTrack;
  onTrackSelect: (t: SttTrack) => void;
  onNext: () => void;
}) {
  const isNextDisabled = !selected || (selected === "cloud" && track === "");

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">Wie willst du Klarvo nutzen?</h2>
        <p className="text-sm text-klarvo-muted">Beide Varianten sind kostengünstig — du entscheidest.</p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Cloud card */}
        <button
          type="button"
          onClick={() => onSelect("cloud")}
          className={[
            "flex flex-col gap-3 p-4 rounded-xl border text-left transition-all duration-150",
            selected === "cloud"
              ? "border-klarvo-primary/50 bg-klarvo-primary/8"
              : "border-klarvo-border/60 bg-klarvo-bg hover:border-klarvo-border/60",
          ].join(" ")}
        >
          <div className={`flex items-center gap-2 ${selected === "cloud" ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
            <CloudIcon className="w-5 h-5" />
            <span className="text-sm font-semibold text-klarvo-text">Cloud</span>
            {selected === "cloud" && (
              <span className="ml-auto text-[10px] font-medium text-amber-400 bg-amber-400/10 border border-amber-400/20 rounded-full px-2 py-0.5">
                empfohlen
              </span>
            )}
            {selected !== "cloud" && (
              <span className="ml-auto text-[10px] font-medium text-amber-400/60 bg-amber-400/5 border border-amber-400/20 rounded-full px-2 py-0.5">
                empfohlen
              </span>
            )}
          </div>
          <ul className="flex flex-col gap-1">
            {["Beste Qualität", "API-Key benötigt", "Groq kostenlos (mit Limit)"].map((b) => (
              <li key={b} className="text-[11px] text-klarvo-dim flex items-start gap-1.5">
                <span className="text-klarvo-primary/60 mt-0.5">•</span>
                {b}
              </li>
            ))}
          </ul>
        </button>

        {/* Offline card */}
        {isDesktop ? (
          <button
            type="button"
            onClick={() => { onSelect("offline"); onTrackSelect(""); }}
            className={[
              "flex flex-col gap-3 p-4 rounded-xl border text-left transition-all duration-150",
              selected === "offline"
                ? "border-klarvo-primary/50 bg-klarvo-primary/8"
                : "border-klarvo-border/60 bg-klarvo-bg hover:border-klarvo-border/60",
            ].join(" ")}
          >
            <div className={`flex items-center gap-2 ${selected === "offline" ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
              <ShieldIcon className="w-5 h-5" />
              <span className="text-sm font-semibold text-klarvo-text">Offline</span>
            </div>
            <ul className="flex flex-col gap-1">
              {["Läuft ohne Internet", "Privacy-First", "488 MB Download"].map((b) => (
                <li key={b} className="text-[11px] text-klarvo-dim flex items-start gap-1.5">
                  <span className="text-klarvo-dim mt-0.5">•</span>
                  {b}
                </li>
              ))}
            </ul>
          </button>
        ) : (
          <div className="flex flex-col gap-3 p-4 rounded-xl border border-klarvo-border/40 bg-[#0e0e10] opacity-50 cursor-not-allowed">
            <div className="flex items-center gap-2 text-klarvo-dim">
              <ShieldIcon className="w-5 h-5" />
              <span className="text-sm font-semibold text-klarvo-dim">Offline</span>
            </div>
            <p className="text-[11px] text-klarvo-dim">Nicht verfügbar auf Android</p>
          </div>
        )}
      </div>

      {/* Gate block — only visible when cloud selected */}
      {selected === "cloud" && (
        <div className="flex flex-col gap-2">
          <p className="text-sm text-klarvo-muted mb-1">Klarvo braucht einen Spracherkennungs-Dienst. Wir empfehlen Groq — kostenlos und schnell.</p>
          <button
            type="button"
            onClick={() => onTrackSelect("expert")}
            className={[
              "flex items-center px-4 py-3 rounded-xl border text-sm font-medium text-left transition-all duration-150",
              track === "expert"
                ? "border-klarvo-primary/50 bg-klarvo-primary/8 text-klarvo-primary"
                : "border-klarvo-border/60 bg-klarvo-bg text-klarvo-muted hover:border-klarvo-border/80 hover:text-klarvo-text",
            ].join(" ")}
          >
            Ich habe schon einen Schlüssel
          </button>
          <button
            type="button"
            onClick={() => onTrackSelect("beginner")}
            className={[
              "flex items-center px-4 py-3 rounded-xl border text-sm font-medium text-left transition-all duration-150",
              track === "beginner"
                ? "border-klarvo-primary/50 bg-klarvo-primary/8 text-klarvo-primary"
                : "border-klarvo-border/60 bg-klarvo-bg text-klarvo-muted hover:border-klarvo-border/80 hover:text-klarvo-text",
            ].join(" ")}
          >
            Einrichten — dauert 2 Minuten
          </button>
          {isDesktop && (
            <button
              type="button"
              onClick={() => { onSelect("offline"); onTrackSelect(""); }}
              className="flex items-center px-4 py-3 rounded-xl border border-klarvo-border/60 bg-klarvo-bg text-sm font-medium text-klarvo-muted hover:border-klarvo-border/80 hover:text-klarvo-text text-left transition-all duration-150"
            >
              Ohne Internet nutzen
            </button>
          )}
        </div>
      )}

      <p className="text-[11px] text-klarvo-dim text-center">Du kannst jederzeit in den Einstellungen wechseln.</p>

      <button
        onClick={onNext}
        disabled={previewOr(isNextDisabled)}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Android permission step icons
// ---------------------------------------------------------------------------

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
// Android permission step (consolidated info card)
// ---------------------------------------------------------------------------

interface PermItem {
  Icon: () => React.ReactElement;
  label: string;
  detail: string;
}

const PERM_ITEMS: PermItem[] = [
  {
    Icon: OverlayIcon,
    label: "Overlay-Berechtigung",
    detail: "Diktat-Button über anderen Apps anzeigen",
  },
  {
    Icon: () => <MicIconSm className="w-6 h-6" />,
    label: "Mikrofon",
    detail: "Sprache aufnehmen",
  },
  {
    Icon: AccessibilityIcon,
    label: "Bedienungshilfen",
    detail: "Text direkt ins Textfeld einfügen",
  },
  {
    Icon: BatteryIcon,
    label: "Akku-Optimierung",
    detail: "Klarvo im Hintergrund aktiv halten",
  },
];

function PermAllStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1.5">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">
          Berechtigungen einrichten
        </h2>
        <p className="text-sm text-klarvo-muted leading-relaxed">
          Klarvo braucht ein paar Android-Berechtigungen. Die meisten hast du gerade schon erteilt.
        </p>
      </div>

      <div className="rounded-xl border border-klarvo-border/50 bg-klarvo-surface/40 overflow-hidden">
        {PERM_ITEMS.map(({ Icon, label, detail }, i) => (
          <div
            key={label}
            className={[
              "flex items-start gap-3 px-4 py-3",
              i < PERM_ITEMS.length - 1 ? "border-b border-klarvo-border/40" : "",
            ].join(" ")}
          >
            <span className="text-klarvo-muted mt-0.5 shrink-0"><Icon /></span>
            <div className="flex flex-col gap-0.5">
              <span className="text-sm font-medium text-klarvo-text">{label}</span>
              <span className="text-xs text-klarvo-dim">{detail}</span>
            </div>
          </div>
        ))}
      </div>

      <p className="text-xs text-klarvo-dim text-center leading-relaxed">
        Falls eine Berechtigung fehlt, fragt Android beim nächsten Start.
      </p>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Expert track: single key insertion step
// ---------------------------------------------------------------------------

function StepSttKeyExpert({ onNext }: { onNext: (key: string) => void }) {
  const [groqKey, setGroqKey] = useState("");
  const validation = useKeyValidation("groq", groqKey);
  const [othersOpen, setOthersOpen] = useState(false);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">
          Schlüssel einfügen
        </h2>
        <p className="text-sm text-klarvo-muted">
          Füge deinen Groq API-Key hier ein.
        </p>
      </div>

      <ApiKeyField
        label="Groq API Key"
        value={groqKey}
        onChange={setGroqKey}
        placeholder="gsk_..."
        provider="groq"
        magicLinkUrl="https://console.groq.com/keys"
        magicLinkLabel="Neuen Key erstellen"
        validationState={validation.state}
        validationError={validation.error}
        onValidate={validation.validate}
      />

      {/* Collapsible other providers */}
      <div className="flex flex-col gap-2">
        <button
          type="button"
          onClick={() => setOthersOpen((v) => !v)}
          className="flex items-center justify-between w-full py-1 text-xs text-klarvo-dim hover:text-klarvo-muted transition-colors focus:outline-none"
        >
          <span>Andere Provider (OpenAI)</span>
          <svg className={`w-3.5 h-3.5 transition-transform duration-200 ${othersOpen ? "rotate-180" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
        {othersOpen && (
          <div className="rounded-xl bg-klarvo-bg border border-klarvo-border/60 p-4 text-xs text-klarvo-dim">
            OpenAI und andere Provider kannst du nach dem Setup in den Einstellungen konfigurieren.
          </div>
        )}
      </div>

      <button
        onClick={() => onNext(groqKey.trim())}
        disabled={previewOr(validation.state !== "valid")}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
      {groqKey.trim().length > 0 && validation.state !== "valid" && (
        <p className="text-[11px] text-klarvo-dim text-center">
          {validation.state === "loading" ? "Key wird geprüft..." : "Bitte warten, Key wird geprüft"}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Beginner track: 2 steps (account + key creation combined, then key insertion)
// ---------------------------------------------------------------------------

function StepSttKeyBeginner1({ onNext }: { onNext: () => void }) {
  const [linkClicked, setLinkClicked] = useState(false);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">
          Groq einrichten
        </h2>
        <p className="text-sm text-klarvo-muted leading-relaxed">
          Erstelle ein kostenloses Groq-Konto und hole deinen Schlüssel. Dauert etwa 2 Minuten.
        </p>
      </div>

      {/* Numbered steps */}
      <div className="rounded-xl border border-klarvo-border/50 bg-klarvo-surface/30 overflow-hidden">
        <div className="flex flex-col divide-y divide-klarvo-border/40">
          <div className="flex items-start gap-3 px-4 py-3">
            <span className="flex-shrink-0 w-5 h-5 rounded-full bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary text-[11px] font-bold flex items-center justify-center mt-0.5">1</span>
            <p className="text-sm text-klarvo-text">Öffne die Groq-Seite und melde dich an (Google oder GitHub)</p>
          </div>
          <div className="flex items-start gap-3 px-4 py-3">
            <span className="flex-shrink-0 w-5 h-5 rounded-full bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary text-[11px] font-bold flex items-center justify-center mt-0.5">2</span>
            <p className="text-sm text-klarvo-text">Klicke auf <strong>API Keys</strong> → <strong>Create API Key</strong></p>
          </div>
          <div className="flex items-start gap-3 px-4 py-3">
            <span className="flex-shrink-0 w-5 h-5 rounded-full bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary text-[11px] font-bold flex items-center justify-center mt-0.5">3</span>
            <p className="text-sm text-klarvo-text">Kopiere den Schlüssel (beginnt mit <code className="text-xs bg-klarvo-surface/60 px-1 py-0.5 rounded font-mono">gsk_</code>)</p>
          </div>
        </div>
      </div>

      <button
        onClick={() => {
          setLinkClicked(true);
          openExternalUrl("https://console.groq.com").catch(console.error);
        }}
        className="w-full rounded-xl py-2.5 px-6 text-sm font-medium bg-klarvo-surface/60 border border-klarvo-border/60 text-klarvo-text hover:bg-klarvo-surface/80 transition-all flex items-center justify-center gap-2"
      >
        {linkClicked ? (
          <>
            <CheckCircleIcon className="w-4 h-4 text-klarvo-primary" />
            <span>Groq-Seite geöffnet</span>
          </>
        ) : (
          "Groq-Seite öffnen"
        )}
      </button>

      <button onClick={onNext} disabled={previewOr(!linkClicked)} className={BTN_PRIMARY}>
        Schlüssel kopiert — weiter
      </button>

      {!linkClicked && (
        <button
          onClick={onNext}
          className="text-xs text-amber-400/70 hover:text-amber-400 transition-colors text-center"
        >
          Schon erledigt? Weiter →
        </button>
      )}
    </div>
  );
}

function StepSttKeyBeginner2({ onNext }: { onNext: (key: string) => void }) {
  const [groqKey, setGroqKey] = useState("");
  const validation = useKeyValidation("groq", groqKey);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">
          Schlüssel einfügen
        </h2>
        <p className="text-sm text-klarvo-muted">
          Füge den kopierten Schlüssel hier ein.
        </p>
      </div>

      <ApiKeyField
        label="Groq API Key"
        value={groqKey}
        onChange={setGroqKey}
        placeholder="gsk_..."
        provider="groq"
        validationState={validation.state}
        validationError={validation.error}
        onValidate={validation.validate}
      />

      <button
        onClick={() => onNext(groqKey.trim())}
        disabled={previewOr(validation.state !== "valid")}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
      {groqKey.trim().length > 0 && validation.state !== "valid" && (
        <p className="text-[11px] text-klarvo-dim text-center">
          {validation.state === "loading" ? "Key wird geprüft..." : "Bitte warten, Key wird geprüft"}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Whisper model download (offline desktop path)
// ---------------------------------------------------------------------------

function StepModelDownload({ onNext }: { onNext: () => void }) {
  const [downloadState, setDownloadState] = useState<"idle" | "downloading" | "done" | "error">("idle");
  const [progress, setProgress] = useState(0); // 0–1
  const [errorMsg, setErrorMsg] = useState("");
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
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">Offline-Modell herunterladen</h2>
        <p className="text-sm text-klarvo-muted">Einmaliger Download — danach läuft Klarvo ohne Internet.</p>
      </div>

      {/* Model card */}
      <div className="rounded-xl bg-klarvo-bg border border-klarvo-border/60 p-4 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-semibold text-klarvo-text">Whisper small</p>
            <p className="text-xs text-klarvo-dim">488 MB — gute Qualität, schnell</p>
          </div>
          {downloadState === "done" && (
            <CheckCircleIcon className="w-5 h-5 text-klarvo-primary" />
          )}
        </div>

        {downloadState === "downloading" && (
          <div className="flex flex-col gap-1.5">
            <div className="h-1.5 bg-klarvo-surface rounded-full overflow-hidden">
              <div
                className="h-full bg-klarvo-primary/70 rounded-full transition-all duration-300"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <p className="text-[11px] text-klarvo-dim">{progressPct}%</p>
          </div>
        )}

        {downloadState === "error" && (
          <p className="text-xs text-klarvo-danger">{errorMsg}</p>
        )}

        {downloadState === "idle" && (
          <button
            onClick={startDownload}
            className="w-full rounded-lg py-2 text-sm font-medium bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary hover:bg-klarvo-primary/20 transition-all"
          >
            Jetzt herunterladen
          </button>
        )}
        {downloadState === "error" && (
          <button
            onClick={startDownload}
            className="w-full rounded-lg py-2 text-sm font-medium bg-klarvo-surface/60 border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface transition-all"
          >
            Erneut versuchen
          </button>
        )}
      </div>

      <button
        onClick={onNext}
        disabled={previewOr(downloadState !== "done")}
        className={BTN_PRIMARY}
      >
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Test dictation step
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
    recording: "Aufnahme läuft... Drücke erneut um zu stoppen",
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
          <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">So funktioniert das Diktat</h2>
          <p className="text-sm text-klarvo-muted">Tippe auf die schwebende Blase über deinem Bildschirm um ein Diktat zu starten.</p>
        </div>

        <div className="rounded-xl bg-klarvo-bg border border-klarvo-border/60 p-5 flex flex-col items-center gap-4 text-center">
          <div className="w-14 h-14 rounded-2xl bg-klarvo-primary/10 border border-klarvo-primary/20 flex items-center justify-center text-klarvo-primary">
            <MicIconSm className="w-7 h-7" />
          </div>
          <div className="flex flex-col gap-1.5">
            <p className="text-sm font-semibold text-klarvo-text">Tippe auf die schwebende Blase</p>
            <p className="text-xs text-klarvo-dim leading-relaxed max-w-[220px]">
              Die Blase erscheint über anderen Apps und startet das Diktat mit einem Tipp.
            </p>
          </div>
        </div>

        <button onClick={onNext} className={BTN_PRIMARY}>
          Weiter
        </button>
        <button onClick={onNext} className="text-sm text-klarvo-dim hover:text-klarvo-muted transition-colors text-center">
          Später ausprobieren
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">Probiere es aus!</h2>
        <p className="text-sm text-klarvo-muted">Starte eine Aufnahme und sprich etwas.</p>
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
              ? "bg-klarvo-danger/20 text-klarvo-danger shadow-[0_0_40px_rgba(255,115,105,0.3)]"
              : isBusy
              ? "bg-klarvo-warning/15 text-klarvo-warning"
              : "bg-klarvo-primary/15 text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.2)] hover:bg-klarvo-primary/20",
          ].join(" ")}
        >
          <span className={[
            "absolute inset-0 rounded-full border-2 transition-colors",
            isRecording ? "border-klarvo-danger/40" : isBusy ? "border-klarvo-warning/30" : "border-klarvo-primary/25",
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
          testState === "error" ? "text-klarvo-danger" : testState === "done" ? "text-klarvo-primary" : isBusy ? "text-klarvo-warning" : "text-klarvo-dim",
        ].join(" ")}>
          {statusText[testState]}
        </p>

        {resultText && (
          <textarea
            readOnly
            value={resultText}
            rows={3}
            className="w-full bg-klarvo-bg border border-klarvo-border/60 rounded-xl px-3.5 py-2.5 text-sm text-klarvo-text resize-none focus:outline-none"
          />
        )}
      </div>

      {isDesktop && (
        <div className="rounded-xl bg-klarvo-surface/30 border border-klarvo-border/30 px-4 py-3">
          <p className="text-xs text-klarvo-dim">
            Im Alltag: Drücke <kbd className="inline-flex items-center px-1.5 py-0.5 rounded bg-klarvo-elevated border border-klarvo-border-active text-[11px] font-mono text-klarvo-muted">Ctrl+Shift+D</kbd> zum Diktieren — Klarvo fügt den Text direkt ein.
          </p>
        </div>
      )}

      <button onClick={onNext} disabled={previewOr(!hasDone)} className={BTN_PRIMARY}>
        Weiter
      </button>
      <button onClick={onNext} className="text-sm text-klarvo-dim hover:text-klarvo-muted transition-colors text-center">
        Später ausprobieren
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Language selection step
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
        <h2 className="text-xl font-semibold text-klarvo-text tracking-tight">Welche Sprache sprichst du?</h2>
        <p className="text-sm text-klarvo-muted">Du kannst das jederzeit in den Einstellungen ändern.</p>
      </div>

      <div className="flex flex-col gap-2">
        {[
          { value: "de", label: "Deutsch" },
          { value: "en", label: "English" },
          { value: "", label: "Automatisch erkennen" },
        ].map((opt) => (
          <button
            key={opt.value}
            type="button"
            onClick={() => onLanguageChange(opt.value)}
            className={[
              "flex items-center justify-between px-4 py-3 rounded-xl border text-sm font-medium transition-all duration-150",
              language === opt.value
                ? "border-klarvo-primary/50 bg-klarvo-primary/8 text-klarvo-primary"
                : "border-klarvo-border/60 bg-klarvo-bg text-klarvo-muted hover:border-klarvo-border/60 hover:text-klarvo-muted",
            ].join(" ")}
          >
            {opt.label}
            {language === opt.value && (
              <CheckCircleIcon className="w-4 h-4 text-klarvo-primary" />
            )}
          </button>
        ))}
        {language === "" && (
          <p className="text-[11px] text-klarvo-dim">Empfohlen wenn du in mehreren Sprachen diktierst.</p>
        )}
      </div>

      <button onClick={onNext} className={BTN_PRIMARY}>
        Weiter
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Done step
// ---------------------------------------------------------------------------

function SummaryRow({ label, value, positive }: { label: string; value: string; positive?: boolean }) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-xs text-klarvo-dim">{label}</span>
      <span className={`text-xs font-medium ${positive === false ? "text-klarvo-dim" : positive === true ? "text-klarvo-primary" : "text-klarvo-muted"}`}>
        {value}
      </span>
    </div>
  );
}

function StepDone({ mode, language, onFinish }: {
  mode: WizardMode;
  language: string;
  onFinish: () => void;
}) {
  // Cloud always has LLM active — Groq-Llama default kicks in on the backend
  const isCloud = mode !== "offline";
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col items-center text-center gap-5">
        {/* Animated checkmark */}
        <div className="w-16 h-16 rounded-full bg-klarvo-primary/15 border border-klarvo-primary/30 flex items-center justify-center text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.18)]">
          <svg className="w-8 h-8" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ strokeDasharray: 40, strokeDashoffset: 0 }}>
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </div>
        <div className="flex flex-col gap-1.5">
          <h2 className="text-2xl font-bold text-klarvo-text">Du bist startklar!</h2>
          <p className="text-sm text-klarvo-muted">Alles eingerichtet. Zeit zu diktieren.</p>
        </div>
      </div>

      {/* Summary */}
      <div className="rounded-xl bg-klarvo-bg border border-klarvo-border/60 p-4 flex flex-col gap-2.5">
        <SummaryRow label="Modus" value={mode === "offline" ? "Offline (Whisper small)" : "Cloud (Groq Whisper)"} />
        <SummaryRow label="Sprache" value={language === "de" ? "Deutsch" : language === "en" ? "English" : language || "Automatisch erkennen"} />
        {isCloud && <SummaryRow label="Text-Cleanup" value="Aktiv (KI-Textbereinigung)" positive={true} />}
      </div>

      <button onClick={onFinish} className={BTN_PRIMARY}>Los geht's</button>
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
  const [track, setTrack] = useState<SttTrack>((initialState?.track as SttTrack) ?? "");
  const [collectedGroqKey, setCollectedGroqKey] = useState("");

  // Visible step index in the *current* step list
  const stepList = buildStepList(mode, track);
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
        track,
        ...overrides,
      };
      await setOnboardingState(state).catch(console.error);
    },
    [stepIndex, mode, language, track],
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

  const goBack = useCallback(() => {
    setVisible(false);
    setTimeout(() => {
      setStepIndex((i) => {
        const prev = Math.max(0, i - 1);
        persist({ currentStep: prev }).catch(console.error);
        return prev;
      });
      setVisible(true);
    }, 120);
  }, [persist]);

  const handleSkip = useCallback(async () => {
    // Persist the skip decision first -- must not silently fail or the wizard
    // will reappear on next launch.
    try {
      await setOnboardingState({
        completed: true,
        skipped: true,
        currentStep: stepIndex,
        mode,
        language,
        track,
      });
    } catch (err) {
      console.error("[onboarding] Failed to persist skip state:", err);
    }
    // Don't call saveSettings here -- it sends hotkey="" which fails backend
    // validation and is redundant anyway (defaults are already correct).
    // Just fetch current settings for the onComplete callback.
    try {
      const updated = await getSettings();
      onComplete(updated);
    } catch {
      onComplete({} as AppSettings);
    }
  }, [stepIndex, mode, language, track, onComplete]);

  const handleModeSelect = useCallback(
    (m: WizardMode) => {
      setMode(m);
      persist({ mode: m }).catch(console.error);
    },
    [persist],
  );

  const handleTrackSelect = useCallback(
    (t: SttTrack) => {
      setTrack(t);
      persist({ track: t }).catch(console.error);
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

  const handleFinish = useCallback(async () => {
    try {
      await saveSettings(
        collectedGroqKey,
        "",                    // deepseekApiKey — empty (Groq-Llama default kicks in on backend)
        language || "de",
        "polished",
        "ctrl+shift+d",
        "hold",
        null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null,
        mode === "offline" ? "local" : "groq",     // sttProvider
        "groq",                                     // llmProvider — always groq (Groq-Llama default)
      );
      await setOnboardingState({
        completed: true,
        skipped: false,
        currentStep: stepList.length - 1,
        mode,
        language,
        track,
      });
      const updated = await getSettings();
      onComplete(updated);
    } catch (err) {
      console.error("Failed to save onboarding settings:", err);
      onComplete({} as AppSettings);
    }
  }, [collectedGroqKey, language, mode, track, onComplete, stepList.length]);

  // Rebuild step list when mode or track changes (so index stays valid)
  const newStepList = buildStepList(mode, track);
  const clampedIndex = Math.min(stepIndex, newStepList.length - 1);
  const effectiveStepList = newStepList;
  const effectiveStepId = effectiveStepList[clampedIndex] as StepId;

  // Save collected API keys to the backend before the test step runs,
  // so the pipeline has valid credentials when the user tries a dictation.
  // Parameters 5+ are optional and default to null (= keep existing value).
  useEffect(() => {
    if (effectiveStepId !== "test-dictation") return;
    if (!collectedGroqKey) return;
    saveSettings(
      collectedGroqKey,                            // 1: groqApiKey
      "",                                          // 2: deepseekApiKey empty
      language || "de",                            // 3: language
      "polished",                                  // 4: cleanupStyle
      "ctrl+shift+d",                              // 5: hotkey (safe default)
      "hold",                                      // 6: hotkeyMode
      null,                                        // 7: audioDevice
      null,                                        // 8: sttModel
      null,                                        // 9: customPrompt
      null,                                        // 10: autostart
      null,                                        // 11: whisperMode
      null,                                        // 12: openaiApiKey
      null,                                        // 13: anthropicApiKey
      null,                                        // 14: openrouterApiKey
      null,                                        // 15: sttPriority (deprecated)
      null,                                        // 16: llmPriority (deprecated)
      null,                                        // 17: outputLanguage
      null,                                        // 18: webhookUrl
      null,                                        // 19: tursoUrl
      null,                                        // 20: tursoToken
      null,                                        // 21: bubbleSize
      null,                                        // 22: bubbleOpacity
      null,                                        // 23: localWhisperModel
      null,                                        // 24: localWhisperGpu
      mode === "offline" ? "local" : "groq",       // 25: sttProvider
      "groq",                                      // 26: llmProvider always groq
    ).catch((err) => console.error("[onboarding] Failed to pre-save keys for test:", err));
  }, [effectiveStepId, collectedGroqKey, language, mode]);

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
        {/* Header row: back + step dots + skip */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {effectiveStepId !== "welcome" && effectiveStepId !== "done" && (
              <button
                onClick={goBack}
                className="text-xs text-klarvo-dim hover:text-klarvo-muted transition-colors flex items-center gap-1"
              >
                <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M15 18l-6-6 6-6" />
                </svg>
                Zurück
              </button>
            )}
            {!effectiveStepId.startsWith("stt-key-beginner") && (
              <StepDots current={clampedIndex} total={totalSteps} />
            )}
            {effectiveStepId.startsWith("stt-key-beginner") && (
              <p className="text-xs text-klarvo-dim">
                Schritt{" "}
                {effectiveStepId === "stt-key-beginner-1" ? "1" : "2"}{" "}
                von 2
              </p>
            )}
          </div>
          {effectiveStepId !== "welcome" && effectiveStepId !== "done" && (
            <button
              onClick={handleSkip}
              className="text-xs text-amber-400/60 hover:text-amber-400 transition-colors"
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
            track={track}
            onTrackSelect={handleTrackSelect}
            onNext={() => advance()}
          />
        )}

        {effectiveStepId === "perm-all" && (
          <PermAllStep onNext={() => advance()} />
        )}

        {effectiveStepId === "stt-key-expert" && (
          <StepSttKeyExpert onNext={handleSttKeyNext} />
        )}

        {effectiveStepId === "stt-key-beginner-1" && (
          <StepSttKeyBeginner1 onNext={() => advance()} />
        )}

        {effectiveStepId === "stt-key-beginner-2" && (
          <StepSttKeyBeginner2 onNext={handleSttKeyNext} />
        )}

        {effectiveStepId === "model-download" && (
          <StepModelDownload onNext={() => advance()} />
        )}

        {effectiveStepId === "test-dictation" && (
          <StepTestDictation
            language={language || "de"}
            cleanupStyle="polished"
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

        {effectiveStepId === "done" && (
          <StepDone
            mode={mode || "cloud"}
            language={language}
            onFinish={handleFinish}
          />
        )}
      </div>
    </div>
  );
}
