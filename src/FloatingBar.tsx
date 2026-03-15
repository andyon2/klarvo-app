import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import type { RecordingState } from "./types";
import {
  onStateChanged,
  setBarShape,
  transcribeLivePreview,
  cancelRecording,
  saveBarPosition,
  getBarPosition,
} from "./tauri-commands";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

interface AudioLevelPayload {
  level: number;
}

/** Number of waveform bars. */
const BAR_COUNT = 5;

/** Expanded pill dimensions. */
const PILL_WIDTH = 200;
const PILL_HEIGHT = 36;

// ---------------------------------------------------------------------------
// Inline style reset + keyframes
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

  @keyframes bar-expand {
    from { transform: scale(0.7); opacity: 0; }
    to   { transform: scale(1);   opacity: 1; }
  }

  @keyframes bar-collapse {
    from { transform: scale(1);    opacity: 1; }
    to   { transform: scale(0.85); opacity: 0; }
  }
`;

// Phase offsets per bar, spread evenly.
const BAR_PHASE_DELAYS = [0, 0.2, 0.4, 0.6, 0.8];
const BAR_ANIMATION_DURATION = 600;

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Minimal mic icon as inline SVG. */
function MicIcon() {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ width: 16, height: 16, flexShrink: 0 }}
    >
      {/* Capsule body */}
      <rect
        x="5.5"
        y="1"
        width="5"
        height="8"
        rx="2.5"
        fill="rgba(147,197,253,0.85)"
      />
      {/* Stand arc */}
      <path
        d="M3 7.5a5 5 0 0 0 10 0"
        stroke="rgba(147,197,253,0.85)"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      {/* Stem */}
      <line
        x1="8"
        y1="12.5"
        x2="8"
        y2="15"
        stroke="rgba(147,197,253,0.85)"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      {/* Base */}
      <line
        x1="5.5"
        y1="15"
        x2="10.5"
        y2="15"
        stroke="rgba(147,197,253,0.85)"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** Animated waveform: 5 bars, soft color. */
function Waveform({ levels }: { levels: number[] }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 3,
        height: 20,
        flex: 1,
        minWidth: 0,
      }}
    >
      {Array.from({ length: BAR_COUNT }, (_, i) => {
        const levelIdx = Math.round((i / (BAR_COUNT - 1)) * (levels.length - 1));
        const amplitude = Math.max(0.12, levels[levelIdx] ?? 0);
        const heightPx = Math.max(3, amplitude * 19);
        const delayMs = BAR_PHASE_DELAYS[i] * BAR_ANIMATION_DURATION;
        return (
          <div
            key={i}
            style={{
              flex: 1,
              borderRadius: 9999,
              background: "rgba(147,197,253,0.85)",
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
        width: 13,
        height: 13,
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
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="3"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ width: 11, height: 11, flexShrink: 0 }}
    >
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
        width: 14,
        height: 14,
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
          width: 8,
          height: 8,
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
  const [collapsing, setCollapsing] = useState(false);
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const collapseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Stored logical position of the bar's top-left corner after drags.
  const barX = useRef<number | null>(null);
  const barY = useRef<number | null>(null);

  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "cleaning";
  const isActive = isRecording || isProcessing;
  // The pill is visible when active, showing done flash, or showing an error.
  const isError = state === "error" && !showDone;
  const isPillVisible = isActive || showDone || isError;
  const isIdle = !isPillVisible && !collapsing;

  const isDone = showDone && !isActive;

  // --- Load stored position on mount, fall back to screen-center-bottom ---
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      try {
        const saved = await getBarPosition();
        if (saved) {
          barX.current = saved.x;
          barY.current = saved.y;
        } else {
          // Fallback: compute center-bottom from current window position.
          const pos = await win.outerPosition();
          const scale = (await win.scaleFactor()) || 1;
          barX.current = pos.x / scale;
          barY.current = pos.y / scale;
        }
      } catch {
        // Non-critical: bar will appear wherever Tauri placed it initially.
      }
    })();
  }, []);

  // --- Show / hide the Tauri window based on pill visibility ---
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      if (isPillVisible) {
        // Resize first so the window has correct dimensions before showing.
        await win.setSize(new LogicalSize(PILL_WIDTH, PILL_HEIGHT));
        await setBarShape("pill").catch(() => {});
        if (barX.current != null && barY.current != null) {
          await win.setPosition(new LogicalPosition(barX.current, barY.current));
        }
        await win.show();
      }
      // Hiding is handled by the collapse animation handler below.
    })();
  }, [isPillVisible]);

  // --- Trigger collapse animation then hide ---
  // When the bar transitions from visible to idle we play bar-collapse first.
  const prevIsPillVisible = useRef(isPillVisible);
  useEffect(() => {
    const wasVisible = prevIsPillVisible.current;
    prevIsPillVisible.current = isPillVisible;

    if (wasVisible && !isPillVisible) {
      // Start collapse animation.
      setCollapsing(true);

      if (collapseTimerRef.current) clearTimeout(collapseTimerRef.current);
      collapseTimerRef.current = setTimeout(async () => {
        setCollapsing(false);
        try {
          const win = getCurrentWebviewWindow();
          await win.hide();
        } catch { /* non-critical */ }
      }, 200);
    }
  });

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

  // --- Manual drag via startDragging() + save position after drag ends ---
  function handleMouseDown(e: React.MouseEvent) {
    // Only primary button; don't interfere with the StopButton click.
    if (e.button !== 0) return;
    const win = getCurrentWebviewWindow();
    win.startDragging().then(() => {
      // startDragging() resolves when the drag ends -- save new position.
      win.outerPosition().then(async (pos) => {
        const scale = (await win.scaleFactor()) || 1;
        const lx = pos.x / scale;
        const ly = pos.y / scale;
        barX.current = lx;
        barY.current = ly;
        saveBarPosition(lx, ly).catch(() => {});
      }).catch(() => {});
    }).catch(() => {});
  }

  // ---------------------------------------------------------------------------
  // Render: idle -- window is hidden, render nothing
  // ---------------------------------------------------------------------------

  if (isIdle) {
    return <style>{RESET_CSS}</style>;
  }

  // ---------------------------------------------------------------------------
  // Render: expanded pill (recording / processing / done / error / collapsing)
  // ---------------------------------------------------------------------------

  const accentColor = isRecording ? "#93c5fd"
    : isProcessing ? "#fbbf24"
    : isDone ? "#34d399"
    : "#f87171";

  const borderColor = isRecording ? "rgba(147,197,253,0.25)"
    : isProcessing ? "rgba(245,158,11,0.2)"
    : isDone ? "rgba(52,211,153,0.25)"
    : "rgba(248,113,113,0.2)";

  const pillAnimation = collapsing
    ? "bar-collapse 180ms ease-in forwards"
    : "bar-expand 220ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards";

  return (
    <>
      <style>{RESET_CSS}</style>
      <div
        onMouseDown={handleMouseDown}
        style={{
          width: "100%",
          height: "100%",
          borderRadius: 9999,
          background: "rgba(15,15,18,0.95)",
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
          border: `1px solid ${borderColor}`,
          display: "flex",
          alignItems: "center",
          gap: 6,
          paddingLeft: 10,
          paddingRight: 10,
          cursor: "move",
          fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
          userSelect: "none",
          overflow: "hidden",
          animation: pillAnimation,
        }}
      >

        {/* Mic logo -- always visible as brand anchor */}
        <MicIcon />

        {/* Recording: stop button + waveform or live preview */}
        {isRecording && (
          <>
            <StopButton onClick={() => { cancelRecording().catch(() => {}); }} />
            {livePreview ? (
              <span
                style={{
                  fontSize: 11,
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
              gap: 6,
              flex: 1,
              minWidth: 0,
            }}
          >
            <Spinner color={accentColor} />
            <span
              style={{
                fontSize: 11,
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
              gap: 6,
              flex: 1,
              minWidth: 0,
              animation: "done-pop 280ms cubic-bezier(0.34,1.56,0.64,1) forwards",
            }}
          >
            <CheckIcon color={accentColor} />
            <span style={{ fontSize: 11, color: "#34d399", letterSpacing: "0.01em" }}>Done</span>
          </div>
        )}

        {/* Error */}
        {isError && (
          <span style={{ fontSize: 11, color: "#f87171", flex: 1, letterSpacing: "0.01em" }}>Error</span>
        )}

      </div>
    </>
  );
}
