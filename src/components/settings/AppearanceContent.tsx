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
import { KToggle, KSelect, KSlider, KSegmented } from "./FormControls";

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
        <KToggle
          checked={localLivePreviewEnabled}
          onChange={setLocalLivePreviewEnabled}
        />
      </div>

      {localLivePreviewEnabled && (
        <>
          {/* Preview Pause */}
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className={LABEL_CLS}>Preview Pause</span>
              <span className="text-xs font-mono text-klarvo-teal">{localPreviewPauseSilenceSecs.toFixed(1)}s</span>
            </div>
            <KSlider
              min={0.5}
              max={5.0}
              step={0.1}
              value={localPreviewPauseSilenceSecs}
              onChange={setLocalPreviewPauseSilenceSecs}
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
                  className="flex-1 py-1 rounded-md text-xs font-medium text-klarvo-muted hover:text-klarvo-text bg-klarvo-bg border border-klarvo-border/60 hover:border-klarvo-teal/40 transition-all duration-100 whitespace-nowrap"
                >
                  {theme.label}
                </button>
              ))}
            </div>

            {/* Text Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Text color</span>
                <span className="text-xs font-mono text-klarvo-teal">
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
                <div className="flex-1">
                  <KSlider
                    min={0}
                    max={100}
                    step={1}
                    value={rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88).opacityPct}
                    onChange={(v) => {
                      const { hex } = rgbaToHexOpacity(localPreviewTextColor, "#dcdcdc", 88);
                      setLocalPreviewTextColor(hexOpacityToRgba(hex, Math.round(v), DEFAULT_TEXT_COLOR));
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Background Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Bg color</span>
                <span className="text-xs font-mono text-klarvo-teal">
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
                <div className="flex-1">
                  <KSlider
                    min={0}
                    max={100}
                    step={1}
                    value={rgbaToHexOpacity(localPreviewBgColor, "#191919", 96).opacityPct}
                    onChange={(v) => {
                      const { hex } = rgbaToHexOpacity(localPreviewBgColor, "#191919", 96);
                      setLocalPreviewBgColor(hexOpacityToRgba(hex, Math.round(v), DEFAULT_BG_COLOR));
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Bg Blur */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Bg blur</span>
                <span className="text-xs font-mono text-klarvo-teal">{localPreviewBgBlur}px</span>
              </div>
              <KSlider
                min={0}
                max={20}
                step={1}
                value={localPreviewBgBlur}
                onChange={(v) => setLocalPreviewBgBlur(Math.round(v))}
              />
            </div>

            {/* Border Color */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Border color</span>
                <span className="text-xs font-mono text-klarvo-teal">
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
                <div className="flex-1">
                  <KSlider
                    min={0}
                    max={100}
                    step={1}
                    value={rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25).opacityPct}
                    onChange={(v) => {
                      const { hex } = rgbaToHexOpacity(localPreviewBorderColor, "#2ac3a8", 25);
                      setLocalPreviewBorderColor(hexOpacityToRgba(hex, Math.round(v), DEFAULT_BORDER_COLOR));
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Border Width */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Border thickness</span>
                <span className="text-xs font-mono text-klarvo-teal">{localPreviewBorderWidth}px</span>
              </div>
              <KSlider
                min={0}
                max={5}
                step={1}
                value={localPreviewBorderWidth}
                onChange={(v) => setLocalPreviewBorderWidth(Math.round(v))}
              />
            </div>

            {/* Corner Radius */}
            <div className="flex flex-col gap-0.5">
              <div className="flex items-center justify-between">
                <span className="text-[11px] text-klarvo-muted">Corner radius</span>
                <span className="text-xs font-mono text-klarvo-teal">{localPreviewBorderRadius}px</span>
              </div>
              <KSlider
                min={0}
                max={24}
                step={1}
                value={localPreviewBorderRadius}
                onChange={(v) => setLocalPreviewBorderRadius(Math.round(v))}
              />
            </div>

            {/* Font family */}
            <div className="flex items-center justify-between gap-2">
              <span className="text-[11px] text-klarvo-muted min-w-[90px]">Font family</span>
              <div className="flex-1">
                <KSelect
                  value={
                    PREVIEW_FONTS.find((f) => f.stack === localPreviewFontFamily)?.stack
                    ?? PREVIEW_FONTS[0].stack
                  }
                  onChange={setLocalPreviewFontFamily}
                  options={PREVIEW_FONTS.map((f) => ({ value: f.stack, label: f.label }))}
                />
              </div>
            </div>

            {/* Font-size picker — affects card geometry (k-scaling) */}
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>Schriftgröße</span>
              <KSegmented
                value={localPreviewFontSize}
                onChange={setLocalPreviewFontSize}
                options={[
                  { value: "small", label: "Klein" },
                  { value: "medium", label: "Mittel" },
                  { value: "large", label: "Groß" },
                ]}
              />
              <p className="text-[11px] text-klarvo-muted">
                Skaliert Breite, Höhe und Schrift der Vorschau proportional.
              </p>
            </div>

            {/* Display Form preset picker */}
            <div className="flex flex-col gap-1.5">
              <span className={LABEL_CLS}>Darstellung</span>
              <KSegmented
                value={localPreviewPanelForm}
                onChange={setLocalPreviewPanelForm}
                options={[
                  { value: "compact", label: "Compact" },
                  { value: "comfortable", label: "Comfortable" },
                  { value: "wide", label: "Wide" },
                ]}
              />
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
