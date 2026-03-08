// Small reusable UI components shared across panels.

export function StatusDot({ active }: { active: boolean }) {
  return (
    <span className={`inline-block w-2 h-2 rounded-full flex-shrink-0 ${active ? "bg-emerald-500" : "bg-zinc-600"}`} />
  );
}

export function DictionaryTag({ term, onRemove }: { term: string; onRemove: (t: string) => void }) {
  // Import isMobile here via dynamic check to keep ui.tsx dependency-free of platform.ts at module level.
  const mobile = /Android|iPhone|iPad/i.test(navigator.userAgent);
  return (
    <span className="inline-flex items-center gap-1 bg-[#111113] text-zinc-300 pl-2.5 pr-1.5 py-1 rounded-full text-xs border border-zinc-800/60">
      {term}
      <button
        onClick={() => onRemove(term)}
        className={[
          "text-zinc-500 hover:text-red-400 rounded-full transition-colors",
          mobile ? "p-2 min-w-[32px] min-h-[32px] flex items-center justify-center" : "p-0.5",
        ].join(" ")}
      >
        <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </span>
  );
}

interface FillerEntry {
  word: string;
  count: number;
}

export function FillerStatsChart({ entries }: { entries: FillerEntry[] }) {
  if (entries.length === 0) {
    return <p className="text-xs text-zinc-500 italic">No filler words tracked yet.</p>;
  }

  const max = entries[0].count;

  return (
    <div className="flex flex-col gap-1.5">
      {entries.slice(0, 10).map(({ word, count }) => (
        <div key={word} className="flex items-center gap-2">
          <span className="text-[11px] text-zinc-400 w-16 shrink-0 font-mono truncate">{word}</span>
          <div className="flex-1 bg-zinc-800/60 rounded-full h-1.5 overflow-hidden">
            <div
              className="h-full bg-emerald-500/50 rounded-full transition-all duration-300"
              style={{ width: `${Math.round((count / max) * 100)}%` }}
            />
          </div>
          <span className="text-[11px] text-zinc-500 w-6 text-right shrink-0">{count}</span>
        </div>
      ))}
    </div>
  );
}

/** Renders text with search query highlighted and context around first match. */
export function HighlightedText({ text, query, className }: { text: string; query: string; className?: string }) {
  if (!query.trim()) {
    return <p className={`${className} line-clamp-3`}>{text}</p>;
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const firstIdx = lowerText.indexOf(lowerQuery);

  let displayText = text;
  let prefix = "";
  let suffix = "";
  if (firstIdx > 60) {
    const start = text.lastIndexOf(" ", firstIdx - 20);
    displayText = text.slice(start > 0 ? start : firstIdx - 40);
    prefix = "…";
  }
  if (displayText.length > 200) {
    const end = displayText.indexOf(" ", 180);
    displayText = displayText.slice(0, end > 0 ? end : 200);
    suffix = "…";
  }

  const parts: { text: string; highlight: boolean }[] = [];
  const lowerDisplay = displayText.toLowerCase();
  let cursor = 0;
  let matchIdx = lowerDisplay.indexOf(lowerQuery, cursor);
  while (matchIdx !== -1) {
    if (matchIdx > cursor) {
      parts.push({ text: displayText.slice(cursor, matchIdx), highlight: false });
    }
    parts.push({ text: displayText.slice(matchIdx, matchIdx + query.length), highlight: true });
    cursor = matchIdx + query.length;
    matchIdx = lowerDisplay.indexOf(lowerQuery, cursor);
  }
  if (cursor < displayText.length) {
    parts.push({ text: displayText.slice(cursor), highlight: false });
  }

  return (
    <p className={className}>
      {prefix}{parts.map((p, i) =>
        p.highlight
          ? <mark key={i} className="bg-emerald-500/30 text-emerald-300 rounded-sm px-0.5">{p.text}</mark>
          : <span key={i}>{p.text}</span>
      )}{suffix}
    </p>
  );
}

export function StatCard({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3">
      <p className="text-[11px] text-zinc-500 uppercase tracking-wide">{label}</p>
      <p className="text-lg font-semibold text-zinc-200 mt-0.5">
        {value}
        {sub && <span className="text-[11px] text-zinc-500 font-normal ml-1">{sub}</span>}
      </p>
    </div>
  );
}

// Shared CSS class strings for form inputs, used across Settings panels.
export const INPUT_CLS = "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors";
export const LABEL_CLS = "text-xs text-zinc-300";
export const SECTION_TITLE_CLS = "text-[11px] font-semibold text-zinc-400 uppercase tracking-widest";

// Mobile-aware variants -- one size larger on touch screens.
const _mobile = /Android|iPhone|iPad/i.test(navigator.userAgent);
export const INPUT_CLS_M = _mobile
  ? "w-full bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
  : INPUT_CLS;
export const LABEL_CLS_M = _mobile ? "text-sm text-zinc-300" : LABEL_CLS;
export const SECTION_TITLE_CLS_M = _mobile ? "text-xs font-semibold text-zinc-400 uppercase tracking-widest" : SECTION_TITLE_CLS;
