import { isMobile } from "../../platform";
import { LABEL_CLS_M } from "../ui";

// --- Language options --------------------------------------------------------

const LANGUAGES = [
  { code: "", label: "Auto (DE + EN)" },
  { code: "de", label: "Deutsch" },
  { code: "en", label: "English" },
];

export const OUTPUT_LANGUAGES = [
  { code: "", label: "No translation" },
  { code: "en", label: "English" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
  { code: "es", label: "Español" },
  { code: "it", label: "Italiano" },
  { code: "pt", label: "Português" },
  { code: "nl", label: "Nederlands" },
  { code: "pl", label: "Polski" },
  { code: "ru", label: "Русский" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "中文" },
  { code: "ko", label: "한국어" },
];

// --- Props ------------------------------------------------------------------

export interface LanguageContentProps {
  localLang: string;
  handleLangChange: (lang: string) => void;
  localOutputLanguage: string;
  handleOutputLanguageChange: (lang: string) => void;
}

// --- Component --------------------------------------------------------------

/**
 * Language settings: dictation language + output translation.
 *
 * Split out of the former "Appearance & Language" section — that section
 * carried zero appearance controls (those now live in AppearanceContent), so
 * this is now an honest, language-only section.
 */
export function LanguageContent({
  localLang,
  handleLangChange,
  localOutputLanguage,
  handleOutputLanguageChange,
}: LanguageContentProps) {
  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">

      <div className="flex flex-col gap-2.5">
        <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
          <span className={LABEL_CLS_M}>Dictation language</span>
          <select
            value={localLang}
            onChange={(e) => handleLangChange(e.target.value)}
            className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
          >
            {LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>{l.label}</option>
            ))}
          </select>
        </div>

        <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
          <span className={LABEL_CLS_M}>Translate to</span>
          <select
            value={localOutputLanguage}
            onChange={(e) => handleOutputLanguageChange(e.target.value)}
            className={`bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-2.5 py-1.5 text-xs text-klarvo-text focus:outline-none focus:border-klarvo-primary/40 transition-colors cursor-pointer ${isMobile ? "w-full" : ""}`}
          >
            {OUTPUT_LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>{l.label}</option>
            ))}
          </select>
        </div>
      </div>

    </div>
  );
}
