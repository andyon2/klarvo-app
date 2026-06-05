import { useState, useEffect, useCallback, useRef } from "react";
import type { HotkeyMode, AppSettings } from "../../types";
import { isDesktop } from "../../platform";
import { LABEL_CLS } from "../ui";

// --- Shortcut Recorder -------------------------------------------------------

function ShortcutRecorder({ value, onChange }: { value: string; onChange: (s: string) => void }) {
  const [listening, setListening] = useState(false);
  // Track which modifier keys are currently held. This is needed for the
  // keyup-based Alt fallback: when Alt is the last key released, e.altKey is
  // already false in the keyup event, so we cannot derive the modifier state
  // from the event alone.
  const heldModifiers = useRef<Set<string>>(new Set());
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cancel = useCallback(() => {
    heldModifiers.current.clear();
    setListening(false);
  }, []);

  // Pause/resume the global hotkey while the recorder is listening,
  // so pressing the current shortcut doesn't trigger the pipeline.
  useEffect(() => {
    if (listening) {
      import("@tauri-apps/api/core")
        .then(({ invoke: inv }) => inv("set_hotkey_paused", { paused: true }))
        .catch(console.error);
    }
    return () => {
      if (listening) {
        import("@tauri-apps/api/core")
          .then(({ invoke: inv }) => inv("set_hotkey_paused", { paused: false }))
          .catch(console.error);
      }
    };
  }, [listening]);

  // Auto-cancel after 5 seconds so the button never stays stuck.
  useEffect(() => {
    if (!listening) return;
    timeoutRef.current = setTimeout(cancel, 5000);
    return () => {
      if (timeoutRef.current !== null) clearTimeout(timeoutRef.current);
    };
  }, [listening, cancel]);

  useEffect(() => {
    if (!listening) return;

    const KEY_MAP: Record<string, string> = {
      " ": "space", Enter: "enter", Tab: "tab",
      Backspace: "backspace", Delete: "delete", Insert: "insert",
      Home: "home", End: "end", PageUp: "pageup", PageDown: "pagedown",
      ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
    };
    const MODIFIERS = new Set(["Control", "Shift", "Alt", "Meta"]);

    const buildParts = (ctrl: boolean, shift: boolean, alt: boolean, meta: boolean, rawKey: string): string | null => {
      const parts: string[] = [];
      if (ctrl) parts.push("ctrl");
      if (shift) parts.push("shift");
      if (alt) parts.push("alt");
      if (meta) parts.push("super");
      if (parts.length === 0) return null;
      let key = KEY_MAP[rawKey] ?? rawKey.toLowerCase();
      if (/^F\d+$/.test(rawKey)) key = rawKey.toLowerCase();
      parts.push(key);
      return parts.join("+");
    };

    const keydownHandler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Escape cancels listening regardless of modifiers.
      if (e.key === "Escape") {
        cancel();
        return;
      }

      // Track held modifiers for the keyup fallback.
      if (MODIFIERS.has(e.key)) {
        heldModifiers.current.add(e.key);
        return;
      }

      // Normal path: at least one modifier must be held.
      const combo = buildParts(e.ctrlKey, e.shiftKey, e.altKey, e.metaKey, e.key);
      if (combo === null) return;

      onChange(combo);
      cancel();
    };

    // keyup fallback for Alt-based shortcuts on Windows. WebView2 sometimes
    // swallows keydown events that include Alt before JS sees them (the browser
    // treats Alt as "focus the menu bar"). The keyup event is more reliably
    // delivered. We only use this path when the keydown handler did NOT already
    // commit a combo (i.e. listening is still true when keyup fires).
    //
    // Caveat: on keyup the modifier flags already reflect the *released* state,
    // so e.altKey is false when Alt itself is being released. We therefore fall
    // back to heldModifiers to reconstruct the held set at the moment the
    // non-modifier key was pressed.
    const keyupHandler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (MODIFIERS.has(e.key)) {
        heldModifiers.current.delete(e.key);
        return;
      }

      // Only engage if this is an Alt-combo that keydown might have missed.
      if (!heldModifiers.current.has("Alt")) return;

      const ctrl = heldModifiers.current.has("Control") || e.ctrlKey;
      const shift = heldModifiers.current.has("Shift") || e.shiftKey;
      const alt = true; // we know Alt is/was held
      const meta = heldModifiers.current.has("Meta") || e.metaKey;

      const combo = buildParts(ctrl, shift, alt, meta, e.key);
      if (combo === null) return;

      onChange(combo);
      cancel();
    };

    document.addEventListener("keydown", keydownHandler, true);
    document.addEventListener("keyup", keyupHandler, true);
    return () => {
      document.removeEventListener("keydown", keydownHandler, true);
      document.removeEventListener("keyup", keyupHandler, true);
      heldModifiers.current.clear();
    };
  }, [listening, onChange, cancel]);

  return (
    <button
      type="button"
      onClick={() => setListening(true)}
      onBlur={cancel}
      className={[
        "w-full bg-klarvo-bg border rounded-lg px-3 py-2 text-sm text-left font-mono",
        listening
          ? "border-klarvo-primary/50 text-klarvo-primary animate-pulse"
          : "border-klarvo-border/50 text-klarvo-text hover:border-klarvo-border-active",
        "focus:outline-none transition-all duration-150",
      ].join(" ")}
    >
      {listening ? "Press shortcut... (Esc to cancel)" : value || "Click to set"}
    </button>
  );
}

// --- Props -------------------------------------------------------------------

export interface ShortcutsContentProps {
  // Desktop hotkey slot 1
  localHotkey: string;
  setLocalHotkey: (v: string) => void;
  localHotkeyMode: HotkeyMode;
  setLocalHotkeyMode: (v: HotkeyMode) => void;
  // Desktop hotkey slot 2
  localHotkeySlot2: string;
  setLocalHotkeySlot2: (v: string) => void;
  localHotkeyModeSlot2: HotkeyMode;
  setLocalHotkeyModeSlot2: (v: HotkeyMode) => void;
  // Shared silence duration (autostop / auto modes)
  localSilenceSecs: number;
  setLocalSilenceSecs: (v: number) => void;
  // Insert & Send toggles per slot
  localInsertAndSendSlot1: boolean;
  setLocalInsertAndSendSlot1: (v: boolean | ((prev: boolean) => boolean)) => void;
  localInsertAndSendSlot2: boolean;
  setLocalInsertAndSendSlot2: (v: boolean | ((prev: boolean) => boolean)) => void;
  // Hotkey tab state
  hotkeyTab: 0 | 1;
  setHotkeyTab: (v: 0 | 1) => void;
  // Mobile bubble tab state
  bubbleTab: 0 | 1;
  setBubbleTab: (v: 0 | 1) => void;
  // Mobile bubble tap controls
  localBubbleTapMode: HotkeyMode;
  setLocalBubbleTapMode: (v: HotkeyMode) => void;
  localBubbleTapAutoSend: boolean;
  setLocalBubbleTapAutoSend: (v: boolean) => void;
  localBubbleTapSilenceSecs: number;
  setLocalBubbleTapSilenceSecs: (v: number) => void;
  // Mobile bubble long-press controls
  localBubbleLongPressMode: HotkeyMode;
  setLocalBubbleLongPressMode: (v: HotkeyMode) => void;
  localBubbleLongPressAutoSend: boolean;
  setLocalBubbleLongPressAutoSend: (v: boolean) => void;
  localBubbleLongPressSilenceSecs: number;
  setLocalBubbleLongPressSilenceSecs: (v: number) => void;
  // Silence threshold (advanced audio setting)
  localSilenceThreshold: number;
  // Loaded settings (needed to restore defaults when switching modes)
  loadedSettings: AppSettings | null;
  // Hotkey change handlers (propagate up for dirty tracking)
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  isPaid: boolean;
  // Paste & Behavior (from AdvancedSettings, displayed here)
  localAutoPaste: boolean;
  setLocalAutoPaste: (v: boolean) => void;
  localPasteDelayMs: number;
  setLocalPasteDelayMs: (v: number) => void;
  localAutoCapitalize: boolean;
  setLocalAutoCapitalize: (v: boolean) => void;
  // Live Preview toggle (desktop only)
  localLivePreviewEnabled: boolean;
  setLocalLivePreviewEnabled: (v: boolean) => void;
  localPreviewPauseSilenceSecs: number;
  setLocalPreviewPauseSilenceSecs: (v: number) => void;
}

// --- Component ---------------------------------------------------------------

export function ShortcutsContent({
  localHotkey, setLocalHotkey, localHotkeyMode, setLocalHotkeyMode,
  localHotkeySlot2, setLocalHotkeySlot2, localHotkeyModeSlot2, setLocalHotkeyModeSlot2,
  localSilenceSecs, setLocalSilenceSecs,
  localInsertAndSendSlot1, setLocalInsertAndSendSlot1,
  localInsertAndSendSlot2: _localInsertAndSendSlot2, setLocalInsertAndSendSlot2: _setLocalInsertAndSendSlot2,
  hotkeyTab, setHotkeyTab,
  bubbleTab, setBubbleTab,
  localBubbleTapMode, setLocalBubbleTapMode,
  localBubbleTapSilenceSecs, setLocalBubbleTapSilenceSecs,
  localBubbleLongPressMode, setLocalBubbleLongPressMode,
  localBubbleLongPressSilenceSecs, setLocalBubbleLongPressSilenceSecs,
  loadedSettings, onHotkeyChange, onHotkeyModeChange,
  localAutoPaste, setLocalAutoPaste, localPasteDelayMs, setLocalPasteDelayMs, localAutoCapitalize, setLocalAutoCapitalize,
  localLivePreviewEnabled, setLocalLivePreviewEnabled, localPreviewPauseSilenceSecs, setLocalPreviewPauseSilenceSecs,
}: ShortcutsContentProps) {

  const handleHotkeyChange = useCallback((h: string) => {
    setLocalHotkey(h);
    onHotkeyChange(h);
  }, [setLocalHotkey, onHotkeyChange]);

  const handleHotkeyModeChange = useCallback((m: HotkeyMode) => {
    setLocalHotkeyMode(m);
    onHotkeyModeChange(m);
  }, [setLocalHotkeyMode, onHotkeyModeChange]);

  return (
    <>
      {/* --- Hotkey -- desktop only --- */}
      {isDesktop && (
        <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
          {/* Tab bar */}
          <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 self-start">
            <button
              onClick={() => setHotkeyTab(0)}
              className={[
                "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                hotkeyTab === 0 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
              ].join(" ")}
            >
              Hotkey 1
            </button>
            <button
              onClick={() => setHotkeyTab(1)}
              className={[
                "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                hotkeyTab === 1 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
              ].join(" ")}
            >
              Hotkey 2
            </button>
          </div>

          {/* Tab 1: Hotkey 1 */}
          {hotkeyTab === 0 && (
            <>
              <div className="flex flex-col gap-1.5">
                <span className="text-xs text-klarvo-muted">Shortcut</span>
                <ShortcutRecorder value={localHotkey} onChange={handleHotkeyChange} />
              </div>

              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>Mode</span>
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                  {([
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                    <button
                      key={value}
                      onClick={() => {
                        handleHotkeyModeChange(value);
                        // When switching modes, load the appropriate silence default from persisted settings
                        if (value === "auto") {
                          setLocalSilenceSecs(loadedSettings?.autoModeSilenceSecs ?? 2.0);
                        } else if (value === "autostop") {
                          setLocalSilenceSecs(loadedSettings?.autostopSilenceSecs ?? 2.0);
                        }
                      }}
                      title={tooltip}
                      className={[
                        "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                        localHotkeyMode === value
                          ? "bg-klarvo-primary/15 text-klarvo-primary"
                          : "text-klarvo-dim hover:text-klarvo-muted",
                      ].join(" ")}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
              <p className="text-[11px] text-klarvo-muted">
                {localHotkeyMode === "hold" && "Hold to record, release to process"}
                {localHotkeyMode === "toggle" && "Press once to start, press again to stop"}
                {localHotkeyMode === "autostop" && "Press to start, stops automatically on silence"}
                {localHotkeyMode === "auto" && "Continuous — restarts after each silence gap"}
              </p>

              {(localHotkeyMode === "autostop" || localHotkeyMode === "auto") && (
                <>
                  <div className="flex flex-col gap-1.5">
                    <div className="flex items-center justify-between">
                      <span className={LABEL_CLS}>Silence Duration</span>
                      <span className="text-xs font-mono text-klarvo-primary">{localSilenceSecs.toFixed(1)}s</span>
                    </div>
                    <input
                      type="range"
                      min={1.0}
                      max={5.0}
                      step={0.1}
                      value={localSilenceSecs}
                      onChange={(e) => setLocalSilenceSecs(parseFloat(e.target.value))}
                      className="w-full accent-klarvo-primary"
                    />
                    <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                  </div>

                </>
              )}

            </>
          )}

          {/* Tab 2: Hotkey 2 */}
          {hotkeyTab === 1 && (
            <>
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-klarvo-muted">Shortcut</span>
                  {localHotkeySlot2 && (
                    <button
                      type="button"
                      onClick={() => setLocalHotkeySlot2("")}
                      className="text-[11px] text-klarvo-dim hover:text-klarvo-muted transition-colors"
                    >
                      Clear
                    </button>
                  )}
                </div>
                {localHotkeySlot2 ? (
                  <ShortcutRecorder value={localHotkeySlot2} onChange={setLocalHotkeySlot2} />
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-klarvo-dim italic">Not set</span>
                    <ShortcutRecorder value="" onChange={setLocalHotkeySlot2} />
                  </div>
                )}
              </div>

              {localHotkeySlot2 && (
                <div className="flex flex-col gap-1.5">
                  <span className={LABEL_CLS}>Mode</span>
                  <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                    {([
                      { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                      { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                      { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                      { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                    ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                      <button
                        key={value}
                        onClick={() => setLocalHotkeyModeSlot2(value)}
                        title={tooltip}
                        className={[
                          "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                          localHotkeyModeSlot2 === value
                            ? "bg-klarvo-primary/15 text-klarvo-primary"
                            : "text-klarvo-dim hover:text-klarvo-muted",
                        ].join(" ")}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                  <p className="text-[11px] text-klarvo-dim">
                    {localHotkeyModeSlot2 === "hold" && "Hold to record, release to process"}
                    {localHotkeyModeSlot2 === "toggle" && "Press once to start, press again to stop"}
                    {localHotkeyModeSlot2 === "autostop" && "Press to start, stops automatically on silence"}
                    {localHotkeyModeSlot2 === "auto" && "Continuous — restarts after each silence gap"}
                  </p>

                  {(localHotkeyModeSlot2 === "autostop" || localHotkeyModeSlot2 === "auto") && (
                    <>
                      <div className="flex flex-col gap-1.5 mt-1">
                        <div className="flex items-center justify-between">
                          <span className={LABEL_CLS}>Silence Duration</span>
                          <span className="text-xs font-mono text-klarvo-primary">{localSilenceSecs.toFixed(1)}s</span>
                        </div>
                        <input
                          type="range"
                          min={1.0}
                          max={5.0}
                          step={0.1}
                          value={localSilenceSecs}
                          onChange={(e) => setLocalSilenceSecs(parseFloat(e.target.value))}
                          className="w-full accent-klarvo-primary"
                        />
                        <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                      </div>

                    </>
                  )}
                </div>
              )}

            </>
          )}

          {/* --- Live Preview --- */}
          <div className="flex flex-col gap-3 border-t border-klarvo-border/30 pt-3 mt-1">
            <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Live Preview</span>
            <div className="flex items-center justify-between gap-3">
              <div className="flex flex-col gap-0.5">
                <span className={LABEL_CLS}>Live Preview</span>
                <span className="text-[11px] text-klarvo-muted">Show raw transcription while you dictate in Toggle/Hold mode.</span>
              </div>
              <button
                role="switch"
                aria-checked={localLivePreviewEnabled}
                onClick={() => setLocalLivePreviewEnabled(!localLivePreviewEnabled)}
                className={[
                  "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
                  localLivePreviewEnabled ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
                ].join(" ")}
              >
                <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", localLivePreviewEnabled ? "translate-x-4" : ""].join(" ")} />
              </button>
            </div>
            {localLivePreviewEnabled && (
              <div className="flex flex-col gap-1.5">
                <div className="flex items-center justify-between">
                  <span className={LABEL_CLS}>Preview Pause</span>
                  <span className="text-xs font-mono text-klarvo-primary">{localPreviewPauseSilenceSecs.toFixed(1)}s</span>
                </div>
                <input
                  type="range"
                  min={0.5}
                  max={5.0}
                  step={0.1}
                  value={localPreviewPauseSilenceSecs}
                  onChange={(e) => setLocalPreviewPauseSilenceSecs(parseFloat(e.target.value))}
                  className="w-full accent-klarvo-primary"
                />
                <p className="text-[11px] text-klarvo-muted">Short = more responsive, more Groq calls, less context per segment. Long = less responsive, fewer calls, better context.</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* --- Bubble Controls -- mobile only --- */}
      {!isDesktop && (
        <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
          {/* Tab bar: Tap / Long Press */}
          <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 self-start">
            <button
              onClick={() => setBubbleTab(0)}
              className={[
                "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                bubbleTab === 0 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
              ].join(" ")}
            >
              Tap
            </button>
            <button
              onClick={() => setBubbleTab(1)}
              className={[
                "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                bubbleTab === 1 ? "bg-klarvo-primary/15 text-klarvo-primary" : "text-klarvo-dim hover:text-klarvo-muted",
              ].join(" ")}
            >
              Long Press
            </button>
          </div>

          {/* Tab 0: Tap */}
          {bubbleTab === 0 && (
            <>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>Mode</span>
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                  {([
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                    <button
                      key={value}
                      onClick={() => setLocalBubbleTapMode(value)}
                      title={tooltip}
                      className={[
                        "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                        localBubbleTapMode === value
                          ? "bg-klarvo-primary/15 text-klarvo-primary"
                          : "text-klarvo-dim hover:text-klarvo-muted",
                      ].join(" ")}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
              <p className="text-[11px] text-klarvo-dim">
                {localBubbleTapMode === "hold" && "Hold to record, release to process"}
                {localBubbleTapMode === "toggle" && "Press once to start, press again to stop"}
                {localBubbleTapMode === "autostop" && "Press to start, stops automatically on silence"}
                {localBubbleTapMode === "auto" && "Continuous — restarts after each silence gap"}
              </p>

              {(localBubbleTapMode === "autostop" || localBubbleTapMode === "auto") && (
                <div className="flex flex-col gap-1.5">
                  <div className="flex items-center justify-between">
                    <span className={LABEL_CLS}>Silence Duration</span>
                    <span className="text-xs font-mono text-klarvo-primary">{localBubbleTapSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <input
                    type="range"
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localBubbleTapSilenceSecs}
                    onChange={(e) => setLocalBubbleTapSilenceSecs(parseFloat(e.target.value))}
                    className="w-full accent-klarvo-primary"
                  />
                  <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                </div>
              )}

              {/* Insert & Send hidden on Android — Enter key rarely works in mobile apps */}
            </>
          )}

          {/* Tab 1: Long Press */}
          {bubbleTab === 1 && (
            <>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>Mode</span>
                <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                  {([
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ] as { value: HotkeyMode; label: string; tooltip: string }[]).map(({ value, label, tooltip }) => (
                    <button
                      key={value}
                      onClick={() => setLocalBubbleLongPressMode(value)}
                      title={tooltip}
                      className={[
                        "px-2.5 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                        localBubbleLongPressMode === value
                          ? "bg-klarvo-primary/15 text-klarvo-primary"
                          : "text-klarvo-dim hover:text-klarvo-muted",
                      ].join(" ")}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
              <p className="text-[11px] text-klarvo-dim">
                {localBubbleLongPressMode === "hold" && "Hold to record, release to process"}
                {localBubbleLongPressMode === "toggle" && "Press once to start, press again to stop"}
                {localBubbleLongPressMode === "autostop" && "Press to start, stops automatically on silence"}
                {localBubbleLongPressMode === "auto" && "Continuous — restarts after each silence gap"}
              </p>

              {(localBubbleLongPressMode === "autostop" || localBubbleLongPressMode === "auto") && (
                <div className="flex flex-col gap-1.5">
                  <div className="flex items-center justify-between">
                    <span className={LABEL_CLS}>Silence Duration</span>
                    <span className="text-xs font-mono text-klarvo-primary">{localBubbleLongPressSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <input
                    type="range"
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localBubbleLongPressSilenceSecs}
                    onChange={(e) => setLocalBubbleLongPressSilenceSecs(parseFloat(e.target.value))}
                    className="w-full accent-klarvo-primary"
                  />
                  <p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
                </div>
              )}

              {/* Insert & Send hidden on Android — Enter key rarely works in mobile apps */}
            </>
          )}
        </div>
      )}
      {/* --- Paste & Behavior --- */}
      <div className="flex flex-col gap-3 pl-4 pb-3 pt-3 border-t border-klarvo-border/30 mt-1">
        <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Paste & Behavior</span>
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Paste</span><span className="text-[11px] text-klarvo-muted">Automatically paste result into active window.</span></div>
          <button
            role="switch"
            aria-checked={localAutoPaste}
            onClick={() => setLocalAutoPaste(!localAutoPaste)}
            className={[
              "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
              localAutoPaste ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
            ].join(" ")}
          >
            <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", localAutoPaste ? "translate-x-4" : ""].join(" ")} />
          </button>
        </div>
        <div className={`flex items-center justify-between gap-3${!localAutoPaste ? " opacity-40 pointer-events-none" : ""}`}>
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Send</span><span className="text-[11px] text-klarvo-muted">Send Enter after pasting (useful for chat apps)</span></div>
          <button
            role="switch"
            aria-checked={localInsertAndSendSlot1}
            onClick={() => setLocalInsertAndSendSlot1((v) => !v)}
            disabled={!localAutoPaste}
            className={[
              "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
              localInsertAndSendSlot1 ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
            ].join(" ")}
          >
            <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", localInsertAndSendSlot1 ? "translate-x-4" : ""].join(" ")} />
          </button>
        </div>
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Capitalize</span><span className="text-[11px] text-klarvo-muted">Capitalize first letter of every result.</span></div>
          <button
            role="switch"
            aria-checked={localAutoCapitalize}
            onClick={() => setLocalAutoCapitalize(!localAutoCapitalize)}
            className={[
              "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
              localAutoCapitalize ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
            ].join(" ")}
          >
            <span className={["absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200", localAutoCapitalize ? "translate-x-4" : ""].join(" ")} />
          </button>
        </div>
        <div className={`flex items-center justify-between gap-3${!localAutoPaste ? " opacity-40 pointer-events-none" : ""}`}>
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Paste Delay (ms)</span><span className="text-[11px] text-klarvo-muted">Wait time before sending paste keystroke.</span></div>
          <input type="number" min={0} max={2000} step={10} value={localPasteDelayMs} onChange={(e) => setLocalPasteDelayMs(parseInt(e.target.value, 10) || 0)} disabled={!localAutoPaste} className={`w-16 bg-klarvo-bg border border-klarvo-border/50 rounded-md px-2 py-1 text-xs text-right text-klarvo-text focus:outline-none focus:border-klarvo-primary/40${!localAutoPaste ? " cursor-not-allowed" : ""}`} />
        </div>
      </div>
    </>
  );
}
