import { useEffect } from "react";
import { isMobile, isDesktop } from "../../platform";
import { SETTINGS_CATEGORIES } from "./types";
import type { SettingsCategory } from "./types";
import { SettingsRow } from "./SettingsRow";

interface SettingsHomeProps {
  onSelectCategory: (id: SettingsCategory) => void;
  onClose: () => void;
  isTrial: boolean;
}

export function SettingsHome({ onSelectCategory, onClose, isTrial }: SettingsHomeProps) {
  // Escape key on home view: close settings entirely
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const visibleCategories = SETTINGS_CATEGORIES.filter((cat) => {
    if (cat.desktopOnly && !isDesktop) return false;
    if (cat.mobileOnly && !isMobile) return false;
    return true;
  });

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header */}
      <div className="flex items-center px-4 py-3 border-b border-klarvo-border shrink-0">
        <span className="flex-1 text-lg font-bold text-klarvo-text">Settings</span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close settings"
          className="w-8 h-8 rounded-lg flex items-center justify-center transition-colors group"
        >
          <svg
            className="w-4 h-4 text-klarvo-dim group-hover:text-klarvo-muted transition-colors"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
          >
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Category list */}
      <div className="flex-1 overflow-y-auto divide-y divide-klarvo-border/40">
        {visibleCategories.map((cat) => (
          <SettingsRow
            key={cat.id}
            category={cat}
            onClick={() => onSelectCategory(cat.id)}
            showBadge={isTrial}
          />
        ))}
      </div>
    </div>
  );
}
