import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";
import { isMobile } from "../platform";

export interface MobileTextareaProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  rows?: number;
  className?: string;
  hint?: string;
  disabled?: boolean;
}

// Fullscreen overlay shown on mobile when the user taps the preview div.
function MobileEditOverlay({
  label,
  hint,
  initialValue,
  placeholder,
  className,
  onDone,
}: {
  label: string;
  hint?: string;
  initialValue: string;
  placeholder?: string;
  className?: string;
  onDone: (value: string) => void;
}) {
  const [draft, setDraft] = useState(initialValue);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-focus the textarea as soon as the overlay mounts.
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    // Small delay so the overlay paint completes before keyboard appears.
    const id = setTimeout(() => {
      el.focus();
      // Place cursor at end.
      el.setSelectionRange(el.value.length, el.value.length);
    }, 80);
    return () => clearTimeout(id);
  }, []);

  const handleDone = useCallback(() => {
    onDone(draft);
  }, [draft, onDone]);

  return createPortal(
    <div
      className="fixed inset-0 z-[9999] flex flex-col bg-[#0a0a0b]"
      style={{ WebkitOverflowScrolling: "touch" }}
    >
      {/* Header -- pt-9 clears Android status bar (env(safe-area-inset-top) returns 0 in WebView) */}
      <div className="flex items-center justify-between px-4 pt-9 pb-3 border-b border-zinc-800/60 flex-shrink-0">
        <div className="flex flex-col gap-0.5 min-w-0 pr-4">
          <span className="text-sm font-semibold text-zinc-200 truncate">{label}</span>
          {hint && <span className="text-xs text-zinc-500 leading-relaxed line-clamp-2">{hint}</span>}
        </div>
        <button
          onClick={handleDone}
          className="flex-shrink-0 px-4 py-2 rounded-xl text-sm font-semibold bg-emerald-500/15 border border-emerald-500/30 text-emerald-400 hover:bg-emerald-500/25 hover:border-emerald-500/50 transition-all duration-150 active:scale-95"
        >
          Done
        </button>
      </div>

      {/* Textarea fills remaining height */}
      <textarea
        ref={textareaRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder={placeholder}
        className={[
          "flex-1 w-full bg-transparent resize-none px-4 py-4",
          "text-sm text-zinc-100 placeholder:text-zinc-600",
          "focus:outline-none",
          "mobile-safe-bottom",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      />
    </div>,
    document.body,
  );
}

/**
 * A textarea that renders normally on desktop and as a tappable preview on
 * mobile. Tapping the preview opens a fullscreen overlay for comfortable
 * editing, matching the pattern used by Notion, Linear, etc.
 */
export function MobileTextarea({
  label,
  value,
  onChange,
  placeholder,
  rows = 3,
  className,
  hint,
  disabled = false,
}: MobileTextareaProps) {
  const [overlayOpen, setOverlayOpen] = useState(false);

  const handleDone = useCallback(
    (newValue: string) => {
      onChange(newValue);
      setOverlayOpen(false);
    },
    [onChange],
  );

  // Desktop: plain textarea, identical to what was there before.
  if (!isMobile) {
    return (
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        rows={rows}
        disabled={disabled}
        className={className}
      />
    );
  }

  // Mobile: tappable preview div + fullscreen overlay on tap.
  const isEmpty = value.trim().length === 0;

  return (
    <>
      <div
        role={disabled ? undefined : "button"}
        tabIndex={disabled ? undefined : 0}
        onClick={() => { if (!disabled) setOverlayOpen(true); }}
        onKeyDown={(e) => {
          if (!disabled && (e.key === "Enter" || e.key === " ")) setOverlayOpen(true);
        }}
        className={[
          // Match INPUT_CLS_M appearance so it looks like the surrounding inputs.
          "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2.5 cursor-pointer",
          "text-sm leading-relaxed",
          // Two-line clamp so long values show enough context.
          "line-clamp-2",
          isEmpty ? "text-zinc-600" : "text-zinc-100",
          "transition-colors active:border-zinc-600",
          className ?? "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        {isEmpty ? (placeholder ?? "Tap to edit...") : value}
      </div>

      {overlayOpen && (
        <MobileEditOverlay
          label={label}
          hint={hint}
          initialValue={value}
          placeholder={placeholder}
          onDone={handleDone}
        />
      )}
    </>
  );
}
