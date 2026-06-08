import { useState, useEffect, useCallback } from "react";
import type { AdvancedSettings } from "../types";
import { getAdvancedSettings, saveAdvancedSettings } from "../tauri-commands";
import { CloseIcon, SpinnerIcon, LockIcon } from "./icons";
import { applyUiScale } from "../hooks/useUiScale";
import { INPUT_CLS, LABEL_CLS } from "./ui";
import { isMobile } from "../platform";
import { MobileTextarea } from "./MobileTextarea";

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
  uiScale: "medium",
  expertMode: false,
};

type ActiveSection = "home" | "stt" | "llm" | "audio" | "system";

interface AdvancedSettingsPanelProps {
  onClose?: () => void;
  isPaid: boolean;
  isTrial?: boolean;
  /** When true: no outer container/shadow and no header are rendered.
   *  Use this when embedding inside another panel (e.g. SettingsPanel). */
  embedded?: boolean;
}

export function AdvancedSettingsPanel({ onClose, isPaid, isTrial = false, embedded = false }: AdvancedSettingsPanelProps) {
  const [settings, setSettings] = useState<AdvancedSettings>(ADVANCED_DEFAULTS);
  const [loadedSettings, setLoadedSettings] = useState<AdvancedSettings>(ADVANCED_DEFAULTS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<ActiveSection>("home");
  // Expert mode (settings.expertMode) gates raw internal tuning knobs (audio thresholds,
  // chunking, STT temperature). It is a persisted AdvancedSettings field, so toggling it
  // behaves like any other setting: it marks the panel dirty (→ Save button) and survives
  // navigation + app restart. Off by default; footguns stay hidden until deliberately enabled.
  const expertMode = settings.expertMode;
  // Subsections within the LLM section: free subsections open by default, paid ones closed.
  const [openSubSections, setOpenSubSections] = useState<Record<string, boolean>>({
    llmParams: true,   // "Model & Parameters" -- free, default open
    llmCustom: false,  // "Custom Cleanup Instructions" -- paid, default closed
  });

  const hintCls = "text-[11px] text-klarvo-muted leading-relaxed";
  const numberInputCls = `${INPUT_CLS} w-28`;
  const modelInputCls = "bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-3 py-2 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-klarvo-primary/40 transition-colors w-44";

  // Independent toggle -- no accordion behavior, multiple subsections can be open at once.
  const toggleSubSection = useCallback((key: string) => {
    setOpenSubSections((prev) => ({ ...prev, [key]: !prev[key] }));
  }, []);

  useEffect(() => {
    getAdvancedSettings()
      .then((s) => { setSettings(s); setLoadedSettings(s); setLoaded(true); })
      .catch((err) => {
        console.warn("get_advanced_settings failed, using defaults:", err);
        setLoaded(true);
      });
  }, []);

  // Escape key handling:
  // - In detail view: go back to home
  // - In home view (standalone mode only): close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (activeSection !== "home") {
        e.preventDefault();
        setActiveSection("home");
      } else if (!embedded && onClose) {
        onClose();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose, embedded, activeSection]);

  const set = useCallback(<K extends keyof AdvancedSettings>(key: K, value: AdvancedSettings[K]) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }, []);

  const isDirty = JSON.stringify(settings) !== JSON.stringify(loadedSettings);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveMsg(null);
    try {
      await saveAdvancedSettings(settings);
      setLoadedSettings({ ...settings });
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [settings]);

  if (!loaded) {
    return (
      <div className={embedded ? "p-4 text-center" : "w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl p-6 text-center"}>
        <SpinnerIcon className="w-5 h-5 text-klarvo-dim mx-auto" />
      </div>
    );
  }

  const TrialBadge = () => (
    <span className="text-[10px] font-semibold uppercase tracking-wider bg-klarvo-primary/15 text-klarvo-primary px-1.5 py-0.5 rounded border border-klarvo-primary/25">
      Trial
    </span>
  );

  // On Android the system nav bar (~48 px) overlaps the WebView bottom edge.
  // The panel needs flex-col so the footer stays below the scroll area, and the
  // scroll area must leave enough room for the footer + nav bar clearance.
  const scrollMaxH = isMobile ? "max-h-[calc(100vh-230px)]" : "max-h-[calc(100vh-150px)]";

  const outerCls = embedded
    ? "flex flex-col"
    : `w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col ${isMobile ? "max-h-[calc(100vh-168px)]" : "max-h-[calc(100vh-120px)]"}`;

  // Inline chevron-right SVG (used for home row navigation indicator)
  const ChevronRight = () => (
    <svg className="w-4 h-4 text-klarvo-dim shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M9 18l6-6-6-6" />
    </svg>
  );

  // Inline chevron-left SVG (used in detail view back button)
  const ChevronLeft = () => (
    <svg className="w-5 h-5 text-klarvo-primary" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M15 18l-6-6 6-6" />
    </svg>
  );

  const minHeight = isMobile ? "min-h-[60px]" : "min-h-[52px]";
  const verticalPadding = isMobile ? "py-4" : "py-3.5";

  // Section titles used in the detail view header
  const SECTION_TITLES: Record<Exclude<ActiveSection, "home">, string> = {
    stt: "Speech-to-Text",
    llm: "Text Cleanup",
    audio: "Audio",
    system: "System",
  };

  // --- Home view: 4 navigation rows ---
  const renderHome = () => (
    <div className="flex flex-col">
      {/* Speech-to-Text */}
      <button
        type="button"
        onClick={() => setActiveSection("stt")}
        className={`w-full flex items-center gap-3 px-5 ${verticalPadding} ${minHeight} text-left hover:bg-klarvo-surface/40 transition-colors`}
      >
        <span
          className="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
          style={{ backgroundColor: "#14b8a626", color: "#14b8a6" }}
        >
          {/* Microphone icon */}
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
          </svg>
        </span>
        <span className="flex-1 flex flex-col min-w-0">
          <span className="text-sm font-medium text-klarvo-text leading-tight">Speech-to-Text</span>
          <span className="text-xs text-klarvo-muted mt-0.5 leading-tight">Custom prompts &amp; temperature</span>
        </span>
        <span className="flex items-center gap-2 shrink-0">
          {isPaid && isTrial && <TrialBadge />}
          <ChevronRight />
        </span>
      </button>

      {/* Text Cleanup */}
      <button
        type="button"
        onClick={() => setActiveSection("llm")}
        className={`w-full flex items-center gap-3 px-5 ${verticalPadding} ${minHeight} text-left hover:bg-klarvo-surface/40 transition-colors`}
      >
        <span
          className="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
          style={{ backgroundColor: "#8b5cf626", color: "#8b5cf6" }}
        >
          {/* Sparkles / AI icon */}
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5z" />
            <path d="M2 17l10 5 10-5" />
            <path d="M2 12l10 5 10-5" />
          </svg>
        </span>
        <span className="flex-1 flex flex-col min-w-0">
          <span className="text-sm font-medium text-klarvo-text leading-tight">Text Cleanup</span>
          <span className="text-xs text-klarvo-muted mt-0.5 leading-tight">Models, parameters &amp; instructions</span>
        </span>
        <span className="flex items-center gap-2 shrink-0">
          {isPaid && isTrial && <TrialBadge />}
          <ChevronRight />
        </span>
      </button>

      {/* Audio -- entirely raw VAD tuning knobs; only surfaced in expert mode */}
      {expertMode && (
      <button
        type="button"
        onClick={() => setActiveSection("audio")}
        className={`w-full flex items-center gap-3 px-5 ${verticalPadding} ${minHeight} text-left hover:bg-klarvo-surface/40 transition-colors`}
      >
        <span
          className="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
          style={{ backgroundColor: "#f59e0b26", color: "#f59e0b" }}
        >
          {/* Waveform icon */}
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
          </svg>
        </span>
        <span className="flex-1 flex flex-col min-w-0">
          <span className="text-sm font-medium text-klarvo-text leading-tight">Audio</span>
          <span className="text-xs text-klarvo-muted mt-0.5 leading-tight">Thresholds &amp; recording settings</span>
        </span>
        <span className="flex items-center gap-2 shrink-0">
          <ChevronRight />
        </span>
      </button>
      )}

      {/* System */}
      <button
        type="button"
        onClick={() => setActiveSection("system")}
        className={`w-full flex items-center gap-3 px-5 ${verticalPadding} ${minHeight} text-left hover:bg-klarvo-surface/40 transition-colors`}
      >
        <span
          className="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
          style={{ backgroundColor: "#6b728026", color: "#6b7280" }}
        >
          {/* Terminal / system icon */}
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
        </span>
        <span className="flex-1 flex flex-col min-w-0">
          <span className="text-sm font-medium text-klarvo-text leading-tight">System</span>
          <span className="text-xs text-klarvo-muted mt-0.5 leading-tight">Logging &amp; diagnostics</span>
        </span>
        <span className="flex items-center gap-2 shrink-0">
          <ChevronRight />
        </span>
      </button>
    </div>
  );

  // --- Detail view: sub-page header + section content ---
  const renderDetailHeader = (section: Exclude<ActiveSection, "home">) => (
    <div className="flex items-center gap-3 px-4 py-3 border-b border-klarvo-border/50 h-12 shrink-0">
      <button
        type="button"
        onClick={() => setActiveSection("home")}
        aria-label="Back to advanced settings"
        className="w-8 h-8 rounded-lg flex items-center justify-center hover:bg-klarvo-surface/50 transition-colors"
      >
        <ChevronLeft />
      </button>
      <span className="flex-1 text-base font-semibold text-klarvo-text text-center">
        {SECTION_TITLES[section]}
      </span>
      {/* Spacer to keep title visually centred */}
      <div className="w-8" />
    </div>
  );

  const renderSttContent = () => (
    <div className="flex flex-col gap-3 p-4">
      <div className={`flex flex-col gap-3${!isPaid ? " opacity-50" : ""}`}>
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] font-semibold text-klarvo-primary/85 uppercase tracking-widest">Custom STT Prompts</span>
          {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
          {isPaid && isTrial && <TrialBadge />}
        </div>
        <div className="flex flex-col gap-1.5">
          <span className={LABEL_CLS}>STT Prompt (German)</span>
          <MobileTextarea label="STT Prompt (German)" hint="Injected as context when language is set to German." value={settings.sttPromptDe} onChange={isPaid ? (v) => set("sttPromptDe", v) : () => {}} placeholder={isPaid ? "Context prompt sent with German transcriptions" : "Requires Klarvo License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
          <span className={hintCls}>Injected as context when language is set to German.</span>
        </div>
        <div className="flex flex-col gap-1.5">
          <span className={LABEL_CLS}>STT Prompt (English)</span>
          <MobileTextarea label="STT Prompt (English)" hint="Injected as context when language is set to English." value={settings.sttPromptEn} onChange={isPaid ? (v) => set("sttPromptEn", v) : () => {}} placeholder={isPaid ? "Context prompt for English transcriptions" : "Requires Klarvo License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
          <span className={hintCls}>Injected as context when language is set to English.</span>
        </div>
        <div className="flex flex-col gap-1.5">
          <span className={LABEL_CLS}>STT Prompt (Auto-detect)</span>
          <MobileTextarea label="STT Prompt (Auto-detect)" hint="Used when language is set to Auto (DE + EN)." value={settings.sttPromptAuto} onChange={isPaid ? (v) => set("sttPromptAuto", v) : () => {}} placeholder={isPaid ? "Context prompt for auto-detect mode" : "Requires Klarvo License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
          <span className={hintCls}>Used when language is set to Auto (DE + EN).</span>
        </div>
        {expertMode && (
          <div className={`flex items-center justify-between gap-3${!isPaid ? " pointer-events-none" : ""}`}>
            <div className="flex flex-col gap-0.5">
              <span className={LABEL_CLS}>STT Temperature</span>
              <span className={hintCls}>0.0 = deterministic, 1.0 = more creative. Default: 0.0</span>
            </div>
            <input type="number" min={0} max={1} step={0.1} value={settings.sttTemperature} onChange={(e) => { if (isPaid) set("sttTemperature", parseFloat(e.target.value) || 0); }} disabled={!isPaid} className={numberInputCls} />
          </div>
        )}
      </div>
    </div>
  );

  const renderLlmContent = () => (
    <div className="flex flex-col gap-1 p-4">
      {/* Subsection: Model & Parameters -- free, default open */}
      <button
        onClick={() => toggleSubSection("llmParams")}
        className="flex items-center gap-1.5 w-full py-1.5 text-left"
      >
        <svg
          className={`w-3 h-3 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSubSections.llmParams ? "rotate-90" : ""}`}
          viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        <span className="text-[11px] font-semibold text-klarvo-primary/85 uppercase tracking-widest">Model &amp; Parameters</span>
      </button>
      {openSubSections.llmParams && (
        <div className="flex flex-col gap-3 pl-3 pb-2 pt-0.5 border-l border-klarvo-border/50 ml-1.5">
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>LLM Temperature</span><span className={hintCls}>0.0 – 2.0. Lower = more focused.</span></div>
            <input type="number" min={0} max={2} step={0.1} value={settings.llmTemperature} onChange={(e) => set("llmTemperature", parseFloat(e.target.value) || 0)} className={numberInputCls} />
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Max Tokens</span><span className={hintCls}>Maximum output tokens per LLM request.</span></div>
            <input type="number" min={64} max={8192} step={1} value={settings.llmMaxTokens} onChange={(e) => set("llmMaxTokens", parseInt(e.target.value, 10) || 1024)} className={numberInputCls} />
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Model: DeepSeek</span><span className={hintCls}>Model ID sent to the DeepSeek API.</span></div>
            <input type="text" placeholder="deepseek-chat" value={settings.llmModelDeepseek} onChange={(e) => set("llmModelDeepseek", e.target.value)} className={modelInputCls} />
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Model: OpenAI</span><span className={hintCls}>Model ID sent to the OpenAI API.</span></div>
            <input type="text" placeholder="gpt-4o-mini" value={settings.llmModelOpenai} onChange={(e) => set("llmModelOpenai", e.target.value)} className={modelInputCls} />
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Model: Anthropic</span><span className={hintCls}>Model ID sent to the Anthropic API.</span></div>
            <input type="text" placeholder="claude-haiku-4-5-20251001" value={settings.llmModelAnthropic} onChange={(e) => set("llmModelAnthropic", e.target.value)} className={modelInputCls} />
          </div>
          <div className="flex items-center justify-between gap-3">
            <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Model: Groq</span><span className={hintCls}>Model ID sent to the Groq LLM API.</span></div>
            <input type="text" placeholder="llama-3.3-70b-versatile" value={settings.llmModelGroq} onChange={(e) => set("llmModelGroq", e.target.value)} className={modelInputCls} />
          </div>
          {expertMode && (
            <>
              <div className="flex items-center justify-between gap-3">
                <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Chunk Threshold</span><span className={hintCls}>Word count above which text is split into parallel chunks.</span></div>
                <input type="number" min={50} step={1} value={settings.chunkThreshold} onChange={(e) => set("chunkThreshold", parseInt(e.target.value, 10) || 400)} className={numberInputCls} />
              </div>
              <div className="flex items-center justify-between gap-3">
                <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Chunk Target Size</span><span className={hintCls}>Target word count per chunk.</span></div>
                <input type="number" min={50} step={1} value={settings.chunkTargetSize} onChange={(e) => set("chunkTargetSize", parseInt(e.target.value, 10) || 300)} className={numberInputCls} />
              </div>
            </>
          )}
        </div>
      )}

      {/* Subsection: Custom Cleanup Instructions -- paid, default collapsed */}
      <button
        onClick={() => toggleSubSection("llmCustom")}
        className="flex items-center gap-1.5 w-full py-1.5 text-left mt-1"
      >
        <svg
          className={`w-3 h-3 text-klarvo-dim flex-shrink-0 transition-transform duration-150 ${openSubSections.llmCustom ? "rotate-90" : ""}`}
          viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        <span className="flex items-center gap-1.5 text-[11px] font-semibold text-klarvo-primary/85 uppercase tracking-widest">
          Custom Cleanup Instructions
          {!isPaid && <LockIcon className="w-3 h-3 text-klarvo-dim" />}
          {isPaid && isTrial && <TrialBadge />}
        </span>
      </button>
      {openSubSections.llmCustom && (
        <div className={`flex flex-col gap-3 pl-3 pb-2 pt-0.5 border-l border-klarvo-border/50 ml-1.5${!isPaid ? " opacity-50" : ""}`}>
          <p className={hintCls}>
            Base system prompt for each cleanup style. Your "Cleanup Instructions" from Settings are appended on top -- they stack, not conflict.
          </p>
          <div className={`flex flex-col gap-3${!isPaid ? " pointer-events-none" : ""}`}>
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>System Prompt: Polished</span>
              <MobileTextarea label="System Prompt: Polished" hint="Overrides the built-in system prompt for Polished mode." value={settings.llmSystemPromptPolished} onChange={isPaid ? (v) => set("llmSystemPromptPolished", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Klarvo License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
              <span className={hintCls}>Overrides the built-in system prompt for Polished mode.</span>
            </div>
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>System Prompt: Verbatim</span>
              <MobileTextarea label="System Prompt: Verbatim" hint="Overrides the built-in system prompt for Verbatim mode." value={settings.llmSystemPromptVerbatim} onChange={isPaid ? (v) => set("llmSystemPromptVerbatim", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Klarvo License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
              <span className={hintCls}>Overrides the built-in system prompt for Verbatim mode.</span>
            </div>
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>System Prompt: Chat</span>
              <MobileTextarea label="System Prompt: Chat" hint="Overrides the built-in system prompt for Chat mode." value={settings.llmSystemPromptChat} onChange={isPaid ? (v) => set("llmSystemPromptChat", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Klarvo License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
              <span className={hintCls}>Overrides the built-in system prompt for Chat mode.</span>
            </div>
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>Command Mode Prompt</span>
              <MobileTextarea label="Command Mode Prompt" hint="System prompt for Command Mode (Ctrl+Shift+E)." value={settings.llmCommandModePrompt} onChange={isPaid ? (v) => set("llmCommandModePrompt", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Klarvo License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
              <span className={hintCls}>System prompt for Command Mode (Ctrl+Shift+E).</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );

  const renderAudioContent = () => (
    <div className="flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Silence Threshold</span><span className={hintCls}>RMS below which audio is silence (0.0 – 0.1).</span></div>
        <input type="number" min={0} max={0.1} step={0.001} value={settings.silenceThreshold} onChange={(e) => set("silenceThreshold", parseFloat(e.target.value) || 0)} className={numberInputCls} />
      </div>
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Whisper Mode Threshold</span><span className={hintCls}>Silence threshold in Whisper Mode (lower than normal).</span></div>
        <input type="number" min={0} max={0.1} step={0.001} value={settings.whisperModeThreshold} onChange={(e) => set("whisperModeThreshold", parseFloat(e.target.value) || 0)} className={numberInputCls} />
      </div>
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Min Recording (ms)</span><span className={hintCls}>Shorter recordings are discarded.</span></div>
        <input type="number" min={0} step={50} value={settings.minRecordingMs} onChange={(e) => set("minRecordingMs", parseInt(e.target.value, 10) || 500)} className={numberInputCls} />
      </div>
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Whisper Mode Gain</span><span className={hintCls}>Amplification multiplier in Whisper Mode.</span></div>
        <input type="number" min={1} max={20} step={0.5} value={settings.whisperModeGain} onChange={(e) => set("whisperModeGain", parseFloat(e.target.value) || 1)} className={numberInputCls} />
      </div>
    </div>
  );

  const renderSystemContent = () => {
    const scaleOptions: { value: string; label: string }[] = [
      { value: "small", label: "S" },
      { value: "medium", label: "M" },
      { value: "large", label: "L" },
    ];
    return (
      <div className="flex flex-col gap-3 p-4">
        {/* UI Scale */}
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-0.5">
            <span className={LABEL_CLS}>UI Scale</span>
            <span className={hintCls}>Controls the overall size of text and UI elements.</span>
          </div>
          <div className="flex gap-0.5 bg-klarvo-bg border border-klarvo-border/60 rounded-lg p-0.5">
            {scaleOptions.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => { set("uiScale", opt.value); applyUiScale(opt.value); }}
                className={[
                  "w-8 h-7 rounded-md text-xs font-semibold transition-all duration-100",
                  settings.uiScale === opt.value
                    ? "bg-klarvo-primary/15 text-klarvo-primary"
                    : "text-klarvo-dim hover:text-klarvo-muted",
                ].join(" ")}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
        {/* Log Level */}
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Log Level</span><span className={hintCls}>Use "debug" when troubleshooting.</span></div>
          <select value={settings.logLevel} onChange={(e) => set("logLevel", e.target.value)} className="bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-2 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer">
            <option value="debug">debug</option>
            <option value="info">info</option>
            <option value="warn">warn</option>
            <option value="error">error</option>
          </select>
        </div>
        {/* Expert mode -- reveals raw internal tuning knobs (audio thresholds, chunking, STT temperature) */}
        <div className="flex items-center justify-between gap-3 pt-3 mt-1 border-t border-klarvo-border/40">
          <div className="flex flex-col gap-0.5">
            <span className={LABEL_CLS}>Expert mode</span>
            <span className={hintCls}>Reveals raw audio thresholds, chunking and STT temperature. Wrong values can stop recording or transcription — only enable if you know what they do.</span>
          </div>
          <button
            role="switch"
            aria-checked={expertMode}
            onClick={() => set("expertMode", !expertMode)}
            className={[
              "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
              expertMode ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
            ].join(" ")}
          >
            <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", expertMode ? "translate-x-4" : ""].join(" ")} />
          </button>
        </div>
      </div>
    );
  };

  const renderSectionContent = () => {
    switch (activeSection) {
      case "stt": return renderSttContent();
      case "llm": return renderLlmContent();
      case "audio": return renderAudioContent();
      case "system": return renderSystemContent();
      default: return null;
    }
  };

  return (
    <div className={outerCls}>
      {/* Standalone mode header (home view only) */}
      {!embedded && onClose && activeSection === "home" && (
        <div className="flex items-center justify-between px-4 py-3 border-b border-klarvo-border/40 flex-shrink-0">
          <span className="text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest">Advanced Settings</span>
          <button aria-label="Close" onClick={onClose} className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50">
            <CloseIcon />
          </button>
        </div>
      )}

      {/* Detail view header (back button + centred title) */}
      {activeSection !== "home" && renderDetailHeader(activeSection)}

      {/* Scrollable content area */}
      <div className={`overflow-y-auto flex-1 min-h-0 ${embedded ? "" : scrollMaxH}`}>
        {activeSection === "home" ? renderHome() : renderSectionContent()}
      </div>

      {/* Save footer -- only shown when there are unsaved changes */}
      {isDirty && (
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
                : "bg-klarvo-primary/10 border-klarvo-primary/20 text-klarvo-primary hover:bg-klarvo-primary/15 hover:border-klarvo-primary/30",
              "disabled:opacity-50 disabled:cursor-not-allowed",
            ].join(" ")}
          >
            {saving ? "Saving..." : saveMsg ?? "Save"}
          </button>
        </div>
      )}
    </div>
  );
}
