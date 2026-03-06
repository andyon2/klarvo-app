import { useState, useCallback } from "react";
import "./styles.css";
import type { RecordingState, CleanupStyle } from "./types";
import { STATUS_LABELS, STYLE_OPTIONS } from "./types";
import { startRecording, stopRecording, cleanupText } from "./tauri-commands";

// --- Sub-components -------------------------------------------------------

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

      {/* Icon */}
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
      <path
        d="M12 2a10 10 0 0 1 10 10"
        strokeLinecap="round"
      />
    </svg>
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
  const label = errorMessage && isError ? errorMessage : STATUS_LABELS[recordingState];

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

// --- Main component -------------------------------------------------------

export default function App() {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [currentStyle, setCurrentStyle] = useState<CleanupStyle>("polished");
  const [resultText, setResultText] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const isBusy =
    recordingState === "transcribing" || recordingState === "cleaning";
  const isRecording = recordingState === "recording";

  const handleRecordToggle = useCallback(async () => {
    if (isRecording) {
      // --- Stop flow ---
      try {
        setRecordingState("transcribing");
        const { rawText } = await stopRecording();

        setRecordingState("cleaning");
        const { cleanedText } = await cleanupText(rawText, currentStyle);

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
  }, [isRecording, currentStyle]);

  // Allow clicking again after done/error to reset.
  const handleReset = useCallback(() => {
    if (recordingState === "done" || recordingState === "error") {
      setRecordingState("idle");
      setErrorMessage(null);
    }
  }, [recordingState]);

  return (
    <main
      className="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col items-center justify-between p-4 select-none"
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      {/* Header */}
      <div className="w-full flex items-center justify-between">
        <span className="text-sm font-semibold tracking-widest text-zinc-500 uppercase">
          Dikta
        </span>
        <StylePicker
          value={currentStyle}
          onChange={setCurrentStyle}
          disabled={isBusy || isRecording}
        />
      </div>

      {/* Center section -- record button */}
      <div className="flex flex-col items-center gap-6 flex-1 justify-center">
        <RecordButton
          recordingState={recordingState === "done" || recordingState === "error"
            ? "idle"
            : recordingState}
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
      <StatusBar
        recordingState={recordingState}
        errorMessage={errorMessage}
      />
    </main>
  );
}
