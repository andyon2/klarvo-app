import { useState, useCallback, useEffect } from "react";
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
  reformatText,
  isFirstRun,
  isPreviewMode,
} from "./tauri-commands";
import { isMobile, isDesktop } from "./platform";
import Onboarding from "./Onboarding";

// Components
import {
  MicIcon, StopIcon, SpinnerIcon, GearIcon, CloseIcon,
  MailIcon, ListIcon, SummaryIcon, NoteIcon, LockIcon,
} from "./components/icons";
import { FillerStatsChart, HighlightedText, StatCard } from "./components/ui";
import { SettingsPanel } from "./components/SettingsPanel";
import { AdvancedSettingsPanel } from "./components/AdvancedSettingsPanel";
import { VoiceNotesPanel } from "./components/VoiceNotesPanel";

// Hooks
import { useRecording } from "./hooks/useRecording";
import { useSettings } from "./hooks/useSettings";
import { usePanels } from "./hooks/usePanels";
import { useLicense } from "./hooks/useLicense";

// --- Helpers -----------------------------------------------------------------

function formatHotkeyDisplay(hotkey: string): string {
  return hotkey.split("+").map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(" + ");
}

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds.toFixed(0)}s`;
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (mins < 60) return `${mins}m ${secs}s`;
  const hrs = Math.floor(mins / 60);
  const remainMins = mins % 60;
  return `${hrs}h ${remainMins}m`;
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
          ? "bg-red-500/20 text-red-400 shadow-[0_0_40px_rgba(239,68,68,0.3)]"
          : isBusy
          ? "bg-amber-500/15 text-amber-400 shadow-[0_0_30px_rgba(245,158,11,0.2)]"
          : "bg-emerald-500/15 text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.2)] hover:shadow-[0_0_50px_rgba(16,185,129,0.3)] hover:bg-emerald-500/20",
      ].join(" ")}
    >
      <span
        className={[
          "absolute inset-0 rounded-full border-2 transition-colors duration-200",
          isRecording ? "border-red-500/40" : isBusy ? "border-amber-500/30" : "border-emerald-500/25",
        ].join(" ")}
      />
      {isRecording && (
        <span className="absolute inset-0 rounded-full border-2 border-red-400 opacity-40 animate-ping" />
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
    <div className={`flex gap-0.5 bg-[#111113] rounded-lg p-0.5 border border-zinc-800/60 ${isMobile ? "w-full" : "w-fit"}`}>
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
              ? "bg-emerald-500/15 text-emerald-400"
              : "text-zinc-500 hover:text-zinc-300",
          ].join(" ")}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

// OutputLanguagePicker removed from header -- available in Settings only.

// --- Reformat Buttons --------------------------------------------------------

interface ReformatButtonsProps {
  text: string;
  originalText: string;
  onResult: (text: string) => void;
}

function ReformatButtons({ text, originalText, onResult }: ReformatButtonsProps) {
  const [loading, setLoading] = useState<string | null>(null);
  const isReformatted = text !== originalText;

  const FORMATS = [
    { id: "email", label: "Email", Icon: MailIcon },
    { id: "bullets", label: "Bullets", Icon: ListIcon },
    { id: "summary", label: "Summary", Icon: SummaryIcon },
  ] as const;

  const handleReformat = async (format: string) => {
    if (loading) return;
    setLoading(format);
    try {
      const result = await reformatText(originalText, format);
      onResult(result);
      navigator.clipboard.writeText(result).catch(console.error);
    } catch (err) {
      console.error("reformat_text failed:", err);
    } finally {
      setLoading(null);
    }
  };

  const handleReset = () => {
    onResult(originalText);
    navigator.clipboard.writeText(originalText).catch(console.error);
  };

  return (
    <div className="flex items-center gap-1.5">
      {isReformatted && (
        <button
          onClick={handleReset}
          title="Reset to original"
          className="flex items-center gap-1 px-2 py-1 rounded-lg text-[11px] font-medium border bg-zinc-800/60 border-zinc-700/60 text-zinc-300 hover:text-zinc-100 transition-all duration-100"
        >
          <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
            <path d="M3 3v5h5" />
          </svg>
          Reset
        </button>
      )}
      {FORMATS.map(({ id, label, Icon }) => (
        <button
          key={id}
          onClick={() => handleReformat(id)}
          disabled={loading !== null}
          title={`Reformat as ${label}`}
          className={[
            "flex items-center gap-1 px-2 py-1 rounded-lg text-[11px] font-medium border",
            "transition-all duration-100 disabled:opacity-50 disabled:cursor-not-allowed",
            loading === id
              ? "bg-amber-500/10 border-amber-500/20 text-amber-400"
              : "bg-[#111113] border-zinc-800/60 text-zinc-400 hover:text-zinc-200 hover:border-zinc-700/60",
          ].join(" ")}
        >
          {loading === id ? (
            <SpinnerIcon className="w-3 h-3" />
          ) : (
            <Icon className="w-3 h-3" />
          )}
          {label}
        </button>
      ))}
    </div>
  );
}

// --- Main App ----------------------------------------------------------------

export default function App() {
  // --- Hooks ---
  const settings = useSettings();
  const recording = useRecording(settings.cleanupStyle, settings.language);
  const license = useLicense();

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
  const [historySearch, setHistorySearch] = useState("");
  const [historyAppSearch, setHistoryAppSearch] = useState("");
  const [expandedHistoryRaw, setExpandedHistoryRaw] = useState<Set<number>>(new Set());

  // Stats state
  const [usageStats, setUsageStats] = useState<UsageSummary | null>(null);
  const [fillerStats, setFillerStats] = useState<{ word: string; count: number }[]>([]);
  const [showFillerStats, setShowFillerStats] = useState(false);

  // Notes state
  const [notes, setNotes] = useState<HistoryEntry[]>([]);

  // Onboarding
  const [showOnboarding, setShowOnboarding] = useState(false);

  // Panel callbacks for lazy loading
  const panels = usePanels({
    onOpenHistory: () => getHistory(50).then(setHistoryEntries).catch(console.error),
    onOpenStats: () => {
      getUsageStats().then(setUsageStats).catch(console.error);
      getFillerStats().then(setFillerStats).catch(console.error);
    },
    onOpenNotes: () => getNotes(50).then(setNotes).catch(console.error),
  });

  // Check for first run / onboarding
  useEffect(() => {
    isFirstRun()
      .then((firstRun) => { if (firstRun) setShowOnboarding(true); })
      .catch(console.error);
  }, []);

  // Android back button: close open panel instead of leaving the app
  useEffect(() => {
    if (!isMobile) return;
    if (panels.anyOpen) {
      window.history.pushState({ panel: true }, "");
    }
  }, [panels.anyOpen]);

  useEffect(() => {
    if (!isMobile) return;
    const handler = () => { panels.closeAll(); };
    window.addEventListener("popstate", handler);
    return () => window.removeEventListener("popstate", handler);
  }, [panels.closeAll]);

  // --- Derived state ---
  const isBusy = recording.recordingState === "transcribing" || recording.recordingState === "cleaning";
  const isRecording = recording.recordingState === "recording";
  const headerBtnPad = isMobile ? "p-2.5" : "p-1.5";
  const hotkeyDisplay = formatHotkeyDisplay(settings.loadedSettings?.hotkey ?? "ctrl+shift+d");

  // --- History handlers ---
  const handleHistorySearch = useCallback(async (textQ: string, appQ: string) => {
    setHistorySearch(textQ);
    setHistoryAppSearch(appQ);
    if (textQ.trim() || appQ.trim()) {
      const results = await searchHistory(textQ.trim() || undefined, appQ.trim() || undefined);
      setHistoryEntries(results);
    } else {
      const entries = await getHistory(50);
      setHistoryEntries(entries);
    }
  }, []);

  const handleDeleteHistoryEntry = useCallback(async (id: number) => {
    await deleteHistoryEntry(id);
    setHistoryEntries((prev) => prev.filter((e) => e.id !== id));
  }, []);

  // --- Onboarding handler ---
  const handleOnboardingComplete = useCallback(async (updated: import("./types").AppSettings) => {
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
    setShowOnboarding(false);
  }, [settings]);

  // Output language change is handled in SettingsPanel only.

  if (showOnboarding) {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

  return (
    <main
      className="h-screen bg-[#0a0a0c] text-zinc-100 flex flex-col select-none overflow-y-auto"
      style={{
        fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
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
        <div className="w-7 h-7 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center">
          <MicIcon className="w-3.5 h-3.5 text-emerald-400" />
        </div>
        <span className="text-sm font-semibold text-zinc-300 tracking-wide">Dikta</span>

        {/* Settings toggle */}
        <button
          aria-label="Toggle settings"
          aria-expanded={panels.showSettings}
          onClick={() => panels.toggle("settings")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showSettings
              ? "text-emerald-400 bg-emerald-500/10"
              : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
          ].join(" ")}
        >
          <GearIcon />
        </button>

        {/* History toggle */}
        <button
          aria-label="Toggle history"
          aria-expanded={panels.showHistory}
          onClick={() => panels.toggle("history")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showHistory
              ? "text-emerald-400 bg-emerald-500/10"
              : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
          ].join(" ")}
        >
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
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
              ? "text-emerald-400 bg-emerald-500/10"
              : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
          ].join(" ")}
        >
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
            <path d="M5 9.2h3V19H5V9.2zM10.6 5h2.8v14h-2.8V5zm5.6 8H19v6h-2.8v-6z" />
          </svg>
        </button>

        {/* Notes toggle */}
        <button
          aria-label="Toggle voice notes"
          aria-expanded={panels.showNotes}
          onClick={() => panels.toggle("notes")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showNotes
              ? "text-emerald-400 bg-emerald-500/10"
              : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
          ].join(" ")}
        >
          <NoteIcon className="w-4 h-4" />
        </button>

        {/* Integrations toggle -- desktop only */}
        {isDesktop && (
          <button
            aria-label="Toggle integrations"
            aria-expanded={panels.showIntegrations}
            onClick={() => panels.toggle("integrations")}
            className={[
              `${headerBtnPad} rounded-lg transition-all duration-150`,
              panels.showIntegrations
                ? "text-emerald-400 bg-emerald-500/10"
                : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
            ].join(" ")}
          >
            {/* Plug/integration icon */}
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 2v6M12 16v6M4.93 4.93l4.24 4.24M14.83 14.83l4.24 4.24M2 12h6M16 12h6M4.93 19.07l4.24-4.24M14.83 9.17l4.24-4.24" />
            </svg>
          </button>
        )}

        {/* Advanced settings toggle */}
        <button
          title="Advanced settings"
          aria-label="Toggle advanced settings"
          aria-expanded={panels.showAdvanced}
          onClick={() => panels.toggle("advanced")}
          className={[
            `${headerBtnPad} rounded-lg transition-all duration-150`,
            panels.showAdvanced
              ? "text-emerald-400 bg-emerald-500/10"
              : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50",
          ].join(" ")}
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
          </svg>
        </button>
      </div>

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
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
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
            licenseLoading={license.licenseLoading}
            onValidateLicense={license.validateLicense}
            onRemoveLicense={license.removeLicense}
          />
        )}
      </div>

      {/* ── History Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          panels.showHistory ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showHistory && (
          <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
              <span className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest">History</span>
              <button
                onClick={() => panels.close("history")}
                className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
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
                className="flex-1 bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
              />
              <input
                type="text"
                placeholder="App..."
                value={historyAppSearch}
                onChange={(e) => handleHistorySearch(historySearch, e.target.value)}
                className="w-24 bg-[#111113] border border-zinc-800/60 rounded-lg px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:outline-none focus:border-emerald-500/40 transition-colors"
              />
            </div>

            <div className="overflow-y-auto max-h-[calc(100vh-250px)] p-4 flex flex-col gap-2">
              {historyEntries.length === 0 ? (
                <p className="text-xs text-zinc-500 italic text-center py-4">No dictations yet.</p>
              ) : (
                historyEntries.map((entry) => (
                  <div
                    key={entry.id}
                    className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors"
                  >
                    <HighlightedText text={entry.text} query={historySearch} className="text-xs text-zinc-300 whitespace-pre-wrap" />
                    {entry.rawText && entry.rawText !== entry.text && (
                      <div className="mt-1.5">
                        <button
                          onClick={() => setExpandedHistoryRaw((prev) => {
                            const next = new Set(prev);
                            next.has(entry.id) ? next.delete(entry.id) : next.add(entry.id);
                            return next;
                          })}
                          className="text-[11px] text-zinc-600 hover:text-zinc-400 transition-colors"
                        >
                          {expandedHistoryRaw.has(entry.id) ? "Hide original" : "Show original"}
                        </button>
                        {expandedHistoryRaw.has(entry.id) && (
                          <div className="mt-1 relative group/raw">
                            <p className="text-[11px] text-zinc-500 whitespace-pre-wrap bg-[#0c0c0e] rounded-lg px-2.5 py-1.5 border border-zinc-800/40">
                              {entry.rawText}
                            </p>
                            <button
                              onClick={() => navigator.clipboard.writeText(entry.rawText!)}
                              className="absolute top-1 right-1 text-[11px] text-zinc-600 hover:text-zinc-300 opacity-0 group-hover/raw:opacity-100 transition-opacity"
                            >
                              Copy
                            </button>
                          </div>
                        )}
                      </div>
                    )}
                    <div className="flex items-center justify-between mt-2">
                      <span className="text-[11px] text-zinc-500">
                        {new Date(entry.createdAt + "Z").toLocaleString()}
                        {entry.style !== "polished" && ` · ${entry.style}`}
                        {entry.appName && (
                          <span className="ml-1 px-1.5 py-0.5 bg-zinc-800/60 rounded text-[9px] text-zinc-400">{entry.appName}</span>
                        )}
                      </span>
                      <div className="flex gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={() => navigator.clipboard.writeText(entry.text).catch(console.error)}
                          className="text-[11px] text-zinc-500 hover:text-emerald-400 transition-colors"
                        >
                          Copy
                        </button>
                        <button
                          onClick={() => handleDeleteHistoryEntry(entry.id)}
                          className="text-[11px] text-zinc-500 hover:text-red-400 transition-colors"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* ── Stats Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          panels.showStats ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showStats && (
          <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
              <span className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest">Statistics & Costs</span>
              <button
                onClick={() => panels.close("stats")}
                className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
              >
                <CloseIcon />
              </button>
            </div>

            {usageStats ? (
              <>
                <div className="p-4 grid grid-cols-2 gap-3">
                  <StatCard label="Today" value={`${usageStats.dictationsToday}`} sub="dictations" />
                  <StatCard label="Cost Today" value={formatCost(usageStats.costTodayUsd)} sub="USD" />
                  <StatCard label="Total Dictations" value={`${usageStats.totalDictations}`} />
                  <StatCard label="Total Words" value={usageStats.totalWords.toLocaleString()} />
                  <StatCard label="Audio Recorded" value={formatDuration(usageStats.totalAudioSeconds)} />
                  <StatCard label="Total Cost" value={formatCost(usageStats.totalCostUsd)} sub="USD" />
                  <StatCard label="STT (Groq)" value={formatCost(usageStats.totalSttCostUsd)} sub="USD" />
                  <StatCard label="LLM (DeepSeek)" value={formatCost(usageStats.totalLlmCostUsd)} sub="USD" />
                </div>

                {!isPaid ? (
                  <div className="px-4 pb-4 flex items-center gap-2">
                    <LockIcon className="w-3.5 h-3.5 text-zinc-600 flex-shrink-0" />
                    <p className="text-xs text-zinc-500">Filler word analysis requires a Dikta license.</p>
                  </div>
                ) : fillerStats.length > 0 ? (
                  <div className="px-4 pb-4">
                    <button
                      onClick={() => setShowFillerStats((v) => !v)}
                      className="flex items-center gap-1.5 text-[11px] font-semibold text-zinc-500 uppercase tracking-widest hover:text-zinc-300 transition-colors w-full text-left"
                    >
                      <span className={`transition-transform duration-150 ${showFillerStats ? "rotate-90" : ""}`}>▸</span>
                      Top Filler Words
                    </button>
                    {showFillerStats && (
                      <div className="mt-2">
                        <FillerStatsChart entries={fillerStats} />
                      </div>
                    )}
                  </div>
                ) : null}
              </>
            ) : null}
          </div>
        )}
      </div>

      {/* ── Voice Notes Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
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

      {/* ── Integrations Panel (desktop only) ── */}
      {isDesktop && (
        <div
          className={[
            "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
            panels.showIntegrations ? "max-h-[600px] opacity-100 py-2" : "max-h-0 opacity-0 py-0",
          ].join(" ")}
        >
          {panels.showIntegrations && (
            <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
              <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
                <span className="text-[11px] font-semibold text-zinc-500 uppercase tracking-widest">Integrations</span>
                <button
                  onClick={() => panels.close("integrations")}
                  className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
                >
                  <CloseIcon />
                </button>
              </div>
              <div className="px-4 py-8 flex flex-col items-center gap-3 text-center">
                {!isPaid ? (
                  <>
                    <LockIcon className="w-5 h-5 text-zinc-600" />
                    <p className="text-sm font-medium text-zinc-400">Integrations require a Dikta license</p>
                    <p className="text-xs text-zinc-600 max-w-[240px]">Connect Dikta with Notion, Todoist, and more with a license key.</p>
                  </>
                ) : (
                  <>
                    <svg className="w-8 h-8 text-zinc-700 mb-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 2v6M12 16v6M4.93 4.93l4.24 4.24M14.83 14.83l4.24 4.24M2 12h6M16 12h6M4.93 19.07l4.24-4.24M14.83 9.17l4.24-4.24" />
                    </svg>
                    <p className="text-sm font-medium text-zinc-400">Integrations</p>
                    <p className="text-xs text-zinc-600 max-w-[220px]">Coming soon -- connect Dikta with Notion, Todoist, and more.</p>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      )}

      {/* ── Advanced Settings Panel ── */}
      <div
        className={[
          "px-4 overflow-hidden transition-all duration-250 ease-in-out flex-shrink-0",
          // Same 48 px nav-bar adjustment as the Settings panel above.
          panels.showAdvanced
            ? (isMobile ? "max-h-[calc(100vh-128px)] opacity-100 py-2" : "max-h-[calc(100vh-80px)] opacity-100 py-2")
            : "max-h-0 opacity-0 py-0",
        ].join(" ")}
      >
        {panels.showAdvanced && (
          <AdvancedSettingsPanel
            onClose={() => panels.close("advanced")}
            isPaid={isPaid}
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
            recording.recordingState === "error" ? "text-red-400"
              : recording.recordingState === "recording" ? "text-red-400"
              : recording.recordingState === "done" ? "text-emerald-400"
              : isBusy ? "text-amber-400"
              : "text-zinc-500",
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
              className="w-full bg-[#111113] border border-zinc-800/60 rounded-xl px-3.5 py-2.5 text-sm text-zinc-200 resize-none focus:outline-none focus:border-emerald-500/30 transition-colors"
            />
            {recording.recordingState === "done" && (
              <ReformatButtons
                text={recording.resultText}
                originalText={recording.originalResultText ?? recording.resultText}
                onResult={(t) => recording.setResultText(t)}
              />
            )}
            {recording.rawText && recording.rawText !== recording.resultText && (
              <div>
                <button
                  onClick={() => recording.setShowRawText((v) => !v)}
                  className="text-[11px] text-zinc-500 hover:text-zinc-300 transition-colors"
                >
                  {recording.showRawText ? "Hide original" : "Show original"}
                </button>
                {recording.showRawText && (
                  <div className="mt-1 relative group">
                    <textarea
                      readOnly
                      value={recording.rawText}
                      rows={2}
                      className="w-full bg-[#0c0c0e] border border-zinc-800/40 rounded-lg px-3 py-2 text-xs text-zinc-400 resize-none focus:outline-none"
                    />
                    <button
                      onClick={() => navigator.clipboard.writeText(recording.rawText!)}
                      className="absolute top-1.5 right-1.5 text-[11px] text-zinc-600 hover:text-zinc-300 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      Copy
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
          <span className="text-[11px] font-mono text-zinc-500">{hotkeyDisplay}</span>
        </div>
      )}

      {/* ── Preview-mode banner ── */}
      {isPreviewMode && (
        <div
          className="fixed bottom-3 right-3 z-50 pointer-events-none"
          aria-hidden="true"
        >
          <span className="px-2 py-1 rounded-md text-[10px] font-mono font-semibold tracking-wide bg-zinc-900/80 border border-zinc-700/50 text-zinc-500 backdrop-blur-sm">
            Preview Mode
          </span>
        </div>
      )}
    </main>
  );
}
