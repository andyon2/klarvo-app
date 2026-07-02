import { useState, useEffect, useRef } from "react";
import { listen, emit } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import type { RecordingState, HotkeyMode } from "./types";
import {
  onStateChanged,
  cancelRecording,
  saveBarPosition,
  getBarPosition,
  getSettings,
  ensureBarWindow,
} from "./tauri-commands";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

interface AudioLevelPayload {
  level: number;
}

/** Number of waveform bars. */
const BAR_COUNT = 5;

// Pill dimensions: 200×36 logical px, set once in create_bar_window (Rust).
// The frontend never calls setSize — the window is created at full pill size.

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

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Klarvo brand logo: teal K on dark circle. */
function KlarvoLogo() {
  return (
    <div
      style={{
        width: 24,
        height: 24,
        borderRadius: 6,
        background: "#14B8A6",
        flexShrink: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontWeight: 700,
        fontSize: 14,
        color: "#fff",
        lineHeight: 1,
      }}
    >
      K
    </div>
  );
}

/** Real-time waveform: 5 bars driven by audio level events (~15 Hz). */
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
        return (
          <div
            key={i}
            style={{
              flex: 1,
              borderRadius: 9999,
              background: "rgba(42,195,168,0.85)",
              height: heightPx,
              // No animation or transition: bars respond instantly to the
              // 15 Hz audio-level events from the Rust backend.
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
      data-stop-btn
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

/** Maps HotkeyMode enum to a short display label. */
function hotkeyModeLabel(mode: HotkeyMode): string {
  switch (mode) {
    case "hold":     return "Hold";
    case "toggle":   return "Toggle";
    case "autostop": return "Auto Stop";
    case "auto":     return "Auto";
  }
}

export default function FloatingBar() {
  const [state, setState] = useState<RecordingState>("idle");
  const [levels, setLevels] = useState<number[]>(new Array(20).fill(0));
  const [showDone, setShowDone] = useState(false);
  // AC4: transient one-line status text for a cleanup/STT fallback or
  // degrade (e.g. "⚠ Groq am Limit → lokale Transkription"). Set when the
  // backend emits state="warning"; the pipeline always follows a warn()
  // with a done()/error() shortly after, so no separate timer is needed here.
  const [warningMessage, setWarningMessage] = useState("");
  const [clipboardOnly, setClipboardOnly] = useState(false);
  const [collapsing, setCollapsing] = useState(false);
  const [hotkeyMode, setHotkeyMode] = useState<HotkeyMode>("hold");
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
  // AC4: the pill briefly shows the warning label before the backend's
  // follow-up done()/error() event transitions state again.
  const isWarning = state === "warning";
  const isPillVisible = isActive || showDone || isError || isWarning;
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

  // --- Load hotkey mode from settings on mount ---
  useEffect(() => {
    getSettings()
      .then((s) => {
        setHotkeyMode(s.hotkeyMode);
      })
      .catch((e) => console.warn("[bar] getSettings failed (non-critical):", e));
  }, []);

  // Listen for active-mode events from the hotkey handler so the badge
  // reflects the correct mode when Hotkey 2 fires (which may differ from
  // Hotkey 1's mode loaded above).
  useEffect(() => {
    const unlisten = listen<HotkeyMode>("klarvo://active-mode", (event) => {
      setHotkeyMode(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);


  // --- Show the Tauri window: position + show only ---
  // The bar window is created at 200×36 logical px and never resizes.
  // The pill region is set once at creation in create_bar_window (Rust).
  // No setSize, no setBarShape — just position and show.
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      if (isPillVisible) {
        // Guard: skip setPosition during an active drag so the window doesn't
        // teleport to stale barX/barY mid-drag when a state-changed event arrives.
        if (barX.current != null && barY.current != null && dragRef.current == null) {
          await win.setPosition(new LogicalPosition(barX.current, barY.current));
        }
        try {
          await win.show();
        } catch (e) {
          console.error("[bar] show failed, attempting recovery:", e);
          try {
            const recreated = await ensureBarWindow();
            if (recreated) console.log("[bar] window recreated via recovery");
          } catch (re) {
            console.error("[bar] recovery also failed:", re);
          }
        }
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
          console.log("[bar] hide: success");
        } catch (e) {
          console.error("[bar] hide failed:", e);
        }
      }, 200);
    }
  });

  // --- Backend pipeline events ---
  useEffect(() => {
    const unlisten = onStateChanged((payload) => {
      const newState = payload.state as RecordingState;
      console.log(`[bar] state-changed: ${newState}`); // bridged to Klarvo.log for overlay observability
      // AC4: warning is transient but must be surfaced, not discarded — show
      // the taxonomy message text in the pill, then fall through to the
      // normal "done"/"error" transition the backend always sends next.
      if (newState === "warning") {
        setWarningMessage(payload.warning ?? "");
      }
      setState(newState);

      if (newState === "recording") {
        // Safety net: ensure the bar window is healthy before the user sees
        // recording feedback. Runs fire-and-forget so it never blocks the UI.
        ensureBarWindow().catch((e) => console.error("[bar] pre-recording recovery failed:", e));
      }

      if (newState === "done") {
        const isClipboardOnly = !!payload.clipboardOnly;
        setClipboardOnly(isClipboardOnly);
        setShowDone(true);
        if (doneTimerRef.current) clearTimeout(doneTimerRef.current);
        // ClipboardOnly: show longer (4s) so user notices; normal done: 1.5s
        const doneTimeout = isClipboardOnly ? 4000 : 1500;
        doneTimerRef.current = setTimeout(() => {
          setShowDone(false);
          setClipboardOnly(false);
          // Only transition to idle if we're still in "done" state.
          // In Auto-Loop mode, the next recording cycle may have already
          // started (state = "recording"), and we must not overwrite it.
          setState((prev) => (prev === "done" ? "idle" : prev));
        }, doneTimeout);
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
    const unlisten = listen<AudioLevelPayload>("klarvo://audio-level", (event) => {
      // Scale RMS to visual range. Typical speech RMS is 0.01–0.1.
      // Multiplier of 10 maps 0.1 RMS to full scale (1.0).
      // Power of 0.4 compresses the range so quiet speech is still visible.
      //
      // Noise gate: the 0.4-power compression also amplifies the noise floor
      // (RMS ~0.002 background hum → ~30% bar height = visible idle wiggle). Gate
      // anything below NOISE_FLOOR to a flat bar so steady background noise reads
      // as silence; real speech (RMS ≥ 0.01) sits above it and still scales. This
      // is purely visual — it does not affect the recording/preview activation
      // threshold (which Andi confirmed is fine).
      const NOISE_FLOOR = 0.006;
      const level = event.payload.level;
      const boosted =
        level <= NOISE_FLOOR ? 0 : Math.pow(Math.min(1, level * 10), 0.4);
      setLevels((prev) => [...prev.slice(1), boosted]);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // --- Live preview polling while recording ---
  // Disabled: causes 10-20x Groq API quota usage with no meaningful UX benefit.
  // The waveform provides sufficient recording feedback. Re-enable via a
  // settings flag if needed in the future.
  // useEffect(() => {
  //   if (!isRecording) {
  //     setLivePreview("");
  //     return;
  //   }
  //   const initialDelay = setTimeout(() => {
  //     transcribeLivePreview().then((t) => { if (t) setLivePreview(t); }).catch(() => {});
  //   }, 2000);
  //   const interval = setInterval(() => {
  //     transcribeLivePreview().then((t) => { if (t) setLivePreview(t); }).catch(() => {});
  //   }, 3000);
  //   return () => { clearTimeout(initialDelay); clearInterval(interval); };
  // }, [isRecording]);

  // --- Manual drag via mouse events + setPosition() ---
  // Tauri's startDragging() and data-tauri-drag-region don't work reliably
  // on transparent decorationless WebView2 windows. We implement drag manually.
  const dragRef = useRef<{ startX: number; startY: number; winX: number; winY: number } | null>(null);
  // Story 6.4: pending rAF ID for throttled bar-moved emit — cancel stale before scheduling new.
  const dragRafRef = useRef<number | null>(null);

  function handleMouseDown(e: React.MouseEvent) {
    if (e.button !== 0) return;
    // Don't drag when clicking the StopButton.
    if ((e.target as HTMLElement).closest("[data-stop-btn]")) return;
    const win = getCurrentWebviewWindow();
    win.outerPosition().then(async (pos) => {
      const scale = (await win.scaleFactor()) || 1;
      dragRef.current = {
        startX: e.screenX,
        startY: e.screenY,
        winX: pos.x / scale,
        winY: pos.y / scale,
      };
    }).catch((e) => console.error("[bar] drag init failed:", e));
  }

  useEffect(() => {
    function onMouseMove(e: MouseEvent) {
      const d = dragRef.current;
      if (!d) return;
      const dx = e.screenX - d.startX;
      const dy = e.screenY - d.startY;
      const win = getCurrentWebviewWindow();
      win.setPosition(new LogicalPosition(d.winX + dx, d.winY + dy)).catch((e) => console.warn("[bar] setPosition during drag:", e));
      // Story 6.4 (AC-1): throttled bar-moved emit — one rAF at a time.
      if (dragRafRef.current !== null) cancelAnimationFrame(dragRafRef.current);
      const pillX = d.winX + dx;
      // Pill window top-left IS the pill anchor (window never grows).
      const pillY = d.winY + dy;
      dragRafRef.current = requestAnimationFrame(() => {
        dragRafRef.current = null;
        emit("klarvo://bar-moved", { x: pillX, y: pillY }).catch(
          (e) => console.warn("[bar] bar-moved emit failed:", e)
        );
      });
    }
    function onMouseUp() {
      // Story 6.4 (AC-2, 1.5): cancel any pending rAF before emitting the final settled position.
      if (dragRafRef.current !== null) {
        cancelAnimationFrame(dragRafRef.current);
        dragRafRef.current = null;
      }
      const d = dragRef.current;
      if (!d) return;
      dragRef.current = null;
      // Save final position.
      const win = getCurrentWebviewWindow();
      win.outerPosition().then(async (pos) => {
        const scale = (await win.scaleFactor()) || 1;
        const lx = pos.x / scale;
        const ly = pos.y / scale;
        // Pill window top-left IS the pill anchor (window never grows).
        barX.current = lx;
        barY.current = ly;
        saveBarPosition(lx, ly).catch((e) => console.error("[bar] saveBarPosition failed:", e));
        // Story 6.4 (AC-2): emit the settled position once more so the preview snaps to
        // the exact final anchor (same values as saveBarPosition — no divergence).
        emit("klarvo://bar-moved", { x: lx, y: ly }).catch(
          (e) => console.warn("[bar] bar-moved final emit failed:", e)
        );
      }).catch((e) => console.error("[bar] outerPosition failed:", e));
    }
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, []);

  // ---------------------------------------------------------------------------
  // Render: idle -- window is hidden, render nothing
  // ---------------------------------------------------------------------------

  if (isIdle) {
    return <style>{RESET_CSS}</style>;
  }

  // ---------------------------------------------------------------------------
  // Render: expanded pill (recording / processing / done / error / collapsing)
  // ---------------------------------------------------------------------------

  const accentColor = isRecording ? "#2AC3A8"
    : isProcessing ? "#FFA344"
    : isWarning ? "#FFA344"
    : (isDone && clipboardOnly) ? "#FFA344"
    : isDone ? "#4ADE80"
    : "#FF7369";

  const borderColor = isRecording ? "rgba(42,195,168,0.25)"
    : isProcessing ? "rgba(255,163,68,0.2)"
    : isWarning ? "rgba(255,163,68,0.25)"
    : (isDone && clipboardOnly) ? "rgba(255,163,68,0.25)"
    : isDone ? "rgba(74,222,128,0.25)"
    : "rgba(255,115,105,0.2)";

  const pillAnimation = collapsing
    ? "bar-collapse 180ms ease-in forwards"
    : "bar-expand 220ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards";

  return (
    <>
      <style>{RESET_CSS}</style>
      {/* Outer wrapper: pill shape, fixed size 200×36, never resizes.
          The window is created at 200×36 in create_bar_window (Rust) and the pill
          region is set once at creation — no setSize or setBarShape from the frontend.
          overflow: "hidden" clips content to the rounded corners. */}
      <div
        onMouseDown={handleMouseDown}
        style={{
          width: "100%",
          height: "100%",
          justifyContent: "flex-end",
          borderRadius: 9999,
          background: "rgba(25,25,25,0.96)",
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
          border: `1px solid ${borderColor}`,
          display: "flex",
          flexDirection: "column",
          cursor: "move",
          fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
          userSelect: "none",
          overflow: "hidden",
          animation: pillAnimation,
        }}
      >
        {/* Pill row: logo + content. flex:1 fills the whole bordered wrapper so
            the content stays vertically centred. */}
        <div
          style={{
            flexShrink: 0,
            flex: 1,
            display: "flex",
            alignItems: "center",
            gap: 6,
            paddingLeft: 10,
            paddingRight: 10,
          }}
        >
          {/* Klarvo logo -- always visible as brand anchor */}
          <KlarvoLogo />

          {/* Recording: stop button + waveform + mode badge */}
          {isRecording && (
            <>
              <StopButton onClick={() => { cancelRecording().catch((e) => console.error("[bar] cancelRecording failed:", e)); }} />
              <Waveform levels={levels} />
              <span
                style={{
                  fontSize: 10,
                  color: "#808385",
                  flexShrink: 0,
                  letterSpacing: "0.02em",
                  lineHeight: 1,
                }}
              >
                {hotkeyModeLabel(hotkeyMode)}
              </span>
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
                  color: "#AAACAD",
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

          {/* Done: check icon + label (or clipboard hint) */}
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
              {clipboardOnly ? (
                <>
                  <span style={{ fontSize: 13, flexShrink: 0, lineHeight: 1 }}>📋</span>
                  <span style={{ fontSize: 12, fontWeight: 600, color: "#FFA344", letterSpacing: "0.02em", whiteSpace: "nowrap" }}>
                    In Clipboard
                  </span>
                </>
              ) : (
                <>
                  <CheckIcon color={accentColor} />
                  <span style={{ fontSize: 11, color: "#4ADE80", letterSpacing: "0.01em" }}>Done</span>
                </>
              )}
            </div>
          )}

          {/* Error */}
          {isError && (
            <span style={{ fontSize: 11, color: "#FF7369", flex: 1, letterSpacing: "0.01em" }}>Error</span>
          )}

          {/* AC4: transient fallback/degrade warning — same treatment as Error
              (short label, one line) but amber, not red, and with the
              "Cleaning up..." label's ellipsis handling for a longer string. */}
          {isWarning && (
            <span
              style={{
                fontSize: 11,
                color: "#FFA344",
                flex: 1,
                minWidth: 0,
                letterSpacing: "0.01em",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {warningMessage || "Warning"}
            </span>
          )}
        </div>
      </div>
    </>
  );
}
