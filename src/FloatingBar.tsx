import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { RecordingState } from "./types";
import { onStateChanged } from "./tauri-commands";

interface AudioLevelPayload {
  level: number;
}

const BAR_SEGMENTS = 32;

export default function FloatingBar() {
  const [state, setState] = useState<RecordingState>("idle");
  const [levels, setLevels] = useState<number[]>(new Array(BAR_SEGMENTS).fill(0));
  const [showDone, setShowDone] = useState(false);
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "cleaning";
  const isActive = isRecording || isProcessing;
  const isIdle = state === "idle" && !showDone;

  // Resize the window based on state.
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    if (isActive || showDone) {
      win.setSize(new LogicalSize(260, 40));
    } else {
      win.setSize(new LogicalSize(44, 32));
    }
  }, [isActive, showDone]);

  // Subscribe to backend pipeline events.
  useEffect(() => {
    const unlisten = onStateChanged((payload) => {
      const newState = payload.state as RecordingState;
      setState(newState);

      if (newState === "done") {
        // Show "Done" briefly, then collapse
        setShowDone(true);
        if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
        doneTimerRef.current = setTimeout(() => {
          setShowDone(false);
          setState("idle");
        }, 1500);
      } else if (newState === "idle" || newState === "error") {
        setLevels(new Array(BAR_SEGMENTS).fill(0));
        if (newState === "error") {
          // Show error briefly
          setShowDone(false);
          if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
          doneTimerRef.current = setTimeout(() => setState("idle"), 2000);
        }
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Subscribe to real-time audio level events.
  useEffect(() => {
    const unlisten = listen<AudioLevelPayload>("dikta://audio-level", (event) => {
      const newLevel = Math.min(1, event.payload.level * 3);
      setLevels((prev) => [...prev.slice(1), newLevel]);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Collapsed idle state: tiny emerald dot
  if (isIdle) {
    return (
      <div
        data-tauri-drag-region
        className="h-full w-full flex items-center justify-center rounded-full bg-[#0e0e11]/90 border border-zinc-800/50 cursor-move"
      >
        <div className="w-2.5 h-2.5 rounded-full bg-emerald-500/60" />
      </div>
    );
  }

  return (
    <div
      data-tauri-drag-region
      className={[
        "h-full flex items-center gap-2 px-3 rounded-full transition-all duration-200 select-none cursor-move",
        "bg-[#0e0e11]/95 border",
        isRecording
          ? "border-red-500/30 shadow-[0_0_20px_rgba(239,68,68,0.2)]"
          : isProcessing
          ? "border-amber-500/20 shadow-[0_0_15px_rgba(245,158,11,0.15)]"
          : showDone
          ? "border-emerald-500/30 shadow-[0_0_15px_rgba(16,185,129,0.2)]"
          : "border-zinc-800/50",
      ].join(" ")}
      style={{ fontFamily: "'Inter', system-ui, sans-serif" }}
    >
      {/* Status dot */}
      <div
        className={[
          "w-2 h-2 rounded-full flex-shrink-0",
          isRecording
            ? "bg-red-500 animate-pulse"
            : isProcessing
            ? "bg-amber-400 animate-pulse"
            : showDone
            ? "bg-emerald-400"
            : state === "error"
            ? "bg-red-500"
            : "bg-emerald-500/60",
        ].join(" ")}
      />

      {/* Waveform while recording */}
      {isRecording && (
        <div className="flex items-center gap-[1.5px] h-6 flex-1 min-w-0">
          {levels.map((level, i) => (
            <div
              key={i}
              className="w-[2.5px] rounded-full bg-emerald-400 transition-all duration-75"
              style={{
                height: `${Math.max(2, level * 22)}px`,
                opacity: 0.3 + level * 0.7,
              }}
            />
          ))}
        </div>
      )}

      {/* Processing spinner */}
      {isProcessing && (
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <svg
            className="w-3.5 h-3.5 text-amber-400 animate-spin flex-shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
          >
            <circle cx="12" cy="12" r="10" strokeOpacity="0.2" />
            <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
          </svg>
          <span className="text-[11px] text-zinc-400 truncate">
            {state === "transcribing" ? "Transcribing..." : "Cleaning..."}
          </span>
        </div>
      )}

      {/* Done text (shown briefly after completion) */}
      {showDone && !isProcessing && !isRecording && (
        <span className="text-[11px] text-emerald-400 flex-1 min-w-0 truncate">Done</span>
      )}

      {/* Error text */}
      {state === "error" && !showDone && (
        <span className="text-[11px] text-red-400 flex-1 min-w-0 truncate">Error</span>
      )}
    </div>
  );
}
