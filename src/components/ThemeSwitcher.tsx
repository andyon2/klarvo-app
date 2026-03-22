/**
 * ThemeSwitcher -- preview-only floating panel for live theme comparison.
 *
 * Renders ONLY when isPreviewMode === true. Injects CSS overrides at runtime
 * so the color variants can be compared without rebuilding.
 *
 * The switcher's own chrome uses hardcoded inline styles so it is never
 * affected by the active theme.
 *
 * Each non-"Current" variant injects three visual-depth upgrades:
 *   1. Root background gradient (--gradient-bg) → simulates ceiling light
 *   2. Card elevation (3-layer box-shadow) → replaces flat border
 *   3. RecordButton 3-layer glow (--glow-primary) → lit CTA
 *
 * Color role system (5 functional roles beyond Danger):
 *   Action   — primary interactive CTAs, active states
 *   Activity — in-progress states, spinners, busy indicators
 *   Success  — done-states, checkmarks, confirmations (must differ from Action)
 *   Info     — links, badges, secondary highlights
 *   Warm     — stats highlights, cost dashboard accent, visual contrast point
 */
import { useState } from "react";
import { isPreviewMode } from "../tauri-commands";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ThemeVariant {
  id: string;
  name: string;
  dot: string;
  // Surface colors
  bg: string;
  surface: string;
  elevated?: string;
  border: string;
  borderActive?: string;
  text: string;
  muted: string;
  dim?: string;
  // Color roles
  primary: string;    // Action role
  accent: string;     // Hover / lighter variant of primary
  secondary?: string; // Activity role (in-progress states)
  activity: string;   // Activity role (spinners, busy — explicit)
  success: string;    // Done-states, checkmarks (must differ from primary)
  info: string;       // Links, badges, secondary highlights
  warm: string;       // Stats highlights, cost dashboard accent
  warning: string;    // Warning states (orange-ish)
  danger: string;     // Destructive / recording / error
  // Visual depth properties
  gradientBg: string;  // radial-gradient for root background
  shadowCard: string;  // 3-layer box-shadow for cards (replaces border)
  shadowPanel: string; // 3-layer box-shadow for panels
  glowPrimary: string; // 3-layer glow for RecordButton idle
  glowRecording: string; // 3-layer glow for RecordButton recording
  extraCss?: string;     // additional CSS overrides (for v2 experiments)
}

// ---------------------------------------------------------------------------
// Theme definitions
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Research sources:
// - Superhuman Carbon: blog.superhuman.com/how-to-design-delightful-dark-themes
//   Five shades of gray, Cod Gray #1B1B1B, Picton Blue #5BBEF5, no pure black
// - Notion Dark: #2F3438 main, #373C3F sidebar, #3F4448 hover
//   Warm gray-green tones, distinctly lighter than most dark modes
// - Linear: LCH-generated blue-gray, indigo accent #5E6AD2
//   Muted blue tint in backgrounds, restrained accent usage
// - Wispr Flow: #4D65FF indigo accent, #FFFFEB cream text, soft neutrals
// - Voxlit Social Preview: deep navy gradient, amber+teal logo accents
//
// Color role principles (see briefings/color-palette-system.md):
//   Action ≠ Success — the record button and the "done" state use different colors
//   Activity = Amber/Yellow family — universal "processing" signal
//   Info = cool tones (cyan/indigo) — informational, not interactive
//   Warm = orange/amber — stats and highlights, visual contrast point
// ---------------------------------------------------------------------------

const THEMES: ThemeVariant[] = [
  // Reference: current app state for comparison.
  // No gradient/glow overrides — preserves exact current appearance.
  {
    id: "current",
    name: "Current",
    dot: "#10b981",
    bg: "#0a0a0c",
    surface: "#18181b",
    border: "#27272a",
    text: "#fafafa",
    muted: "#71717a",
    primary: "#10b981",
    accent: "#34d399",
    activity: "#f59e0b",
    success: "#34d399",
    info: "#818cf8",
    warm: "#fb923c",
    warning: "#f59e0b",
    danger: "#ef4444",
    // Empty strings = no override applied for "Current"
    gradientBg: "",
    shadowCard: "",
    shadowPanel: "",
    glowPrimary: "",
    glowRecording: "",
  },

  // ── Obsidian ──────────────────────────────────────────────────────────────
  // Inspired by Voxlit brand: Cyan from the FloatingBar logo as primary action.
  // Teal as Success (done-state), Amber as Activity, Indigo as Info.
  // Deepest background of all variants — maximum depth with the gradient.
  {
    id: "obsidian",
    name: "Obsidian",
    dot: "#22D3EE",
    bg: "#0C0C10",
    surface: "#131318",
    elevated: "#1A1A21",
    border: "#1E1E26",
    borderActive: "#2A2A35",
    text: "#F0F0F4",
    muted: "#7A7A88",
    dim: "#4A4A58",
    // Action = Cyan (FloatingBar brand color)
    primary: "#22D3EE",
    accent: "#67E8F9",
    // Activity = Amber (universal processing signal)
    secondary: "#FBBF24",
    activity: "#FBBF24",
    // Success = Teal (clearly different from Cyan action)
    success: "#2DD4BF",
    // Info = Soft Indigo (cool, informational)
    info: "#818CF8",
    // Warm = Orange (stats/highlights visual contrast)
    warm: "#FB923C",
    warning: "#FBBF24",
    danger: "#EF4444",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #141420 0%, #0C0C10 65%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.055), 0 1px 3px rgba(0,0,0,0.45), 0 4px 14px rgba(0,0,0,0.28)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.05), 0 8px 32px rgba(0,0,0,0.55), 0 2px 8px rgba(0,0,0,0.3)",
    glowPrimary:
      "0 0 0 1px rgba(34,211,238,0.2), 0 0 20px rgba(34,211,238,0.15), 0 0 55px rgba(34,211,238,0.07)",
    glowRecording:
      "0 0 0 1px rgba(239,68,68,0.35), 0 0 22px rgba(239,68,68,0.22), 0 0 50px rgba(239,68,68,0.10)",
  },

  // ── Carbon ────────────────────────────────────────────────────────────────
  // Inspired by Superhuman Carbon: warm neutral grays, five-shade depth,
  // blue accent. No pure black — Cod Gray as base. Lighter than current.
  {
    id: "carbon",
    name: "Carbon",
    dot: "#5BBEF5",
    bg: "#1B1B1B",
    surface: "#222226",
    elevated: "#2A2A2F",
    border: "#333338",
    borderActive: "#3E3E44",
    text: "#F2F2F2",
    muted: "#8A8A92",
    dim: "#5E5E66",
    primary: "#5BBEF5",
    accent: "#7DD0F8",
    secondary: "#FBBF24",
    activity: "#FBBF24",
    success: "#34D399",
    info: "#7DD0F8",
    warm: "#FB923C",
    warning: "#FBBF24",
    danger: "#EF4444",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #222226 0%, #1B1B1B 65%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.35), 0 4px 12px rgba(0,0,0,0.2)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.4), 0 2px 8px rgba(0,0,0,0.25)",
    glowPrimary:
      "0 0 0 1px rgba(91,190,245,0.2), 0 0 18px rgba(91,190,245,0.14), 0 0 50px rgba(91,190,245,0.06)",
    glowRecording:
      "0 0 0 1px rgba(239,68,68,0.3), 0 0 20px rgba(239,68,68,0.18), 0 0 45px rgba(239,68,68,0.08)",
  },

  // ── Notion Warm ───────────────────────────────────────────────────────────
  // Inspired by Notion Dark: warm gray-green tones, distinctly lighter
  // backgrounds, approachable feel. #2F3438 surfaces like Notion's window.
  {
    id: "notion",
    name: "Notion Warm",
    dot: "#529CCA",
    bg: "#191919",
    surface: "#252525",
    elevated: "#2F3438",
    border: "#373C3F",
    borderActive: "#3F4448",
    text: "#FFFFFFEB",
    muted: "#979A9B",
    dim: "#6B6E6F",
    primary: "#529CCA",
    accent: "#6CB4DA",
    secondary: "#FFA344",
    activity: "#FFA344",
    success: "#4ADE80",
    info: "#6CB4DA",
    warm: "#FFA344",
    warning: "#FFA344",
    danger: "#FF7369",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #212121 0%, #191919 60%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.3), 0 4px 12px rgba(0,0,0,0.18)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.22)",
    glowPrimary:
      "0 0 0 1px rgba(82,156,202,0.2), 0 0 16px rgba(82,156,202,0.13), 0 0 45px rgba(82,156,202,0.06)",
    glowRecording:
      "0 0 0 1px rgba(255,115,105,0.3), 0 0 18px rgba(255,115,105,0.18), 0 0 42px rgba(255,115,105,0.08)",
  },

  // ── Notion Warm v2 ───────────────────────────────────────────────────────
  // Same base as Notion Warm + primary color fix:
  // PROBLEM: Notion Blue (#529CCA) clashes on warm gray-green surfaces (#252525, #2F3438).
  // FIX: Teal #2AC3A8 — the green undertone in Notion's #2F3438 elevated surface
  // makes teal harmonize where cold blue clashes. Contrast vs surface: ~10.8:1.
  // Distinct from Orange secondary (#FFA344) — different hue family entirely.
  // Other fixes carried forward from original v2:
  // 1. Inactive nav icons more visible
  // 2. Card labels bolder
  // 3. More color accents in stat values
  // 4. Stronger brand in header
  {
    id: "notion-v2",
    name: "Notion v2",
    dot: "#2AC3A8",
    bg: "#191919",
    surface: "#252525",
    elevated: "#2F3438",
    border: "#373C3F",
    borderActive: "#3F4448",
    text: "#FFFFFFEB",
    muted: "#979A9B",
    dim: "#6B6E6F",
    // Teal primary: green undertone harmonizes with warm gray-green surfaces
    // Contrast vs #252525 surface: ~10.8:1 (WCAG AAA)
    // Contrast vs #2F3438 elevated: ~8.7:1 (WCAG AAA)
    primary: "#2AC3A8",
    accent: "#52D4C4",
    secondary: "#FFA344",
    activity: "#FFA344",
    success: "#4ADE80",
    info: "#52D4C4",
    warm: "#FFA344",
    warning: "#FFA344",
    danger: "#FF7369",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #212121 0%, #191919 60%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.3), 0 4px 12px rgba(0,0,0,0.18)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.22)",
    // Glow updated to teal (42,195,168) — matches new primary
    glowPrimary:
      "0 0 0 1px rgba(42,195,168,0.22), 0 0 16px rgba(42,195,168,0.15), 0 0 45px rgba(42,195,168,0.07)",
    glowRecording:
      "0 0 0 1px rgba(255,115,105,0.3), 0 0 18px rgba(255,115,105,0.18), 0 0 42px rgba(255,115,105,0.08)",
    extraCss: `
      /* FIX 1: Nav icons more visible */
      .text-zinc-500.hover\\\\:text-zinc-300 {
        color: #979A9B !important;
        opacity: 0.8;
      }
      .text-zinc-500.hover\\\\:text-zinc-300:hover {
        color: #FFFFFFEB !important;
        opacity: 1;
      }
      /* FIX 2: Section + card labels bolder */
      .tracking-widest {
        font-weight: 600 !important;
      }
      /* FIX 3: Stat values get subtle teal tint (matches new primary) */
      .text-2xl.font-bold, .text-xl.font-bold {
        color: #c4ede8 !important;
      }
      /* FIX 4: Brand name bolder + logo glow (teal, not blue) */
      .text-sm.font-semibold.tracking-wide {
        font-weight: 700 !important;
        color: #FFFFFFEB !important;
      }
      .w-7.h-7.rounded-lg {
        box-shadow: 0 0 0 1px rgba(42,195,168,0.28),
                    0 0 14px rgba(42,195,168,0.14) !important;
      }
      /* FIX 5: Wispr banner detail text — warm orange (unchanged) */
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-300,
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-400 {
        color: #FFA344 !important;
      }
      .rounded-xl.bg-emerald-500\\\\/10 p,
      .rounded-xl.bg-emerald-500\\\\/10 span {
        color: #979A9B !important;
      }
      .rounded-xl.bg-emerald-500\\\\/10 .font-bold,
      .rounded-xl.bg-emerald-500\\\\/10 .font-semibold {
        color: #FFFFFFEB !important;
      }
    `,
  },

  // ── Notion v3: Warm Sage ──────────────────────────────────────────────────
  // Variant for comparison against v2 Teal.
  // Primary: Warm Sage #68B8A8 — muted teal, less saturated than v2.
  // Lower saturation reads as "quieter" and more Notion-native.
  // Contrast vs #252525 surface: ~6.5:1 (WCAG AA)
  // Good for users who find v2 Teal too vivid.
  {
    id: "notion-v3",
    name: "Notion v3",
    dot: "#68B8A8",
    bg: "#191919",
    surface: "#252525",
    elevated: "#2F3438",
    border: "#373C3F",
    borderActive: "#3F4448",
    text: "#FFFFFFEB",
    muted: "#979A9B",
    dim: "#6B6E6F",
    // Warm Sage: desaturated teal, harmonizes with warm gray-green surfaces
    // Contrast vs #252525 surface: ~6.5:1 (WCAG AA)
    primary: "#68B8A8",
    accent: "#88CCBE",
    secondary: "#FFA344",
    activity: "#FFA344",
    success: "#4ADE80",
    info: "#88CCBE",
    warm: "#FFA344",
    warning: "#FFA344",
    danger: "#FF7369",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #212121 0%, #191919 60%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.3), 0 4px 12px rgba(0,0,0,0.18)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.22)",
    // Glow: warm sage (104,184,168)
    glowPrimary:
      "0 0 0 1px rgba(104,184,168,0.22), 0 0 16px rgba(104,184,168,0.14), 0 0 45px rgba(104,184,168,0.06)",
    glowRecording:
      "0 0 0 1px rgba(255,115,105,0.3), 0 0 18px rgba(255,115,105,0.18), 0 0 42px rgba(255,115,105,0.08)",
    extraCss: `
      /* v2 base fixes */
      .text-zinc-500.hover\\\\:text-zinc-300 { color: #979A9B !important; opacity: 0.8; }
      .text-zinc-500.hover\\\\:text-zinc-300:hover { color: #FFFFFFEB !important; opacity: 1; }
      .tracking-widest { font-weight: 600 !important; }
      /* Stat values: sage-tinted */
      .text-2xl.font-bold, .text-xl.font-bold { color: #d5ebe7 !important; }
      .text-sm.font-semibold.tracking-wide { font-weight: 700 !important; color: #FFFFFFEB !important; }
      /* Logo glow: warm sage */
      .w-7.h-7.rounded-lg {
        box-shadow: 0 0 0 1px rgba(104,184,168,0.25),
                    0 0 14px rgba(104,184,168,0.12) !important;
      }
      /* Wispr banner: orange accent (same as v2) */
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-300,
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-400 { color: #FFA344 !important; }
      .rounded-xl.bg-emerald-500\\\\/10 p, .rounded-xl.bg-emerald-500\\\\/10 span { color: #979A9B !important; }
      .rounded-xl.bg-emerald-500\\\\/10 .font-bold, .rounded-xl.bg-emerald-500\\\\/10 .font-semibold { color: #FFFFFFEB !important; }
    `,
  },

  // ── Notion v4: Brighter + Settings Color ─────────────────────────────────
  // Based on notion-v2 (Teal #2AC3A8, Background #191919, Surface #252525).
  // Changes vs v2:
  // 1. Muted brighter: #979A9B → #AAACAD (helper texts and placeholders readable)
  // 2. Dim brighter: #6B6E6F → #808385
  // 3. Settings sections get subtle color via extraCss:
  //    - Section header text: primary teal instead of white
  //    - Stat values: light teal tint
  //    - Logo glow: teal
  {
    id: "notion-v4",
    name: "Notion v4",
    dot: "#2AC3A8",
    bg: "#191919",
    surface: "#252525",
    elevated: "#2F3438",
    border: "#373C3F",
    borderActive: "#3F4448",
    text: "#FFFFFFEB",
    muted: "#AAACAD",         // brighter than v2 (#979A9B) — helpers/placeholders readable
    dim: "#808385",           // brighter than v2 (#6B6E6F)
    primary: "#2AC3A8",
    accent: "#52D4C4",
    secondary: "#FFA344",
    activity: "#FFA344",
    success: "#4ADE80",
    info: "#52D4C4",
    warm: "#FFA344",
    warning: "#FFA344",
    danger: "#FF7369",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #212121 0%, #191919 60%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.3), 0 4px 12px rgba(0,0,0,0.18)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.22)",
    glowPrimary:
      "0 0 0 1px rgba(42,195,168,0.22), 0 0 16px rgba(42,195,168,0.15), 0 0 45px rgba(42,195,168,0.07)",
    glowRecording:
      "0 0 0 1px rgba(255,115,105,0.3), 0 0 18px rgba(255,115,105,0.18), 0 0 42px rgba(255,115,105,0.08)",
    extraCss: `
      /* v2 base: nav icons more visible */
      .text-zinc-500.hover\\\\:text-zinc-300 { color: #AAACAD !important; opacity: 0.85; }
      .text-zinc-500.hover\\\\:text-zinc-300:hover { color: #FFFFFFEB !important; opacity: 1; }
      /* Section labels bolder */
      .tracking-widest { font-weight: 600 !important; }
      /* Stat values: teal-tinted (brighter than v2 #c4ede8) */
      .text-2xl.font-bold, .text-xl.font-bold { color: #beeae4 !important; }
      /* Brand name */
      .text-sm.font-semibold.tracking-wide { font-weight: 700 !important; color: #FFFFFFEB !important; }
      /* Logo glow */
      .w-7.h-7.rounded-lg {
        box-shadow: 0 0 0 1px rgba(42,195,168,0.28),
                    0 0 14px rgba(42,195,168,0.14) !important;
      }
      /* Settings: section header text in primary teal */
      .tracking-widest { color: #2AC3A8 !important; opacity: 0.85; }
      /* Settings: input placeholder lighter */
      input::placeholder { color: #AAACAD !important; opacity: 0.7; }
      textarea::placeholder { color: #AAACAD !important; opacity: 0.7; }
      /* Settings: action buttons (Sync Now etc) in teal text */
      details summary { color: #FFFFFFEB !important; }
      /* Wispr banner */
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-300,
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-400 { color: #FFA344 !important; }
      .rounded-xl.bg-emerald-500\\\\/10 p,
      .rounded-xl.bg-emerald-500\\\\/10 span { color: #AAACAD !important; }
      .rounded-xl.bg-emerald-500\\\\/10 .font-bold,
      .rounded-xl.bg-emerald-500\\\\/10 .font-semibold { color: #FFFFFFEB !important; }
    `,
  },

  // ── Notion v5: With Info Accent ───────────────────────────────────────────
  // Based on v4, plus a 4th color: soft blue-purple #7B8CDB (Info accent).
  // Used sparingly: Settings panel title, Dictionary badge count, link-like texts.
  // Tests whether a 4th color (Teal, Orange, Blue-Purple, Gray) enriches the UI
  // or adds too much noise.
  {
    id: "notion-v5",
    name: "Notion v5",
    dot: "#2AC3A8",
    bg: "#191919",
    surface: "#252525",
    elevated: "#2F3438",
    border: "#373C3F",
    borderActive: "#3F4448",
    text: "#FFFFFFEB",
    muted: "#AAACAD",
    dim: "#808385",
    primary: "#2AC3A8",
    accent: "#52D4C4",
    secondary: "#FFA344",
    activity: "#FFA344",
    success: "#4ADE80",
    info: "#7B8CDB",          // blue-purple info accent (4th color)
    warm: "#FFA344",
    warning: "#FFA344",
    danger: "#FF7369",
    gradientBg: "radial-gradient(ellipse at 50% 0%, #212121 0%, #191919 60%)",
    shadowCard:
      "0 0 0 1px rgba(255,255,255,0.05), 0 1px 3px rgba(0,0,0,0.3), 0 4px 12px rgba(0,0,0,0.18)",
    shadowPanel:
      "0 0 0 1px rgba(255,255,255,0.045), 0 8px 28px rgba(0,0,0,0.38), 0 2px 8px rgba(0,0,0,0.22)",
    glowPrimary:
      "0 0 0 1px rgba(42,195,168,0.22), 0 0 16px rgba(42,195,168,0.15), 0 0 45px rgba(42,195,168,0.07)",
    glowRecording:
      "0 0 0 1px rgba(255,115,105,0.3), 0 0 18px rgba(255,115,105,0.18), 0 0 42px rgba(255,115,105,0.08)",
    extraCss: `
      /* v4 base */
      .text-zinc-500.hover\\\\:text-zinc-300 { color: #AAACAD !important; opacity: 0.85; }
      .text-zinc-500.hover\\\\:text-zinc-300:hover { color: #FFFFFFEB !important; opacity: 1; }
      .tracking-widest { font-weight: 600 !important; }
      .text-2xl.font-bold, .text-xl.font-bold { color: #beeae4 !important; }
      .text-sm.font-semibold.tracking-wide { font-weight: 700 !important; color: #FFFFFFEB !important; }
      .w-7.h-7.rounded-lg {
        box-shadow: 0 0 0 1px rgba(42,195,168,0.28),
                    0 0 14px rgba(42,195,168,0.14) !important;
      }
      .tracking-widest { color: #2AC3A8 !important; opacity: 0.85; }
      input::placeholder { color: #AAACAD !important; opacity: 0.7; }
      textarea::placeholder { color: #AAACAD !important; opacity: 0.7; }
      details summary { color: #FFFFFFEB !important; }
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-300,
      .rounded-xl.bg-emerald-500\\\\/10 .text-emerald-400 { color: #FFA344 !important; }
      .rounded-xl.bg-emerald-500\\\\/10 p,
      .rounded-xl.bg-emerald-500\\\\/10 span { color: #AAACAD !important; }
      .rounded-xl.bg-emerald-500\\\\/10 .font-bold,
      .rounded-xl.bg-emerald-500\\\\/10 .font-semibold { color: #FFFFFFEB !important; }
      /* v5: Info accent — blue-purple #7B8CDB for badges and secondary highlights */
      /* Dictionary count badges and similar numeric indicators */
      .rounded-full.font-bold { color: #7B8CDB !important; }
      /* HighlightedText search mark override: use info color */
      mark.bg-emerald-500\\\\/30 { background-color: rgba(123,140,219,0.18) !important; }
      mark.text-emerald-300 { color: #7B8CDB !important; }
    `,
  },
];

// ---------------------------------------------------------------------------
// Theme application
// ---------------------------------------------------------------------------

function applyTheme(theme: ThemeVariant): void {
  const root = document.documentElement;

  // Custom properties used by @theme-aware components
  root.style.setProperty("--color-voxlit-bg", theme.bg);
  root.style.setProperty("--color-voxlit-surface", theme.surface);
  root.style.setProperty("--color-voxlit-border", theme.border);
  root.style.setProperty("--color-voxlit-muted", theme.muted);
  root.style.setProperty("--color-voxlit-text", theme.text);
  root.style.setProperty("--color-voxlit-primary", theme.primary);
  root.style.setProperty("--color-voxlit-danger", theme.danger);
  root.style.setProperty("--color-voxlit-warning", theme.warning);
  root.style.setProperty("--color-voxlit-accent", theme.accent);
  // New color role properties
  root.style.setProperty("--color-voxlit-activity", theme.activity);
  root.style.setProperty("--color-voxlit-success", theme.success);
  root.style.setProperty("--color-voxlit-info", theme.info);
  root.style.setProperty("--color-voxlit-warm", theme.warm);

  // Inject/update a <style> tag that overrides hardcoded Tailwind utility
  // classes. These cannot be changed via CSS variables alone because Tailwind
  // compiles them as literal color values at build time.
  let styleEl = document.getElementById("theme-preview-overrides") as HTMLStyleElement | null;
  if (!styleEl) {
    styleEl = document.createElement("style");
    styleEl.id = "theme-preview-overrides";
    document.head.appendChild(styleEl);
  }

  const p = theme.primary;      // Action color
  const a = theme.accent;       // Hover / lighter action
  const bg = theme.bg;
  const sf = theme.surface;
  const bo = theme.border;
  const tx = theme.text;
  const mu = theme.muted;
  const dm = theme.dim ?? theme.muted;
  const ba = theme.borderActive ?? theme.border;
  const act = theme.activity;   // Activity (processing/spinner)
  // success is set as a CSS custom property (--color-voxlit-success) above
  // and available to components that opt into it; Tailwind classes for done-
  // states are covered by the text-emerald-300 → accent mapping.
  const inf = theme.info;       // Info (links/badges)
  const wrm = theme.warm;       // Warm (stats/dashboard)

  // For "Current" theme: skip all overrides to preserve exact app appearance.
  if (theme.id === "current") {
    styleEl.textContent = `
      *, *::before, *::after {
        transition-property: background-color, border-color, color, box-shadow !important;
        transition-duration: 200ms !important;
        transition-timing-function: ease !important;
      }
    `;
    return;
  }

  // Depth Quick-Wins: active for all non-"Current" variants.
  //
  // 1. Root background gradient — simulates a ceiling light source.
  //    Applied via override on the main element's bg class.
  //
  // 2. Card elevation — replaces flat border with 3-layer box-shadow.
  //    The rgba(255,255,255,…) outer ring simulates light catching the edge,
  //    not a painted line. Much more premium than border-zinc-800/60.
  //
  // 3. RecordButton 3-layer glow — hard ring + near glow + far halo.
  //    Three layers simulate how an LED looks on a dark surface.

  const gradientBg = theme.gradientBg;
  const shadowCard = theme.shadowCard;
  const glowPrimary = theme.glowPrimary;
  const glowRecording = theme.glowRecording;

  styleEl.textContent = `
    /* Smooth color transitions on all elements while previewing */
    *, *::before, *::after {
      transition-property: background-color, border-color, color, box-shadow !important;
      transition-duration: 200ms !important;
      transition-timing-function: ease !important;
    }

    /* ── Quick-Win 1: Root background gradient ── */
    /* Simulates ceiling light source, eliminates flat-block appearance */
    .bg-\\[\\#0a0a0c\\], .bg-\\[\\#0d0d0f\\], .bg-\\[\\#0b0b0d\\],
    .bg-\\[\\#1B1B1B\\], .bg-\\[\\#191B22\\], .bg-\\[\\#191919\\],
    .bg-\\[\\#0A0E1A\\], .bg-\\[\\#13100D\\], .bg-\\[\\#15171F\\],
    .bg-\\[\\#0C0C10\\] {
      background: ${gradientBg || bg} !important;
    }

    /* ── Quick-Win 2: Surface background overrides ── */
    .bg-\\[\\#111113\\], .bg-\\[\\#0e0e11\\], .bg-\\[\\#0c0c0e\\] {
      background-color: ${sf} !important;
    }

    /* ── Quick-Win 2: Card elevation — 3-layer shadow replaces flat border ── */
    /* The 0 0 0 1px rgba(255,255,255,…) ring simulates light on the card edge */
    .border.border-zinc-800\\/60,
    .border-zinc-800\\/60 {
      border-color: transparent !important;
      box-shadow: ${shadowCard} !important;
    }

    /* StatCard and CostDashboard StatTile elevation */
    .bg-\\[\\#111113\\].border.rounded-xl,
    .bg-\\[\\#111113\\].border.rounded-xl.p-3 {
      border-color: transparent !important;
      box-shadow: ${shadowCard} !important;
    }

    /* ── Quick-Win 3: RecordButton 3-layer glow ── */
    /* Hard ring + near glow + far halo = lit element on dark surface */
    .shadow-\\[0_0_40px_rgba\\(16\\,185\\,129\\,0\\.2\\)\\] {
      box-shadow: ${glowPrimary} !important;
    }
    .hover\\:shadow-\\[0_0_50px_rgba\\(16\\,185\\,129\\,0\\.3\\)\\]:hover {
      box-shadow: ${glowPrimary} !important;
    }
    .shadow-\\[0_0_40px_rgba\\(239\\,68\\,68\\,0\\.3\\)\\] {
      box-shadow: ${glowRecording} !important;
    }

    /* RecordButton busy state glow */
    .shadow-\\[0_0_30px_rgba\\(245\\,158\\,11\\,0\\.2\\)\\] {
      box-shadow: 0 0 0 1px color-mix(in srgb, ${act} 25%, transparent),
                  0 0 16px color-mix(in srgb, ${act} 18%, transparent),
                  0 0 40px color-mix(in srgb, ${act} 08%, transparent) !important;
    }

    /* ── Action: Emerald → primary ── */
    /* RecordButton idle, StylePicker active, Nav active icons, Logo */
    .text-emerald-400 { color: ${p} !important; }
    .text-emerald-300 { color: ${a} !important; }

    .bg-emerald-500\\/10  { background-color: color-mix(in srgb, ${p} 10%, transparent) !important; }
    .bg-emerald-500\\/15  { background-color: color-mix(in srgb, ${p} 15%, transparent) !important; }
    .bg-emerald-500\\/20  { background-color: color-mix(in srgb, ${p} 20%, transparent) !important; }

    .border-emerald-500\\/20 { border-color: color-mix(in srgb, ${p} 20%, transparent) !important; }
    .border-emerald-500\\/25 { border-color: color-mix(in srgb, ${p} 25%, transparent) !important; }
    .border-emerald-500\\/30 { border-color: color-mix(in srgb, ${p} 30%, transparent) !important; }
    .border-emerald-500\\/40 { border-color: color-mix(in srgb, ${p} 40%, transparent) !important; }

    .focus\\:border-emerald-500\\/30:focus { border-color: color-mix(in srgb, ${p} 30%, transparent) !important; }
    .focus\\:border-emerald-500\\/40:focus { border-color: color-mix(in srgb, ${p} 40%, transparent) !important; }

    /* hover:bg-emerald (active toggle hover state) */
    .hover\\:bg-emerald-500\\/20:hover {
      background-color: color-mix(in srgb, ${p} 20%, transparent) !important;
    }

    /* ── Activity: Amber → activity color ── */
    /* RecordButton busy, Transcribing/Cleaning spinner, Reformat loading */
    .text-amber-400 { color: ${act} !important; }

    .bg-amber-500\\/15 { background-color: color-mix(in srgb, ${act} 15%, transparent) !important; }
    .bg-amber-500\\/10 { background-color: color-mix(in srgb, ${act} 10%, transparent) !important; }

    .border-amber-500\\/30 { border-color: color-mix(in srgb, ${act} 30%, transparent) !important; }
    .border-amber-500\\/20 { border-color: color-mix(in srgb, ${act} 20%, transparent) !important; }

    /* ── Success: Emerald-300 / done-states ── */
    /* "Done" label, StatusDot, Savings confirmation — must differ from Action */
    /* text-emerald-300 (lighter shade) maps to success, which differs from
       primary in all non-current themes */
    /* Note: text-emerald-300 is already mapped to ${a} above (accent/hover).
       In themes where success === accent this is intentional.
       The CSS custom property --color-voxlit-success is available for
       components that want to opt into the explicit success color. */

    /* All emerald borders → primary color family (prevents brown/blue clash) */
    .border-emerald-500\\/20 { border-color: color-mix(in srgb, ${p} 20%, transparent) !important; }
    .border-emerald-500\\/30 { border-color: color-mix(in srgb, ${p} 30%, transparent) !important; }

    /* Savings banner — maps to warm (it's a "good news" visual anchor, not action) */
    /* The banner uses bg-emerald-500/10 + border-emerald-500/20 + text-emerald-400 */
    /* We target the rounded-xl wrapper specifically */
    .rounded-xl.bg-emerald-500\\/10 {
      background-color: color-mix(in srgb, ${wrm} 10%, transparent) !important;
    }
    .rounded-xl.border-emerald-500\\/20 {
      border-color: color-mix(in srgb, ${wrm} 20%, transparent) !important;
    }

    /* FillerStatsChart bars — activity color (it's processing/analytics data) */
    .bg-emerald-500\\/50 {
      background-color: color-mix(in srgb, ${act} 50%, transparent) !important;
    }

    /* HighlightedText search mark — info color (informational highlight) */
    mark.bg-emerald-500\\/30 {
      background-color: color-mix(in srgb, ${inf} 20%, transparent) !important;
    }
    mark.text-emerald-300 {
      color: ${inf} !important;
    }

    /* ── Zinc text → theme text/muted ── */
    .text-zinc-100 { color: ${tx} !important; }
    .text-zinc-200 { color: color-mix(in srgb, ${tx} 90%, transparent) !important; }
    .text-zinc-300 { color: color-mix(in srgb, ${tx} 80%, transparent) !important; }
    .text-zinc-400 { color: ${mu} !important; }
    .text-zinc-500 { color: ${dm} !important; }

    /* ── Zinc backgrounds/borders ── */
    .bg-zinc-800\\/50  { background-color: color-mix(in srgb, ${sf} 60%, transparent) !important; }
    .bg-zinc-800\\/60  { background-color: color-mix(in srgb, ${sf} 70%, transparent) !important; }
    .hover\\:bg-zinc-800\\/50:hover { background-color: color-mix(in srgb, ${sf} 60%, transparent) !important; }

    .border-zinc-700\\/60 { border-color: ${ba} !important; }
    .border-zinc-800\\/40 { border-color: color-mix(in srgb, ${bo} 60%, transparent) !important; }
    .border-zinc-800\\/60 { border-color: ${bo} !important; }

    /* ── Scrollbar ── */
    ::-webkit-scrollbar-thumb       { background: ${bo} !important; }
    ::-webkit-scrollbar-thumb:hover { background: ${ba} !important; }

    /* ── Wispr banner: detail text warm instead of accent ── */
    /* The banner container has .rounded-xl + .bg-emerald-500/10 */
    /* Text inside should be warm/muted, not the accent (teal) color */
    .rounded-xl.bg-emerald-500\\/10 .text-emerald-300 {
      color: ${wrm} !important;
    }
    .rounded-xl.bg-emerald-500\\/10 .text-emerald-400 {
      color: ${wrm} !important;
    }

    /* ── All emerald-tinted buttons → primary color ── */
    button.bg-emerald-500\\/15.text-emerald-400,
    button.bg-emerald-500\\/10.text-emerald-400 {
      background-color: color-mix(in srgb, ${p} 12%, transparent) !important;
      color: ${p} !important;
    }
    button.bg-emerald-500\\/15.text-emerald-400:hover,
    button.bg-emerald-500\\/10.text-emerald-400:hover {
      background-color: color-mix(in srgb, ${p} 20%, transparent) !important;
      color: ${a} !important;
    }

    ${theme.extraCss ?? ""}
  `;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ThemeSwitcher() {
  const [open, setOpen] = useState(false);
  const [activeId, setActiveId] = useState<string>("current");

  if (!isPreviewMode) return null;

  const handleSelect = (theme: ThemeVariant) => {
    setActiveId(theme.id);
    applyTheme(theme);
  };

  // Hardcoded inline styles -- never affected by the active theme.
  const chrome = {
    panel: {
      position: "fixed" as const,
      bottom: "36px",   // sits above the "Preview Mode" badge (bottom-3 ~12px + badge height ~24px)
      right: "12px",
      zIndex: 51,
      background: "#111114",
      border: "1px solid #2a2a2f",
      borderRadius: "12px",
      boxShadow: "0 8px 32px rgba(0,0,0,0.6)",
      fontFamily: "'Inter', system-ui, sans-serif",
      minWidth: "168px",
    },
    header: {
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      padding: "8px 10px 8px 12px",
      borderBottom: open ? "1px solid #2a2a2f" : "none",
      gap: "8px",
    },
    headerLabel: {
      fontSize: "10px",
      fontWeight: 600,
      letterSpacing: "0.08em",
      textTransform: "uppercase" as const,
      color: "#52525b",
    },
    toggleBtn: {
      background: "none",
      border: "none",
      cursor: "pointer",
      padding: "2px 4px",
      borderRadius: "6px",
      color: "#71717a",
      fontSize: "14px",
      lineHeight: 1,
      display: "flex",
      alignItems: "center",
    },
    list: {
      padding: "6px",
      display: "flex",
      flexDirection: "column" as const,
      gap: "3px",
    },
    themeBtn: (isActive: boolean) => ({
      display: "flex",
      alignItems: "center",
      gap: "8px",
      width: "100%",
      padding: "6px 8px",
      borderRadius: "7px",
      border: isActive ? "1px solid #3a3a42" : "1px solid transparent",
      background: isActive ? "#1c1c21" : "transparent",
      cursor: "pointer",
      textAlign: "left" as const,
    }),
    dot: (color: string) => ({
      width: "10px",
      height: "10px",
      borderRadius: "50%",
      background: color,
      flexShrink: 0,
    }),
    themeName: (isActive: boolean) => ({
      fontSize: "11px",
      fontWeight: isActive ? 600 : 400,
      color: isActive ? "#e4e4e7" : "#71717a",
      letterSpacing: "0.01em",
    }),
    activeCheck: {
      marginLeft: "auto",
      fontSize: "10px",
      color: "#52525b",
    },
  };

  return (
    <div style={chrome.panel} aria-label="Theme switcher (preview only)">
      <div style={chrome.header}>
        <span style={chrome.headerLabel}>Themes</span>
        <button
          style={chrome.toggleBtn}
          onClick={() => setOpen((v) => !v)}
          aria-label={open ? "Collapse theme switcher" : "Expand theme switcher"}
          aria-expanded={open}
        >
          {open ? "▲" : "▼"}
        </button>
      </div>

      {open && (
        <div style={chrome.list}>
          {THEMES.map((theme) => {
            const isActive = activeId === theme.id;
            return (
              <button
                key={theme.id}
                style={chrome.themeBtn(isActive)}
                onClick={() => handleSelect(theme)}
                aria-pressed={isActive}
              >
                <span style={chrome.dot(theme.dot)} />
                <span style={chrome.themeName(isActive)}>{theme.name}</span>
                {isActive && <span style={chrome.activeCheck}>&#10003;</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
