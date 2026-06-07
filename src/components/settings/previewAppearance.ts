/**
 * Helpers and constants for the preview-box appearance panel (Story 6.6).
 *
 * Color model: stored values are always `rgba(r,g,b,a)` strings.
 * Native `<input type="color">` is hex-only (no alpha), so we pair it with
 * an opacity slider (0–100%) and compose/decompose as needed.
 */

// ---------------------------------------------------------------------------
// Defaults (mirrors PreviewPanel.tsx / AppConfig defaults)
// ---------------------------------------------------------------------------

export const DEFAULT_TEXT_COLOR   = "rgba(220,220,220,0.88)";
export const DEFAULT_BG_COLOR     = "rgba(25,25,25,0.96)";
export const DEFAULT_BORDER_COLOR = "rgba(42,195,168,0.25)";
export const DEFAULT_FONT_FAMILY  = "'Inter', system-ui, -apple-system, sans-serif";

// ---------------------------------------------------------------------------
// Task 1 — rgba ↔ hex / opacity helpers
// ---------------------------------------------------------------------------

/**
 * Parse an `rgba(r,g,b,a)` string into a `{ hex: "#rrggbb", opacityPct: number }`.
 * Falls back gracefully to `defaultHex`/`defaultOpacityPct` for any malformed input.
 *
 * Inversion test (AC-6): feed a malformed/empty rgba → must return the defaults,
 * not crash or yield `#NaNNaNNaN`.
 */
export function rgbaToHexOpacity(
  rgba: string,
  defaultHex = "#dcdcdc",
  defaultOpacityPct = 88,
): { hex: string; opacityPct: number } {
  if (!rgba || typeof rgba !== "string") {
    return { hex: defaultHex, opacityPct: defaultOpacityPct };
  }
  const m = rgba.match(
    /rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([\d.]+))?\s*\)/,
  );
  if (!m) {
    return { hex: defaultHex, opacityPct: defaultOpacityPct };
  }
  const r = parseInt(m[1], 10);
  const g = parseInt(m[2], 10);
  const b = parseInt(m[3], 10);
  const a = m[4] !== undefined ? parseFloat(m[4]) : 1;

  // Clamp and validate to avoid NaN / out-of-range
  if ([r, g, b].some((v) => isNaN(v) || v < 0 || v > 255) || isNaN(a)) {
    return { hex: defaultHex, opacityPct: defaultOpacityPct };
  }

  const toHexByte = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, "0");
  const hex = `#${toHexByte(r)}${toHexByte(g)}${toHexByte(b)}`;
  const opacityPct = Math.round(Math.max(0, Math.min(1, a)) * 100);
  return { hex, opacityPct };
}

/**
 * Compose a `#rrggbb` hex color + an opacity percentage (0–100) into an
 * `rgba(r,g,b,a)` string suitable for storage and CSS.
 *
 * Returns the `defaultRgba` if the hex is malformed.
 */
export function hexOpacityToRgba(
  hex: string,
  opacityPct: number,
  defaultRgba = DEFAULT_BG_COLOR,
): string {
  if (!hex || typeof hex !== "string") return defaultRgba;
  const clean = hex.startsWith("#") ? hex.slice(1) : hex;
  if (clean.length !== 6) return defaultRgba;
  const r = parseInt(clean.slice(0, 2), 16);
  const g = parseInt(clean.slice(2, 4), 16);
  const b = parseInt(clean.slice(4, 6), 16);
  if ([r, g, b].some(isNaN)) return defaultRgba;
  const a = Math.round(Math.max(0, Math.min(100, opacityPct))) / 100;
  return `rgba(${r},${g},${b},${a})`;
}

// ---------------------------------------------------------------------------
// Task 2 — theme presets
// ---------------------------------------------------------------------------

export interface PreviewTheme {
  label: string;
  textColor: string;
  bgColor: string;
  bgBlur: number;
  borderColor: string;
  borderWidth: number;
  borderRadius: number;
}

/**
 * Three one-click legible looks for the preview box.
 * Each must look good on its target background with zero additional tuning.
 *
 * - Dark:          light text on a dark translucent background (the default look)
 * - Light:         dark text on a near-white translucent background
 * - High-contrast: pure white text on a near-opaque dark background, wide border
 */
export const PREVIEW_THEMES: PreviewTheme[] = [
  {
    label: "Dark",
    textColor:    "rgba(220,220,220,0.95)",
    bgColor:      "rgba(20,20,20,0.92)",
    bgBlur:       12,
    borderColor:  "rgba(100,100,100,0.40)",
    borderWidth:  1,
    borderRadius: 14,
  },
  {
    label: "Light",
    textColor:    "rgba(30,30,30,0.95)",
    bgColor:      "rgba(248,248,248,0.94)",
    bgBlur:       8,
    borderColor:  "rgba(150,150,150,0.35)",
    borderWidth:  1,
    borderRadius: 14,
  },
  {
    label: "High-contrast",
    textColor:    "rgba(255,255,255,1.0)",
    bgColor:      "rgba(10,10,10,0.97)",
    bgBlur:       0,
    borderColor:  "rgba(255,255,255,0.85)",
    borderWidth:  2,
    borderRadius: 8,
  },
];

// ---------------------------------------------------------------------------
// Task 4 — curated font-family list
// ---------------------------------------------------------------------------

export interface PreviewFont {
  label: string;
  /** The concrete CSS font-family stack stored in config */
  stack: string;
}

export const PREVIEW_FONTS: PreviewFont[] = [
  { label: "Inter",      stack: "'Inter', system-ui, -apple-system, sans-serif" },
  { label: "System UI",  stack: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
  { label: "Serif",      stack: "Georgia, 'Times New Roman', Times, serif" },
  { label: "Monospace",  stack: "'Cascadia Code', 'Fira Code', 'Consolas', 'Courier New', monospace" },
];
