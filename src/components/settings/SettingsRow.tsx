import type { ReactElement } from "react";
import { isMobile } from "../../platform";
import type { SettingsCategoryDef, SettingsCategory } from "./types";

// Inline SVG icons keyed by category id.
// Each icon is 16x16 (w-4 h-4) and uses currentColor so the parent
// can control colour via the `color` style prop.
const CATEGORY_ICONS: Record<SettingsCategory, ReactElement> = {
  home: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
      <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z" />
    </svg>
  ),
  "recording-audio": (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 1a4 4 0 0 1 4 4v6a4 4 0 0 1-8 0V5a4 4 0 0 1 4-4zm-1 17.93V21h2v-2.07A8.001 8.001 0 0 0 20 11h-2a6 6 0 0 1-12 0H4a8.001 8.001 0 0 0 7 7.93z" />
    </svg>
  ),
  "ai-providers": (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2L2 7l10 5 10-5-10-5z" />
      <path d="M2 17l10 5 10-5" />
      <path d="M2 12l10 5 10-5" />
    </svg>
  ),
  appearance: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
      <circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
      <circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
      <circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
      <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996C19.846 15.668 22 13.514 22 10.838 22 6.5 17.5 2 12 2z" />
    </svg>
  ),
  language: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  ),
  shortcuts: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="4" width="20" height="16" rx="2" />
      <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10" />
    </svg>
  ),
  license: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  ),
  dictionary: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
      <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
    </svg>
  ),
  advanced: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="4" y1="6" x2="20" y2="6" />
      <line x1="4" y1="12" x2="20" y2="12" />
      <line x1="4" y1="18" x2="20" y2="18" />
      <circle cx="9" cy="6" r="2" fill="currentColor" stroke="none" />
      <circle cx="15" cy="12" r="2" fill="currentColor" stroke="none" />
      <circle cx="9" cy="18" r="2" fill="currentColor" stroke="none" />
    </svg>
  ),
  about: (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="8" x2="12" y2="12" />
      <line x1="12" y1="16" x2="12.01" y2="16" />
    </svg>
  ),
};

const CHEVRON = (
  <svg className="w-4 h-4 text-klarvo-dim shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M9 18l6-6-6-6" />
  </svg>
);

interface SettingsRowProps {
  category: SettingsCategoryDef;
  onClick: () => void;
  showBadge?: boolean;
}

export function SettingsRow({ category, onClick, showBadge }: SettingsRowProps) {
  const minHeight = isMobile ? "min-h-[60px]" : "min-h-[52px]";
  const verticalPadding = isMobile ? "py-4" : "py-3.5";
  const showActualBadge = showBadge && category.badge;

  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full flex items-center gap-3 px-5 ${verticalPadding} ${minHeight} text-left hover:bg-klarvo-surface/40 transition-colors`}
    >
      {/* Coloured icon badge */}
      <span
        className="w-8 h-8 rounded-klarvo-md flex items-center justify-center shrink-0"
        style={{
          backgroundColor: `${category.iconColor}26`, // ~15% opacity (hex 26 = 38/255 ≈ 15%)
          color: category.iconColor,
        }}
      >
        {CATEGORY_ICONS[category.id]}
      </span>

      {/* Label + subtitle */}
      <span className="flex-1 flex flex-col min-w-0">
        <span className="text-sm font-medium text-klarvo-text leading-tight">
          {category.label}
        </span>
        {category.subtitle && (
          <span className="text-xs text-klarvo-muted mt-0.5 leading-tight">
            {category.subtitle}
          </span>
        )}
      </span>

      {/* Badge + chevron */}
      <span className="flex items-center gap-2 shrink-0">
        {showActualBadge && (
          <span className="px-1.5 py-0.5 rounded-klarvo-md text-[10px] font-semibold uppercase tracking-wide border bg-klarvo-teal/15 text-klarvo-teal border-klarvo-teal/25">
            {category.badge}
          </span>
        )}
        {CHEVRON}
      </span>
    </button>
  );
}
