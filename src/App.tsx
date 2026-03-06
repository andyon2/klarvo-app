import { useState, useCallback, useEffect, useRef } from "react";
import "./styles.css";
import type { RecordingState, CleanupStyle, ApiKeyStatus } from "./types";
import { STATUS_LABELS, STYLE_OPTIONS } from "./types";
import {
  startRecording,
  stopRecording,
  transcribeAudio,
  cleanupText,
  getApiKeyStatus,
  updateApiKeys,
} from "./tauri-commands";

// --- Icons -------------------------------------------------------------------

function MicIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  );
}

function StopIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <rect x="5" y="5" width="14" height="14" rx="2" />
    </svg>
  );
}

function SpinnerIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-8 h-8 text-white animate-spin"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
    >
      <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
      <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg
      aria-hidden="true"
      className="w-4 h-4"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.01 7.01 0 0 0-1.62-.94l-.36-2.54A.484.484 0 0 0 14 2h-4a.484.484 0 0 0-.48.41l-.36 2.54a7.07 7.07 0 0 0-1.62.94l-2.39-.96a.49.49 0 0 0-.59.22L2.74 8.87a.48.48 0 0 0 .12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.27.41.49.41h4c.22 0 .43-.17.47-.41l.36-2.54a7.07 7.07 0 0 0 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.49.49 0 0 0-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
    </svg>
  );
}

// --- Sub-components ----------------------------------------------------------

interface RecordButtonProps {
  recordingState: RecordingState;
  onClick: () => void;
}

function RecordButton({ recordingState, onClick }: RecordButtonProps) {
  const isRecording = recordingState === "recording";
  const isBusy =
    recordingState === "transcribing" || recordingState === "cleaning";

  return (
    <button
      aria-label={
        isRecording
          ? "Stop recording"
          : isBusy
          ? "Processing audio"
          : "Start recording"
      }
      disabled={isBusy}
      onClick={onClick}
      className={[
        "relative flex items-center justify-center",
        "w-20 h-20 rounded-full",
        "transition-all duration-150",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
        "disabled:cursor-not-allowed disabled:opacity-60",
        isRecording
          ? "bg-red-600 hover:bg-red-500 shadow-[0_0_24px_rgba(239,68,68,0.5)]"
          : isBusy
          ? "bg-amber-500 shadow-[0_0_20px_rgba(245,158,11,0.4)]"
          : "bg-blue-600 hover:bg-blue-500 shadow-[0_0_16px_rgba(59,130,246,0.35)]",
      ].join(" ")}
    >
      {/* Pulse ring -- only while recording */}
      {isRecording && (
        <span className="absolute inset-0 rounded-full bg-red-500 opacity-40 animate-ping" />
      )}

      {isBusy ? (
        <SpinnerIcon />
      ) : isRecording ? (
        <StopIcon />
      ) : (
        <MicIcon />
      )}
    </button>
  );
}

interface StylePickerProps {
  value: CleanupStyle;
  onChange: (style: CleanupStyle) => void;
  disabled: boolean;
}

function StylePicker({ value, onChange, disabled }: StylePickerProps) {
  return (
    <div
      role="radiogroup"
      aria-label="Cleanup style"
      className="flex gap-1 bg-zinc-800 rounded-lg p-1"
    >
      {STYLE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          role="radio"
          aria-checked={value === opt.value}
          aria-label={`${opt.label}: ${opt.description}`}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          title={opt.description}
          className={[
            "flex-1 px-3 py-1.5 rounded-md text-xs font-medium",
            "transition-colors duration-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
            "disabled:cursor-not-allowed disabled:opacity-50",
            value === opt.value
              ? "bg-zinc-600 text-white"
              : "text-zinc-400 hover:text-zinc-200",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

interface StatusBarProps {
  recordingState: RecordingState;
  errorMessage: string | null;
}

function StatusBar({ recordingState, errorMessage }: StatusBarProps) {
  const isError = recordingState === "error";
  const label =
    errorMessage && isError ? errorMessage : STATUS_LABELS[recordingState];

  return (
    <div
      aria-live="polite"
      aria-atomic="true"
      className={[
        "text-xs font-mono",
        isError ? "text-red-400" : "text-zinc-500",
      ].join(" ")}
    >
      {label}
    </div>
  );
}

// Dot indicator for API key status.
function StatusDot({ configured }: { configured: boolean }) {
  return (
    <span
      aria-label={configured ? "Configured" : "Not configured"}
      className={[
        "inline-block w-2 h-2 rounded-full flex-shrink-0",
        configured ? "bg-green-500" : "bg-red-500",
      ].join(" ")}
    />
  );
}

interface SettingsPanelProps {
  onClose: () => void;
  language: string;
  onLanguageChange: (lang: string) => void;
}

function SettingsPanel({ onClose, language, onLanguageChange }: SettingsPanelProps) {
  const [groqKey, setGroqKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [apiStatus, setApiStatus] = useState<ApiKeyStatus | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Load API key status on mount.
  useEffect(() => {
    getApiKeyStatus()
      .then((status) => setApiStatus(status))
      .catch((err) => console.error("[dikta] get_api_key_status error:", err));
  }, []);

  // Close on Escape.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onClose]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await updateApiKeys(
        groqKey.trim() || undefined,
        deepseekKey.trim() || undefined
      );
      // Refresh status after save.
      const status = await getApiKeyStatus();
      setApiStatus(status);
      setGroqKey("");
      setDeepseekKey("");
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [groqKey, deepseekKey]);

  return (
    <div
      ref={panelRef}
      className={[
        "w-full bg-zinc-900 border border-zinc-700 rounded-xl",
        "p-4 flex flex-col gap-3",
        "transition-all duration-200",
      ].join(" ")}
      role="region"
      aria-label="Settings"
    >
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
          Settings
        </span>
        <button
          aria-label="Close settings"
          onClick={onClose}
          className={[
            "text-zinc-500 hover:text-zinc-200",
            "transition-colors duration-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
            "rounded p-0.5",
          ].join(" ")}
        >
          <svg
            aria-hidden="true"
            className="w-3.5 h-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Groq API Key */}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-1.5">
          <label
            htmlFor="groq-key"
            className="text-xs text-zinc-400 flex-1"
          >
            Groq API Key
          </label>
          {apiStatus && <StatusDot configured={apiStatus.groqConfigured} />}
        </div>
        <input
          id="groq-key"
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder="gsk_..."
          value={groqKey}
          onChange={(e) => setGroqKey(e.target.value)}
          className={[
            "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
            "px-3 py-1.5 text-xs text-zinc-100",
            "placeholder:text-zinc-600",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
            "transition-shadow duration-100",
          ].join(" ")}
        />
      </div>

      {/* DeepSeek API Key */}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-1.5">
          <label
            htmlFor="deepseek-key"
            className="text-xs text-zinc-400 flex-1"
          >
            DeepSeek API Key
          </label>
          {apiStatus && <StatusDot configured={apiStatus.deepseekConfigured} />}
        </div>
        <input
          id="deepseek-key"
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder="sk-..."
          value={deepseekKey}
          onChange={(e) => setDeepseekKey(e.target.value)}
          className={[
            "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
            "px-3 py-1.5 text-xs text-zinc-100",
            "placeholder:text-zinc-600",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
            "transition-shadow duration-100",
          ].join(" ")}
        />
      </div>

      {/* Language */}
      <div className="flex items-center gap-3">
        <label htmlFor="language-select" className="text-xs text-zinc-400 flex-1">
          Language
        </label>
        <select
          id="language-select"
          value={language}
          onChange={(e) => onLanguageChange(e.target.value)}
          className={[
            "bg-zinc-800 border border-zinc-700 rounded-lg",
            "px-2 py-1.5 text-xs text-zinc-100",
            "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
            "transition-shadow duration-100",
            "cursor-pointer",
          ].join(" ")}
        >
          <option value="de">Deutsch</option>
          <option value="en">English</option>
        </select>
      </div>

      {/* Save button + error */}
      {saveError && (
        <p className="text-xs text-red-400 break-all">{saveError}</p>
      )}
      <button
        onClick={handleSave}
        disabled={saving || (!groqKey.trim() && !deepseekKey.trim())}
        className={[
          "w-full py-1.5 rounded-lg text-xs font-semibold",
          "bg-blue-600 hover:bg-blue-500 text-white",
          "disabled:opacity-50 disabled:cursor-not-allowed",
          "transition-colors duration-100",
          "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
        ].join(" ")}
      >
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
  );
}

// --- Main component ----------------------------------------------------------

export default function App() {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [currentStyle, setCurrentStyle] = useState<CleanupStyle>("polished");
  const [resultText, setResultText] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [language, setLanguage] = useState("de");

  const isBusy =
    recordingState === "transcribing" || recordingState === "cleaning";
  const isRecording = recordingState === "recording";

  const handleRecordToggle = useCallback(async () => {
    if (isRecording) {
      // --- Stop -> transcribe -> cleanup flow ---
      try {
        setRecordingState("transcribing");
        await stopRecording();

        const rawText = await transcribeAudio(language);

        setRecordingState("cleaning");
        const cleanedText = await cleanupText(rawText, currentStyle);

        setResultText(cleanedText);
        setRecordingState("done");
      } catch (err) {
        console.error("[dikta] stop/process error:", err);
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      // --- Start flow ---
      setResultText(null);
      setErrorMessage(null);
      try {
        await startRecording();
        setRecordingState("recording");
      } catch (err) {
        console.error("[dikta] start_recording error:", err);
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    }
  }, [isRecording, currentStyle, language]);

  // Allow clicking again after done/error to reset.
  const handleReset = useCallback(() => {
    if (recordingState === "done" || recordingState === "error") {
      setRecordingState("idle");
      setErrorMessage(null);
    }
  }, [recordingState]);

  const toggleSettings = useCallback(() => {
    setShowSettings((prev) => !prev);
  }, []);

  return (
    <main
      className="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col items-center justify-between p-4 select-none"
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      {/* Header */}
      <div className="w-full flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold tracking-widest text-zinc-500 uppercase">
            Dikta
          </span>
          <button
            aria-label="Toggle settings"
            aria-expanded={showSettings}
            onClick={toggleSettings}
            className={[
              "p-1 rounded-md",
              "transition-colors duration-100",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/40",
              showSettings
                ? "text-zinc-200 bg-zinc-800"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800",
            ].join(" ")}
          >
            <GearIcon />
          </button>
        </div>
        <StylePicker
          value={currentStyle}
          onChange={setCurrentStyle}
          disabled={isBusy || isRecording}
        />
      </div>

      {/* Settings panel -- inline, no modal */}
      <div
        className={[
          "w-full overflow-hidden transition-all duration-200",
          showSettings ? "max-h-96 opacity-100 mt-3" : "max-h-0 opacity-0",
        ].join(" ")}
        aria-hidden={!showSettings}
      >
        {showSettings && (
          <SettingsPanel
            onClose={() => setShowSettings(false)}
            language={language}
            onLanguageChange={setLanguage}
          />
        )}
      </div>

      {/* Center section -- record button */}
      <div className="flex flex-col items-center gap-6 flex-1 justify-center">
        <RecordButton
          recordingState={
            recordingState === "done" || recordingState === "error"
              ? "idle"
              : recordingState
          }
          onClick={
            recordingState === "done" || recordingState === "error"
              ? handleReset
              : handleRecordToggle
          }
        />

        {/* Result textarea */}
        {resultText !== null && (
          <div className="w-full max-w-xs">
            <label
              htmlFor="result-text"
              className="block text-xs text-zinc-500 mb-1"
            >
              Result
            </label>
            <textarea
              id="result-text"
              readOnly
              value={resultText}
              rows={4}
              className={[
                "w-full bg-zinc-800 border border-zinc-700 rounded-lg",
                "px-3 py-2 text-sm text-zinc-100 resize-none",
                "focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50",
                "placeholder:text-zinc-600",
              ].join(" ")}
            />
          </div>
        )}
      </div>

      {/* Footer -- status */}
      <StatusBar recordingState={recordingState} errorMessage={errorMessage} />
    </main>
  );
}
