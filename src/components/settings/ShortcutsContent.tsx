import { useState, useEffect, useCallback, useRef } from "react";
import type { HotkeyMode, AppSettings } from "../../types";
import { isDesktop } from "../../platform";
import { LABEL_CLS } from "../ui";
import { KSlider, KSegmented, KToggle } from "./FormControls";

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
          ? "border-klarvo-teal/50 text-klarvo-teal animate-pulse"
          : "border-klarvo-border/50 text-klarvo-text hover:border-klarvo-border-2",
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
  // Auto-mode pause sliders (split — each writes its own backend key).
  // Stop-Pause → autostop_silence_secs (AutoStop mode: silence ends recording).
  // Segment-Pause → auto_mode_silence_secs (Auto loop: silence sends a segment, keeps going).
  localAutostopSilenceSecs: number;
  setLocalAutostopSilenceSecs: (v: number) => void;
  localAutoModeSilenceSecs: number;
  setLocalAutoModeSilenceSecs: (v: number) => void;
  // Insert & Send toggle (slot 1)
  localInsertAndSendSlot1: boolean;
  setLocalInsertAndSendSlot1: (v: boolean | ((prev: boolean) => boolean)) => void;
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
}

// --- Component ---------------------------------------------------------------

export function ShortcutsContent({
  localHotkey, setLocalHotkey, localHotkeyMode, setLocalHotkeyMode,
  localHotkeySlot2, setLocalHotkeySlot2, localHotkeyModeSlot2, setLocalHotkeyModeSlot2,
  localAutostopSilenceSecs, setLocalAutostopSilenceSecs,
  localAutoModeSilenceSecs, setLocalAutoModeSilenceSecs,
  localInsertAndSendSlot1, setLocalInsertAndSendSlot1,
  hotkeyTab, setHotkeyTab,
  bubbleTab, setBubbleTab,
  localBubbleTapMode, setLocalBubbleTapMode,
  localBubbleTapSilenceSecs, setLocalBubbleTapSilenceSecs,
  localBubbleLongPressMode, setLocalBubbleLongPressMode,
  localBubbleLongPressSilenceSecs, setLocalBubbleLongPressSilenceSecs,
  loadedSettings: _loadedSettings, onHotkeyChange, onHotkeyModeChange,
  localAutoPaste, setLocalAutoPaste, localPasteDelayMs, setLocalPasteDelayMs, localAutoCapitalize, setLocalAutoCapitalize,
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
          <div className="self-start">
            <KSegmented
              value={String(hotkeyTab)}
              onChange={(v) => setHotkeyTab(Number(v) as 0 | 1)}
              options={[
                { value: "0", label: "Hotkey 1" },
                { value: "1", label: "Hotkey 2" },
              ]}
            />
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
                <KSegmented
                  value={localHotkeyMode}
                  onChange={(v) => handleHotkeyModeChange(v as HotkeyMode)}
                  options={[
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ]}
                />
              </div>
              <p className="text-[11px] text-klarvo-muted">
                {localHotkeyMode === "hold" && "Hold to record, release to process"}
                {localHotkeyMode === "toggle" && "Press once to start, press again to stop"}
                {localHotkeyMode === "autostop" && "Press to start, stops automatically on silence"}
                {localHotkeyMode === "auto" && "Continuous — restarts after each silence gap"}
              </p>



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
                  <KSegmented
                    value={localHotkeyModeSlot2}
                    onChange={(v) => setLocalHotkeyModeSlot2(v as HotkeyMode)}
                    options={[
                      { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                      { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                      { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                      { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                    ]}
                  />
                  <p className="text-[11px] text-klarvo-dim">
                    {localHotkeyModeSlot2 === "hold" && "Hold to record, release to process"}
                    {localHotkeyModeSlot2 === "toggle" && "Press once to start, press again to stop"}
                    {localHotkeyModeSlot2 === "autostop" && "Press to start, stops automatically on silence"}
                    {localHotkeyModeSlot2 === "auto" && "Continuous — restarts after each silence gap"}
                  </p>

                </div>
              )}

            </>
          )}

          {/* --- Auto-mode pauses --- shown only when a hotkey actually uses that mode.
              Two independent sliders, each bound to its own backend key:
              Stop-Pause → autostop_silence_secs, Segment-Pause → auto_mode_silence_secs. */}
          {(localHotkeyMode === "autostop" || localHotkeyModeSlot2 === "autostop"
            || localHotkeyMode === "auto" || localHotkeyModeSlot2 === "auto") && (
            <div className="flex flex-col gap-3 border-t border-klarvo-border/30 pt-3 mt-1">
              <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Auto Modes</span>

              {(localHotkeyMode === "autostop" || localHotkeyModeSlot2 === "autostop") && (
                <div className="flex flex-col gap-1.5">
                  <div className="flex items-center justify-between">
                    <span className={LABEL_CLS}>Stop-Pause</span>
                    <span className="text-xs font-mono text-klarvo-teal">{localAutostopSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <KSlider
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localAutostopSilenceSecs}
                    onChange={setLocalAutostopSilenceSecs}
                  />
                  <p className="text-[11px] text-klarvo-muted">Silence before <strong>Auto Stop</strong> mode ends the recording.</p>
                </div>
              )}

              {(localHotkeyMode === "auto" || localHotkeyModeSlot2 === "auto") && (
                <div className="flex flex-col gap-1.5">
                  <div className="flex items-center justify-between">
                    <span className={LABEL_CLS}>Segment-Pause</span>
                    <span className="text-xs font-mono text-klarvo-teal">{localAutoModeSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <KSlider
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localAutoModeSilenceSecs}
                    onChange={setLocalAutoModeSilenceSecs}
                  />
                  <p className="text-[11px] text-klarvo-muted">Silence before <strong>Auto</strong> mode sends a segment and keeps recording.</p>
                </div>
              )}
            </div>
          )}

        </div>
      )}

      {/* --- Bubble Controls -- mobile only --- */}
      {!isDesktop && (
        <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
          {/* Tab bar: Tap / Long Press */}
          <div className="self-start">
            <KSegmented
              value={String(bubbleTab)}
              onChange={(v) => setBubbleTab(Number(v) as 0 | 1)}
              options={[
                { value: "0", label: "Tap" },
                { value: "1", label: "Long Press" },
              ]}
            />
          </div>

          {/* Tab 0: Tap */}
          {bubbleTab === 0 && (
            <>
              <div className="flex flex-col gap-1.5">
                <span className={LABEL_CLS}>Mode</span>
                <KSegmented
                  value={localBubbleTapMode}
                  onChange={(v) => setLocalBubbleTapMode(v as HotkeyMode)}
                  options={[
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ]}
                />
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
                    <span className="text-xs font-mono text-klarvo-teal">{localBubbleTapSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <KSlider
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localBubbleTapSilenceSecs}
                    onChange={setLocalBubbleTapSilenceSecs}
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
                <KSegmented
                  value={localBubbleLongPressMode}
                  onChange={(v) => setLocalBubbleLongPressMode(v as HotkeyMode)}
                  options={[
                    { value: "hold", label: "Hold", tooltip: "Hold to record, release to process" },
                    { value: "toggle", label: "Toggle", tooltip: "Press to start, press again to stop" },
                    { value: "autostop", label: "Auto Stop ⚠", tooltip: "Experimental — Press to start, stops automatically on silence" },
                    { value: "auto", label: "Auto ⚠", tooltip: "Experimental — Continuous: restarts after each silence gap" },
                  ]}
                />
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
                    <span className="text-xs font-mono text-klarvo-teal">{localBubbleLongPressSilenceSecs.toFixed(1)}s</span>
                  </div>
                  <KSlider
                    min={1.0}
                    max={5.0}
                    step={0.1}
                    value={localBubbleLongPressSilenceSecs}
                    onChange={setLocalBubbleLongPressSilenceSecs}
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
          <KToggle checked={localAutoPaste} onChange={setLocalAutoPaste} />
        </div>
        <div className={`flex items-center justify-between gap-3${!localAutoPaste ? " opacity-40 pointer-events-none" : ""}`}>
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Send</span><span className="text-[11px] text-klarvo-muted">Send Enter after pasting (useful for chat apps)</span></div>
          <KToggle
            checked={localInsertAndSendSlot1 as boolean}
            onChange={(v) => setLocalInsertAndSendSlot1(v)}
            disabled={!localAutoPaste}
          />
        </div>
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Auto-Capitalize</span><span className="text-[11px] text-klarvo-muted">Capitalize first letter of every result.</span></div>
          <KToggle checked={localAutoCapitalize} onChange={setLocalAutoCapitalize} />
        </div>
        <div className={`flex items-center justify-between gap-3${!localAutoPaste ? " opacity-40 pointer-events-none" : ""}`}>
          <div className="flex flex-col gap-0.5"><span className={LABEL_CLS}>Paste Delay (ms)</span><span className="text-[11px] text-klarvo-muted">Wait time before sending paste keystroke.</span></div>
          <input type="number" min={0} max={2000} step={10} value={localPasteDelayMs} onChange={(e) => setLocalPasteDelayMs(parseInt(e.target.value, 10) || 0)} disabled={!localAutoPaste} className={`w-16 bg-klarvo-bg border border-klarvo-border/50 rounded-md px-2 py-1 text-xs text-right text-klarvo-text focus:outline-none focus-klarvo${!localAutoPaste ? " cursor-not-allowed" : ""}`} />
        </div>
      </div>
    </>
  );
}
