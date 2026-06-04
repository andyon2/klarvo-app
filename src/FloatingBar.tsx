import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import type { RecordingState, HotkeyMode } from "./types";
import {
  onStateChanged,
  setBarShape,
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

/** Expanded pill dimensions. */
const PILL_WIDTH = 200;
const PILL_WIDTH_CLIPBOARD = 220;
const PILL_HEIGHT = 36;

/** Max-height of the expanded preview panel (logical px). */
const PANEL_MAX_HEIGHT = 160;
/** Width of the bar when the preview panel is open. */
const PANEL_WIDTH = 220;
/** Full height of bar+panel when preview is active. */
const PANEL_HEIGHT = PILL_HEIGHT + PANEL_MAX_HEIGHT; // 196

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
  const [clipboardOnly, setClipboardOnly] = useState(false);
  const [livePreview, setLivePreview] = useState(""); // push sink for klarvo://live-preview-chunk events
  const [collapsing, setCollapsing] = useState(false);
  const [hotkeyMode, setHotkeyMode] = useState<HotkeyMode>("hold");
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const collapseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewPanelRef = useRef<HTMLDivElement>(null);

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

  // --- Load hotkey mode from settings on mount, update on hotkey events ---
  useEffect(() => {
    getSettings()
      .then((s) => setHotkeyMode(s.hotkeyMode))
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

  // --- Live preview push listener (Story 5.2, AC-1, AC-7) ---
  // Backend emits klarvo://live-preview-chunk only when live_preview_enabled == true.
  // Guard: skip empty chunks (AC-7, fail-soft from Story 5.1 AC-8).
  useEffect(() => {
    const unlisten = listen<string>("klarvo://live-preview-chunk", (event) => {
      const chunk = event.payload.trim();
      // Guard: skip empty and whitespace-only chunks (AC-7 hardening — backend
      // filters strictly-empty strings, but leading/trailing whitespace can still
      // produce a blank panel open).
      if (chunk) setLivePreview((prev) => prev ? prev + " " + chunk : chunk);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // --- Auto-scroll preview panel to newest text (AC-2) ---
  useEffect(() => {
    if (previewPanelRef.current) {
      previewPanelRef.current.scrollTop = previewPanelRef.current.scrollHeight;
    }
  }, [livePreview]);

  // --- Show / hide the Tauri window based on pill visibility ---
  const pillWidth = (isDone && clipboardOnly) ? PILL_WIDTH_CLIPBOARD : PILL_WIDTH;

  // Compute active dimensions: expand to PANEL_WIDTH × PANEL_HEIGHT when panel is open (AC-3).
  // isPanelOpen is defined in the render section below; forward-declare equivalent here for the effect.
  const isPanelOpenForEffect = isRecording && livePreview.length > 0;
  const activePillWidth = isPanelOpenForEffect ? PANEL_WIDTH : pillWidth;
  const activePillHeight = isPanelOpenForEffect ? PANEL_HEIGHT : PILL_HEIGHT;

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      if (isPillVisible) {
        // Resize and shape first so the window has correct dimensions before
        // showing (guards against the "white line" bug where the window appears
        // before its shape mask is applied — AC-3 shape-guard ordering preserved:
        // setSize → setBarShape → setPosition → show).
        console.log(`[bar] showing pill: ${activePillWidth}x${activePillHeight}`);
        await win.setSize(new LogicalSize(activePillWidth, activePillHeight));
        // Shape the OS window region to match the actual size: the tall preview
        // "panel" card when open (else the region stays pill-sized and clips the
        // panel + right edge), the "pill" otherwise. Radius matches the CSS below.
        await setBarShape(isPanelOpenForEffect ? "panel" : "pill").catch((e) => console.error("[bar] setBarShape failed:", e));
        // Guard: skip setPosition during an active drag so the window doesn't
        // teleport to stale barX/barY mid-drag when a preview chunk arrives (AC-4).
        if (barX.current != null && barY.current != null && dragRef.current == null) {
          await win.setPosition(new LogicalPosition(barX.current, barY.current));
        }
        try {
          await win.show();
          console.log("[bar] show: success");
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
  // isPanelOpenForEffect (via activePillWidth/activePillHeight) drives panel expand/collapse
  // resize transitions in addition to the original isPillVisible/pillWidth triggers (AC-3).
  }, [isPillVisible, activePillWidth, activePillHeight]);

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
      // Warning is transient — don't change bar state, pipeline continues to "done".
      if (newState === "warning") return;
      setState(newState);

      if (newState === "recording") {
        // Clear any stale preview text from a previous recording so the panel
        // starts clean (covers cancel→idle, error, and any non-done exit path).
        setLivePreview("");
        // Safety net: ensure the bar window is healthy before the user sees
        // recording feedback. Runs fire-and-forget so it never blocks the UI.
        ensureBarWindow().catch((e) => console.error("[bar] pre-recording recovery failed:", e));
      }

      if (newState === "done") {
        // Clear live preview so no panel text lingers after done pop (AC-5, FR7).
        setLivePreview("");
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
      const raw = Math.min(1, event.payload.level * 10);
      const boosted = Math.pow(raw, 0.4);
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
    }
    function onMouseUp() {
      const d = dragRef.current;
      if (!d) return;
      dragRef.current = null;
      // Save final position.
      const win = getCurrentWebviewWindow();
      win.outerPosition().then(async (pos) => {
        const scale = (await win.scaleFactor()) || 1;
        const lx = pos.x / scale;
        const ly = pos.y / scale;
        barX.current = lx;
        barY.current = ly;
        saveBarPosition(lx, ly).catch((e) => console.error("[bar] saveBarPosition failed:", e));
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
    : (isDone && clipboardOnly) ? "#FFA344"
    : isDone ? "#4ADE80"
    : "#FF7369";

  const borderColor = isRecording ? "rgba(42,195,168,0.25)"
    : isProcessing ? "rgba(255,163,68,0.2)"
    : (isDone && clipboardOnly) ? "rgba(255,163,68,0.25)"
    : isDone ? "rgba(74,222,128,0.25)"
    : "rgba(255,115,105,0.2)";

  const pillAnimation = collapsing
    ? "bar-collapse 180ms ease-in forwards"
    : "bar-expand 220ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards";

  // Panel is open when recording and preview text is present (AC-2, AC-3).
  const isPanelOpen = isRecording && livePreview.length > 0;

  return (
    <>
      <style>{RESET_CSS}</style>
      {/* Outer wrapper: flex-column so the pill row and panel stack vertically.
          overflow: "hidden" is moved here so the rounded corners clip both layers.
          animation applies to the whole bar including any open panel. */}
      <div
        onMouseDown={handleMouseDown}
        style={{
          width: "100%",
          height: "100%",
          // Pill = stadium (9999 clamps to half-height = 18). Open panel = rounded
          // CARD (14): a tall stadium would curve the text panel's sides. This value
          // MUST equal the "panel" region radius in set_bar_shape (Rust), else the
          // OS-region vs CSS-shape mismatch shows as the white-line artifact.
          borderRadius: isPanelOpen ? 14 : 9999,
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
        {/* Pill row: logo + content. When the panel is CLOSED the row fills the
            whole (bordered) wrapper via flex:1 so the content stays vertically
            centred exactly like the pre-5.2 single-pill div — a fixed PILL_HEIGHT
            here left the content ~2px low because the wrapper's 1px border shrinks
            its content box below PILL_HEIGHT. When the panel is OPEN the row is
            pinned to PILL_HEIGHT so the panel below gets the remaining height. */}
        <div
          style={{
            flexShrink: 0,
            ...(isPanelOpen ? { height: PILL_HEIGHT } : { flex: 1 }),
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
        </div>

        {/* Preview panel: accumulates live-preview chunks below the pill row (AC-2, UX-DR1).
            Only visible when isPanelOpen (recording + livePreview non-empty).
            The panel sits outside the pill's row so it is never clipped by it.
            Top-fade via mask-image; thin teal scrollbar via CSS Scrollbars spec.
            WebView2 globally hides ::-webkit-scrollbar via RESET_CSS line 43 —
            override locally with a scoped style tag so the panel thumb is visible. */}
        {isPanelOpen && (
          <>
            <style>{`
              #preview-panel::-webkit-scrollbar { display: block !important; width: 4px !important; }
              #preview-panel::-webkit-scrollbar-thumb { background: rgba(42,195,168,0.35); border-radius: 9999px; }
            `}</style>
            <div
              id="preview-panel"
              ref={previewPanelRef}
              style={{
                flex: 1,
                overflowY: "auto",
                overflowX: "hidden",
                padding: "6px 10px",
                fontSize: 11,
                color: "rgba(220,220,220,0.88)",
                lineHeight: 1.5,
                letterSpacing: "0.01em",
                // Top fade for scrolled-off text (UX-DR1)
                WebkitMaskImage: "linear-gradient(to bottom, transparent 0%, black 18%)",
                maskImage: "linear-gradient(to bottom, transparent 0%, black 18%)",
                // Thin scroll indicator via CSS Scrollbars spec (non-WebKit)
                scrollbarWidth: "thin",
                scrollbarColor: "rgba(42,195,168,0.35) transparent",
                // Wrap long unbroken tokens (URLs, identifiers) so they don't
                // clip off the right edge of the panel (AC-7 visual hardening).
                overflowWrap: "anywhere",
              }}
            >
              {livePreview}
            </div>
          </>
        )}
      </div>
    </>
  );
}
