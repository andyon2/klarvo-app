/**
 * QuickTip — contextual toast/snackbar shown at the bottom of the screen.
 *
 * Appears with a slide-up + fade animation. The dismiss button calls onDismiss
 * immediately; the action button calls onAction (which should also dismiss).
 */

import { useEffect, useState } from "react";
import { CloseIcon } from "./icons";

export interface QuickTipProps {
  title: string;
  text: string;
  actionLabel?: string;
  onAction?: () => void;
  onDismiss: () => void;
}

export function QuickTip({ title, text, actionLabel, onAction, onDismiss }: QuickTipProps) {
  // Animate in: start invisible/shifted, then transition to visible.
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Trigger on next paint so the CSS transition fires.
    const raf = requestAnimationFrame(() => setVisible(true));
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <div
      role="status"
      aria-live="polite"
      className={[
        // Position: fixed, above the bottom nav bar.
        "fixed bottom-20 left-4 right-4 md:left-auto md:right-6 md:w-80 z-50",
        // Card styling.
        "bg-klarvo-surface border border-klarvo-border/60 rounded-2xl shadow-xl shadow-black/40 p-4",
        // Slide-up + fade animation via Tailwind transform/opacity.
        "transition-all duration-300 ease-out",
        visible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4",
      ].join(" ")}
    >
      {/* Dismiss button — top right */}
      <button
        onClick={onDismiss}
        aria-label="Tip schließen"
        className="absolute top-3 right-3 text-klarvo-dim hover:text-klarvo-muted transition-colors p-0.5 rounded"
      >
        <CloseIcon />
      </button>

      {/* Content */}
      <div className="pr-6">
        <p className="text-xs font-semibold text-klarvo-text leading-tight mb-1">{title}</p>
        <p className="text-[11px] text-klarvo-muted leading-snug">{text}</p>
      </div>

      {/* Action button (optional) */}
      {actionLabel && onAction && (
        <button
          onClick={onAction}
          className="mt-3 w-full py-1.5 rounded-lg bg-klarvo-primary/15 text-klarvo-primary text-xs font-medium hover:bg-klarvo-primary/25 transition-colors"
        >
          {actionLabel}
        </button>
      )}
    </div>
  );
}
