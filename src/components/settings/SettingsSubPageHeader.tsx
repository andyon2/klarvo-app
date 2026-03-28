import { useEffect } from "react";

interface SettingsSubPageHeaderProps {
  title: string;
  onBack: () => void;
  onClose: () => void;
}

export function SettingsSubPageHeader({ title, onBack, onClose }: SettingsSubPageHeaderProps) {
  // Escape key: go back to home (not close)
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onBack();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onBack]);

  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b border-klarvo-border/50 h-12 shrink-0">
      {/* Back button */}
      <button
        type="button"
        onClick={onBack}
        aria-label="Back to settings"
        className="w-8 h-8 rounded-lg flex items-center justify-center hover:bg-klarvo-surface/50 transition-colors"
      >
        <svg
          className="w-5 h-5 text-klarvo-primary"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>

      {/* Centred title */}
      <span className="flex-1 text-base font-semibold text-klarvo-text text-center">
        {title}
      </span>

      {/* Close button */}
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
  );
}
