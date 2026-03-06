import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { RecordingState } from "./types";
import { stopRecording, onStateChanged } from "./tauri-commands";

// Audio level payload emitted by the backend during recording.
interface AudioLevelPayload {
  level: number; // RMS amplitude 0.0..1.0
}

const BAR_SEGMENTS = 24;

export default function FloatingBar() {
  const [state, setState] = useState<RecordingState>("idle");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [levels, setLevels] = useState<number[]>(new Array(BAR_SEGMENTS).fill(0));
  const levelsRef = useRef(levels);

  // Subscribe to backend pipeline events.
  useEffect(() => {
    const unlisten = onStateChanged((payload) => {
      setState(payload.state as RecordingState);
      if (payload.error) setErrorMsg(payload.error);
      if (payload.state === "done" || payload.state === "idle") {
        setLevels(new Array(BAR_SEGMENTS).fill(0));
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Subscribe to real-time audio level events.
  useEffect(() => {
    const unlisten = listen<AudioLevelPayload>("dikta://audio-level", (event) => {
      const newLevel = Math.min(1, event.payload.level * 3); // amplify for visibility
      setLevels((prev) => {
        const next = [...prev.slice(1), newLevel];
        levelsRef.current = next;
        return next;
      });
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Cancel recording on button click.
  const handleCancel = useCallback(async () => {
    try {
      await stopRecording();
    } catch {
      // ignore -- might not be recording
    }
    setState("idle");
    setLevels(new Array(BAR_SEGMENTS).fill(0));
  }, []);

  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "cleaning";
  const isActive = isRecording || isProcessing;
  const isError = state === "error";

  return (
    <div
      data-tauri-drag-region
      className={[
        "h-full flex items-center gap-2 px-3 rounded-2xl transition-all duration-200 select-none",
        "bg-zinc-900/90 backdrop-blur-md border border-zinc-700/50",
        isActive ? "shadow-[0_0_20px_rgba(59,130,246,0.3)]" : "",
      ].join(" ")}
      style={{ fontFamily: "Inter, system-ui, sans-serif" }}
    >
      {/* Status indicator dot */}
      <div
        className={[
          "w-2.5 h-2.5 rounded-full flex-shrink-0 transition-colors duration-200",
          isRecording
            ? "bg-red-500 animate-pulse"
            : isProcessing
            ? "bg-amber-500 animate-pulse"
            : isError
            ? "bg-red-500"
            : "bg-zinc-600",
        ].join(" ")}
      />

      {/* Waveform visualization */}
      {isRecording && (
        <div className="flex items-center gap-[2px] h-6 flex-1 min-w-0">
          {levels.map((level, i) => (
            <div
              key={i}
              className="w-[3px] rounded-full bg-blue-400 transition-all duration-75"
              style={{
                height: `${Math.max(3, level * 24)}px`,
                opacity: 0.4 + level * 0.6,
              }}
            />
          ))}
        </div>
      )}

      {/* Processing spinner */}
      {isProcessing && (
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <svg
            className="w-4 h-4 text-amber-400 animate-spin flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
            <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
          </svg>
          <span className="text-xs text-zinc-400 truncate">
            {state === "transcribing" ? "Transcribing..." : "Cleaning up..."}
          </span>
        </div>
      )}

      {/* Idle / Done / Error text */}
      {!isActive && (
        <span
          className={[
            "text-xs flex-1 min-w-0 truncate",
            isError ? "text-red-400" : "text-zinc-500",
          ].join(" ")}
        >
          {isError ? (errorMsg || "Error") : state === "done" ? "Done" : "Dikta"}
        </span>
      )}

      {/* Cancel button -- only while recording */}
      {isRecording && (
        <button
          onClick={handleCancel}
          aria-label="Cancel recording"
          className={[
            "flex-shrink-0 w-5 h-5 flex items-center justify-center rounded-full",
            "bg-zinc-700 hover:bg-red-600 text-zinc-400 hover:text-white",
            "transition-colors duration-100",
          ].join(" ")}
        >
          <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}
