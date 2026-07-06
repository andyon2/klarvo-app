/**
 * Shared form control primitives for the Settings system.
 * Story 8.2 — Studio-Dark visual overhaul.
 *
 * KToggle  — boolean pill toggle (role="switch")
 * KSelect  — fully custom dropdown, no native <select>
 * KSlider  — styled range input with teal fill
 * KSegmented — segmented button group
 */

import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";

// ---------------------------------------------------------------------------
// KToggle
// ---------------------------------------------------------------------------

export interface KToggleProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

/** Pill toggle — 36×20px, solid teal fill when on, klarvo-elevated when off. */
export function KToggle({ checked, onChange, disabled }: KToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => !disabled && onChange(!checked)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          if (!disabled) onChange(!checked);
        }
      }}
      className={[
        "relative flex-shrink-0 w-9 h-5 rounded-full focus-klarvo",
        "focus:outline-none",
        checked ? "bg-klarvo-teal" : "bg-klarvo-elevated",
        disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer",
      ].join(" ")}
      style={{
        transition: `background-color var(--motion-state) var(--ease-standard)`,
      }}
    >
      <span
        className={[
          "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white",
          checked ? "translate-x-4" : "",
        ].join(" ")}
        style={{
          transition: `transform var(--motion-state) var(--ease-standard)`,
        }}
      />
    </button>
  );
}

// ---------------------------------------------------------------------------
// KSelect
// ---------------------------------------------------------------------------

export interface KSelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface KSelectProps {
  value: string;
  onChange: (v: string) => void;
  options: KSelectOption[];
  disabled?: boolean;
  /** Optional wrapper className — kept for call sites that size the trigger (e.g. w-auto/w-full). */
  className?: string;
}

/**
 * Custom dropdown — no native <select> inside.
 * Keyboard: ArrowUp/Down navigate, Enter/Space select, Escape closes.
 * Renders dropdown via React portal to avoid overflow-hidden/overflow-y-auto clipping.
 */
export function KSelect({ value, onChange, options, disabled, className }: KSelectProps) {
  const [open, setOpen] = useState(false);
  const [focusedIdx, setFocusedIdx] = useState<number>(-1);
  const [dropdownStyle, setDropdownStyle] = useState<React.CSSProperties>({});
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const currentLabel = options.find((o) => o.value === value)?.label ?? value;

  // Close without stealing focus back to the trigger — used when focus is
  // already moving elsewhere (e.g. Tab out of the list).
  const closeSilently = useCallback(() => {
    setOpen(false);
    setFocusedIdx(-1);
  }, []);

  const close = useCallback(() => {
    closeSilently();
    triggerRef.current?.focus();
  }, [closeSilently]);

  // Focus listbox when opened so handleListKeyDown receives arrow/enter keys
  useEffect(() => {
    if (open) {
      listRef.current?.focus();
    }
  }, [open]);

  // Compute portal dropdown position from trigger's bounding rect.
  // Re-runs on scroll/resize so the fixed dropdown tracks the trigger even
  // when the settings panel (overflow-y-auto) is scrolled while the dropdown
  // is open. Without listeners the position is stale after scroll.
  //
  // Finding 1: walk up from the trigger to find the actual scroll ancestor
  // (not the first .overflow-y-auto in DOM order, which may be a different panel).
  //
  // Finding 4: flip upward when insufficient space below the trigger.
  useEffect(() => {
    if (!open || !triggerRef.current) return;

    function updatePosition() {
      if (!triggerRef.current) return;
      const rect = triggerRef.current.getBoundingClientRect();
      const listHeight = 192; // max-h-48 = 192px
      const spaceBelow = window.innerHeight - rect.bottom;
      // Clamp top into the viewport so an upward-flip near the top edge
      // never renders the listbox off-screen above the visible area.
      const top = Math.max(
        4,
        spaceBelow >= listHeight + 8
          ? rect.bottom + 4
          : rect.top - listHeight - 4,
      );
      // Clamp left so a trigger near the right edge doesn't overflow the viewport.
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - rect.width - 8));
      setDropdownStyle({
        position: "fixed",
        top,
        left,
        width: rect.width,
        zIndex: 9999,
      });
    }

    updatePosition();

    // Walk up from trigger to find the actual scroll container, not the first
    // .overflow-y-auto element in DOM order (which can be a different panel).
    const sc = triggerRef.current?.closest(".overflow-y-auto");
    sc?.addEventListener("scroll", updatePosition);
    window.addEventListener("resize", updatePosition);
    return () => {
      sc?.removeEventListener("scroll", updatePosition);
      window.removeEventListener("resize", updatePosition);
    };
  }, [open]);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (
        triggerRef.current && !triggerRef.current.contains(e.target as Node) &&
        listRef.current && !listRef.current.contains(e.target as Node)
      ) {
        close();
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open, close]);

  // Scroll focused item into view
  useEffect(() => {
    if (!open || focusedIdx < 0) return;
    const item = listRef.current?.children[focusedIdx] as HTMLElement | undefined;
    item?.scrollIntoView({ block: "nearest" });
  }, [open, focusedIdx]);

  // Finding 5b: seed focus to the first enabled option at/after the current value index.
  function firstEnabledIdx(startAt: number): number {
    if (options.length === 0) return -1;
    // forward scan
    for (let i = startAt; i < options.length; i++) {
      if (!options[i]?.disabled) return i;
    }
    // fallback: scan from beginning
    for (let i = 0; i < startAt; i++) {
      if (!options[i]?.disabled) return i;
    }
    return -1;
  }

  function handleTriggerKeyDown(e: React.KeyboardEvent) {
    if (disabled) return;
    if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!open) {
        // Finding 5b: seed to first enabled at/after current selection
        const curIdx = options.findIndex((o) => o.value === value);
        setFocusedIdx(firstEnabledIdx(curIdx >= 0 ? curIdx : 0));
        setOpen(true);
      } else if (focusedIdx >= 0) {
        const opt = options[focusedIdx];
        if (opt && !opt.disabled) {
          onChange(opt.value);
          close();
        }
      } else {
        // Finding 5c: Enter with focusedIdx === -1 — confirm current value / first enabled
        const fallback = firstEnabledIdx(0);
        if (fallback >= 0) {
          onChange(options[fallback].value);
          close();
        }
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) {
        setOpen(true);
        // seed to last enabled option
        let last = options.length - 1;
        while (last > 0 && options[last]?.disabled) last--;
        setFocusedIdx(last);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  function handleListKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setFocusedIdx((i) => {
        // Finding 5a: clamp to nearest enabled index, not just length-1
        let next = i + 1;
        while (next < options.length && options[next]?.disabled) next++;
        // if we ran off the end, stay at last enabled
        if (next >= options.length) return i;
        return next;
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setFocusedIdx((i) => {
        // Finding 5a: clamp to nearest enabled index, not just 0
        let prev = i - 1;
        while (prev >= 0 && options[prev]?.disabled) prev--;
        // if we ran before the start, stay at first enabled
        if (prev < 0) return i;
        return prev;
      });
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (options.length === 0) return;
      if (focusedIdx === -1) {
        // Finding 5c: fallback — confirm first enabled option
        const fallback = firstEnabledIdx(0);
        if (fallback >= 0) {
          onChange(options[fallback].value);
          close();
        }
        return;
      }
      const opt = options[focusedIdx];
      if (opt && !opt.disabled) {
        onChange(opt.value);
        close();
      }
      // Committing on a disabled option must not close-without-selecting — leave the list open.
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  // Close when focus leaves the open list entirely (e.g. Tab out), not only on
  // outside mousedown — relatedTarget is the element receiving focus next.
  function handleListBlur(e: React.FocusEvent) {
    if (!listRef.current?.contains(e.relatedTarget as Node)) {
      closeSilently();
    }
  }

  const dropdown = open ? (
    <ul
      ref={listRef}
      role="listbox"
      aria-label="Options"
      onKeyDown={handleListKeyDown}
      onBlur={handleListBlur}
      tabIndex={-1}
      className={[
        "max-h-48 overflow-y-auto",
        "rounded-klarvo-sm bg-klarvo-elevated shadow-klarvo-e2",
        "border border-klarvo-border",
        "focus:outline-none",
      ].join(" ")}
      style={dropdownStyle}
    >
      {options.map((opt, idx) => (
        <li
          key={opt.value}
          role="option"
          aria-selected={opt.value === value}
          aria-disabled={opt.disabled}
          onMouseEnter={() => !opt.disabled && setFocusedIdx(idx)}
          onClick={() => {
            if (opt.disabled) return;
            onChange(opt.value);
            close();
          }}
          className={[
            "px-2.5 py-1.5 text-xs transition-colors",
            // Finding 5d: disabled options always get explicit dim color in all states
            // (selected+disabled previously left text-color branch empty → inherited)
            opt.disabled
              ? "text-klarvo-dim opacity-50 cursor-not-allowed"
              : opt.value === value
                ? "text-klarvo-teal cursor-pointer"
                : "text-klarvo-text cursor-pointer",
            !opt.disabled && focusedIdx === idx ? "bg-klarvo-surface-2" : "",
            !opt.disabled ? "hover:bg-klarvo-surface-2" : "",
          ].join(" ")}
        >
          {opt.label}
        </li>
      ))}
    </ul>
  ) : null;

  return (
    <div className={`relative ${className ?? ""}`}>
      {/* Trigger button */}
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => {
          if (disabled) return;
          if (!open) {
            // Seed via firstEnabledIdx for parity with the keyboard-open path,
            // so a mouse click never lands the initial highlight on a disabled option.
            const idx = options.findIndex((o) => o.value === value);
            setFocusedIdx(firstEnabledIdx(idx >= 0 ? idx : 0));
          }
          setOpen((v) => !v);
        }}
        onKeyDown={handleTriggerKeyDown}
        className={[
          "flex w-full items-center justify-between gap-2 rounded-klarvo-sm",
          "bg-klarvo-surface-2 border border-klarvo-border px-2.5 py-1.5",
          "text-xs text-klarvo-text transition-colors",
          "focus:outline-none focus-klarvo",
          open ? "border-klarvo-border-2" : "hover:border-klarvo-border-2",
          disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer",
        ].join(" ")}
      >
        <span className="truncate">{currentLabel}</span>
        <svg
          className={[
            "w-3.5 h-3.5 text-klarvo-dim flex-shrink-0 transition-transform",
            open ? "rotate-180" : "",
          ].join(" ")}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      {/* Portal-rendered dropdown — not clipped by panel overflow-y-auto/overflow-hidden.
          Gated on `open` so closed selects don't hold a live portal subscription. */}
      {open && createPortal(dropdown, document.body)}
    </div>
  );
}

// ---------------------------------------------------------------------------
// KSlider
// ---------------------------------------------------------------------------

export interface KSliderProps {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  className?: string;
}

/**
 * Styled range input — teal fill up to thumb, muted track beyond.
 * Uses CSS class `k-slider` for webkit pseudo-element styling.
 */
export function KSlider({ value, onChange, min, max, step, disabled, className }: KSliderProps) {
  const pct = max > min ? ((value - min) / (max - min)) * 100 : 0;

  return (
    <input
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(parseFloat(e.target.value))}
      className={["k-slider w-full", disabled ? "opacity-40 cursor-not-allowed" : "", className ?? ""].join(" ")}
      style={{
        background: `linear-gradient(to right, var(--color-klarvo-teal) ${pct}%, var(--color-klarvo-border-2) ${pct}%)`,
      }}
    />
  );
}

// ---------------------------------------------------------------------------
// KSegmented
// ---------------------------------------------------------------------------

export interface KSegmentedOption {
  value: string;
  label: string;
  tooltip?: string;
}

export interface KSegmentedProps {
  value: string;
  onChange: (v: string) => void;
  options: KSegmentedOption[];
  className?: string;
}

/**
 * Segmented control — plain <div> container with <button aria-pressed> per option.
 * Each segment is individually Tab-focusable; no radiogroup ARIA semantics.
 */
export function KSegmented({ value, onChange, options, className }: KSegmentedProps) {
  return (
    <div
      className={`flex gap-0.5 bg-klarvo-bg rounded-klarvo-sm p-0.5 border border-klarvo-border ${className ?? ""}`}
    >
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          aria-pressed={opt.value === value}
          title={opt.tooltip}
          onClick={() => onChange(opt.value)}
          className={[
            "flex-1 px-2.5 py-1 rounded-klarvo-xs text-xs font-medium whitespace-nowrap",
            "focus:outline-none focus-klarvo",
            opt.value === value
              ? "bg-klarvo-elevated text-klarvo-text border border-klarvo-border-2"
              : "text-klarvo-dim hover:text-klarvo-muted border border-transparent",
          ].join(" ")}
          style={{ transition: `all var(--motion-state) var(--ease-standard)` }}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
