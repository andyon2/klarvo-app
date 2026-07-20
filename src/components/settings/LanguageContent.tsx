import { isMobile } from "../../platform";
import { LABEL_CLS_M } from "../ui";
import { KSelect } from "./FormControls";

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
          <KSelect
            value={localLang}
            onChange={handleLangChange}
            options={LANGUAGES.map((l) => ({ value: l.code, label: l.label }))}
            className={isMobile ? "w-full" : "w-auto"}
          />
        </div>

        <div className={`flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`}>
          <span className={LABEL_CLS_M}>Translate to</span>
          <KSelect
            value={localOutputLanguage}
            onChange={handleOutputLanguageChange}
            options={OUTPUT_LANGUAGES.map((l) => ({ value: l.code, label: l.label }))}
            className={isMobile ? "w-full" : "w-auto"}
          />
        </div>
      </div>

    </div>
  );
}
