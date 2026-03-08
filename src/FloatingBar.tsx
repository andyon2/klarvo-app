import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import type { RecordingState } from "./types";
import { onStateChanged, setBarShape, transcribeLivePreview, cancelRecording } from "./tauri-commands";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

interface AudioLevelPayload {
  level: number;
}

/** Number of waveform bars. */
const BAR_COUNT = 5;

/** Idle state: thin semi-transparent pill. */
const IDLE_WIDTH = 80;
const IDLE_HEIGHT = 10;

/** Expanded pill dimensions (compact). */
const PILL_WIDTH = 164;
const PILL_HEIGHT = 18;

// ---------------------------------------------------------------------------
// Inline style reset
// ---------------------------------------------------------------------------

const RESET_CSS = `
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  html, body, #root {
    width: 100%; height: 100%;
    overflow: hidden !important;
    background: transparent !important;
  }
  ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }

  @keyframes bar-bounce-0 { 0%,100%{transform:scaleY(0.15)} 50%{transform:scaleY(1)} }
  @keyframes bar-bounce-1 { 0%,100%{transform:scaleY(0.15)} 50%{transform:scaleY(1)} }
  @keyframes bar-bounce-2 { 0%,100%{transform:scaleY(0.15)} 50%{transform:scaleY(1)} }
  @keyframes bar-bounce-3 { 0%,100%{transform:scaleY(0.15)} 50%{transform:scaleY(1)} }
  @keyframes bar-bounce-4 { 0%,100%{transform:scaleY(0.15)} 50%{transform:scaleY(1)} }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }

  @keyframes done-pop {
    0%   { transform: scale(0.85); opacity: 0; }
    60%  { transform: scale(1.08); opacity: 1; }
    100% { transform: scale(1);    opacity: 1; }
  }
`;

// Phase offsets per bar, spread evenly.
const BAR_PHASE_DELAYS = [0, 0.2, 0.4, 0.6, 0.8];
const BAR_ANIMATION_DURATION = 600;

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Animated waveform: 5 bars, soft color. */
function Waveform({ levels }: { levels: number[] }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 2,
        height: 12,
        flex: 1,
        minWidth: 0,
      }}
    >
      {Array.from({ length: BAR_COUNT }, (_, i) => {
        const levelIdx = Math.round((i / (BAR_COUNT - 1)) * (levels.length - 1));
        const amplitude = Math.max(0.12, levels[levelIdx] ?? 0);
        const heightPx = Math.max(2, amplitude * 11);
        const delayMs = BAR_PHASE_DELAYS[i] * BAR_ANIMATION_DURATION;
        return (
          <div
            key={i}
            style={{
              flex: 1,
              borderRadius: 9999,
              background: "rgba(147,197,253,0.85)", // soft blue
              height: heightPx,
              transformOrigin: "center",
              animation: `bar-bounce-${i} ${BAR_ANIMATION_DURATION}ms ease-in-out ${delayMs}ms infinite`,
              willChange: "transform",
              transition: "height 75ms ease-out",
            }}
          />
        );
      })}
    </div>
  );
}

/** Rotating arc spinner. */
function Spinner({ color }: { color: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="2.5"
      strokeLinecap="round"
      style={{
        width: 9,
        height: 9,
        flexShrink: 0,
        animation: "spin 0.9s linear infinite",
        willChange: "transform",
      }}
    >
      <circle cx="12" cy="12" r="10" strokeOpacity="0.18" />
      <path d="M12 2a10 10 0 0 1 10 10" />
    </svg>
  );
}

/** Small check icon. */
function CheckIcon({ color }: { color: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"
      style={{ width: 8, height: 8, flexShrink: 0 }}>
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

/** Stop button (square icon) for canceling recording. */
function StopButton({ onClick }: { onClick: () => void }) {
  return (
    <div
      onClick={(e) => { e.stopPropagation(); onClick(); }}
      style={{
        width: 10,
        height: 10,
        flexShrink: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: "pointer",
        borderRadius: 2,
      }}
    >
      <div
        style={{
          width: 6,
          height: 6,
          borderRadius: 1,
          background: "rgba(248,113,113,0.9)",
        }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function FloatingBar() {
  const [state, setState] = useState<RecordingState>("idle");
  const [levels, setLevels] = useState<number[]>(new Array(20).fill(0));
  const [showDone, setShowDone] = useState(false);
  const [livePreview, setLivePreview] = useState("");
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Store initial position so expand/collapse stays centered.
  const screenCenterX = useRef<number | null>(null);
  const baseY = useRef<number | null>(null);

  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "cleaning";
  const isActive = isRecording || isProcessing;
  const isIdle = state === "idle" && !showDone;
  const isDone = showDone && !isActive;
  const isError = state === "error" && !showDone;

  // --- Capture initial position once on mount ---
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      try {
        const pos = await win.outerPosition();
        const scale = (await win.scaleFactor()) || 1;
        screenCenterX.current = pos.x / scale + IDLE_WIDTH / 2;
        baseY.current = pos.y / scale;
      } catch { /* non-critical */ }
    })();
  }, []);

  // --- Resize + reposition on state change ---
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      const cx = screenCenterX.current;
      const by = baseY.current;

      if (isActive || showDone) {
        await win.setSize(new LogicalSize(PILL_WIDTH, PILL_HEIGHT));
        await setBarShape("pill").catch(() => {});
        if (cx != null && by != null) {
          await win.setPosition(new LogicalPosition(
            cx - PILL_WIDTH / 2,
            by - (PILL_HEIGHT - IDLE_HEIGHT) / 2,
          ));
        }
      } else {
        await win.setSize(new LogicalSize(IDLE_WIDTH, IDLE_HEIGHT));
        await setBarShape("idle").catch(() => {});
        if (cx != null && by != null) {
          await win.setPosition(new LogicalPosition(cx - IDLE_WIDTH / 2, by));
        }
      }
    })();
  }, [isActive, showDone]);

  // --- Backend pipeline events ---
  useEffect(() => {
    const unlisten = onStateChanged((payload) => {
      const newState = payload.state as RecordingState;
      setState(newState);

      if (newState === "done") {
        setShowDone(true);
        if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
        doneTimerRef.current = setTimeout(() => {
          setShowDone(false);
          setState("idle");
        }, 1500);
      } else if (newState === "idle") {
        setLevels(new Array(20).fill(0));
      } else if (newState === "error") {
        setLevels(new Array(20).fill(0));
        setShowDone(false);
        if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
        doneTimerRef.current = setTimeout(() => setState("idle"), 2500);
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // --- Real-time audio level ring buffer ---
  useEffect(() => {
    const unlisten = listen<AudioLevelPayload>("dikta://audio-level", (event) => {
      const raw = Math.min(1, event.payload.level * 2.8);
      const boosted = Math.pow(raw, 0.6);
      setLevels((prev) => [...prev.slice(1), boosted]);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // --- Live preview polling while recording ---
  useEffect(() => {
    if (!isRecording) {
      setLivePreview("");
      return;
    }
    const initialDelay = setTimeout(() => {
      transcribeLivePreview().then((t) => { if (t) setLivePreview(t); }).catch(() => {});
    }, 2000);
    const interval = setInterval(() => {
      transcribeLivePreview().then((t) => { if (t) setLivePreview(t); }).catch(() => {});
    }, 3000);
    return () => { clearTimeout(initialDelay); clearInterval(interval); };
  }, [isRecording]);

  // ---------------------------------------------------------------------------
  // Render: idle -- thin semi-transparent pill, barely visible
  // ---------------------------------------------------------------------------

  if (isIdle) {
    return (
      <>
        <style>{RESET_CSS}</style>
        <div
          data-tauri-drag-region
          style={{
            width: "100%",
            height: "100%",
            borderRadius: 9999,
            background: "rgba(255,255,255,0.08)",
            border: "1px solid rgba(255,255,255,0.06)",
            cursor: "move",
            overflow: "hidden",
          }}
        />
      </>
    );
  }

  // ---------------------------------------------------------------------------
  // Render: expanded pill (recording / processing / done / error)
  // ---------------------------------------------------------------------------

  const accentColor = isRecording ? "#93c5fd"  // soft blue
    : isProcessing ? "#fbbf24"
    : isDone ? "#34d399"
    : "#f87171";

  const borderColor = isRecording ? "rgba(147,197,253,0.25)"
    : isProcessing ? "rgba(245,158,11,0.2)"
    : isDone ? "rgba(52,211,153,0.25)"
    : "rgba(248,113,113,0.2)";

  return (
    <>
      <style>{RESET_CSS}</style>
      <div
        data-tauri-drag-region
        style={{
          width: "100%",
          height: "100%",
          borderRadius: 9999,
          background: "rgba(20,20,24,0.92)",
          border: `1px solid ${borderColor}`,
          display: "flex",
          alignItems: "center",
          gap: 4,
          paddingLeft: 6,
          paddingRight: 6,
          cursor: "move",
          fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
          userSelect: "none",
          overflow: "hidden",
        }}
      >

        {/* Recording: stop button + waveform or live preview */}
        {isRecording && (
          <>
            <StopButton onClick={() => { cancelRecording().catch(() => {}); }} />
            {livePreview ? (
              <span
                style={{
                  fontSize: 8,
                  color: "#d4d4d8",
                  flex: 1,
                  minWidth: 0,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  direction: "rtl",
                  textAlign: "left",
                  lineHeight: 1,
                }}
              >
                {livePreview}
              </span>
            ) : (
              <Waveform levels={levels} />
            )}
          </>
        )}

        {/* Processing: spinner + label */}
        {isProcessing && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              flex: 1,
              minWidth: 0,
            }}
          >
            <Spinner color={accentColor} />
            <span
              style={{
                fontSize: 8,
                color: "#a1a1aa",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                letterSpacing: "0.01em",
              }}
            >
              {state === "transcribing" ? "Transcribing..." : "Cleaning up..."}
            </span>
          </div>
        )}

        {/* Done: check icon + label */}
        {isDone && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 3,
              flex: 1,
              minWidth: 0,
              animation: "done-pop 280ms cubic-bezier(0.34,1.56,0.64,1) forwards",
            }}
          >
            <CheckIcon color={accentColor} />
            <span style={{ fontSize: 8, color: "#34d399", letterSpacing: "0.01em" }}>Done</span>
          </div>
        )}

        {/* Error */}
        {isError && (
          <span style={{ fontSize: 8, color: "#f87171", flex: 1, letterSpacing: "0.01em" }}>Error</span>
        )}

      </div>
    </>
  );
}
