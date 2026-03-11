/**
 * First-run onboarding wizard.
 *
 * Shown once when no API keys are configured. Guides the user through:
 *   Step 1 - Welcome
 *   Step 2 - API key setup (Groq recommended, others optional)
 *   Step 3 - Ready / usage instructions
 */
import { useState, useCallback } from "react";
import { saveSettings, getSettings } from "./tauri-commands";

/** Opens a URL in the system browser. No-op in preview mode (no Tauri runtime). */
async function openExternalUrl(url: string): Promise<void> {
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    // In preview mode the plugin is not available -- fall back to window.open.
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
import type { AppSettings } from "./types";

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

function MicIcon() {
  return (
    <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function ChevronDownIcon({ open: isOpen }: { open: boolean }) {
  return (
    <svg
      className={`w-4 h-4 transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
    >
      <path d="M6 9l6 6 6-6" />
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

function KeyIcon() {
  return (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="15" r="4" />
      <path d="m21 3-9.4 9.4M15 9l2 2" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Step indicator
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
// Input field
// ---------------------------------------------------------------------------

function ApiKeyInput({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-zinc-400 font-medium">{label}</label>
      <input
        type="password"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? "sk-..."}
        autoComplete="off"
        spellCheck={false}
        className={[
          "bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2",
          "text-sm text-zinc-200 font-mono placeholder:text-zinc-600",
          "focus:outline-none focus:border-emerald-500/40 focus:ring-1 focus:ring-emerald-500/20",
          "transition-colors duration-150",
        ].join(" ")}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1: Welcome
// ---------------------------------------------------------------------------

function StepWelcome({ onNext }: { onNext: () => void }) {
  return (
    <div className="flex flex-col items-center text-center gap-6">
      {/* Logo mark */}
      <div className="w-16 h-16 rounded-2xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.12)]">
        <MicIcon />
      </div>

      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold text-zinc-100 tracking-tight">
          Welcome to Dikta
        </h1>
        <p className="text-sm text-zinc-400 leading-relaxed max-w-xs">
          Free voice dictation with AI text cleanup. Speak naturally — Dikta
          transcribes and polishes your text, then pastes it wherever your
          cursor is.
        </p>
      </div>

      <button
        onClick={onNext}
        className={[
          "w-full rounded-xl py-2.5 px-6 text-sm font-medium",
          "bg-emerald-500/15 border border-emerald-500/30 text-emerald-400",
          "hover:bg-emerald-500/20 hover:border-emerald-500/40",
          "transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40",
        ].join(" ")}
      >
        Get Started
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2: API key setup
// ---------------------------------------------------------------------------

function StepApiKeys({
  groqKey,
  onGroqKey,
  openaiKey,
  onOpenaiKey,
  deepseekKey,
  onDeepseekKey,
  anthropicKey,
  onAnthropicKey,
  onNext,
}: {
  groqKey: string;
  onGroqKey: (v: string) => void;
  openaiKey: string;
  onOpenaiKey: (v: string) => void;
  deepseekKey: string;
  onDeepseekKey: (v: string) => void;
  anthropicKey: string;
  onAnthropicKey: (v: string) => void;
  onNext: () => void;
}) {
  const [othersOpen, setOthersOpen] = useState(false);

  const hasKey = groqKey.trim() || openaiKey.trim() || deepseekKey.trim() || anthropicKey.trim();

  const handleGroqLink = useCallback(() => {
    openExternalUrl("https://console.groq.com").catch(console.error);
  }, []);

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold text-zinc-100 tracking-tight">
          Connect an API Provider
        </h2>
        <p className="text-sm text-zinc-400 leading-relaxed">
          Dikta needs an API key for speech-to-text and text cleanup. We
          recommend Groq — it's free and fast.
        </p>
      </div>

      {/* Groq block (highlighted) */}
      <div className="flex flex-col gap-3 rounded-xl bg-emerald-500/5 border border-emerald-500/20 p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-emerald-400">
              <KeyIcon />
            </span>
            <span className="text-sm font-medium text-zinc-200">
              Groq
            </span>
            <span className="text-[10px] font-medium text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded-full px-2 py-0.5">
              recommended — free tier
            </span>
          </div>
          <button
            type="button"
            onClick={handleGroqLink}
            className="flex items-center gap-1 text-xs text-emerald-400 hover:text-emerald-300 transition-colors duration-100"
          >
            Get your free key
            <ExternalLinkIcon />
          </button>
        </div>
        <ApiKeyInput
          label="Groq API Key"
          value={groqKey}
          onChange={onGroqKey}
          placeholder="gsk_..."
        />
      </div>

      {/* Collapsible: other providers */}
      <div className="flex flex-col gap-0">
        <button
          type="button"
          onClick={() => setOthersOpen((v) => !v)}
          className="flex items-center justify-between w-full py-1.5 text-xs text-zinc-500 hover:text-zinc-300 transition-colors duration-100 focus:outline-none"
        >
          <span>Other providers</span>
          <ChevronDownIcon open={othersOpen} />
        </button>

        {othersOpen && (
          <div className="flex flex-col gap-3 mt-2 rounded-xl bg-[#111113] border border-zinc-800/60 p-4">
            <ApiKeyInput
              label="OpenAI API Key"
              value={openaiKey}
              onChange={onOpenaiKey}
              placeholder="sk-..."
            />
            <ApiKeyInput
              label="DeepSeek API Key"
              value={deepseekKey}
              onChange={onDeepseekKey}
              placeholder="sk-..."
            />
            <ApiKeyInput
              label="Anthropic API Key"
              value={anthropicKey}
              onChange={onAnthropicKey}
              placeholder="sk-ant-..."
            />
          </div>
        )}
      </div>

      <button
        onClick={onNext}
        disabled={!hasKey}
        className={[
          "w-full rounded-xl py-2.5 px-6 text-sm font-medium",
          "bg-emerald-500/15 border border-emerald-500/30 text-emerald-400",
          "hover:bg-emerald-500/20 hover:border-emerald-500/40",
          "disabled:opacity-40 disabled:cursor-not-allowed",
          "transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40",
        ].join(" ")}
      >
        Continue
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3: Ready
// ---------------------------------------------------------------------------

function InstructionRow({ keys, description }: { keys: string[]; description: string }) {
  return (
    <div className="flex items-center gap-3">
      <div className="flex items-center gap-1 flex-shrink-0">
        {keys.map((k, i) => (
          <span key={i}>
            <kbd className="inline-flex items-center px-1.5 py-0.5 rounded-md bg-zinc-800 border border-zinc-700/60 text-[11px] font-mono text-zinc-300 shadow-sm">
              {k}
            </kbd>
            {i < keys.length - 1 && (
              <span className="text-zinc-600 text-xs mx-0.5">+</span>
            )}
          </span>
        ))}
      </div>
      <span className="text-sm text-zinc-400">{description}</span>
    </div>
  );
}

function StepReady({ onComplete, saving }: { onComplete: () => void; saving: boolean }) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold text-zinc-100 tracking-tight">
          You're all set!
        </h2>
        <p className="text-sm text-zinc-400">
          Here's how to use Dikta.
        </p>
      </div>

      <div className="flex flex-col gap-4 rounded-xl bg-[#111113] border border-zinc-800/60 p-4">
        <InstructionRow
          keys={["Ctrl", "Shift", "D"]}
          description="Hold to start dictating"
        />
        <div className="border-t border-zinc-800/60" />
        <InstructionRow
          keys={["Release"]}
          description="Transcribe and paste automatically"
        />
        <div className="border-t border-zinc-800/60" />
        <div className="flex items-center gap-3">
          <div className="w-7 h-7 flex-shrink-0 rounded-md bg-zinc-800 border border-zinc-700/60 flex items-center justify-center">
            <span className="text-zinc-400">
              <MicIcon />
            </span>
          </div>
          <span className="text-sm text-zinc-400">
            Click the tray icon to open settings
          </span>
        </div>
      </div>

      <button
        onClick={onComplete}
        disabled={saving}
        className={[
          "w-full rounded-xl py-2.5 px-6 text-sm font-medium",
          "bg-emerald-500/15 border border-emerald-500/30 text-emerald-400",
          "hover:bg-emerald-500/20 hover:border-emerald-500/40",
          "disabled:opacity-50 disabled:cursor-not-allowed",
          "transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40",
        ].join(" ")}
      >
        {saving ? "Saving..." : "Start Dikta"}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export interface OnboardingProps {
  onComplete: (settings: AppSettings) => void;
}

export default function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState(0);
  const [visible, setVisible] = useState(true);

  // Key state
  const [groqKey, setGroqKey] = useState("");
  const [openaiKey, setOpenaiKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [saving, setSaving] = useState(false);

  const advance = useCallback(() => {
    // Brief fade-out / fade-in between steps
    setVisible(false);
    setTimeout(() => {
      setStep((s) => s + 1);
      setVisible(true);
    }, 120);
  }, []);

  const handleComplete = useCallback(async () => {
    setSaving(true);
    try {
      await saveSettings(
        groqKey.trim(),
        deepseekKey.trim(),
        "de",           // sensible default language
        "polished",     // default cleanup style
        "ctrl+shift+d", // default hotkey
        "hold",         // default hotkey mode
        null,           // audioDevice
        null,           // sttModel
        null,           // customPrompt
        null,           // autostart
        null,           // whisperMode
        openaiKey.trim() || null,
        anthropicKey.trim() || null,
        null,           // sttPriority
        null,           // llmPriority
      );
      const updated = await getSettings();
      onComplete(updated);
    } catch (err) {
      console.error("Failed to save onboarding settings:", err);
      setSaving(false);
    }
  }, [groqKey, openaiKey, deepseekKey, anthropicKey, onComplete]);

  const TOTAL_STEPS = 3;

  return (
    <div
      className="h-screen bg-[#09090b] flex flex-col items-center justify-center px-6"
      style={{ fontFamily: "'Inter', system-ui, -apple-system, sans-serif" }}
    >
      {/* Card */}
      <div
        className={[
          "w-full max-w-sm flex flex-col gap-6",
          "transition-all duration-150",
          visible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-1",
        ].join(" ")}
      >
        {/* Step dots */}
        <div className="flex justify-center">
          <StepDots current={step} total={TOTAL_STEPS} />
        </div>

        {/* Step content */}
        {step === 0 && <StepWelcome onNext={advance} />}

        {step === 1 && (
          <StepApiKeys
            groqKey={groqKey}
            onGroqKey={setGroqKey}
            openaiKey={openaiKey}
            onOpenaiKey={setOpenaiKey}
            deepseekKey={deepseekKey}
            onDeepseekKey={setDeepseekKey}
            anthropicKey={anthropicKey}
            onAnthropicKey={setAnthropicKey}
            onNext={advance}
          />
        )}

        {step === 2 && (
          <StepReady onComplete={handleComplete} saving={saving} />
        )}
      </div>
    </div>
  );
}
