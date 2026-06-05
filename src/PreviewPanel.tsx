// PreviewPanel.tsx — Story 6.2: live preview in the standalone "preview" window.
// This window is transparent, click-through, and always-on-top (created in Story 6.1).
// CSS-only growth: the dark card grows upward inside a fixed-max window — zero per-chunk IPC.

import React, { useState, useEffect, useLayoutEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import { currentMonitor, monitorFromPoint } from "@tauri-apps/api/window";
import {
  getSettings,
  getBarPosition,
  setPreviewShape,
  onStateChanged,
  isPreviewMode,
} from "./tauri-commands";

// ---------------------------------------------------------------------------
// Geometry constants (docs/bar-redesign-spec.md §2)
// ---------------------------------------------------------------------------

const FONT_PX = { small: 11, medium: 13, large: 15 } as const;
const BASE_WIDTH = { compact: 260, comfortable: 320, wide: 400 } as const;
const BASE_MAX_HEIGHT = 600;
const GAP = 8; // px between preview bottom and pill top
const CARD_RADIUS = 14; // matches set_preview_shape + CSS borderRadius (R11)
const PILL_WIDTH = 200;

function previewGeometry(
  widthPreset: string,
  fontSize: "small" | "medium" | "large" = "small",
) {
  const fontPx = FONT_PX[fontSize] ?? FONT_PX.small;
  const k = fontPx / FONT_PX.small;
  const baseW = BASE_WIDTH[widthPreset as keyof typeof BASE_WIDTH] ?? BASE_WIDTH.comfortable;
  return {
    fontPx,
    width: Math.round(baseW * k),
    maxHeight: Math.round(BASE_MAX_HEIGHT * k),
  };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function PreviewPanel(): React.ReactElement {
  const [livePreview, setLivePreview] = useState("");
  const [panelScrolls, setPanelScrolls] = useState(false);

  // Stale-chunk guard (mirrors FloatingBar's isRecordingRef, R1).
  // Inversion (AC-4): remove `if (!isRecordingRef.current) return;` → stale post-done
  // chunk repopulates livePreview, causing text to bleed into the next cycle → RED.
  const isRecordingRef = useRef(false);

  // One-shot gate: set to true after the first-chunk geometry sequence runs,
  // cleared on recording-end so geometry re-runs next cycle.
  const showOnceRef = useRef(false);

  // Max height in logical px set during the show sequence; used for cap/scroll logic.
  const clampedMaxHeightRef = useRef(BASE_MAX_HEIGHT);

  // Saved pill anchor (logical px) — read from getBarPosition at show-time.
  const pillXRef = useRef<number | null>(null);
  const pillYRef = useRef<number | null>(null);

  // Ref for the scrollable card element (auto-scroll + overflow detection).
  const previewPanelRef = useRef<HTMLDivElement>(null);

  // Lifecycle trace (forwarded to Klarvo.log via the console bridge) — proves the
  // preview window's JS actually ran and subscribed. If "[preview] mounted" is
  // absent from the log, the webview never initialized (e.g. zero-size surface).
  useEffect(() => {
    console.log("[preview] PreviewPanel mounted + subscriptions attached");
  }, []);

  // ---------------------------------------------------------------------------
  // show sequence — geometry computed once per cycle (AC-1, AC-2, AC-6)
  // ---------------------------------------------------------------------------
  // Inversion (AC-2/NFR1): moving setSize into the chunk append re-introduces the
  // cold first-expansion / pre-measure clip (R3/R4) — proves the static-window
  // invariant is load-bearing. (smoke-time inversion — no Linux unit test)
  async function runShowSequence() {
    const win = getCurrentWebviewWindow();
    try {
      // Read pill anchor (latest position).
      const saved = await getBarPosition();
      if (saved) {
        pillXRef.current = saved.x;
        pillYRef.current = saved.y;
      } else {
        // Pill position unavailable — skip show this cycle.
        console.warn("[preview] runShowSequence: no pill position, skipping");
        showOnceRef.current = false;
        return;
      }
      const pillX = pillXRef.current!;
      const pillY = pillYRef.current!;

      // Read widthPreset reactively (NOT from mount-time state — separate-window rule).
      // Trap #3: a mount-only getSettings() freezes on app-start value.
      let widthPreset = "comfortable";
      try {
        const s = await getSettings();
        widthPreset = s.previewPanelForm ?? "comfortable";
      } catch (e) {
        console.warn("[preview] getSettings failed, using comfortable:", e);
      }

      const geom = previewGeometry(widthPreset, "small");

      const W = geom.width;

      // Compute clampedMaxHeight + horizontal clamp — both from the same monitor query (AR3).
      // [Review-fix: original code clamped only the vertical axis; horizontal clamp is now
      //  applied in the same monitor block — single IPC call, consistent scale factor.]
      let clampedMaxH = geom.maxHeight;
      const pillCenterX = pillX + PILL_WIDTH / 2;
      let previewLeft = pillCenterX - W / 2; // will be clamped below if monitor available
      try {
        const monitor = await monitorFromPoint(pillX, pillY) ?? await currentMonitor();
        if (monitor) {
          const scale = monitor.scaleFactor || 1;
          const wa = monitor.workArea ?? { position: monitor.position, size: monitor.size };
          // Vertical clamp: how much room is above the pill?
          const workAreaTop = wa.position.y / scale;
          const room = (pillY - GAP) - (workAreaTop + 12);
          clampedMaxH = Math.max(40, Math.min(geom.maxHeight, room));
          // Horizontal clamp: keep preview within [screenLeft+12, screenRight-W-12] (AC-1/AR3).
          const screenLeft = wa.position.x / scale;
          const screenRight = (wa.position.x + wa.size.width) / scale;
          previewLeft = Math.max(screenLeft + 12, Math.min(previewLeft, screenRight - W - 12));
        }
      } catch (e) {
        console.warn("[preview] monitor clamp failed, using unclipped geometry:", e);
      }
      clampedMaxHeightRef.current = clampedMaxH;

      const H = clampedMaxH;
      const previewTop = pillY - GAP - H;

      // Sequence: setSize → set_preview_shape (region) → setPosition → show.
      // Order is CRITICAL: region must be applied after size is set (inner_size() in
      // Rust reads the actual size after setSize returns). Show must be last.
      // Inversion (R11): changing borderRadius to 28 while leaving Rust r=14 → white-line gap → RED.
      await win.setSize(new LogicalSize(W, H));
      await setPreviewShape();
      await win.setPosition(new LogicalPosition(previewLeft, previewTop));
      await win.show();

      console.log(`[preview] shown: ${W}x${H} at (${previewLeft.toFixed(0)}, ${previewTop.toFixed(0)})`);
    } catch (e) {
      console.error("[preview] runShowSequence failed:", e);
      showOnceRef.current = false; // allow retry on next chunk
    }
  }

  // ---------------------------------------------------------------------------
  // Task 4: klarvo://state-changed subscription (AC-5)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const unlisten = onStateChanged((payload) => {
      const s = payload.state as string;
      console.log(`[preview] state-changed: ${s}`);
      if (s === "recording") {
        // Arm for the next cycle.
        isRecordingRef.current = true;
        // Clear stale state from the previous cycle.
        setLivePreview("");
        setPanelScrolls(false);
      } else if (s === "done" || s === "idle" || s === "error") {
        isRecordingRef.current = false;
        showOnceRef.current = false;
        setLivePreview("");
        setPanelScrolls(false);
        // Hide the preview window.
        if (!isPreviewMode) {
          const win = getCurrentWebviewWindow();
          win.hide().catch((e) => console.warn("[preview] hide failed:", e));
        }
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // ---------------------------------------------------------------------------
  // Task 5: klarvo://live-preview-chunk subscription with show-once geometry (AC-1–AC-4)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const unlisten = listen<string>("klarvo://live-preview-chunk", async (event) => {
      // R1 stale-chunk guard (AC-4).
      // Inversion: remove this → stale post-done chunk repopulates livePreview.
      if (!isRecordingRef.current) {
        console.log("[preview] chunk dropped (not recording — R1 guard)");
        return;
      }
      const chunk = event.payload.trim();
      if (!chunk) return;
      if (!showOnceRef.current) {
        console.log("[preview] first chunk received → running show sequence");
      }

      // Append chunk SYNCHRONOUSLY before any await — preserves arrival order on the
      // single-threaded event loop and prevents later chunks (which skip the show gate)
      // from committing their setLivePreview before the first chunk's post-await append.
      // [Review-fix: async-gap — chunk was appended AFTER await runShowSequence(), causing
      //  (a) out-of-order render for concurrent chunks, (b) stale-repopulate if recording ends
      //  mid-await, (c) orphan show() after hide() from the done-handler.]
      setLivePreview((prev) => (prev ? prev + " " + chunk : chunk));

      // Show-once geometry sequence: runs exactly once per recording cycle.
      // No resize/reposition per chunk — NFR1.
      // Inversion: moving setSize here re-introduces R3/R4 cold-expansion clip.
      if (!showOnceRef.current) {
        showOnceRef.current = true;
        await runShowSequence();
        // Post-await re-check: if recording ended while the multi-IPC sequence was in
        // flight, hide the window so it is not left visible/empty after the done-handler.
        if (!isRecordingRef.current) {
          const win = getCurrentWebviewWindow();
          win.hide().catch((e) => console.warn("[preview] post-show hide (stale cycle):", e));
        }
      }
    });
    return () => { unlisten.then((fn) => fn()); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ---------------------------------------------------------------------------
  // Task 7.1: Auto-scroll to newest text (AC-3)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    if (panelScrolls && previewPanelRef.current) {
      previewPanelRef.current.scrollTop = previewPanelRef.current.scrollHeight;
    }
  }, [livePreview, panelScrolls]);

  // ---------------------------------------------------------------------------
  // Task 7.2: Detect overflow — trigger scroll mode when content exceeds cap (AC-3)
  // ---------------------------------------------------------------------------
  useLayoutEffect(() => {
    if (!previewPanelRef.current) return;
    const contentH = previewPanelRef.current.scrollHeight;
    if (contentH > clampedMaxHeightRef.current) {
      setPanelScrolls(true);
    }
  }, [livePreview]);

  // ---------------------------------------------------------------------------
  // Task 9: Render the preview card (CSS-grow, bottom-aligned) (AC-2, AC-3, UX-DR1)
  // ---------------------------------------------------------------------------
  // The outer wrapper is full-window with justifyContent: flex-end so the card
  // sits at the bottom — bottom edge = window bottom = GAP above pill top.
  // The card grows upward as text accumulates; when it fills clampedMaxH the
  // window does not resize — scrolling takes over.
  //
  // R11 invariant: borderRadius: CARD_RADIUS (14) MUST match set_preview_shape's
  // r = (14.0 * scale) as i32 — deviation = white-line corner artifact.
  return (
    <>
      <style>{`
        *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
        html, body, #root {
          width: 100%; height: 100%;
          overflow: hidden !important;
          background: transparent !important;
        }
        ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
      `}</style>
      <div style={{
        width: "100%", height: "100%",
        display: "flex", flexDirection: "column",
        justifyContent: "flex-end", // card grows upward from bottom
      }}>
        {livePreview && (
          <>
            <style>{`
              #preview-card::-webkit-scrollbar { display: block !important; width: 4px !important; }
              #preview-card::-webkit-scrollbar-thumb { background: rgba(42,195,168,0.35); border-radius: 9999px; }
            `}</style>
            <div
              id="preview-card"
              ref={previewPanelRef}
              style={{
                background: "rgba(25,25,25,0.96)",
                backdropFilter: "blur(12px)",
                WebkitBackdropFilter: "blur(12px)",
                border: "1px solid rgba(42,195,168,0.25)",
                borderRadius: CARD_RADIUS,
                overflow: "hidden",
                overflowY: panelScrolls ? "auto" : "hidden",
                // Top-fade when scrolled (oldest lines fade out at top).
                WebkitMaskImage: panelScrolls
                  ? "linear-gradient(to bottom, transparent 0%, black 18%)"
                  : undefined,
                maskImage: panelScrolls
                  ? "linear-gradient(to bottom, transparent 0%, black 18%)"
                  : undefined,
                padding: "8px 14px",
                fontSize: 11,
                lineHeight: 1.5,
                letterSpacing: "0.01em",
                color: "rgba(220,220,220,0.88)",
                fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
                overflowWrap: "anywhere",
                scrollbarWidth: "thin",
                scrollbarColor: "rgba(42,195,168,0.35) transparent",
                userSelect: "none",
                cursor: "default",
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
