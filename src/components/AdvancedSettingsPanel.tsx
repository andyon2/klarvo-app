import { useState, useEffect, useCallback } from "react";
import type { AdvancedSettings } from "../types";
import { getAdvancedSettings, saveAdvancedSettings } from "../tauri-commands";
import { CloseIcon, SpinnerIcon, LockIcon } from "./icons";
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
};

interface AdvancedSettingsPanelProps {
  onClose: () => void;
  isPaid: boolean;
}

export function AdvancedSettingsPanel({ onClose, isPaid }: AdvancedSettingsPanelProps) {
  const [settings, setSettings] = useState<AdvancedSettings>(ADVANCED_DEFAULTS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  // All sections collapsed by default -- user expands what they need.
  const [openSections, setOpenSections] = useState<Record<string, boolean>>({});
  // Subsections within sections: free subsections open by default, paid ones closed.
  const [openSubSections, setOpenSubSections] = useState<Record<string, boolean>>({
    llmParams: true,   // "Model & Parameters" -- free, default open
    llmCustom: false,  // "Custom Cleanup Instructions" -- paid, default closed
  });

  const hintCls = "text-[11px] text-zinc-500 leading-relaxed";
  const numberInputCls = `${INPUT_CLS} w-28`;
  const modelInputCls = "bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors w-44";
  // Larger section title text (text-sm instead of text-[11px]) for better readability.
  const sectionBtnCls = "flex items-center gap-2 w-full py-2 text-left";

  const toggleSection = useCallback((key: string) => {
    setOpenSections((prev) => {
      const wasOpen = prev[key];
      return wasOpen ? {} : { [key]: true };
    });
  }, []);

  // Independent toggle -- no accordion behavior, multiple subsections can be open at once.
  const toggleSubSection = useCallback((key: string) => {
    setOpenSubSections((prev) => ({ ...prev, [key]: !prev[key] }));
  }, []);

  useEffect(() => {
    getAdvancedSettings()
      .then((s) => { setSettings(s); setLoaded(true); })
      .catch((err) => {
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

  // Toggle switch helper (local to this panel).
  const Toggle = ({ checked, onChange }: { checked: boolean; onChange: () => void }) => (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={onChange}
      className={["relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0", checked ? "bg-emerald-500/40" : "bg-zinc-700"].join(" ")}
    >
      <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", checked ? "translate-x-4" : ""].join(" ")} />
    </button>
  );

  // On Android the system nav bar (~48 px) overlaps the WebView bottom edge.
  // The panel needs flex-col so the footer stays below the scroll area, and the
  // scroll area must leave enough room for the footer + nav bar clearance.
  const scrollMaxH = isMobile ? "max-h-[calc(100vh-230px)]" : "max-h-[calc(100vh-150px)]";

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl shadow-xl shadow-black/30 flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className="text-[11px] font-semibold text-zinc-400 uppercase tracking-widest">Advanced Settings</span>
        <button
          aria-label="Close advanced settings"
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Content -- scrollable, constrained so footer stays in view */}
      <div className={`overflow-y-auto ${scrollMaxH} p-4 flex flex-col gap-1`}>

        {/* Speech-to-Text */}
        <button onClick={() => toggleSection("stt")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.stt ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
            Speech-to-Text
            {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
          </span>
        </button>
        {openSections.stt && (
          <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
            {/* Custom STT Prompts -- paid feature */}
            <div className={`flex flex-col gap-3${!isPaid ? " opacity-50" : ""}`}>
              <div className="flex items-center gap-1.5">
                <span className="text-[11px] font-semibold text-zinc-400 uppercase tracking-widest">Custom STT Prompts</span>
                {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
              </div>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>STT Prompt (German)</span>
                <MobileTextarea label="STT Prompt (German)" hint="Injected as context when language is set to German." value={settings.sttPromptDe} onChange={isPaid ? (v) => set("sttPromptDe", v) : () => {}} placeholder={isPaid ? "Context prompt sent with German transcriptions" : "Requires Dikta License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                <span className={hintCls}>Injected as context when language is set to German.</span>
              </div>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>STT Prompt (English)</span>
                <MobileTextarea label="STT Prompt (English)" hint="Injected as context when language is set to English." value={settings.sttPromptEn} onChange={isPaid ? (v) => set("sttPromptEn", v) : () => {}} placeholder={isPaid ? "Context prompt for English transcriptions" : "Requires Dikta License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                <span className={hintCls}>Injected as context when language is set to English.</span>
              </div>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>STT Prompt (Auto-detect)</span>
                <MobileTextarea label="STT Prompt (Auto-detect)" hint="Used when language is set to Auto (DE + EN)." value={settings.sttPromptAuto} onChange={isPaid ? (v) => set("sttPromptAuto", v) : () => {}} placeholder={isPaid ? "Context prompt for auto-detect mode" : "Requires Dikta License"} rows={2} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                <span className={hintCls}>Used when language is set to Auto (DE + EN).</span>
              </div>
              <div className={`flex items-center justify-between gap-3${!isPaid ? " pointer-events-none" : ""}`}>
                <div className="flex flex-col gap-0.5">
                  <span className={LABEL_CLS}>STT Temperature</span>
                  <span className={hintCls}>0.0 = deterministic, 1.0 = more creative. Default: 0.0</span>
                </div>
                <input type="number" min={0} max={1} step={0.1} value={settings.sttTemperature} onChange={(e) => { if (isPaid) set("sttTemperature", parseFloat(e.target.value) || 0); }} disabled={!isPaid} className={numberInputCls} />
              </div>
            </div>
          </div>
        )}

        {/* Text Cleanup */}
        <button onClick={() => toggleSection("llm")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.llm ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Text Cleanup</span>
        </button>
        {openSections.llm && (
          <div className="flex flex-col gap-1 pl-4 pb-3 pt-1">

            {/* Subsection: Model & Parameters -- free, default open */}
            <button
              onClick={() => toggleSubSection("llmParams")}
              className="flex items-center gap-1.5 w-full py-1.5 text-left"
            >
              <svg
                className={`w-3 h-3 text-zinc-600 flex-shrink-0 transition-transform duration-150 ${openSubSections.llmParams ? "rotate-90" : ""}`}
                viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
              >
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest">Model & Parameters</span>
            </button>
            {openSubSections.llmParams && (
              <div className="flex flex-col gap-3 pl-3 pb-2 pt-0.5 border-l border-zinc-800/50 ml-1.5">
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
                <div className="flex items-center justify-between gap-3">
                  <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Chunk Threshold</span><span className={hintCls}>Word count above which text is split into parallel chunks.</span></div>
                  <input type="number" min={50} step={1} value={settings.chunkThreshold} onChange={(e) => set("chunkThreshold", parseInt(e.target.value, 10) || 400)} className={numberInputCls} />
                </div>
                <div className="flex items-center justify-between gap-3">
                  <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Chunk Target Size</span><span className={hintCls}>Target word count per chunk.</span></div>
                  <input type="number" min={50} step={1} value={settings.chunkTargetSize} onChange={(e) => set("chunkTargetSize", parseInt(e.target.value, 10) || 300)} className={numberInputCls} />
                </div>
              </div>
            )}

            {/* Subsection: Custom Cleanup Instructions -- paid, default collapsed */}
            <button
              onClick={() => toggleSubSection("llmCustom")}
              className="flex items-center gap-1.5 w-full py-1.5 text-left mt-1"
            >
              <svg
                className={`w-3 h-3 text-zinc-600 flex-shrink-0 transition-transform duration-150 ${openSubSections.llmCustom ? "rotate-90" : ""}`}
                viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
              >
                <path d="M9 18l6-6-6-6" />
              </svg>
              <span className="flex items-center gap-1.5 text-[11px] font-semibold text-zinc-500 uppercase tracking-widest">
                Custom Cleanup Instructions
                {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
              </span>
            </button>
            {openSubSections.llmCustom && (
              <div className={`flex flex-col gap-3 pl-3 pb-2 pt-0.5 border-l border-zinc-800/50 ml-1.5${!isPaid ? " opacity-50" : ""}`}>
                <p className={hintCls}>
                  Base system prompt for each cleanup style. Your "Cleanup Instructions" from Settings are appended on top -- they stack, not conflict.
                </p>
                <div className={`flex flex-col gap-3${!isPaid ? " pointer-events-none" : ""}`}>
                  <div className="flex flex-col gap-1.5">
                    <span className={LABEL_CLS}>System Prompt: Polished</span>
                    <MobileTextarea label="System Prompt: Polished" hint="Overrides the built-in system prompt for Polished mode." value={settings.llmSystemPromptPolished} onChange={isPaid ? (v) => set("llmSystemPromptPolished", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Dikta License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                    <span className={hintCls}>Overrides the built-in system prompt for Polished mode.</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <span className={LABEL_CLS}>System Prompt: Clean</span>
                    <MobileTextarea label="System Prompt: Clean" hint="Overrides the built-in system prompt for Clean mode." value={settings.llmSystemPromptVerbatim} onChange={isPaid ? (v) => set("llmSystemPromptVerbatim", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Dikta License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                    <span className={hintCls}>Overrides the built-in system prompt for Clean mode.</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <span className={LABEL_CLS}>System Prompt: Chat</span>
                    <MobileTextarea label="System Prompt: Chat" hint="Overrides the built-in system prompt for Chat mode." value={settings.llmSystemPromptChat} onChange={isPaid ? (v) => set("llmSystemPromptChat", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Dikta License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                    <span className={hintCls}>Overrides the built-in system prompt for Chat mode.</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <span className={LABEL_CLS}>Command Mode Prompt</span>
                    <MobileTextarea label="Command Mode Prompt" hint="System prompt for Command Mode (Ctrl+Shift+E)." value={settings.llmCommandModePrompt} onChange={isPaid ? (v) => set("llmCommandModePrompt", v) : () => {}} placeholder={isPaid ? "Leave empty for built-in default" : "Requires Dikta License"} rows={3} className={`${INPUT_CLS} resize-none${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
                    <span className={hintCls}>System prompt for Command Mode (Ctrl+Shift+E).</span>
                  </div>
                </div>
              </div>
            )}

          </div>
        )}

        {/* Audio */}
        <button onClick={() => toggleSection("audio")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.audio ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Audio</span>
        </button>
        {openSections.audio && (
          <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
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
        )}

        {/* Paste & Behavior */}
        <button onClick={() => toggleSection("paste")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.paste ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">Paste & Behavior</span>
        </button>
        {openSections.paste && (
          <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
            <div className="flex items-center justify-between gap-3">
              <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Paste</span><span className={hintCls}>Automatically paste result into active window.</span></div>
              <Toggle checked={settings.autoPaste} onChange={() => set("autoPaste", !settings.autoPaste)} />
            </div>
            <div className="flex items-center justify-between gap-3">
              <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Paste Delay (ms)</span><span className={hintCls}>Wait time before sending paste keystroke.</span></div>
              <input type="number" min={0} max={2000} step={10} value={settings.pasteDelayMs} onChange={(e) => set("pasteDelayMs", parseInt(e.target.value, 10) || 0)} className={numberInputCls} />
            </div>
            <div className="flex items-center justify-between gap-3">
              <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Capitalize</span><span className={hintCls}>Capitalize first letter of every result.</span></div>
              <Toggle checked={settings.autoCapitalize} onChange={() => set("autoCapitalize", !settings.autoCapitalize)} />
            </div>
          </div>
        )}

        {/* Webhook */}
        <button onClick={() => toggleSection("webhook")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.webhook ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
            Webhook
            {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
          </span>
        </button>
        {openSections.webhook && (
          <div className={`flex flex-col gap-3 pl-4 pb-3 pt-1${!isPaid ? " opacity-50" : ""}`}>
            <div className={`flex flex-col gap-1.5${!isPaid ? " pointer-events-none" : ""}`}>
              <span className={LABEL_CLS}>Custom Headers (JSON)</span>
              <MobileTextarea label="Custom Headers (JSON)" hint="Additional HTTP headers sent with each webhook request." value={settings.webhookHeaders} onChange={isPaid ? (v) => set("webhookHeaders", v) : () => {}} placeholder={isPaid ? '{"Authorization": "Bearer ..."}' : "Requires Dikta License"} rows={3} className={`${INPUT_CLS} resize-none font-mono${!isPaid ? " cursor-not-allowed" : ""}`} disabled={!isPaid} />
              <span className={hintCls}>Additional HTTP headers sent with each webhook request.</span>
            </div>
            <div className={`flex items-center justify-between gap-3${!isPaid ? " pointer-events-none" : ""}`}>
              <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Timeout (seconds)</span><span className={hintCls}>Max wait for webhook response.</span></div>
              <input type="number" min={1} max={120} step={1} value={settings.webhookTimeoutSecs} onChange={(e) => { if (isPaid) set("webhookTimeoutSecs", parseInt(e.target.value, 10) || 10); }} disabled={!isPaid} className={`${numberInputCls}${!isPaid ? " cursor-not-allowed" : ""}`} />
            </div>
          </div>
        )}

        {/* Integrations -- paid feature */}
        <button onClick={() => toggleSection("integrations")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.integrations ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="flex items-center gap-1.5 text-sm font-semibold text-zinc-300 uppercase tracking-wide">
            Integrations
            {!isPaid && <LockIcon className="w-3 h-3 text-zinc-600" />}
          </span>
        </button>
        {openSections.integrations && (
          <div className={`flex flex-col gap-3 pl-4 pb-3 pt-1${!isPaid ? " opacity-50" : ""}`}>
            {!isPaid && (
              <p className={hintCls}>Requires Dikta License. Integrations send your transcriptions to external services.</p>
            )}
            <div className={`flex flex-col gap-2${!isPaid ? " pointer-events-none" : ""}`}>
              <div className="flex items-center justify-between gap-3 px-3 py-2.5 rounded-lg bg-[#111113] border border-zinc-800/60">
                <div className="flex flex-col gap-0.5">
                  <span className={LABEL_CLS}>Notion</span>
                  <span className={hintCls}>Append transcriptions to a Notion page.</span>
                </div>
                <span className="text-[10px] text-zinc-600 uppercase tracking-wider">Coming soon</span>
              </div>
              <div className="flex items-center justify-between gap-3 px-3 py-2.5 rounded-lg bg-[#111113] border border-zinc-800/60">
                <div className="flex flex-col gap-0.5">
                  <span className={LABEL_CLS}>Todoist</span>
                  <span className={hintCls}>Create tasks from voice commands.</span>
                </div>
                <span className="text-[10px] text-zinc-600 uppercase tracking-wider">Coming soon</span>
              </div>
            </div>
          </div>
        )}

        {/* System */}
        <button onClick={() => toggleSection("system")} className={sectionBtnCls}>
          <svg className={`w-4 h-4 text-zinc-500 flex-shrink-0 transition-transform duration-150 ${openSections.system ? "rotate-90" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18l6-6-6-6" />
          </svg>
          <span className="text-sm font-semibold text-zinc-300 uppercase tracking-wide">System</span>
        </button>
        {openSections.system && (
          <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
            <div className="flex items-center justify-between gap-3">
              <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Log Level</span><span className={hintCls}>Use "debug" when troubleshooting.</span></div>
              <select value={settings.logLevel} onChange={(e) => set("logLevel", e.target.value)} className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-2 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer">
                <option value="debug">debug</option>
                <option value="info">info</option>
                <option value="warn">warn</option>
                <option value="error">error</option>
              </select>
            </div>
          </div>
        )}

      </div>

      {/* Footer: Save + Reset -- mobile-safe-bottom adds 56 px of bottom padding
          on Android to clear the system nav bar (env() is unreliable in WebView). */}
      <div className={`px-4 py-3 border-t border-zinc-800/40 flex gap-2 ${isMobile ? "mobile-safe-bottom" : ""}`}>
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
