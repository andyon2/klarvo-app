import { LABEL_CLS } from "../ui";
import {
  rgbaToHexOpacity,
  hexOpacityToRgba,
  PREVIEW_THEMES,
  PREVIEW_FONTS,
  DEFAULT_TEXT_COLOR,
  DEFAULT_BG_COLOR,
  DEFAULT_BORDER_COLOR,
  DEFAULT_FONT_FAMILY,
} from "./previewAppearance";

// font-size → px mapping for the live preview card.
// Mirrors the FONT_PX constant in PreviewPanel.tsx (do not rename keys).
const FONT_PX_MAP: Record<string, number> = { small: 11, medium: 13, large: 15 };

// --- Props -------------------------------------------------------------------

export interface AppearanceContentProps {
  // Live Preview toggle + behavior (desktop only)
  localLivePreviewEnabled: boolean;
  setLocalLivePreviewEnabled: (v: boolean) => void;
  localPreviewPauseSilenceSecs: number;
  setLocalPreviewPauseSilenceSecs: (v: number) => void;
  // Live Preview display form preset
  localPreviewPanelForm: string;
  setLocalPreviewPanelForm: (v: string) => void;
  // Preview appearance fields
  localPreviewTextColor: string;
  setLocalPreviewTextColor: (v: string) => void;
  localPreviewBgColor: string;
  setLocalPreviewBgColor: (v: string) => void;
  localPreviewBgBlur: number;
  setLocalPreviewBgBlur: (v: number) => void;
  localPreviewBorderColor: string;
  setLocalPreviewBorderColor: (v: string) => void;
  localPreviewBorderWidth: number;
  setLocalPreviewBorderWidth: (v: number) => void;
  localPreviewBorderRadius: number;
  setLocalPreviewBorderRadius: (v: number) => void;
  localPreviewFontFamily: string;
  setLocalPreviewFontFamily: (v: string) => void;
  localPreviewFontSize: string;
  setLocalPreviewFontSize: (v: string) => void;
}

// --- Component ---------------------------------------------------------------

/**
 * Appearance settings for the desktop live-preview window.
 *
 * Previously buried under Shortcuts → Live Preview. Moved to its own
 * "Appearance" section so visual customization lives where users expect it,
 * and so Shortcuts holds only hotkeys/modes/paste behavior. The section is
 * desktop-only (the preview window does not exist on mobile) and is filtered
 * out of the settings home on mobile via `desktopOnly` in types.ts.
 */
export function AppearanceContent({
  localLivePreviewEnabled, setLocalLivePreviewEnabled,
  localPreviewPauseSilenceSecs, setLocalPreviewPauseSilenceSecs,
  localPreviewPanelForm, setLocalPreviewPanelForm,
  localPreviewTextColor, setLocalPreviewTextColor,
  localPreviewBgColor, setLocalPreviewBgColor,
  localPreviewBgBlur, setLocalPreviewBgBlur,
  localPreviewBorderColor, setLocalPreviewBorderColor,
  localPreviewBorderWidth, setLocalPreviewBorderWidth,
  localPreviewBorderRadius, setLocalPreviewBorderRadius,
  localPreviewFontFamily, setLocalPreviewFontFamily,
  localPreviewFontSize, setLocalPreviewFontSize,
}: AppearanceContentProps) {
  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
      {/* Live Preview enable toggle */}
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
        <>
          {/* Preview Pause */}
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

          {/* Appearance sub-section (themes-first; FR11-FR13) */}
          <div className="flex flex-col gap-2 border-t border-klarvo-border/30 pt-3 mt-1">
            <span className={LABEL_CLS}>Appearance</span>

            {/* Live preview card — driven directly from localPreview* state, updates on
                every control change before and independent of Save. */}
            <div
              style={{
                background: localPreviewBgColor || DEFAULT_BG_COLOR,
                backdropFilter: `blur(${localPreviewBgBlur}px)`,
                WebkitBackdropFilter: `blur(${localPreviewBgBlur}px)`,
                border: `${localPreviewBorderWidth}px solid ${localPreviewBorderColor || DEFAULT_BORDER_COLOR}`,
                borderRadius: `${localPreviewBorderRadius}px`,
                color: localPreviewTextColor || DEFAULT_TEXT_COLOR,
                fontFamily: localPreviewFontFamily || DEFAULT_FONT_FAMILY,
                fontSize: FONT_PX_MAP[localPreviewFontSize] ?? 11,
                padding: "8px 12px",
              }}
              className="leading-relaxed"
            >
              <div className="font-medium">Live-Vorschau</div>
              <div className="opacity-80">Transcribed text appears here…</div>
            </div>

            {/* Theme presets — one click sets all appearance fields at once. */}
            <div className="flex gap-1">
              {PREVIEW_THEMES.map((theme) => (
                <button
                  key={theme.label}
                  onClick={() => {
                    setLocalPreviewTextColor(theme.textColor);
                    setLocalPreviewBgColor(theme.bgColor);
                    setLocalPreviewBgBlur(theme.bgBlur);
                    setLocalPreviewBorderColor(theme.borderColor);
                    setLocalPreviewBorderWidth(theme.borderWidth);
                    setLocalPreviewBorderRadius(theme.borderRadius);
                  }}
                  className="flex-1 py-1 rounded-md text-xs font-medium text-klarvo-muted hover:text-klarvo-text bg-klarvo-bg border border-klarvo-border/60 hover:border-klarvo-primary/40 transition-all duration-100 whitespace-nowrap"
                >
                  {theme.label}
                </button>
              ))}
            </div>

            {/* Text Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Text color</span>
                <span className="text-xs font-mono text-klarvo-primary">
                  {rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88).opacityPct}%
                </span>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="color"
                  value={rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88).hex}
                  onChange={(e) => {
                    const { opacityPct } = rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88);
                    setLocalPreviewTextColor(hexOpacityToRgba(e.target.value, opacityPct, DEFAULT_TEXT_COLOR));
                  }}
                  className="w-7 h-6 rounded cursor-pointer border border-klarvo-border/60"
                />
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88).opacityPct}
                  onChange={(e) => {
                    const { hex } = rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88);
                    setLocalPreviewTextColor(hexOpacityToRgba(hex, parseInt(e.target.value, 10), DEFAULT_TEXT_COLOR));
                  }}
                  className="flex-1 accent-klarvo-primary"
                />
              </div>
            </div>

            {/* Background Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Bg color</span>
                <span className="text-xs font-mono text-klarvo-primary">
                  {rgbaToHexOpacity(localPreviewBgColor, "#191919", 96).opacityPct}%
                </span>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="color"
                  value={rgbaToHexOpacity(localPreviewBgColor, "#191919", 96).hex}
                  onChange={(e) => {
                    const { opacityPct } = rgbaToHexOpacity(localPreviewBgColor, "#191919", 96);
                    setLocalPreviewBgColor(hexOpacityToRgba(e.target.value, opacityPct, DEFAULT_BG_COLOR));
                  }}
                  className="w-7 h-6 rounded cursor-pointer border border-klarvo-border/60"
                />
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={rgbaToHexOpacity(localPreviewBgColor, "#191919", 96).opacityPct}
                  onChange={(e) => {
                    const { hex } = rgbaToHexOpacity(localPreviewBgColor, "#191919", 96);
                    setLocalPreviewBgColor(hexOpacityToRgba(hex, parseInt(e.target.value, 10), DEFAULT_BG_COLOR));
                  }}
                  className="flex-1 accent-klarvo-primary"
                />
              </div>
            </div>

            {/* Bg Blur */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Bg blur</span>
                <span className="text-xs font-mono text-klarvo-primary">{localPreviewBgBlur}px</span>
              </div>
              <input
                type="range"
                min={0}
                max={20}
                step={1}
                value={localPreviewBgBlur}
                onChange={(e) => setLocalPreviewBgBlur(parseInt(e.target.value, 10))}
                className="w-full accent-klarvo-primary"
              />
            </div>

            {/* Border Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Border color</span>
                <span className="text-xs font-mono text-klarvo-primary">
                  {rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25).opacityPct}%
                </span>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="color"
                  value={rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25).hex}
                  onChange={(e) => {
                    const { opacityPct } = rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25);
                    setLocalPreviewBorderColor(hexOpacityToRgba(e.target.value, opacityPct, DEFAULT_BORDER_COLOR));
                  }}
                  className="w-7 h-6 rounded cursor-pointer border border-klarvo-border/60"
                />
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25).opacityPct}
                  onChange={(e) => {
                    const { hex } = rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25);
                    setLocalPreviewBorderColor(hexOpacityToRgba(hex, parseInt(e.target.value, 10), DEFAULT_BORDER_COLOR));
                  }}
                  className="flex-1 accent-klarvo-primary"
                />
              </div>
            </div>

            {/* Border Width */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Border thickness</span>
                <span className="text-xs font-mono text-klarvo-primary">{localPreviewBorderWidth}px</span>
              </div>
              <input
                type="range"
                min={0}
                max={5}
                step={1}
                value={localPreviewBorderWidth}
                onChange={(e) => setLocalPreviewBorderWidth(parseInt(e.target.value, 10))}
                className="w-full accent-klarvo-primary"
              />
            </div>

            {/* Corner Radius */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Corner radius</span>
                <span className="text-xs font-mono text-klarvo-primary">{localPreviewBorderRadius}px</span>
              </div>
              <input
                type="range"
                min={0}
                max={24}
                step={1}
                value={localPreviewBorderRadius}
                onChange={(e) => setLocalPreviewBorderRadius(parseInt(e.target.value, 10))}
                className="w-full accent-klarvo-primary"
              />
            </div>

            {/* Font family */}
            <div className="flex items-center justify-between gap-2">
              <span className="text-[11px] text-klarvo-muted min-w-[90px]">Font family</span>
              <select
                value={
                  PREVIEW_FONTS.find((f) => f.stack === localPreviewFontFamily)?.stack
                  ?? PREVIEW_FONTS[0].stack
                }
                onChange={(e) => setLocalPreviewFontFamily(e.target.value)}
                className="flex-1 bg-klarvo-bg border border-klarvo-border/60 rounded px-2 py-0.5 text-xs text-klarvo-text"
              >
                {PREVIEW_FONTS.map((f) => (
                  <option key={f.label} value={f.stack}>
                    {f.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Font-size picker — affects card geometry (k-scaling) */}
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>Schriftgröße</span>
              <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                {(["small", "medium", "large"] as const).map((size) => (
                  <button
                    key={size}
                    onClick={() => setLocalPreviewFontSize(size)}
                    className={[
                      "flex-1 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      localPreviewFontSize === size
                        ? "bg-klarvo-primary/15 text-klarvo-primary"
                        : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    {size === "small" ? "Klein" : size === "medium" ? "Mittel" : "Groß"}
                  </button>
                ))}
              </div>
              <p className="text-[11px] text-klarvo-muted">
                Skaliert Breite, Höhe und Schrift der Vorschau proportional.
              </p>
            </div>

            {/* Display Form preset picker */}
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>Darstellung</span>
              <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
                {(["compact", "comfortable", "wide"] as const).map((preset) => (
                  <button
                    key={preset}
                    onClick={() => setLocalPreviewPanelForm(preset)}
                    className={[
                      "flex-1 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
                      localPreviewPanelForm === preset
                        ? "bg-klarvo-primary/15 text-klarvo-primary"
                        : "text-klarvo-dim hover:text-klarvo-muted",
                    ].join(" ")}
                  >
                    {preset === "compact" ? "Compact" : preset === "comfortable" ? "Comfortable" : "Wide"}
                  </button>
                ))}
              </div>
              <p className="text-[11px] text-klarvo-muted">
                Changes the width of the preview panel.
              </p>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
