import { useState, useCallback, useEffect, useRef } from "react";
import "./styles.css";
import type { CleanupStyle, HistoryEntry, UsageSummary } from "./types";
import { STATUS_LABELS, STYLE_OPTIONS } from "./types";
import {
  getHistory,
  deleteHistoryEntry,
  searchHistory,
  getUsageStats,
  getFillerStats,
  getNotes,
  isFirstRun,
  isPreviewMode,
  getOnboardingState,
  setOnboardingState,
  clearApiKey,
  reprocessPendingEntry,
  discardPendingEntry,
} from "./tauri-commands";
import { CostDashboard } from "./components/CostDashboard";
import { QuickTip } from "./components/QuickTip";
import { useQuickTip } from "./hooks/useQuickTip";
import { isMobile, isDesktop } from "./platform";
import Onboarding from "./Onboarding";

// Components
import {
  MicIcon, StopIcon, SpinnerIcon, GearIcon, CloseIcon,
  LockIcon, FeedbackIcon,
} from "./components/icons";
import { FillerStatsChart, HighlightedText } from "./components/ui";
import { SettingsPanel } from "./components/SettingsPanel";
import { VoiceNotesPanel } from "./components/VoiceNotesPanel";
import { ThemeSwitcher } from "./components/ThemeSwitcher";
import { FeedbackModal } from "./components/FeedbackModal";
import { PreviewComments } from "./components/PreviewComments";

// Hooks
import { useRecording } from "./hooks/useRecording";
import { useSettings } from "./hooks/useSettings";
import { usePanels } from "./hooks/usePanels";
import { useLicense } from "./hooks/useLicense";
import { useUiScale } from "./hooks/useUiScale";
import { useCopyFeedback, copyLabel, copyColorClassName, UNDO_WINDOW_MS } from "./hooks/useCopyFeedback";

// --- Helpers -----------------------------------------------------------------

function formatHotkeyDisplay(hotkey: string): string {
  return hotkey.split("+").map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(" + ");
}

// --- Sub-components ----------------------------------------------------------

function RecordButton({ recordingState, onClick }: { recordingState: string; onClick: () => void }) {
  const isRecording = recordingState === "recording";
  const isBusy = recordingState === "transcribing" || recordingState === "cleaning";

  return (
    <button
      aria-label={isRecording ? "Stop recording" : isBusy ? "Processing" : "Start recording"}
      disabled={isBusy}
      onClick={onClick}
      className={[
        "relative flex items-center justify-center",
        isMobile ? "w-32 h-32" : "w-24 h-24",
        "rounded-full",
        "transition-all duration-200",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30",
        "disabled:cursor-not-allowed disabled:opacity-60",
        isRecording
          ? "bg-klarvo-amber/20 text-klarvo-amber shadow-[0_0_40px_rgba(233,162,76,0.3)]"
          : isBusy
          ? "bg-klarvo-amber/15 text-klarvo-amber shadow-[0_0_30px_rgba(233,162,76,0.2)]"
          : "bg-klarvo-teal/15 text-klarvo-teal shadow-[0_0_40px_rgba(41,199,172,0.2)] hover:shadow-[0_0_50px_rgba(41,199,172,0.3)] hover:bg-klarvo-teal/20",
      ].join(" ")}
    >
      <span
        className={[
          "absolute inset-0 rounded-full border-2 transition-colors duration-200",
          isRecording ? "border-klarvo-amber/40" : isBusy ? "border-klarvo-amber/30" : "border-klarvo-teal/25",
        ].join(" ")}
      />
      {isRecording && (
        <span className="absolute inset-0 rounded-full border-2 border-klarvo-amber opacity-40 animate-ping" />
      )}
      {isBusy ? (
        <SpinnerIcon className="w-9 h-9" />
      ) : isRecording ? (
        <StopIcon className="w-9 h-9" />
      ) : (
        <MicIcon className="w-9 h-9" />
      )}
    </button>
  );
}

function StylePicker({ value, onChange, disabled }: { value: CleanupStyle; onChange: (s: CleanupStyle) => void; disabled: boolean }) {
  return (
    <div className={`flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60 ${isMobile ? "w-full" : "w-fit"}`}>
      {STYLE_OPTIONS.map((opt) => (
        <button
          key={opt.value}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          title={opt.description}
          className={[
            isMobile
              ? "flex-1 px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-100 whitespace-nowrap"
              : "px-2.5 py-1.5 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
            "disabled:cursor-not-allowed disabled:opacity-50",
            value === opt.value
              ? "bg-klarvo-teal/15 text-klarvo-teal"
              : "text-klarvo-dim hover:text-klarvo-muted",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

// OutputLanguagePicker removed from header -- available in Settings only.

// --- Main App ----------------------------------------------------------------

export default function App() {
  // --- Hooks ---
  useUiScale(); // Apply persisted font-size scale to <html> on startup
  const settings = useSettings();
  const recording = useRecording(settings.cleanupStyle, settings.language);
  const license = useLicense();
  const copyFeedback = useCopyFeedback();

  // Feature gate: active paid license (licensed, active trial, or valid grace period).
  const isPaid =
    license.licenseStatus.type === "licensed" ||
    (license.licenseStatus.type === "trial" &&
      license.licenseStatus.trialUntil !== undefined &&
      license.licenseStatus.trialUntil > Date.now() / 1000) ||
    (license.licenseStatus.type === "grace_period" &&
      license.licenseStatus.graceUntil !== undefined &&
      license.licenseStatus.graceUntil > Date.now() / 1000);

  // History state (loaded lazily when history panel opens)
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [historyLoadState, setHistoryLoadState] = useState<"loading" | "error" | "loaded">("loading");
  const [historySearch, setHistorySearch] = useState("");
  const [historyAppSearch, setHistoryAppSearch] = useState("");
  const [expandedHistoryRaw, setExpandedHistoryRaw] = useState<Set<number>>(new Set());
  // Story 12-2: pending (terminal-failure) entries -- id of the entry
  // currently re-processing (busy/disabled state) and any inline error from
  // the last failed re-process attempt, keyed by entry id.
  const [reprocessingId, setReprocessingId] = useState<number | null>(null);
  const [pendingErrors, setPendingErrors] = useState<Record<number, string>>({});
  // Story 8-8: optimistic delete with an in-place undo strip. The canonical store is the ref
  // (so the commit-once guard is synchronous); pendingDeleteIds only mirrors its keys to drive
  // re-renders. The entry itself is never removed from historyEntries until the delete commits.
  const pendingDeletesRef = useRef<Map<number, { entry: HistoryEntry; timer: number }>>(new Map());
  const [pendingDeleteIds, setPendingDeleteIds] = useState<Set<number>>(new Set());

  // Stats state
  const [usageStats, setUsageStats] = useState<UsageSummary | null>(null);
  const [fillerStats, setFillerStats] = useState<{ word: string; count: number }[]>([]);
  const [showFillerStats, setShowFillerStats] = useState(false);

  // Notes state
  const [notes, setNotes] = useState<HistoryEntry[]>([]);

  // Onboarding
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingInitialState, setOnboardingInitialState] = useState<import("./types").OnboardingState | undefined>(undefined);
  const [onboardingCompleted, setOnboardingCompleted] = useState(false);

  // Trial-expired notification: shown once after a trial ends and user is now unlicensed.
  const [showTrialExpired, setShowTrialExpired] = useState(false);

  // Feedback tooltip
  const [showFeedbackTooltip, setShowFeedbackTooltip] = useState(
    () => localStorage.getItem("klarvo_feedback_tooltip_dismissed") !== "true"
  );

  const dismissFeedbackTooltip = useCallback((permanently: boolean) => {
    if (permanently) {
      localStorage.setItem("klarvo_feedback_tooltip_dismissed", "true");
    }
    setShowFeedbackTooltip(false);
  }, []);

  useEffect(() => {
    if (license.licenseStatus.type === "trial") {
      localStorage.setItem("klarvo_was_in_trial", "true");
    }
    const wasInTrial = localStorage.getItem("klarvo_was_in_trial") === "true";
    const trialExpiredSeen = localStorage.getItem("klarvo_trial_expired_seen") === "true";
    if (wasInTrial && license.licenseStatus.type === "unlicensed" && !trialExpiredSeen) {
      setShowTrialExpired(true);
    } else if (license.licenseStatus.type !== "unlicensed") {
      // Dismiss banner if status updated (e.g., initial "unlicensed" default
      // was replaced by actual status from backend).
      setShowTrialExpired(false);
    }
  }, [license.licenseStatus.type]);

  const dismissTrialExpired = useCallback(() => {
    localStorage.setItem("klarvo_trial_expired_seen", "true");
    setShowTrialExpired(false);
  }, []);

  // Quick-tip system
  const isIdle = recording.recordingState === "idle" || recording.recordingState === "done" || recording.recordingState === "error";
  const quickTip = useQuickTip({ isIdle, onboardingCompleted });

  const loadHistory = useCallback(() => {
    // Story 8-8 (AC7): commit pending deletes first so a wholesale refetch never resurrects
    // a row that is mid-undo-window. The refetch SELECT is sequenced after the flush settles.
    setHistoryLoadState("loading");
    flushPendingDeletes()
      .then(() => getHistory(50))
      .then((entries) => {
        setHistoryEntries(entries);
        setHistoryLoadState("loaded");
      })
      .catch((err) => {
        console.error(err);
        setHistoryLoadState("error");
      });
  }, []);

  // Panel callbacks for lazy loading
  const panels = usePanels({
    onOpenHistory: loadHistory,
    onOpenStats: () => {
      getUsageStats().then(setUsageStats).catch(console.error);
      getFillerStats().then(setFillerStats).catch(console.error);
    },
    onOpenNotes: () => getNotes(50).then(setNotes).catch(console.error),
  });

  // Check for first run / onboarding
  // Show wizard when not completed and not skipped.
  // Auto-skip for existing users: if keys already configured but onboarding never ran,
  // mark it as completed so the wizard doesn't nag returning users.
  useEffect(() => {
    getOnboardingState()
      .then(async (state) => {
        if (!state.completed && !state.skipped) {
          // Check if user already has keys (existing user upgrading)
          try {
            const { getSettings: fetchSettings } = await import("./tauri-commands");
            const s = await fetchSettings();
            const hasKeys = !!(s.groqApiKeyMasked || s.deepseekApiKeyMasked || s.openaiApiKeyMasked || s.openrouterApiKeyMasked);
            if (hasKeys) {
              // Auto-complete onboarding for existing users
              await setOnboardingState({ ...state, completed: true, skipped: true });
              setOnboardingCompleted(true);
              return;
            }
          } catch { /* fall through to show wizard */ }
          setOnboardingInitialState(state);
          setShowOnboarding(true);
        } else {
          setOnboardingCompleted(true);
        }
      })
      .catch(() => {
        // Backend command not yet available (Task 1 not done) — fall back to isFirstRun
        isFirstRun()
          .then((firstRun) => {
            if (firstRun) setShowOnboarding(true);
            else setOnboardingCompleted(true);
          })
          .catch(console.error);
      });
  }, []);

  // Android back button: close open panel instead of leaving the app
  // Ref for settings sub-page back handler
  const settingsBackRef = useRef<(() => boolean) | null>(null);

  useEffect(() => {
    if (!isMobile) return;
    if (panels.anyOpen) {
      window.history.pushState({ panel: true }, "");
    }
  }, [panels.anyOpen]);

  useEffect(() => {
    if (!isMobile) return;
    const handler = () => {
      // Let settings sub-page handle back first (sub-page → home)
      if (settingsBackRef.current?.()) {
        // Re-push panel state so next back closes the panel
        window.history.pushState({ panel: true }, "");
        return;
      }
      panels.closeAll();
    };
    window.addEventListener("popstate", handler);
    return () => window.removeEventListener("popstate", handler);
  }, [panels.closeAll]);

  // --- Derived state ---
  const isBusy = recording.recordingState === "transcribing" || recording.recordingState === "cleaning";
  const isRecording = recording.recordingState === "recording";
  const headerBtnPad = isMobile ? "p-2.5" : "p-1.5";
  const hotkeyDisplay = formatHotkeyDisplay(settings.loadedSettings?.hotkey ?? "ctrl+shift+d");

  // Derive the feedback area from the most recently open panel.
  const feedbackArea = panels.showSettings ? "settings"
    : panels.showHistory ? "history"
    : panels.showStats ? "statistics"
    : panels.showNotes ? "notes"
    : "home";

  // --- History handlers ---
  const handleHistorySearch = useCallback(async (textQ: string, appQ: string) => {
    setHistorySearch(textQ);
    setHistoryAppSearch(appQ);
    // Story 8-8 (AC7): same flush-before-refetch guard as loadHistory, awaited so the DELETE
    // is sequenced before the refetch SELECT.
    await flushPendingDeletes();
    if (textQ.trim() || appQ.trim()) {
      const results = await searchHistory(textQ.trim() || undefined, appQ.trim() || undefined);
      setHistoryEntries(results);
    } else {
      const entries = await getHistory(50);
      setHistoryEntries(entries);
    }
    setHistoryLoadState("loaded");
  }, []);

  // Story 8-8: commits one pending delete to the backend. Guarded by the ref so it runs
  // exactly once per id even if the undo timer and a flush (panel close / refetch) race.
  const commitPendingDelete = useCallback((id: number): Promise<void> => {
    const pending = pendingDeletesRef.current.get(id);
    if (!pending) return Promise.resolve();
    pendingDeletesRef.current.delete(id);
    window.clearTimeout(pending.timer);
    setPendingDeleteIds((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
    setHistoryEntries((prev) => prev.filter((e) => e.id !== id));
    return deleteHistoryEntry(id).catch(console.error);
  }, []);

  // Story 8-8 (AC4/AC5): does not await the backend. The entry stays in historyEntries (its
  // list position is preserved) and the row renders as an undo strip until the timer commits it.
  const handleDeleteHistoryEntry = useCallback((entry: HistoryEntry) => {
    const id = entry.id;
    if (pendingDeletesRef.current.has(id)) return;
    const timer = window.setTimeout(() => commitPendingDelete(id), UNDO_WINDOW_MS);
    pendingDeletesRef.current.set(id, { entry, timer });
    setPendingDeleteIds((prev) => new Set(prev).add(id));
  }, [commitPendingDelete]);

  // Story 8-8 (AC6): clears the pending timer and drops the id. historyEntries is untouched
  // (the entry never left) and the backend is never called for this id.
  const handleUndoDelete = useCallback((id: number) => {
    const pending = pendingDeletesRef.current.get(id);
    if (!pending) return;
    window.clearTimeout(pending.timer);
    pendingDeletesRef.current.delete(id);
    setPendingDeleteIds((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }, []);

  // Story 8-8 (AC8, Task 5 refetch safety): commits every pending delete immediately. Used both
  // when the history panel closes/unmounts and right before any wholesale historyEntries refetch,
  // so a pending-deleted row is never silently resurrected by fresh backend data (AC7).
  const flushPendingDeletes = useCallback((): Promise<void> => {
    const ids = Array.from(pendingDeletesRef.current.keys());
    return Promise.all(ids.map((id) => commitPendingDelete(id))).then(() => undefined);
  }, [commitPendingDelete]);

  // Story 8-8 (AC8): flush when the history panel closes, however it closes (the X button,
  // toggling another panel, Android back via closeAll), and on true unmount.
  useEffect(() => {
    if (!panels.showHistory) flushPendingDeletes();
  }, [panels.showHistory, flushPendingDeletes]);

  useEffect(() => {
    return () => {
      flushPendingDeletes();
    };
  }, [flushPendingDeletes]);

  // Story 12-2 (AC5) — "Erneut verarbeiten": re-runs STT+cleanup on the
  // stored WAV. On success the entry is replaced in place with the promoted
  // (done) entry; on failure it stays pending and shows an inline error.
  const handleReprocessPendingEntry = useCallback(async (id: number) => {
    setReprocessingId(id);
    setPendingErrors((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
    try {
      const updated = await reprocessPendingEntry(id);
      setHistoryEntries((prev) => prev.map((e) => (e.id === id ? updated : e)));
    } catch (err) {
      setPendingErrors((prev) => ({ ...prev, [id]: err instanceof Error ? err.message : String(err) }));
    } finally {
      setReprocessingId(null);
    }
  }, []);

  // Story 12-2 (AC6) — "Verwerfen": deletes the pending entry and its WAV.
  const handleDiscardPendingEntry = useCallback(async (id: number) => {
    try {
      await discardPendingEntry(id);
      setHistoryEntries((prev) => prev.filter((e) => e.id !== id));
      setPendingErrors((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    } catch (err) {
      setPendingErrors((prev) => ({ ...prev, [id]: err instanceof Error ? err.message : String(err) }));
    }
  }, []);

  // --- Onboarding handler ---
  const handleOnboardingComplete = useCallback(async (updated: import("./types").AppSettings) => {
    if (updated && updated.language) {
      settings.setLoadedSettings(updated);
      settings.setLanguage(updated.language);
      settings.setCleanupStyle(updated.cleanupStyle);
      settings.setHotkey(updated.hotkey);
      settings.setHotkeyMode(updated.hotkeyMode);
      settings.setAudioDevice(updated.audioDevice);
      import("./tauri-commands").then(({ setLanguage, setCleanupStyle }) => {
        setLanguage(updated.language).catch(console.error);
        setCleanupStyle(updated.cleanupStyle).catch(console.error);
      });
    }
    setShowOnboarding(false);
    setOnboardingCompleted(true);
  }, [settings]);

  // Called by SettingsPanel "Setup-Assistent erneut starten"
  // Shows a confirmation dialog warning that API keys will be cleared,
  // then resets onboarding state and removes all provider keys so the
  // onboarding flow is realistic (user must re-enter keys).
  const handleRestartOnboarding = useCallback(async () => {
    const confirmed = window.confirm(
      "Onboarding neu starten?\n\n" +
      "Alle API-Keys (Groq, DeepSeek, etc.) werden entfernt, " +
      "damit das Onboarding realistisch durchlaufen werden kann.\n\n" +
      "Du kannst sie danach im Onboarding oder in den Einstellungen neu eingeben."
    );
    if (!confirmed) return;

    // Clear all provider API keys
    for (const provider of ["groq", "deepseek", "openai", "openrouter"]) {
      await clearApiKey(provider).catch(console.error);
    }

    const freshState: import("./types").OnboardingState = {
      completed: false,
      skipped: false,
      currentStep: 0,
      mode: "",
      language: "",
      track: "",
    };
    await setOnboardingState(freshState).catch(console.error);
    setOnboardingInitialState(freshState);
    panels.closeAll();
    setShowOnboarding(true);
  }, [panels]);

  // Output language change is handled in SettingsPanel only.

  if (showOnboarding) {
    return (
      <>
        <Onboarding onComplete={handleOnboardingComplete} initialState={onboardingInitialState} />
        {isPreviewMode && <PreviewComments />}
      </>
    );
  }

  return (
    <main
      className="h-screen bg-klarvo-bg text-klarvo-text flex flex-col select-none overflow-y-auto font-geist"
      style={{
        ...(isMobile ? {
          paddingTop: "env(safe-area-inset-top, 24px)",
          paddingBottom: "env(safe-area-inset-bottom, 24px)",
        } : {}),
      }}
    >
      {/* ── Header ──
           Single row: logo + icon strip. StylePicker lives below as its own row
           (only visible on the home/recording view, hidden when a panel is open). */}
      <div className="flex items-center gap-2.5 px-4 pt-3.5 pb-2 flex-shrink-0">
        {/* Logo */}
        <div className="w-7 h-7 rounded-lg bg-klarvo-teal/10 border border-klarvo-teal/20 flex items-center justify-center">
          <MicIcon className="w-3.5 h-3.5 text-klarvo-teal" />
        </div>
        <span className="text-sm font-semibold text-klarvo-muted tracking-wide">Klarvo</span>
        <span className="text-[9px] font-medium text-klarvo-dim/70 uppercase tracking-widest ml-1">Early Access</span>

        {/* Settings toggle */}
        <button
          aria-label="Toggle settings"
          aria-expanded={panels.showSettings}
          onClick={() => panels.toggle("settings")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showSettings
              ? "text-klarvo-teal bg-klarvo-teal/10"
              : "text-klarvo-dim hover:text-klarvo-muted hover:bg-klarvo-surface/50",
          ].join(" ")}
        >
          <GearIcon className={isMobile ? "w-4 h-4" : "w-5 h-5"} />
        </button>

        {/* History toggle */}
        <button
          aria-label="Toggle history"
          aria-expanded={panels.showHistory}
          onClick={() => panels.toggle("history")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showHistory
              ? "text-klarvo-teal bg-klarvo-teal/10"
              : "text-klarvo-dim hover:text-klarvo-muted hover:bg-klarvo-surface/50",
          ].join(" ")}
        >
          <svg className={isMobile ? "w-4 h-4" : "w-5 h-5"} viewBox="0 0 24 24" fill="currentColor">
            <path d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z" />
          </svg>
        </button>

        {/* Stats toggle */}
        <button
          aria-label="Toggle stats"
          aria-expanded={panels.showStats}
          onClick={() => panels.toggle("stats")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showStats
              ? "text-klarvo-teal bg-klarvo-teal/10"
              : "text-klarvo-dim hover:text-klarvo-muted hover:bg-klarvo-surface/50",
          ].join(" ")}
        >
          <svg className={isMobile ? "w-4 h-4" : "w-5 h-5"} viewBox="0 0 24 24" fill="currentColor">
            <path d="M5 9.2h3V19H5V9.2zM10.6 5h2.8v14h-2.8V5zm5.6 8H19v6h-2.8v-6z" />
          </svg>
        </button>

        {/* Notes toggle -- hidden for Early Access (feature incomplete) */}
        {/* <button
          aria-label="Toggle voice notes"
          aria-expanded={panels.showNotes}
          onClick={() => panels.toggle("notes")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showNotes
              ? "text-klarvo-teal bg-klarvo-teal/10"
              : "text-klarvo-dim hover:text-klarvo-muted hover:bg-klarvo-surface/50",
          ].join(" ")}
        >
          <NoteIcon className={isMobile ? "w-4 h-4" : "w-5 h-5"} />
        </button> */}

      </div>

      {/* ── Trial-expired banner ── */}
      {showTrialExpired && (
        <div className="bg-klarvo-surface/95 border border-klarvo-border rounded-lg mx-4 mt-3 p-4 flex items-start gap-3 flex-shrink-0">
          <div className="flex-1">
            <p className="text-sm text-klarvo-text font-medium">Your 14-day trial has ended</p>
            <p className="text-xs text-klarvo-muted mt-1">
              Klarvo still works — some features are now locked. Unlock everything for €29.
            </p>
          </div>
          <button onClick={dismissTrialExpired} className="text-xs text-klarvo-muted hover:text-klarvo-text px-2 py-1 shrink-0">
            Got it
          </button>
        </div>
      )}

      {/* ── Mode picker row ──
           Shown only on the home/recording view (no panel open). Always fully
           visible regardless of window width because it is its own dedicated row. */}
      {!panels.anyOpen && (
        <div className="px-4 pb-2 flex-shrink-0">
          <StylePicker
            value={settings.cleanupStyle}
            onChange={settings.handleStyleChange}
            disabled={isBusy || isRecording}
          />
        </div>
      )}

      {/* ── Settings Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-100 ease-out flex-shrink-0",
          // On Android the nav bar (~48 px) overlaps the bottom of the WebView, so we
          // subtract an extra 48 px from the available height. Without this the sticky
          // Save-button footer in SettingsPanel ends up behind the nav bar.
          panels.showSettings
            ? (isMobile ? "max-h-[calc(100vh-148px)] opacity-100 py-2" : "max-h-[calc(100vh-100px)] opacity-100 py-2")
            : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showSettings && (
          <SettingsPanel
            onClose={() => panels.close("settings")}
            loadedSettings={settings.loadedSettings}
            language={settings.language}
            cleanupStyle={settings.cleanupStyle}
            hotkey={settings.hotkey}
            hotkeyMode={settings.hotkeyMode}
            hotkeySlot2={settings.hotkeySlot2}
            hotkeyModeSlot2={settings.hotkeyModeSlot2}
            audioDevice={settings.audioDevice}
            audioDevices={settings.audioDevices}
            dictionary={settings.dictionary}
            onSave={settings.handleSaveSettings}
            onLanguageChange={settings.handleLanguageChange}
            onStyleChange={settings.handleStyleChange}
            onHotkeyChange={settings.setHotkey}
            onHotkeyModeChange={settings.setHotkeyMode}
            onAudioDeviceChange={settings.setAudioDevice}
            onAddTerm={settings.handleAddTerm}
            onRemoveTerm={settings.handleRemoveTerm}
            outputLanguage={settings.outputLanguage}
            onOutputLanguageChange={settings.handleOutputLanguageChange}
            licenseStatus={license.licenseStatus}
            licenseSource={license.licenseSource}
            licenseLoading={license.licenseLoading}
            onValidateLicense={license.validateLicense}
            onRemoveLicense={license.removeLicense}
            onDeactivateLicense={license.deactivateLicense}
            onRestartOnboarding={handleRestartOnboarding}
            onRegisterBack={(fn) => { settingsBackRef.current = fn; }}
          />
        )}
      </div>

      {/* ── History Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-100 ease-out flex-shrink-0",
          panels.showHistory ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showHistory && (
          <div className="w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-klarvo-border/40">
              <span className="text-[11px] font-semibold text-klarvo-dim uppercase tracking-widest">History</span>
              <button
                onClick={() => panels.close("history")}
                className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50"
              >
                <CloseIcon />
              </button>
            </div>

            <div className="px-4 pt-3 flex gap-2">
              <input
                type="text"
                placeholder="Search text..."
                value={historySearch}
                onChange={(e) => handleHistorySearch(e.target.value, historyAppSearch)}
                className="flex-1 bg-klarvo-surface-2 border border-klarvo-border/60 rounded-lg px-3 py-2 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-klarvo-teal/40 focus:ring-1 focus:ring-klarvo-teal/20 transition-colors"
              />
              <input
                type="text"
                placeholder="App..."
                value={historyAppSearch}
                onChange={(e) => handleHistorySearch(historySearch, e.target.value)}
                className="w-24 bg-klarvo-surface-2 border border-klarvo-border/60 rounded-lg px-3 py-2 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-klarvo-teal/40 focus:ring-1 focus:ring-klarvo-teal/20 transition-colors"
              />
            </div>

            <div className="overflow-y-auto max-h-[calc(100vh-250px)] p-4 flex flex-col gap-2.5">
              {historyLoadState === "loading" ? (
                <div className="flex flex-col items-center justify-center py-10 gap-2 text-center">
                  <p className="text-sm font-medium text-klarvo-muted">Loading…</p>
                </div>
              ) : historyLoadState === "error" ? (
                <div className="flex flex-col items-center justify-center py-10 gap-3 text-center">
                  <p className="text-sm font-medium text-klarvo-muted">Could not load history</p>
                  <button
                    onClick={loadHistory}
                    className="text-[11px] text-klarvo-teal hover:text-klarvo-teal/80 transition-colors"
                  >
                    Retry
                  </button>
                </div>
              ) : historyEntries.length === 0 ? (
                (historySearch.trim() || historyAppSearch.trim()) ? (
                  <div className="flex flex-col items-center justify-center py-10 gap-2 text-center">
                    <p className="text-sm font-medium text-klarvo-muted">No results</p>
                    <p className="text-xs text-klarvo-dim">No dictations match your search</p>
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center py-10 gap-3 text-center">
                    <div className="w-10 h-10 rounded-full bg-klarvo-surface-2 flex items-center justify-center">
                      <svg className="w-5 h-5 text-klarvo-dim" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                        <path d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z" />
                      </svg>
                    </div>
                    <div>
                      <p className="text-sm font-medium text-klarvo-muted">No dictations yet</p>
                      {isDesktop && (
                        <p className="text-xs text-klarvo-dim mt-0.5">Start recording with {hotkeyDisplay}</p>
                      )}
                    </div>
                  </div>
                )
              ) : (
                historyEntries.map((entry) => {
                  if (entry.status === "pending") {
                    return (
                      <div
                        key={entry.id}
                        className="bg-klarvo-amber/10 border border-klarvo-amber/40 rounded-xl p-3.5 hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60 transition-colors"
                      >
                        <p className="text-xs text-klarvo-amber">
                          ⏳ Audio gesichert — noch nicht transkribiert
                        </p>
                        <div className="flex items-center justify-between mt-2">
                          <span className="text-[11px] text-klarvo-dim">
                            <span className="font-geist-mono">{new Date(entry.createdAt + "Z").toLocaleString()}</span>
                            {entry.appName && (
                              <span className="ml-1 inline-flex items-center px-1.5 py-0.5 bg-klarvo-amber/10 text-klarvo-amber border border-[rgba(233,162,76,.32)] rounded-full text-[9px]">{entry.appName}</span>
                            )}
                          </span>
                          <div className="flex gap-1.5">
                            <button
                              onClick={() => handleReprocessPendingEntry(entry.id)}
                              disabled={reprocessingId !== null}
                              className="text-[11px] text-klarvo-teal hover:text-klarvo-teal/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                            >
                              {reprocessingId === entry.id ? "Verarbeite…" : "Erneut verarbeiten"}
                            </button>
                            <button
                              onClick={() => handleDiscardPendingEntry(entry.id)}
                              disabled={reprocessingId !== null}
                              className="text-[11px] text-klarvo-danger hover:text-klarvo-danger/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                            >
                              Verwerfen
                            </button>
                          </div>
                        </div>
                        {pendingErrors[entry.id] && (
                          <p className="mt-1.5 text-[11px] text-klarvo-danger">{pendingErrors[entry.id]}</p>
                        )}
                      </div>
                    );
                  }

                  if (pendingDeleteIds.has(entry.id)) {
                    return (
                      <div
                        key={entry.id}
                        className="flex items-center gap-2.5 bg-klarvo-surface-2 rounded-xl px-4 py-3"
                      >
                        <span className="font-geist-mono text-[11px] text-klarvo-dim">Deleted</span>
                        <button
                          onClick={() => handleUndoDelete(entry.id)}
                          className="ml-auto text-[11px] text-klarvo-danger hover:text-klarvo-danger/80 transition-colors"
                        >
                          Undo
                        </button>
                      </div>
                    );
                  }

                  const rawToggleCopyId = `hist-raw-toggle-${entry.id}`;
                  const rawOverlayCopyId = `hist-raw-overlay-${entry.id}`;
                  const mainCopyId = `hist-main-${entry.id}`;
                  const rawToggleStatus = copyFeedback.statusOf(rawToggleCopyId);
                  const rawOverlayStatus = copyFeedback.statusOf(rawOverlayCopyId);
                  const mainStatus = copyFeedback.statusOf(mainCopyId);

                  return (
                  <div
                    key={entry.id}
                    className="bg-klarvo-bg border border-klarvo-border/60 rounded-xl p-3.5 group hover:bg-klarvo-elevated/40 hover:border-klarvo-border transition-colors"
                  >
                    <HighlightedText text={entry.text} query={historySearch} className="text-xs text-klarvo-muted whitespace-pre-wrap" />
                    {entry.rawText && entry.rawText !== entry.text && (
                      <div className="mt-1.5">
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => setExpandedHistoryRaw((prev) => {
                              const next = new Set(prev);
                              next.has(entry.id) ? next.delete(entry.id) : next.add(entry.id);
                              return next;
                            })}
                            className="text-[11px] text-klarvo-dim hover:text-klarvo-muted transition-colors"
                          >
                            {expandedHistoryRaw.has(entry.id) ? "Hide original" : "Show original"}
                          </button>
                          {expandedHistoryRaw.has(entry.id) && (
                            <button
                              onClick={() => copyFeedback.copy(rawToggleCopyId, entry.rawText!)}
                              className={`text-[11px] transition-colors ${copyColorClassName(rawToggleStatus, "text-klarvo-teal hover:text-klarvo-teal/80")}`}
                            >
                              {copyLabel(rawToggleStatus, "Copy Original")}
                            </button>
                          )}
                        </div>
                        {expandedHistoryRaw.has(entry.id) && (
                          <div className="mt-1 relative group/raw">
                            <p className="text-[11px] text-klarvo-dim whitespace-pre-wrap bg-klarvo-bg-deep rounded-lg px-2.5 py-1.5 border border-klarvo-border/40">
                              {entry.rawText}
                            </p>
                            <button
                              onClick={() => copyFeedback.copy(rawOverlayCopyId, entry.rawText!)}
                              className={[
                                "absolute top-1 right-1 text-[11px] transition-opacity",
                                rawOverlayStatus !== "idle" ? "opacity-100" : "opacity-0 group-hover/raw:opacity-100",
                                copyColorClassName(rawOverlayStatus, "text-klarvo-dim hover:text-klarvo-muted"),
                              ].join(" ")}
                            >
                              {copyLabel(rawOverlayStatus, "Copy")}
                            </button>
                          </div>
                        )}
                      </div>
                    )}
                    <div className="flex items-center justify-between mt-2">
                      <span className="text-[11px] text-klarvo-dim">
                        <span className="font-geist-mono">{new Date(entry.createdAt + "Z").toLocaleString()}</span>
                        {entry.style !== "polished" && (
                          <span className="text-klarvo-teal font-geist-mono"> · {entry.style}</span>
                        )}
                        {entry.appName && (
                          <span className="ml-1 inline-flex items-center px-1.5 py-0.5 bg-klarvo-amber/10 text-klarvo-amber border border-[rgba(233,162,76,.32)] rounded-full text-[9px]">{entry.appName}</span>
                        )}
                      </span>
                      <div className={[
                        "flex gap-1.5 transition-opacity",
                        mainStatus !== "idle" ? "opacity-100" : "opacity-0 group-hover:opacity-100",
                      ].join(" ")}>
                        <button
                          onClick={() => copyFeedback.copy(mainCopyId, entry.text)}
                          className={`text-[11px] transition-colors ${copyColorClassName(mainStatus, "text-klarvo-teal hover:text-klarvo-teal/80")}`}
                        >
                          {copyLabel(mainStatus, "Copy")}
                        </button>
                        <button
                          onClick={() => handleDeleteHistoryEntry(entry)}
                          aria-hidden={mainStatus !== "idle"}
                          tabIndex={mainStatus !== "idle" ? -1 : undefined}
                          className={[
                            "text-[11px] text-klarvo-danger hover:text-klarvo-danger/80 transition",
                            // Andi 2026-08-21 (canon NACHTRAG 2): while the card is answering a Copy
                            // click, Delete steps aside — a destructive control must not sit next to a
                            // success message the pointer is aiming at. Opacity, not display: the row
                            // keeps its width, so the geometry measured for AC1 still holds.
                            mainStatus !== "idle" ? "opacity-0 pointer-events-none" : "",
                          ].join(" ")}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                  );
                })
              )}
            </div>
          </div>
        )}
      </div>

      {/* ── Stats Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-100 ease-out flex-shrink-0",
          panels.showStats ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showStats && (
          <div className="w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-klarvo-border/40">
              <span className="text-[11px] font-semibold text-klarvo-dim uppercase tracking-widest">Statistics</span>
              <button
                onClick={() => panels.close("stats")}
                className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50"
              >
                <CloseIcon />
              </button>
            </div>

            {usageStats ? (
              <div className="overflow-y-auto max-h-[calc(100vh-200px)]">
                {/* Cost Dashboard */}
                <div className="p-4">
                  <CostDashboard stats={usageStats} />
                </div>

                {/* Filler word analysis — amber accent to restore Teal→Amber→Teal color rhythm */}
                {!isPaid ? (
                  <div className="px-4 pb-4 pt-3">
                    <div className="bg-klarvo-bg border border-klarvo-amber/30 rounded-xl p-4 flex items-center gap-2">
                      <LockIcon className="w-3.5 h-3.5 text-klarvo-amber/60 flex-shrink-0" />
                      <p className="text-xs text-klarvo-amber/70">Filler word analysis requires a Klarvo license.</p>
                    </div>
                  </div>
                ) : fillerStats.length > 0 ? (
                  <div className="px-4 pb-4 pt-3">
                    <button
                      onClick={() => setShowFillerStats((v) => !v)}
                      className="flex items-center gap-1.5 text-[11px] font-semibold text-klarvo-amber uppercase tracking-widest hover:text-klarvo-amber-hi transition-colors w-full text-left"
                    >
                      <span className={`transition-transform duration-150 ${showFillerStats ? "rotate-90" : ""}`}>▸</span>
                      Top Filler Words
                    </button>
                    {showFillerStats && (
                      <div className="mt-3 bg-klarvo-bg border border-klarvo-amber/30 rounded-xl p-4">
                        <FillerStatsChart entries={fillerStats} />
                      </div>
                    )}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        )}
      </div>

      {/* ── Voice Notes Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-100 ease-out flex-shrink-0",
          panels.showNotes ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showNotes && (
          <VoiceNotesPanel
            notes={notes}
            onRefresh={() => getNotes(50).then(setNotes).catch(console.error)}
            onClose={() => panels.close("notes")}
          />
        )}
      </div>

      {/* ── Feedback Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-100 ease-out flex-shrink-0",
          panels.showFeedback ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showFeedback && (
          <FeedbackModal
            isOpen={panels.showFeedback}
            onClose={() => panels.close("feedback")}
            defaultArea={feedbackArea}
          />
        )}
      </div>

      {/* ── Center: Record Button (hidden when any panel is open) ── */}
      {!panels.anyOpen && (
      <div className="flex-1 flex flex-col items-center justify-center gap-4 px-4 min-h-0">
        <RecordButton
          recordingState={recording.recordingState === "done" || recording.recordingState === "error" ? "idle" : recording.recordingState}
          onClick={recording.handleRecordToggle}
        />

        {/* Status label */}
        <div className="text-center">
          <p className={[
            "text-xs font-medium",
            recording.recordingState === "error" ? "text-klarvo-danger"
              : recording.recordingState === "recording" ? "text-klarvo-amber"
              : recording.recordingState === "done" ? "text-klarvo-teal"
              : isBusy ? "text-klarvo-amber"
              : "text-klarvo-dim",
          ].join(" ")}>
            {recording.errorMessage && recording.recordingState === "error"
              ? recording.errorMessage
              : STATUS_LABELS[recording.recordingState]}
          </p>
        </div>

        {/* Result */}
        {recording.resultText !== null && (
          <div className="w-full max-w-xs flex flex-col gap-1.5">
            <textarea
              readOnly
              value={recording.resultText}
              rows={3}
              className="w-full bg-klarvo-bg border border-klarvo-border/60 rounded-xl px-3.5 py-2.5 text-sm text-klarvo-text resize-none focus:outline-none focus:border-klarvo-teal/30 transition-colors"
            />
            {recording.rawText && recording.rawText !== recording.resultText && (
              <div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => recording.setShowRawText((v) => !v)}
                    className="text-[11px] text-klarvo-dim hover:text-klarvo-muted transition-colors"
                  >
                    {recording.showRawText ? "Hide original" : "Show original"}
                  </button>
                  {recording.showRawText && (
                    <button
                      onClick={() => copyFeedback.copy("current-raw-toggle", recording.rawText!)}
                      className={`text-[11px] transition-colors ${copyColorClassName(copyFeedback.statusOf("current-raw-toggle"), "text-klarvo-teal hover:text-klarvo-teal/80")}`}
                    >
                      {copyLabel(copyFeedback.statusOf("current-raw-toggle"), "Copy Original")}
                    </button>
                  )}
                </div>
                {recording.showRawText && (
                  <div className="mt-1 relative group">
                    <textarea
                      readOnly
                      value={recording.rawText}
                      rows={2}
                      className="w-full bg-klarvo-bg-deep border border-klarvo-border/40 rounded-lg px-3 py-2 text-xs text-klarvo-muted resize-none focus:outline-none"
                    />
                    <button
                      onClick={() => copyFeedback.copy("current-raw-overlay", recording.rawText!)}
                      className={[
                        "absolute top-1.5 right-1.5 text-[11px] transition-opacity",
                        copyFeedback.statusOf("current-raw-overlay") !== "idle" ? "opacity-100" : "opacity-0 group-hover:opacity-100",
                        copyColorClassName(copyFeedback.statusOf("current-raw-overlay"), "text-klarvo-dim hover:text-klarvo-muted"),
                      ].join(" ")}
                    >
                      {copyLabel(copyFeedback.statusOf("current-raw-overlay"), "Copy")}
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
      )}

      {/* ── Footer (desktop only) ── */}
      {isDesktop && (
        <div className="flex items-center justify-center px-4 py-3 flex-shrink-0">
          <span className="text-[11px] font-geist-mono text-klarvo-dim">{hotkeyDisplay}</span>
        </div>
      )}

      {/* ── Quick Tip ── */}
      {!showOnboarding && quickTip.activeTip && (
        <QuickTip
          title={quickTip.activeTip.title}
          text={quickTip.activeTip.text}
          actionLabel={quickTip.activeTip.actionLabel}
          onAction={() => {
            quickTip.handleAction();
            if (quickTip.openPanel) panels.toggle(quickTip.openPanel);
          }}
          onDismiss={quickTip.dismissTip}
        />
      )}

      {/* ── Preview-mode banner ── */}
      {isPreviewMode && (
        <div
          className="fixed bottom-3 right-3 z-50 pointer-events-none"
          aria-hidden="true"
        >
          <span className="px-2 py-1 rounded-md text-[10px] font-geist-mono font-semibold tracking-wide bg-klarvo-bg/80 border border-klarvo-border/50 text-klarvo-dim backdrop-blur-sm">
            Preview Mode
          </span>
        </div>
      )}

      {/* ── Theme Switcher (preview only) ── */}
      <ThemeSwitcher />

      {/* ── Feedback FAB (floating, always visible) ── */}
      <div className="fixed right-5 z-[9990] flex flex-col items-end gap-2" style={{ bottom: isMobile ? '128px' : '1.25rem' }}>
        {/* Tooltip — shown every start until permanently dismissed */}
        {showFeedbackTooltip && !panels.showFeedback && (
          <div className={`bg-klarvo-surface border border-klarvo-border/60 rounded-xl shadow-xl shadow-black/40 px-4 py-3 ${isMobile ? "max-w-[280px]" : "max-w-[260px]"} animate-in fade-in`}>
            <p className="text-sm text-klarvo-text font-medium leading-snug">
              Spotted something? Tap here to send feedback anytime!
            </p>
            <div className="flex items-center justify-end gap-2 mt-2">
              <button
                onClick={() => dismissFeedbackTooltip(true)}
                className="text-[10px] text-klarvo-dim hover:text-klarvo-muted transition-colors"
              >
                Don&apos;t show again
              </button>
              <button
                onClick={() => dismissFeedbackTooltip(false)}
                className="text-[10px] text-klarvo-muted hover:text-klarvo-text transition-colors font-medium"
              >
                &times;
              </button>
            </div>
          </div>
        )}
        <button
          title="Send Feedback"
          aria-label="Send feedback"
          onClick={() => { panels.toggle("feedback"); setShowFeedbackTooltip(false); }}
          className={`${isMobile ? "w-20 h-20" : "w-16 h-16"} rounded-full bg-klarvo-amber/20 border border-klarvo-amber/30 text-klarvo-amber shadow-lg shadow-black/30 hover:bg-klarvo-amber/30 hover:scale-105 transition-all duration-150 flex items-center justify-center`}
        >
          <FeedbackIcon className={isMobile ? "w-9 h-9" : "w-7 h-7"} />
        </button>
      </div>

      {/* ── Preview Comments overlay ── */}
      {isPreviewMode && <PreviewComments />}

    </main>
  );
}
