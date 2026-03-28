import { isMobile } from "../../platform";
import { DictionaryTag, INPUT_CLS_M } from "../ui";

// --- Props -------------------------------------------------------------------

export interface DictionaryContentProps {
  dictionary: string[];
  newTerm: string;
  setNewTerm: (v: string) => void;
  onAddTerm: () => void;
  onRemoveTerm: (term: string) => Promise<void>;
  isPaid: boolean;
  isTrial: boolean;
}

// --- Component ---------------------------------------------------------------

export function DictionaryContent({
  dictionary,
  newTerm,
  setNewTerm,
  onAddTerm,
  onRemoveTerm,
  isPaid,
  isTrial,
}: DictionaryContentProps) {
  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
      <div className="flex gap-2">
        <input
          type="text"
          placeholder="Add word or phrase..."
          value={newTerm}
          disabled={!isPaid && dictionary.length >= 20}
          onChange={(e) => setNewTerm(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onAddTerm()}
          className={`flex-1 ${INPUT_CLS_M}${(!isPaid && dictionary.length >= 20) ? " cursor-not-allowed opacity-50" : ""}`}
        />
        <button
          onClick={onAddTerm}
          disabled={!newTerm.trim() || (!isPaid && dictionary.length >= 20)}
          title={(!isPaid && dictionary.length >= 20) ? "Free limit reached (20 terms). Upgrade for unlimited." : undefined}
          className={`px-3 rounded-lg font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 disabled:opacity-30 disabled:cursor-not-allowed transition-colors ${isMobile ? "py-2.5 text-sm min-w-[56px]" : "py-2 text-xs"}`}
        >
          Add
        </button>
      </div>

      {!isPaid && dictionary.length >= 20 && (
        <p className="text-[11px] text-klarvo-warning/80">
          Free limit reached (20 terms). Upgrade for unlimited.
        </p>
      )}
      {isPaid && isTrial && (
        <p className={isMobile ? "text-xs text-klarvo-primary/70" : "text-[11px] text-klarvo-primary/70"}>
          Unlimited terms (Trial) &mdash; stays unlimited with a license.
        </p>
      )}

      {dictionary.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {dictionary.map((t) => <DictionaryTag key={t} term={t} onRemove={onRemoveTerm} />)}
        </div>
      ) : (
        <p className="text-xs text-klarvo-dim italic">No terms yet.</p>
      )}
    </div>
  );
}
