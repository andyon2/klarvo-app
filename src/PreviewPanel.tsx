// PreviewPanel.tsx — Story 6.2: live preview in the standalone "preview" window.
// This window is transparent, click-through, and always-on-top (created in Story 6.1).
// CSS-only growth: the dark card grows upward inside a fixed-max window — zero per-chunk IPC.

import React, { useState, useEffect, useLayoutEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import { currentMonitor } from "@tauri-apps/api/window";
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
// CARD_RADIUS constant removed (Story 6.6): replaced by cardAppearance.borderRadius
// (React state, set via setCardAppearance in runShowSequence). Both CSS borderRadius
// and set_preview_shape use the same fresh appr value to satisfy the R11 invariant.
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

  // Story 6.6: appearance driven by React state so the first/single chunk render
  // gets the correct values (ref mutation never triggers a re-render — was the bug).
  // Initialized to the same defaults as the Rust serde defaults.
  const [cardAppearance, setCardAppearance] = useState({
    textColor:    "rgba(220,220,220,0.88)",
    bgColor:      "rgba(25,25,25,0.96)",
    bgBlur:       12,
    borderColor:  "rgba(42,195,168,0.25)",
    borderWidth:  1,
    borderRadius: 14,
    fontFamily:   "'Inter', system-ui, -apple-system, sans-serif",
  });
  // Story 6.3: fontPx from previewGeometry, reactive via runShowSequence.
  // Default = FONT_PX.small = 11. Typed as number to accept 11/13/15.
  const [cardFontPx, setCardFontPx] = useState<number>(FONT_PX.small);

  // Max height in logical px set during the show sequence; used for cap/scroll logic.
  const clampedMaxHeightRef = useRef(BASE_MAX_HEIGHT);

  // Saved pill anchor (logical px) — read from getBarPosition at show-time.
  const pillXRef = useRef<number | null>(null);
  const pillYRef = useRef<number | null>(null);

  // Story 6.4 (AC-3): cached monitor bounds from runShowSequence — reused for drag repositioning.
  // Updated once per show sequence; monitors don't change during a drag (avoids per-drag IPC).
  const cachedMonitorRef = useRef<{
    screenLeft: number;
    screenRight: number;
  } | null>(null);

  // Story 6.4 (AC-3): preview width from the last runShowSequence call — reused in bar-moved handler.
  // Default matches BASE_WIDTH.comfortable so the ref is always safe to dereference.
  const previewWidthRef = useRef<number>(BASE_WIDTH.comfortable);

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

      // Read all reactive settings: widthPreset + appearance (Trap #3 — single getSettings call).
      // NOT from mount-time state — this window never re-mounts when Settings saves.
      let widthPreset = "comfortable";
      // Story 6.3: fontSize from config; declared in outer scope so the catch block
      // can use the fallback without re-declaring.
      let fontSize: "small" | "medium" | "large" = "small";
      // Build appr as a local const so setCardAppearance, setPreviewShape, and the log
      // all use the SAME freshly-read object — no stale ref, no async state lag.
      // String fields use || (empty string "" is falsy → falls back to default).
      // Numeric fields use ?? (0 is a valid value and must not fall back).
      let appr = {
        textColor:    "rgba(220,220,220,0.88)",
        bgColor:      "rgba(25,25,25,0.96)",
        bgBlur:       12,
        borderColor:  "rgba(42,195,168,0.25)",
        borderWidth:  1,
        borderRadius: 14,
        fontFamily:   "'Inter', system-ui, -apple-system, sans-serif",
      };
      try {
        const s = await getSettings();
        widthPreset = s.previewPanelForm ?? "comfortable";
        // Story 6.3: read fontSize from the same getSettings call (Trap #3).
        fontSize = (s.previewFontSize || "small") as "small" | "medium" | "large";
        // Story 6.6: build appr from the just-read settings so CSS and
        // set_preview_shape both use the latest saved values (Finding 1+2).
        appr = {
          textColor:    s.previewTextColor    || "rgba(220,220,220,0.88)",
          bgColor:      s.previewBgColor      || "rgba(25,25,25,0.96)",
          bgBlur:       s.previewBgBlur       ?? 12,
          borderColor:  s.previewBorderColor  || "rgba(42,195,168,0.25)",
          borderWidth:  s.previewBorderWidth  ?? 1,
          borderRadius: s.previewBorderRadius ?? 14,
          fontFamily:   s.previewFontFamily   || "'Inter', system-ui, -apple-system, sans-serif",
        };
      } catch (e) {
        console.warn("[preview] getSettings failed, using defaults:", e);
        // fontSize falls back to "small" (declared above the try block).
      }
      // Drive React state from the fresh appr BEFORE the show sequence so the DOM
      // has the correct appearance by the time show() reveals the window.
      setCardAppearance(appr);

      // Story 6.3: pass fontSize to previewGeometry (replaces the hardcoded "small").
      const geom = previewGeometry(widthPreset, fontSize);
      // Story 6.3: store fontPx in state so the card render uses geom.fontPx (not hardcoded 11).
      setCardFontPx(geom.fontPx);

      const W = geom.width;
      // Story 6.4 (AC-3): persist W for drag repositioning (no re-call of previewGeometry per drag).
      previewWidthRef.current = W;

      // Compute clampedMaxHeight + horizontal clamp — both from the same monitor query (AR3).
      // [Review-fix: original code clamped only the vertical axis; horizontal clamp is now
      //  applied in the same monitor block — single IPC call, consistent scale factor.]
      let clampedMaxH = geom.maxHeight;
      const pillCenterX = pillX + PILL_WIDTH / 2;
      let previewLeft = pillCenterX - W / 2; // will be clamped below if monitor available
      try {
        // NOTE: monitorFromPoint() is incompatible in this Tauri JS version
        // (rejects with "monitor_from_point missing required key x"), which
        // skipped the whole clamp block. currentMonitor() is reliable and the
        // preview always sits next to the pill on the active monitor.
        const monitor = await currentMonitor();
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
          // Story 6.4 (AC-3): cache monitor bounds for drag repositioning (no per-drag IPC).
          cachedMonitorRef.current = { screenLeft, screenRight };
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
      // Inversion (R11): passing a different radius to setPreviewShape vs CSS borderRadius
      // → white-line corner artifact on Windows → proves the coupling is load-bearing.
      await win.setSize(new LogicalSize(W, H));
      await setPreviewShape(appr.borderRadius);
      await win.setPosition(new LogicalPosition(previewLeft, previewTop));
      await win.show();

      console.log(`[preview] shown: ${W}x${H} at (${previewLeft.toFixed(0)}, ${previewTop.toFixed(0)}) appearance={textColor:${appr.textColor},bgColor:${appr.bgColor},bgBlur:${appr.bgBlur},borderColor:${appr.borderColor},borderWidth:${appr.borderWidth},borderRadius:${appr.borderRadius},fontFamily:${appr.fontFamily}}`);
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
    unlisten.then(() => console.log("[preview] state-changed listener REGISTERED"));
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
    unlisten.then(() => console.log("[preview] live-preview-chunk listener REGISTERED"));
    return () => { unlisten.then((fn) => fn()); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ---------------------------------------------------------------------------
  // Story 6.4: klarvo://bar-moved — reposition preview when pill is dragged (AC-3, AC-4)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const unlisten = listen<{ x: number; y: number }>("klarvo://bar-moved", (event) => {
      // AC-4: only reposition when the preview is currently showing.
      // Inversion (AC-4 dev-time): remove this guard → setPosition fires on a hidden window
      // → log shows spurious bar-moved setPosition calls before first show → RED.
      if (!showOnceRef.current) return;

      const { x: pillX, y: pillY } = event.payload;
      // Update stored pill anchor so future runShowSequence calls (next cycle) use the
      // dragged-to position, not the position at recording start.
      pillXRef.current = pillX;
      pillYRef.current = pillY;

      const W = previewWidthRef.current;
      const H = clampedMaxHeightRef.current;
      const pillCenterX = pillX + PILL_WIDTH / 2;
      let previewLeft = pillCenterX - W / 2;

      // Apply horizontal screen clamp from cached monitor bounds — no new IPC per drag event.
      const m = cachedMonitorRef.current;
      if (m) {
        previewLeft = Math.max(m.screenLeft + 12, Math.min(previewLeft, m.screenRight - W - 12));
      }
      const previewTop = pillY - GAP - H;

      // setPosition ONLY — no setSize, no setPreviewShape, no show() (NFR1 preserved).
      // Inversion (AC-3 smoke-time): remove this call → preview stays at show-time position
      // while pill moves → RED.
      const win = getCurrentWebviewWindow();
      win.setPosition(new LogicalPosition(previewLeft, previewTop)).catch(
        (e) => console.warn("[preview] bar-moved setPosition failed:", e)
      );
    });
    unlisten.then(() => console.log("[preview] bar-moved listener REGISTERED"));
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
    if (contentH > previewPanelRef.current.clientHeight + 1) {
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
  // R11 invariant: borderRadius from cardAppearance state MUST match the radius
  // passed to set_preview_shape (both derived from the same fresh appr object in
  // runShowSequence). Story 6.6 replaced the old CARD_RADIUS=14 hardcode.
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
        // Story 6.6 border-clip fix: the card is stretch-aligned + bottom-anchored, so
        // without this inset its right/bottom border sits exactly on the window content
        // boundary and gets clipped at fractional DPI (objectively measured: left ~79 teal
        // px, right ~10, bottom ~16). A small uniform pad keeps all four borders inside the
        // window — and inside the set_preview_shape region — so they render in full. Uniform
        // (not right/bottom-only) preserves horizontal centering on the pill and avoids any
        // card/region corner coincidence (the R11 white-line case). ~2px is imperceptible
        // vs the pill but >1 physical px at every DPI scale we see (dpr 1.25–2.0).
        padding: 2,
      }}>
        {livePreview && (
          <>
            {/* No visible scrollbar: the window is click-through (set_ignore_cursor_events),
                so the wheel acts on the app behind — a scrollbar would be a false affordance.
                Overflow is handled by auto-scroll-to-newest (Task 7.1) + the top-fade mask
                below, so the latest text is always in view without any manual scroll. The
                global `::-webkit-scrollbar { display:none }` rule keeps it hidden. */}
            <div
              id="preview-card"
              ref={previewPanelRef}
              style={{
                // Story 6.6: all seven appearance values come from cardAppearance state
                // (set via setCardAppearance(appr) in runShowSequence before show()).
                // State-driven: setCardAppearance triggers a re-render, so the DOM is
                // correct on the first/single chunk — ref mutation never would have done that.
                background: cardAppearance.bgColor,
                backdropFilter: `blur(${cardAppearance.bgBlur}px)`,
                WebkitBackdropFilter: `blur(${cardAppearance.bgBlur}px)`,
                border: `${cardAppearance.borderWidth}px solid ${cardAppearance.borderColor}`,
                // R11 invariant: borderRadius MUST equal the radius passed to setPreviewShape.
                // Inversion (smoke-time): set CSS borderRadius to a different value than the
                // Rust region radius → white-line corner artifact → RED.
                borderRadius: cardAppearance.borderRadius,
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
                // Story 6.3: fontPx from previewGeometry via cardFontPx state (replaces hardcoded 11).
                fontSize: cardFontPx,
                lineHeight: 1.5,
                letterSpacing: "0.01em",
                color: cardAppearance.textColor,
                fontFamily: cardAppearance.fontFamily,
                overflowWrap: "anywhere",
                scrollbarWidth: "none", // hidden — see comment above (click-through = no manual scroll)
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
