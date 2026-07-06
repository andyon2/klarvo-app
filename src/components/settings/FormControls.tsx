import { useEffect, useRef, useState } from "react";

// Shared form-control primitives for Settings. Replaces native <select>,
// checkbox/toggle, <input type="range">, and hand-rolled segmented button
// groups with token-styled, keyboard-operable equivalents (Story 8-2).

// --- KToggle ------------------------------------------------------------------

export interface KToggleProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

export function KToggle({ checked, onChange, disabled }: KToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={[
        "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors focus-klarvo",
        "duration-[var(--motion-state)] ease-[var(--ease-standard)]",
        checked ? "bg-klarvo-teal/40" : "bg-klarvo-elevated",
        disabled ? "opacity-40 cursor-not-allowed" : "",
      ].join(" ")}
    >
      <span
        className={[
          "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform",
          "duration-[var(--motion-state)] ease-[var(--ease-standard)]",
          checked ? "translate-x-4" : "",
        ].join(" ")}
      />
    </button>
  );
}

// --- KSelect --------------------------------------------------------------------

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
  className?: string;
}

export function KSelect({ value, onChange, options, disabled, className }: KSelectProps) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);

  const selected = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    function handleOutside(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleOutside);
    return () => document.removeEventListener("mousedown", handleOutside);
  }, [open]);

  function openList() {
    if (disabled) return;
    const idx = Math.max(0, options.findIndex((o) => o.value === value));
    setActiveIndex(idx);
    setOpen(true);
  }

  function commit(idx: number) {
    const opt = options[idx];
    if (opt && !opt.disabled) onChange(opt.value);
    setOpen(false);
  }

  function handleTriggerKeyDown(e: React.KeyboardEvent) {
    if (disabled) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!open) {
        openList();
      }
    }
  }

  function handleListKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(options.length - 1, i + 1));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      commit(activeIndex);
    }
  }

  return (
    <div ref={rootRef} className={`relative ${className ?? ""}`}>
      <button
        type="button"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openList())}
        onKeyDown={handleTriggerKeyDown}
        className={[
          "w-full flex items-center justify-between gap-2 bg-klarvo-surface-2 border border-klarvo-border rounded-klarvo-sm",
          "px-2.5 py-1.5 text-xs text-klarvo-text focus-klarvo transition-colors cursor-pointer",
          disabled ? "opacity-40 cursor-not-allowed" : "",
        ].join(" ")}
      >
        <span className="truncate">{selected?.label ?? ""}</span>
        <svg className="w-3.5 h-3.5 text-klarvo-dim shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>
      {open && (
        <ul
          role="listbox"
          tabIndex={-1}
          onKeyDown={handleListKeyDown}
          ref={(el) => el?.focus()}
          className="absolute z-20 mt-1 w-full max-h-60 overflow-y-auto bg-klarvo-elevated border border-klarvo-border rounded-klarvo-sm shadow-klarvo-e2 focus:outline-none"
        >
          {options.map((opt, i) => (
            <li
              key={opt.value}
              role="option"
              aria-selected={opt.value === value}
              onMouseEnter={() => !opt.disabled && setActiveIndex(i)}
              onClick={() => commit(i)}
              aria-disabled={opt.disabled}
              className={[
                "px-2.5 py-1.5 text-xs truncate",
                opt.disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer",
                i === activeIndex && !opt.disabled ? "bg-klarvo-surface-2" : "",
                opt.value === value ? "text-klarvo-teal" : "text-klarvo-text",
              ].join(" ")}
            >
              {opt.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// --- KSlider --------------------------------------------------------------------

export interface KSliderProps {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  className?: string;
}

export function KSlider({ value, onChange, min, max, step, disabled, className }: KSliderProps) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <input
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(parseFloat(e.target.value))}
      className={`k-slider w-full ${disabled ? "opacity-40 cursor-not-allowed" : ""} ${className ?? ""}`}
      style={{
        background: `linear-gradient(to right, var(--color-klarvo-teal) ${pct}%, var(--color-klarvo-border-2) ${pct}%)`,
      }}
    />
  );
}

// --- KSegmented -----------------------------------------------------------------

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

export function KSegmented({ value, onChange, options, className }: KSegmentedProps) {
  return (
    <div role="group" className={`flex gap-0.5 bg-klarvo-bg rounded-klarvo-sm p-0.5 border border-klarvo-border/60 ${className ?? ""}`}>
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          role="radio"
          aria-checked={opt.value === value}
          title={opt.tooltip}
          onClick={() => onChange(opt.value)}
          className={[
            "px-2.5 py-1 rounded-klarvo-xs text-xs font-medium transition-all duration-100 whitespace-nowrap focus-klarvo",
            opt.value === value
              ? "bg-klarvo-teal/15 text-klarvo-teal"
              : "text-klarvo-dim hover:text-klarvo-muted",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
