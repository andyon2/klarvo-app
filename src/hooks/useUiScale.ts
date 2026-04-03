/**
 * useUiScale — reads uiScale from AdvancedSettings and applies it to <html>
 * as a root font-size.  Because Tailwind uses rem units, every component scales
 * automatically without any per-component changes.
 *
 * Scale map:
 *   small  → 14px
 *   medium → 16px  (browser default — override is removed)
 *   large  → 18px
 */
import { useEffect } from "react";
import { getAdvancedSettings } from "../tauri-commands";

const SCALE_MAP: Record<string, string> = {
  small: "14px",
  medium: "16px",
  large: "18px",
};

function applyScale(scale: string): void {
  const size = SCALE_MAP[scale] ?? SCALE_MAP.medium;
  document.documentElement.style.fontSize = size;
}

/**
 * Call once at app startup.  Reads the persisted uiScale and applies it.
 * Returns quickly — no blocking wait.
 */
export function useUiScale(): void {
  useEffect(() => {
    getAdvancedSettings()
      .then((s) => applyScale(s.uiScale ?? "medium"))
      .catch(() => {
        // Preview mode or backend not yet ready — leave browser default alone.
      });
  }, []);
}

/**
 * Immediately apply a scale value to the DOM.  Call this from the settings
 * panel whenever uiScale changes (live preview).
 */
export { applyScale as applyUiScale };
