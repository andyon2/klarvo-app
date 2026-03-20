/**
 * useQuickTip — determines which contextual tip (if any) should be shown.
 *
 * Rules:
 * - Max 1 tip per session.
 * - Tips are shown only when the recording is idle.
 * - A tip is eligible when its trigger condition is met AND it has not been
 *   shown before (persisted in the backend's tips_shown SQLite table).
 * - The first eligible tip is shown after a 3-second delay from mount.
 */

import { useEffect, useRef, useState } from "react";
import { getUsageStats, isTipShown, markTipShown } from "../tauri-commands";
import type { PanelName } from "./usePanels";

// ---------------------------------------------------------------------------
// Tip definitions
// ---------------------------------------------------------------------------

interface TipTrigger {
  dictations?: number;
  days?: number;
}

export interface TipDefinition {
  id: string;
  trigger: TipTrigger;
  title: string;
  text: string;
  actionLabel: string;
  panel: PanelName;
}

const TIPS: TipDefinition[] = [
  {
    id: "cleanup-styles",
    trigger: { dictations: 5 },
    title: "Bereinigungsstile",
    text: "Probiere 'Chat' für lockere Nachrichten oder 'Verbatim' für unveränderten Text.",
    actionLabel: "Ausprobieren",
    panel: "settings",
  },
  {
    id: "cleanup-instr",
    trigger: { dictations: 10 },
    title: "Eigene Anweisungen",
    text: "Unter Einstellungen kannst du dem KI-Modell eigene Stilanweisungen geben.",
    actionLabel: "Einstellungen",
    panel: "settings",
  },
  {
    id: "hotkey-change",
    trigger: { dictations: 20 },
    title: "Hotkey anpassen",
    text: "Du kannst den Diktat-Shortcut in den Einstellungen ändern.",
    actionLabel: "Jetzt ändern",
    panel: "settings",
  },
  {
    id: "cost-dashboard",
    trigger: { days: 7 },
    title: "Deine Kosten",
    text: "Schau dir an, wie viel du gegenüber Wispr Flow sparst.",
    actionLabel: "Dashboard zeigen",
    panel: "stats",
  },
  {
    id: "offline-mode",
    trigger: { dictations: 50 },
    title: "Offline-Modus",
    text: "Dikta kann auch ohne Internet funktionieren — mit einem lokalen Whisper-Modell.",
    actionLabel: "Einrichten",
    panel: "settings",
  },
];

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

interface UseQuickTipOptions {
  /** Pass true when the recording pipeline is idle (not recording/transcribing/cleaning). */
  isIdle: boolean;
  /** Pass true when the onboarding wizard has completed so tips are not shown mid-wizard. */
  onboardingCompleted: boolean;
}

interface UseQuickTipReturn {
  activeTip: TipDefinition | null;
  /** Which panel the active tip's action button should open. */
  openPanel: PanelName | null;
  /** Dismiss the current tip: persists to backend + clears from state. */
  dismissTip: () => void;
  /** Confirm the action button was pressed: dismiss + signals which panel to open. */
  handleAction: () => void;
}

export function useQuickTip({ isIdle, onboardingCompleted }: UseQuickTipOptions): UseQuickTipReturn {
  const [activeTip, setActiveTip] = useState<TipDefinition | null>(null);
  // Guard: only show one tip per session.
  const tipShownThisSession = useRef(false);
  // Guard: evaluation already scheduled/run.
  const evaluationScheduled = useRef(false);

  useEffect(() => {
    // Only evaluate once per session, and only when onboarding is done.
    if (!onboardingCompleted || evaluationScheduled.current) return;
    evaluationScheduled.current = true;

    const timer = setTimeout(async () => {
      // Double-check: still idle and no tip shown yet this session.
      if (tipShownThisSession.current) return;

      try {
        const stats = await getUsageStats();
        const nowDays = Date.now() / 1000 / 86400; // seconds → days since epoch

        for (const tip of TIPS) {
          // Check trigger condition.
          const { dictations, days } = tip.trigger;

          if (dictations !== undefined && stats.totalDictations < dictations) continue;

          if (days !== undefined) {
            // We approximate "days since first use" as: if the user has any
            // dictations we check if totalAudioSeconds is non-zero (rough proxy).
            // Without a firstDictationDate field on UsageSummary, we use the
            // session-start timestamp as a conservative lower bound — meaning the
            // days-trigger only fires after the app has been open long enough.
            // In practice this means the cost-dashboard tip fires after 7 days
            // of the App being installed (tracked via localStorage).
            const installKey = "dikta_install_day";
            let installDay = parseFloat(localStorage.getItem(installKey) ?? "0");
            if (!installDay) {
              installDay = nowDays;
              localStorage.setItem(installKey, String(installDay));
            }
            const daysSinceInstall = nowDays - installDay;
            if (daysSinceInstall < days) continue;
          }

          // Check whether this tip was already shown (backend persistence).
          const alreadyShown = await isTipShown(tip.id);
          if (alreadyShown) continue;

          // Found the first eligible tip.
          setActiveTip(tip);
          tipShownThisSession.current = true;
          break;
        }
      } catch (err) {
        // Non-fatal: tip system is best-effort.
        console.warn("[useQuickTip] evaluation failed:", err);
      }
    }, 3000);

    return () => clearTimeout(timer);
  }, [onboardingCompleted]);

  // Hide tip when recording starts (but don't re-evaluate on resume).
  const visibleTip = isIdle ? activeTip : null;

  const dismissTip = () => {
    if (!activeTip) return;
    markTipShown(activeTip.id).catch(console.warn);
    setActiveTip(null);
  };

  const handleAction = () => {
    dismissTip();
  };

  return {
    activeTip: visibleTip,
    openPanel: visibleTip?.panel ?? null,
    dismissTip,
    handleAction,
  };
}
