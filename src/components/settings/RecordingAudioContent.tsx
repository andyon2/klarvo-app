import type { CleanupStyle, AppSettings } from "../../types";
import { STYLE_OPTIONS } from "../../types";
import { isDesktop, isMobile } from "../../platform";
import { LABEL_CLS_M } from "../ui";
import { WhisperModelManager } from "../WhisperModelManager";
import { LlmModelManager } from "../LlmModelManager";

// --- Cloud STT models ---------------------------------------------------------

export const CLOUD_STT_MODELS = [
  { value: "whisper-large-v3-turbo", label: "Groq — Large V3 Turbo", price: "~$0.0007/min", provider: "groq" },
  { value: "whisper-large-v3", label: "Groq — Large V3", price: "~$0.002/min", provider: "groq" },
  { value: "whisper-1", label: "OpenAI — Whisper 1", price: "~$0.006/min", provider: "openai" },
];

// --- Props -------------------------------------------------------------------

export interface RecordingAudioContentProps {
  localSttProvider: string;
  setLocalSttProvider: (v: string) => void;
  localSttModel: string;
  setLocalSttModel: (v: string) => void;
  localLlmProvider: string;
  setLocalLlmProvider: (v: string) => void;
  localStyle: CleanupStyle;
  handleStyleChange: (style: CleanupStyle) => void;
  localAudioDevice: string | null;
  handleAudioDeviceChange: (d: string | null) => void;
  audioDevices: string[];
  localWhisperModel: string;
  setLocalWhisperModel: (v: string) => void;
  localWhisperGpu: boolean;
  setLocalWhisperGpu: (v: boolean) => void;
  isPaid: boolean;
  groqOk: boolean;
  deepseekOk: boolean;
  openaiOk: boolean;
  openrouterOk: boolean;
  loadedSettings: AppSettings | null;
}

// --- Component ---------------------------------------------------------------

export function RecordingAudioContent({
  localSttProvider, setLocalSttProvider,
  localSttModel, setLocalSttModel,
  localLlmProvider, setLocalLlmProvider,
  localStyle, handleStyleChange,
  localAudioDevice, handleAudioDeviceChange,
  audioDevices,
  localWhisperModel, setLocalWhisperModel,
  localWhisperGpu, setLocalWhisperGpu,
  isPaid,
  groqOk, deepseekOk, openaiOk, openrouterOk,
}: RecordingAudioContentProps) {
  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">

      {/* Cloud / Offline toggle -- shown on all platforms */}
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
        {localSttProvider === "local" && (
          <div className="flex flex-col gap-3 mt-1">
            <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-klarvo-surface/30 border border-klarvo-border/30">
              <svg className="w-3.5 h-3.5 text-klarvo-muted mt-0.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" />
              </svg>
              <p className="text-[11px] text-klarvo-muted leading-relaxed">
                Speech is transcribed locally on your device.
              </p>
            </div>
            <WhisperModelManager
              selectedModel={localWhisperModel}
              gpuEnabled={localWhisperGpu}
              onModelChange={setLocalWhisperModel}
              onGpuChange={setLocalWhisperGpu}
              isPaid={isPaid}
              showGpuToggle={isDesktop}
            />
          </div>
        )}
        </div>
      </div>

      {/* Text Cleanup -- Offline mode: always local, no toggle needed */}
      {localSttProvider === "local" && (
        <div className="flex flex-col gap-2.5">
          <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Text Cleanup</span>
          <LlmModelManager />
        </div>
      )}

      {/* Text Cleanup -- Cloud mode */}
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
              <option value="local">Local (Offline)</option>
            </select>
          </div>

          {/* Local LLM model manager -- shown when local provider is selected */}
          {localLlmProvider === "local" && (
            <LlmModelManager />
          )}

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
  );
}
