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

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

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
}

function StatTile({ label, value, sub, highlight }: StatTileProps) {
  return (
    <div
      className={[
        "bg-[#111113] border rounded-xl p-3",
        highlight ? "border-emerald-500/30" : "border-zinc-800/60",
      ].join(" ")}
    >
      <p className="text-[11px] text-zinc-500 uppercase tracking-wide leading-tight">{label}</p>
      <p
        className={[
          "text-base font-semibold mt-0.5",
          highlight ? "text-emerald-400" : "text-zinc-200",
        ].join(" ")}
      >
        {value}
        {sub && (
          <span className="text-[11px] text-zinc-500 font-normal ml-1">{sub}</span>
        )}
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Savings calculation
// ---------------------------------------------------------------------------

const WISPR_MONTHLY_USD = 12.0;

/**
 * Estimates monthly Dikta cost and computes savings vs Wispr Flow.
 *
 * We cannot know the first-dictation date from UsageSummary alone (the backend
 * does not expose it yet), so we conservatively assume 1 month of usage.
 * This gives a lower bound on savings rather than over-selling the number.
 */
function computeSavings(stats: UsageSummary): { monthly: number; savings: number } {
  // Denominator: assume at least 1 month so we never divide by zero.
  const months = 1;
  const monthly = stats.totalCostUsd / months;
  const savings = Math.max(0, WISPR_MONTHLY_USD - monthly);
  return { monthly, savings };
}

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

interface CostDashboardProps {
  stats: UsageSummary;
}

export function CostDashboard({ stats }: CostDashboardProps) {
  const hasData = stats.totalDictations > 0;
  const { monthly, savings } = computeSavings(stats);

  return (
    <div className="flex flex-col gap-4">

      {/* Section: Usage statistics */}
      <div>
        <p className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest mb-2">
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

      {/* Section: Cost breakdown */}
      <div>
        <p className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest mb-2">
          Kosten
        </p>
        <div className="grid grid-cols-2 gap-2">
          <StatTile label="STT-Kosten" value={hasData ? formatCost(stats.totalSttCostUsd) : "$0.00"} sub="USD" />
          <StatTile label="LLM-Kosten" value={hasData ? formatCost(stats.totalLlmCostUsd) : "$0.00"} sub="USD" />
          <StatTile
            label="Gesamt"
            value={hasData ? formatCost(stats.totalCostUsd) : "$0.00"}
            sub="USD"
            highlight
          />
          <StatTile
            label="Heute"
            value={hasData ? formatCost(stats.costTodayUsd) : "$0.00"}
            sub="USD"
          />
        </div>
      </div>

      {/* Savings banner */}
      {hasData ? (
        <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-4 flex flex-col gap-1">
          <p className="text-[11px] font-semibold text-emerald-400 uppercase tracking-wide">
            Vergleich mit Wispr Flow
          </p>
          {savings > 0 ? (
            <>
              <p className="text-sm font-semibold text-emerald-300">
                Du sparst {formatCost(savings)}/Monat
              </p>
              <p className="text-[11px] text-emerald-600 leading-snug">
                Wispr Flow kostet ${WISPR_MONTHLY_USD.toFixed(2)}/Monat. Deine
                Dikta-Kosten: {formatCost(monthly)}/Monat.
              </p>
            </>
          ) : (
            <p className="text-xs text-emerald-600">
              Noch zu wenig Daten für einen Vergleich — diktiere mehr!
            </p>
          )}
        </div>
      ) : (
        <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-4">
          <p className="text-[11px] font-semibold text-emerald-400 uppercase tracking-wide mb-1">
            Vergleich mit Wispr Flow
          </p>
          <p className="text-xs text-emerald-600">
            Noch keine Daten — starte dein erstes Diktat!
          </p>
        </div>
      )}

      {/* Footer note */}
      <p className="text-[10px] text-zinc-600 text-center leading-snug">
        Kostenbasiert auf Provider-Preisen (Groq STT: kostenlos, DeepSeek LLM: ~$0.00014/1k Token).
        Wispr Flow: ${WISPR_MONTHLY_USD.toFixed(2)}/Monat.
      </p>
    </div>
  );
}
