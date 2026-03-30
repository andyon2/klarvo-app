/**
 * CostDashboard — aggregated usage and cost statistics with Wispr Flow savings estimate.
 *
 * Receives a UsageSummary prop (already fetched by the parent Stats panel).
 * All formatting helpers are local copies to keep this component self-contained.
 */

import type { UsageSummary } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (mins < 60) return `${mins}m ${secs}s`;
  const hrs = Math.floor(mins / 60);
  const remainMins = mins % 60;
  return `${hrs}h ${remainMins}m`;
}

// ---------------------------------------------------------------------------
// Internal components
// ---------------------------------------------------------------------------

interface StatTileProps {
  label: string;
  value: string;
  sub?: string;
  highlight?: boolean;
  /** "primary" = teal (default), "warm" = orange */
  color?: "primary" | "warm";
}

function StatTile({ label, value, sub, highlight, color = "primary" }: StatTileProps) {
  const borderCls = highlight
    ? color === "warm" ? "border-klarvo-warm/30" : "border-klarvo-primary/30"
    : "border-klarvo-border/60";
  const valueCls = highlight
    ? color === "warm" ? "text-klarvo-warm" : "text-klarvo-primary"
    : "text-klarvo-text";

  return (
    <div className={`bg-klarvo-bg border rounded-xl p-3 ${borderCls}`}>
      <p className="text-[11px] text-klarvo-dim uppercase tracking-wide leading-tight">{label}</p>
      <p className={`text-base font-semibold mt-0.5 ${valueCls}`}>
        {value}
        {sub && (
          <span className="text-[11px] text-klarvo-dim font-normal ml-1">{sub}</span>
        )}
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Savings calculation
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

interface CostDashboardProps {
  stats: UsageSummary;
}

export function CostDashboard({ stats }: CostDashboardProps) {
  const hasData = stats.totalDictations > 0;

  return (
    <div className="flex flex-col gap-4">

      {/* Section: Usage statistics */}
      <div>
        <p className="text-[11px] font-semibold text-klarvo-primary uppercase tracking-widest mb-2">
          Nutzung
        </p>
        <div className="grid grid-cols-2 gap-2">
          <StatTile label="Diktate gesamt" value={`${stats.totalDictations}`} />
          <StatTile
            label="Gesprochene Zeit"
            value={hasData ? formatDuration(stats.totalAudioSeconds) : "—"}
          />
          <StatTile label="Wörter gesamt" value={hasData ? stats.totalWords.toLocaleString() : "—"} />
          <StatTile label="Heute" value={`${stats.dictationsToday}`} sub="Diktate" />
        </div>
      </div>

      {/* Wispr Flow comparison banner */}
      <div className="bg-klarvo-primary/10 border border-klarvo-primary/20 rounded-xl p-4 flex flex-col gap-1">
        <p className="text-[11px] font-semibold text-klarvo-primary uppercase tracking-wide">
          Vergleich mit Wispr Flow
        </p>
        <p className="text-sm font-semibold text-klarvo-accent">
          Wispr Flow kostet $12/Monat ($144/Jahr).
        </p>
        <p className="text-[11px] text-klarvo-primary leading-snug">
          Mit Klarvo: Groq STT ist kostenlos, DeepSeek Cleanup kostet Cents pro Tag.
        </p>
      </div>
    </div>
  );
}
