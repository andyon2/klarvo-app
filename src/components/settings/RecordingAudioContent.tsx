import type { AppSettings } from "../../types";
import { isDesktop, isMobile } from "../../platform";
import { LABEL_CLS_M } from "../ui";
import { WhisperModelManager } from "../WhisperModelManager";
import { LlmModelManager } from "../LlmModelManager";
import { KSelect, KSegmented } from "./FormControls";

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
        <div className="w-fit">
          <KSegmented
            value={localSttProvider === "local" ? "local" : "cloud"}
            onChange={(v) => {
              if (v === "cloud" && localSttProvider === "local") {
                setLocalSttProvider("groq");
              } else if (v === "local") {
                setLocalSttProvider("local");
              }
            }}
            options={[
              { value: "cloud", label: "Cloud" },
              { value: "local", label: "Offline" },
            ]}
          />
        </div>

        {/* Cloud mode: model picker */}
        {localSttProvider !== "local" && (
          <div className="flex flex-col gap-2 mt-1">
            <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
              <span className={LABEL_CLS_M}>Model</span>
              <div className={isMobile ? "w-full" : ""}>
                <KSelect
                  value={localSttModel}
                  onChange={(model) => {
                    setLocalSttModel(model);
                    // Sync provider to match the selected model's API.
                    if (model === "whisper-1") {
                      setLocalSttProvider("openai");
                    } else {
                      setLocalSttProvider("groq");
                    }
                  }}
                  options={CLOUD_STT_MODELS.filter((m) => {
                    if (m.provider === "groq") return groqOk;
                    if (m.provider === "openai") return openaiOk;
                    return true;
                  }).map((m) => ({ value: m.value, label: m.label }))}
                />
              </div>
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
            <div className={isMobile ? "w-full" : ""}>
              <KSelect
                value={localLlmProvider}
                onChange={setLocalLlmProvider}
                options={[
                  { value: "deepseek", label: `DeepSeek${!deepseekOk ? " (no key)" : ""}`, disabled: !deepseekOk },
                  { value: "openai", label: `OpenAI${!openaiOk ? " (no key)" : ""}`, disabled: !openaiOk },
                  { value: "groq", label: `Groq (Llama)${!groqOk ? " (no key)" : ""}`, disabled: !groqOk },
                  { value: "openrouter", label: `OpenRouter${!openrouterOk ? " (no key)" : ""}`, disabled: !openrouterOk },
                  { value: "local", label: "Local (Offline)" },
                ]}
              />
            </div>
          </div>

          {/* Local LLM model manager -- shown when local provider is selected */}
          {localLlmProvider === "local" && (
            <LlmModelManager />
          )}
        </div>
      )}

      {/* Microphone -- desktop only (Android uses its own mic via MediaRecorder) */}
      {isDesktop && (
        <div className="flex items-center justify-between gap-3">
          <span className={LABEL_CLS_M}>Microphone</span>
          <div className="max-w-[180px] w-full">
            <KSelect
              value={localAudioDevice ?? ""}
              onChange={(v) => handleAudioDeviceChange(v || null)}
              options={[
                { value: "", label: "System Default" },
                ...audioDevices.map((n) => ({ value: n, label: n })),
              ]}
            />
          </div>
        </div>
      )}
    </div>
  );
}
