import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import type { RecordingState } from "./types";
import { onStateChanged, setBarShape, transcribeLivePreview } from "./tauri-commands";

interface AudioLevelPayload {
  level: number;
}

const BAR_SEGMENTS = 32;
const IDLE_SIZE = 28;
const EXPANDED_WIDTH = 260;
const EXPANDED_HEIGHT = 40;

export default function FloatingBar() {
  const [state, setState] = useState<RecordingState>("idle");
  const [levels, setLevels] = useState<number[]>(new Array(BAR_SEGMENTS).fill(0));
  const [showDone, setShowDone] = useState(false);
  const [livePreview, setLivePreview] = useState("");
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const screenCenterX = useRef<number | null>(null);
  const baseY = useRef<number | null>(null);

  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "cleaning";
  const isActive = isRecording || isProcessing;
  const isIdle = state === "idle" && !showDone;

  // Compute screen center from the initial bar position (set by backend).
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      try {
        const pos = await win.outerPosition();
        const scale = (await win.scaleFactor()) || 1;
        screenCenterX.current = pos.x / scale + IDLE_SIZE / 2;
        baseY.current = pos.y / scale;
      } catch { /* fallback */ }
    })();
  }, []);

  // Resize and reposition: always centered on screenCenterX.
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      const cx = screenCenterX.current;
      const by = baseY.current;

      if (isActive || showDone) {
        await win.setSize(new LogicalSize(EXPANDED_WIDTH, EXPANDED_HEIGHT));
        await setBarShape("pill").catch(() => {});
        if (cx != null && by != null) {
          await win.setPosition(new LogicalPosition(
            cx - EXPANDED_WIDTH / 2,
            by - (EXPANDED_HEIGHT - IDLE_SIZE) / 2,
          ));
        }
      } else {
        await win.setSize(new LogicalSize(IDLE_SIZE, IDLE_SIZE));
        await setBarShape("circle").catch(() => {});
        if (cx != null && by != null) {
          await win.setPosition(new LogicalPosition(cx - IDLE_SIZE / 2, by));
        }
      }
    })();
  }, [isActive, showDone]);

  // Subscribe to backend pipeline events.
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
      } else if (newState === "idle" || newState === "error") {
        setLevels(new Array(BAR_SEGMENTS).fill(0));
        if (newState === "error") {
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

  // Poll live preview transcription every 3s while recording.
  useEffect(() => {
    if (!isRecording) {
      setLivePreview("");
      return;
    }
    // Wait 2s before first preview to accumulate audio
    const initialDelay = setTimeout(() => {
      transcribeLivePreview().then(t => { if (t) setLivePreview(t); }).catch(() => {});
    }, 2000);
    const interval = setInterval(() => {
      transcribeLivePreview().then(t => { if (t) setLivePreview(t); }).catch(() => {});
    }, 3000);
    return () => { clearTimeout(initialDelay); clearInterval(interval); };
  }, [isRecording]);

  /*
   * All rendering uses inline styles to avoid Tailwind interference.
   * The outermost div MUST fill 100% of the window to prevent WebView2
   * from showing its background color at the edges.
   */

  if (isIdle) {
    // Solid black circle, fills the entire 28x28 window
    return (
      <>
        <style>{`
          *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
          html, body, #root { width: 100%; height: 100%; overflow: hidden !important; background: transparent !important; }
          ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
        `}</style>
        <div
          data-tauri-drag-region
          style={{
            width: "100%",
            height: "100%",
            borderRadius: "50%",
            background: "#000",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "move",
            overflow: "hidden",
          }}
        >
          <div style={{ width: 8, height: 8, borderRadius: "50%", background: "#10b981" }} />
        </div>
      </>
    );
  }

  // Expanded pill bar, fills the entire 260x40 window
  return (
    <div
      data-tauri-drag-region
      style={{
        width: "100%",
        height: "100%",
        borderRadius: 9999,
        background: "#000",
        display: "flex",
        alignItems: "center",
        gap: 8,
        paddingLeft: 12,
        paddingRight: 12,
        cursor: "move",
        fontFamily: "'Inter', system-ui, sans-serif",
        userSelect: "none",
        overflow: "hidden",
        border: isRecording
          ? "1px solid rgba(239,68,68,0.3)"
          : isProcessing
          ? "1px solid rgba(245,158,11,0.2)"
          : showDone
          ? "1px solid rgba(16,185,129,0.3)"
          : "1px solid rgba(63,63,70,0.3)",
        boxShadow: isRecording
          ? "0 0 20px rgba(239,68,68,0.2)"
          : isProcessing
          ? "0 0 15px rgba(245,158,11,0.15)"
          : showDone
          ? "0 0 15px rgba(16,185,129,0.2)"
          : "none",
      }}
    >
      {/* Status dot */}
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          flexShrink: 0,
          background: isRecording ? "#ef4444"
            : isProcessing ? "#fbbf24"
            : showDone ? "#34d399"
            : state === "error" ? "#ef4444"
            : "#10b981",
          animation: (isRecording || isProcessing) ? "pulse 2s infinite" : "none",
        }}
      />

      {/* Waveform or live preview */}
      {isRecording && (
        livePreview ? (
          <span style={{
            fontSize: 11,
            color: "#a1a1aa",
            flex: 1,
            minWidth: 0,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            direction: "rtl",
            textAlign: "left",
          }}>
            {livePreview}
          </span>
        ) : (
          <div style={{ display: "flex", alignItems: "center", gap: 1.5, height: 24, flex: 1, minWidth: 0 }}>
            {levels.map((level, i) => (
              <div
                key={i}
                style={{
                  width: 2.5,
                  borderRadius: 9999,
                  background: "#34d399",
                  height: `${Math.max(2, level * 22)}px`,
                  opacity: 0.3 + level * 0.7,
                  transition: "all 75ms",
                }}
              />
            ))}
          </div>
        )
      )}

      {/* Processing spinner */}
      {isProcessing && (
        <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
          <svg
            style={{ width: 14, height: 14, color: "#fbbf24", flexShrink: 0, animation: "spin 1s linear infinite" }}
            viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"
          >
            <circle cx="12" cy="12" r="10" strokeOpacity="0.2" />
            <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
          </svg>
          <span style={{ fontSize: 11, color: "#a1a1aa", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {state === "transcribing" ? "Transcribing..." : "Cleaning..."}
          </span>
        </div>
      )}

      {showDone && !isProcessing && !isRecording && (
        <span style={{ fontSize: 11, color: "#34d399", flex: 1 }}>Done</span>
      )}

      {state === "error" && !showDone && (
        <span style={{ fontSize: 11, color: "#f87171", flex: 1 }}>Error</span>
      )}

      <style>{`
        @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.5} }
        @keyframes spin { from{transform:rotate(0)} to{transform:rotate(360deg)} }
        *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
        html, body, #root { width: 100%; height: 100%; overflow: hidden !important; background: transparent !important; }
        ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
      `}</style>
    </div>
  );
}
