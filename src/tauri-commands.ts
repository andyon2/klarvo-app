/**
 * Tauri IPC command wrappers.
 *
 * Each function maps to a Rust #[tauri::command] in src-tauri/src/lib.rs.
 * Parameter keys use snake_case to match Rust struct field names.
 *
 * Preview mode: when window.__TAURI_INTERNALS__ is absent (plain browser /
 * `npm run preview`), all exports return mock data so the UI can be developed
 * and inspected without a running Tauri backend.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type { CleanupStyle, HotkeyMode, StopRecordingResult, AppSettings, StateChangedPayload, HistoryEntry, UsageSummary, AppProfile, AdvancedSettings, OnboardingState } from "./types";

// ---------------------------------------------------------------------------
// Preview-mode detection
// ---------------------------------------------------------------------------

/** True when running in a plain browser without a Tauri backend. */
export const isPreviewMode = typeof window !== "undefined" &&
  !(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;

// Safe wrappers that no-op in preview mode.
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isPreviewMode) {
    return Promise.reject(new Error(`[preview] invoke("${cmd}") not available`));
  }
  return tauriInvoke<T>(cmd, args);
}

function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  if (isPreviewMode) {
    return Promise.resolve(() => { /* no-op unlisten */ });
  }
  return tauriListen<T>(event, handler);
}

// ---------------------------------------------------------------------------
// Mock data for preview mode
// ---------------------------------------------------------------------------

const MOCK_SETTINGS: AppSettings = {
  groqApiKeyMasked: "****demo",
  deepseekApiKeyMasked: "****demo",
  language: "de",
  cleanupStyle: "polished",
  hotkey: "ctrl+shift+d",
  hotkeyMode: "hold",
  audioDevice: null,
  sttModel: "whisper-large-v3-turbo",
  customPrompt: "",
  autostart: false,
  whisperMode: false,
  openaiApiKeyMasked: "",
  anthropicApiKeyMasked: "",
  openrouterApiKeyMasked: "",
  sttProvider: "groq",
  llmProvider: "deepseek",
  outputLanguage: "",
  webhookUrl: "",
  tursoUrl: "",
  tursoTokenMasked: "",
  deviceId: "preview-device-0000",
  bubbleSize: 56,
  bubbleOpacity: 90,
  localWhisperModel: "small",
  localWhisperGpu: false,
  insertAndSendSlot1: false,
  insertAndSendSlot2: false,
  autostopSilenceSecs: 2.0,
  autoModeSilenceSecs: 2.0,
  hotkeySlot2: "",
  hotkeyModeSlot2: "hold",
  bubbleTapMode: "toggle",
  bubbleTapAutoSend: false,
  bubbleTapSilenceSecs: 2.0,
  bubbleLongPressMode: "hold",
  bubbleLongPressAutoSend: false,
  bubbleLongPressSilenceSecs: 2.0,
};

const MOCK_ADVANCED_SETTINGS: AdvancedSettings = {
  sttPromptDe: "",
  sttPromptEn: "",
  sttPromptAuto: "",
  sttTemperature: 0,
  llmSystemPromptPolished: "",
  llmSystemPromptVerbatim: "",
  llmSystemPromptChat: "",
  llmCommandModePrompt: "",
  llmTemperature: 0.3,
  llmMaxTokens: 1024,
  llmModelDeepseek: "deepseek-chat",
  llmModelOpenai: "gpt-4o-mini",
  llmModelAnthropic: "claude-haiku-20240307",
  llmModelGroq: "llama-3.3-70b-versatile",
  chunkThreshold: 120,
  chunkTargetSize: 90,
  silenceThreshold: 0.01,
  whisperModeThreshold: 0.005,
  minRecordingMs: 400,
  whisperModeGain: 3.0,
  autoPaste: true,
  pasteDelayMs: 80,
  autoCapitalize: true,
  webhookHeaders: "{}",
  webhookTimeoutSecs: 10,
  logLevel: "info",
  uiScale: "normal",
};

const MOCK_HISTORY: HistoryEntry[] = [
  {
    id: 1,
    text: "Das ist ein Demo-Diktat aus dem Preview-Modus. Hier steht bereinigter Text.",
    rawText: "äh das ist ein demo diktat aus dem preview modus hier steht bereinigter text",
    style: "polished",
    language: "de",
    appName: "VS Code",
    createdAt: new Date(Date.now() - 1000 * 60 * 5).toISOString().replace("T", " ").slice(0, 19),
  },
  {
    id: 2,
    text: "Meeting-Notizen: Nächste Woche Release-Planning, Andy koordiniert das Team.",
    rawText: "meeting notizen nächste woche release planning andy koordiniert das team",
    style: "chat",
    language: "de",
    appName: "Notion",
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString().replace("T", " ").slice(0, 19),
  },
  {
    id: 3,
    text: "Reminder: Tauri v2 supports both desktop and Android with a single codebase.",
    rawText: "reminder tauri v2 supports both desktop and android with a single codebase",
    style: "verbatim",
    language: "en",
    appName: null,
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString().replace("T", " ").slice(0, 19),
  },
];

const MOCK_USAGE: UsageSummary = {
  totalDictations: 10,
  totalWords: 847,
  totalCostUsd: 0.0012,
  totalAudioSeconds: 183,
  totalSttCostUsd: 0.0009,
  totalLlmCostUsd: 0.0003,
  dictationsToday: 3,
  costTodayUsd: 0.0004,
};

// ---------------------------------------------------------------------------
// Preview-mode implementations (bypass invoke / listen entirely)
// ---------------------------------------------------------------------------

function mockAsync<T>(value: T, delayMs = 0): Promise<T> {
  return new Promise((res) => setTimeout(() => res(value), delayMs));
}

/** Returns a no-op unlisten function immediately. */
function mockListen(): Promise<() => void> {
  return Promise.resolve(() => { /* no-op */ });
}

/**
 * Starts audio capture. Backend begins buffering microphone input.
 */
export async function startRecording(): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("start_recording");
}

/**
 * Stops audio capture and saves the recorded audio internally.
 * Returns the duration of the recording in milliseconds.
 */
export async function stopRecording(): Promise<StopRecordingResult> {
  if (isPreviewMode) return mockAsync({ durationMs: 1500 });
  return await invoke<StopRecordingResult>("stop_recording");
}

/**
 * Cancels the active recording and discards captured audio.
 * Emits state=idle so all windows return to dormant state.
 */
export async function cancelRecording(): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("cancel_recording");
}

/**
 * Transcribes the last saved audio buffer via the configured STT engine.
 * @param language - BCP-47 language code, e.g. "de" or "en"
 */
export async function transcribeAudio(_language: string): Promise<string> {
  if (isPreviewMode) return mockAsync("Das ist ein Demo-Transkript aus dem Preview-Modus.", 800);
  return await invoke<string>("transcribe_audio", { language: _language });
}

/**
 * Sends raw transcript to the configured LLM (DeepSeek) for style-based cleanup.
 * @param rawText - Raw transcript string from STT
 * @param style   - Cleanup mode: polished | verbatim | chat
 */
export async function cleanupText(
  rawText: string,
  style: CleanupStyle
): Promise<string> {
  if (isPreviewMode) return mockAsync(`${rawText} (cleaned)`, 600);
  return await invoke<string>("cleanup_text", { rawText, style });
}

/**
 * Returns current settings. API keys are masked (e.g. "****abcd").
 */
export async function getSettings(): Promise<AppSettings> {
  if (isPreviewMode) return mockAsync({ ...MOCK_SETTINGS });
  return invoke<AppSettings>("get_settings");
}

/**
 * Checks whether this is the first run (no settings saved yet).
 */
export async function isFirstRun(): Promise<boolean> {
  if (isPreviewMode) return mockAsync(false);
  return invoke<boolean>("is_first_run");
}

/**
 * Persists settings to config.json on disk. Empty API key strings are ignored
 * by the backend (existing key is kept unchanged).
 * @param groqApiKey     - Groq API key, or "" to keep existing
 * @param deepseekApiKey - DeepSeek API key, or "" to keep existing
 * @param language       - BCP-47 language code, e.g. "de" or "en"
 * @param cleanupStyle   - Cleanup mode: polished | verbatim | chat
 * @param hotkey         - Tauri shortcut string, e.g. "ctrl+shift+d"
 * @param hotkeyMode     - Activation mode: hold | toggle
 */
export async function saveSettings(
  groqApiKey: string,
  deepseekApiKey: string,
  language: string,
  cleanupStyle: CleanupStyle,
  hotkey?: string,
  hotkeyMode?: HotkeyMode,
  audioDevice?: string | null,
  sttModel?: string | null,
  customPrompt?: string | null,
  autostart?: boolean | null,
  whisperMode?: boolean | null,
  openaiApiKey?: string | null,
  anthropicApiKey?: string | null,
  openrouterApiKey?: string | null,
  /** @deprecated Use sttProvider instead. */
  sttPriority?: string[] | null,
  /** @deprecated Use llmProvider instead. */
  llmPriority?: string[] | null,
  outputLanguage?: string | null,
  webhookUrl?: string | null,
  tursoUrl?: string | null,
  tursoToken?: string | null,
  bubbleSize?: number | null,
  bubbleOpacity?: number | null,
  localWhisperModel?: string | null,
  localWhisperGpu?: boolean | null,
  sttProvider?: string | null,
  llmProvider?: string | null,
  insertAndSendSlot1?: boolean | null,
  autostopSilenceSecs?: number | null,
  autoModeSilenceSecs?: number | null,
  hotkey_slot2?: string | null,
  hotkey_mode_slot2?: HotkeyMode | null,
  insertAndSendSlot2?: boolean | null,
  bubbleTapMode?: string | null,
  bubbleTapAutoSend?: boolean | null,
  bubbleTapSilenceSecs?: number | null,
  bubbleLongPressMode?: string | null,
  bubbleLongPressAutoSend?: boolean | null,
  bubbleLongPressSilenceSecs?: number | null,
): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("save_settings", {
    groqApiKey,
    deepseekApiKey,
    language,
    cleanupStyle,
    hotkey: hotkey ?? "",
    hotkeyMode: hotkeyMode ?? "hold",
    audioDevice: audioDevice ?? null,
    sttModel: sttModel ?? null,
    customPrompt: customPrompt ?? null,
    autostart: autostart ?? null,
    whisperMode: whisperMode ?? null,
    openaiApiKey: openaiApiKey ?? null,
    anthropicApiKey: anthropicApiKey ?? null,
    openrouterApiKey: openrouterApiKey ?? null,
    sttPriority: sttPriority ?? null,
    llmPriority: llmPriority ?? null,
    outputLanguage: outputLanguage ?? null,
    webhookUrl: webhookUrl ?? null,
    tursoUrl: tursoUrl ?? null,
    tursoToken: tursoToken ?? null,
    bubbleSize: bubbleSize ?? null,
    bubbleOpacity: bubbleOpacity ?? null,
    localWhisperModel: localWhisperModel ?? null,
    localWhisperGpu: localWhisperGpu ?? null,
    sttProvider: sttProvider ?? null,
    llmProvider: llmProvider ?? null,
    insertAndSendSlot1: insertAndSendSlot1 ?? null,
    autostopSilenceSecs: autostopSilenceSecs ?? null,
    autoModeSilenceSecs: autoModeSilenceSecs ?? null,
    hotkeySlot2: hotkey_slot2 ?? null,
    hotkeyModeSlot2: hotkey_mode_slot2 ?? null,
    insertAndSendSlot2: insertAndSendSlot2 ?? null,
    bubbleTapMode: bubbleTapMode ?? null,
    bubbleTapAutoSend: bubbleTapAutoSend ?? null,
    bubbleTapSilenceSecs: bubbleTapSilenceSecs ?? null,
    bubbleLongPressMode: bubbleLongPressMode ?? null,
    bubbleLongPressAutoSend: bubbleLongPressAutoSend ?? null,
    bubbleLongPressSilenceSecs: bubbleLongPressSilenceSecs ?? null,
  });
}

/**
 * Re-registers the global hotkey at the OS level with a new shortcut and/or mode.
 * Call after save_settings to apply the new binding immediately without restart.
 * @param shortcut - Tauri shortcut string, e.g. "ctrl+shift+d"
 * @param mode     - Activation mode: hold | toggle
 */
export async function setHotkey(shortcut: string, mode: HotkeyMode): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_hotkey", { shortcut, mode });
}

/**
 * Returns all custom dictionary terms.
 */
export async function getDictionaryTerms(): Promise<string[]> {
  if (isPreviewMode) return mockAsync(["Dikta", "Tauri", "Whisper"]);
  return invoke<string[]>("get_dictionary_terms");
}

/**
 * Adds a term to the custom dictionary.
 * @param term - The word or phrase to add
 */
export async function addDictionaryTerm(_term: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("add_dictionary_term", { term: _term });
}

/**
 * Removes a term from the custom dictionary.
 * @param term - The exact word or phrase to remove
 */
export async function removeDictionaryTerm(_term: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("remove_dictionary_term", { term: _term });
}

/**
 * Subscribes to backend pipeline state changes triggered by the global hotkey.
 * Returns a promise that resolves to an unlisten function -- call it on cleanup.
 *
 * The backend emits "dikta://state-changed" at every pipeline step:
 * recording -> transcribing -> cleaning -> done | error
 */
export function onStateChanged(
  _callback: (payload: StateChangedPayload) => void
): Promise<() => void> {
  if (isPreviewMode) return mockListen();
  return listen<StateChangedPayload>("dikta://state-changed", (event) => {
    _callback(event.payload);
  });
}

/**
 * Tells the backend which language to use for the hotkey-triggered pipeline.
 * Must be called whenever the user changes the language setting.
 * @param language - BCP-47 language code, e.g. "de" or "en"
 */
export async function setLanguage(_language: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_language", { language: _language });
}

/**
 * Tells the backend which cleanup style to use for the hotkey-triggered pipeline.
 * Must be called whenever the user changes the style setting.
 * @param style - Cleanup mode: polished | verbatim | chat
 */
export async function setCleanupStyle(_style: CleanupStyle): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_cleanup_style", { style: _style });
}

/**
 * Returns the names of all available audio input devices.
 */
export async function listAudioDevices(): Promise<string[]> {
  if (isPreviewMode) return mockAsync(["Default Microphone", "USB Headset"]);
  return invoke<string[]>("list_audio_devices");
}

// --- History ---

export async function getHistory(limit?: number): Promise<HistoryEntry[]> {
  if (isPreviewMode) return mockAsync(MOCK_HISTORY.slice(0, limit ?? 50));
  return invoke<HistoryEntry[]>("get_history", { limit: limit ?? null });
}

export async function searchHistory(textQuery?: string, appQuery?: string, limit?: number): Promise<HistoryEntry[]> {
  if (isPreviewMode) {
    const q = (textQuery ?? "").toLowerCase();
    const a = (appQuery ?? "").toLowerCase();
    return mockAsync(
      MOCK_HISTORY.filter(
        (e) =>
          (!q || e.text.toLowerCase().includes(q)) &&
          (!a || (e.appName ?? "").toLowerCase().includes(a)),
      ).slice(0, limit ?? 50),
    );
  }
  return invoke<HistoryEntry[]>("search_history", {
    textQuery: textQuery || null,
    appQuery: appQuery || null,
    limit: limit ?? null,
  });
}

export async function deleteHistoryEntry(_id: number): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("delete_history_entry", { id: _id });
}

export async function clearHistory(): Promise<number> {
  if (isPreviewMode) return mockAsync(0);
  return invoke<number>("clear_history");
}

export async function addHistoryEntry(
  text: string,
  rawText: string | null,
  style: string,
  language: string,
): Promise<number> {
  if (isPreviewMode) return mockAsync(0);
  return invoke<number>("add_history_entry", { text, rawText, style, language });
}

// --- Stats ---

export async function getUsageStats(): Promise<UsageSummary> {
  if (isPreviewMode) return mockAsync({ ...MOCK_USAGE });
  return invoke<UsageSummary>("get_usage_stats");
}

// --- Profiles ---

export async function getProfiles(): Promise<AppProfile[]> {
  if (isPreviewMode) return mockAsync([]);
  return invoke<AppProfile[]>("get_profiles");
}

export async function saveProfiles(_profiles: AppProfile[]): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("save_profiles", { profiles: _profiles });
}

// --- Output language ---

/**
 * Tells the backend which output language to translate dictations into.
 * Empty string means no translation.
 * @param language - BCP-47 language code, e.g. "en" or "de", or "" to disable
 */
export async function setOutputLanguage(_language: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_output_language", { language: _language });
}

/**
 * Reformats an existing text into a different format via the configured LLM.
 * @param text   - The text to reformat
 * @param format - Target format: "email" | "bullets" | "summary"
 */
export async function reformatText(text: string, _format: string): Promise<string> {
  if (isPreviewMode) return mockAsync(`${text} [reformatted]`, 700);
  return invoke<string>("reformat_text", { text, format: _format });
}

/**
 * Returns the top filler words detected across all dictations.
 * Each entry contains the word and its occurrence count.
 */
export async function getFillerStats(): Promise<{ word: string; count: number }[]> {
  if (isPreviewMode) return mockAsync([{ word: "ähm", count: 5 }, { word: "quasi", count: 3 }]);
  return invoke<{ word: string; count: number }[]>("get_filler_stats");
}

// --- Bar shape ---

export async function setBarShape(_shape: "idle" | "pill"): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_bar_shape", { shape: _shape });
}

// --- Live preview ---

export async function transcribeLivePreview(): Promise<string> {
  if (isPreviewMode) return mockAsync("Live preview...", 300);
  return invoke<string>("transcribe_live_preview");
}

// --- Window context ---

/**
 * Returns the title of the currently focused window, or null when no window
 * has focus or on non-Windows platforms.
 */
export async function getActiveApp(): Promise<string | null> {
  if (isPreviewMode) return mockAsync("VS Code");
  return invoke<string | null>("get_active_app");
}

// --- Voice Notes ---

export async function getNotes(limit: number): Promise<HistoryEntry[]> {
  if (isPreviewMode) return mockAsync(MOCK_HISTORY.filter((e) => e.isNote).slice(0, limit));
  return invoke<HistoryEntry[]>("get_notes", { limit });
}

export async function saveNote(_text: string, _rawText: string, _style: string): Promise<number> {
  if (isPreviewMode) return mockAsync(0);
  return invoke<number>("save_note", { text: _text, rawText: _rawText, style: _style });
}

// --- Snippets ---

export interface TextSnippet {
  name: string;
  content: string;
}

export async function getSnippets(): Promise<TextSnippet[]> {
  if (isPreviewMode) return mockAsync([]);
  return invoke<TextSnippet[]>("get_snippets");
}

export async function saveSnippets(_snippets: TextSnippet[]): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  return invoke<void>("save_snippets", { snippets: _snippets });
}

export async function pasteSnippet(_content: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  return invoke<void>("paste_snippet", { content: _content });
}

// --- Advanced Settings ---

/**
 * Returns the current advanced (power-user) settings.
 */
export async function getAdvancedSettings(): Promise<AdvancedSettings> {
  if (isPreviewMode) return mockAsync({ ...MOCK_ADVANCED_SETTINGS });
  return invoke<AdvancedSettings>("get_advanced_settings");
}

// --- Mobile audio ---

/**
 * Transcribes audio from raw WAV bytes (used on Android where cpal is not
 * available). The frontend captures audio via MediaRecorder, encodes it as a
 * WAV, and sends the bytes here. The backend feeds them into the same STT
 * pipeline used by the desktop hotkey flow.
 *
 * @param audioData - WAV file contents as a plain number[] (JS Array of u8)
 * @param language  - BCP-47 language code ("de", "en", "") -- "" means auto
 */
export async function transcribeAudioBytes(
  _audioData: number[],
  _language: string,
): Promise<string> {
  if (isPreviewMode) return mockAsync("Das ist ein Demo-Transkript aus dem Preview-Modus.", 900);
  return invoke<string>("transcribe_audio_bytes", { audioData: _audioData, language: _language });
}

/**
 * Persists advanced settings to disk.
 * @param settings - Full AdvancedSettings object
 */
export async function saveAdvancedSettings(_settings: AdvancedSettings): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("save_advanced_settings", { settings: _settings });
}

// --- Sync ---

/**
 * Syncs history with the remote Turso database.
 * Returns [pushed, pulled] counts. Returns [0, 0] if not configured.
 */
export async function syncHistory(): Promise<[number, number]> {
  if (isPreviewMode) return mockAsync([0, 0] as [number, number]);
  return invoke<[number, number]>("sync_history");
}

// --- Whisper Model Management (Desktop only) ---

export interface WhisperModelWithStatus {
  id: string;
  filename: string;
  sizeBytes: number;
  description: string;
  status: "downloaded" | "notDownloaded";
}

export interface ModelDownloadProgressPayload {
  modelId: string;
  bytesReceived: number;
  totalBytes: number;
}

export interface ModelDownloadCompletePayload {
  modelId: string;
}

export interface ModelDownloadErrorPayload {
  modelId: string;
  error: string;
}

/**
 * Returns all available whisper models with their download status.
 */
export async function getWhisperModels(): Promise<WhisperModelWithStatus[]> {
  if (isPreviewMode) {
    return mockAsync([
      { id: "small", filename: "ggml-small.bin", sizeBytes: 466_000_000, description: "Small (466 MB) — fast, good quality", status: "downloaded" as const },
      { id: "medium", filename: "ggml-medium.bin", sizeBytes: 1_500_000_000, description: "Medium (1.5 GB) — slower, better quality", status: "notDownloaded" as const },
    ]);
  }
  return invoke<WhisperModelWithStatus[]>("get_whisper_models");
}

/**
 * Starts downloading a whisper model in the background.
 * Progress is reported via dikta://model-download-progress events.
 * @param modelId - Model identifier, e.g. "base"
 */
export async function downloadWhisperModel(_modelId: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("download_whisper_model", { modelId: _modelId });
}

/**
 * Deletes a downloaded whisper model from disk.
 * @param modelId - Model identifier, e.g. "base"
 */
export async function deleteWhisperModel(_modelId: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("delete_whisper_model", { modelId: _modelId });
}

/**
 * Subscribes to model download progress events.
 * Returns an unlisten function -- call it on cleanup.
 */
export function onModelDownloadProgress(
  _callback: (payload: ModelDownloadProgressPayload) => void
): Promise<() => void> {
  if (isPreviewMode) return mockListen();
  return listen<ModelDownloadProgressPayload>("dikta://model-download-progress", (e) => {
    _callback(e.payload);
  });
}

/**
 * Subscribes to model download complete events.
 * Returns an unlisten function -- call it on cleanup.
 */
export function onModelDownloadComplete(
  _callback: (payload: ModelDownloadCompletePayload) => void
): Promise<() => void> {
  if (isPreviewMode) return mockListen();
  return listen<ModelDownloadCompletePayload>("dikta://model-download-complete", (e) => {
    _callback(e.payload);
  });
}

/**
 * Subscribes to model download error events.
 * Returns an unlisten function -- call it on cleanup.
 */
export function onModelDownloadError(
  _callback: (payload: ModelDownloadErrorPayload) => void
): Promise<() => void> {
  if (isPreviewMode) return mockListen();
  return listen<ModelDownloadErrorPayload>("dikta://model-download-error", (e) => {
    _callback(e.payload);
  });
}

// --- License ---

/**
 * Validates a license key. Returns the raw status string from the backend:
 * "licensed" | "grace_period:{timestamp}" | error string
 * @param key - License key in DIKTA-XXXX-XXXX-XXXX-XXXX format
 */
export async function validateLicense(_key: string): Promise<string> {
  if (isPreviewMode) return mockAsync("licensed");
  return invoke<string>("validate_license", { key: _key });
}

/**
 * Returns the current license status as a raw string:
 * "licensed" | "grace_period:{timestamp}" | "unlicensed"
 */
export async function getLicenseStatus(): Promise<string> {
  // trial:9999999999 means "trial that never expires" -- all paid features visible in preview.
  if (isPreviewMode) return mockAsync("trial:9999999999");
  return invoke<string>("get_license_status");
}

/**
 * Removes the stored license key and resets to unlicensed state.
 */
export async function removeLicense(): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("remove_license");
}

// --- Bar position ---

/**
 * Persists the floating bar window position (logical pixels) to disk.
 * Called after every drag-end so the bar reopens at the same spot.
 * @param x - Logical x coordinate of the window's top-left corner
 * @param y - Logical y coordinate of the window's top-left corner
 */
export async function saveBarPosition(x: number, y: number): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("save_bar_position", { x, y });
}

/**
 * Returns the last persisted bar position, or null when no position has been
 * saved yet (first run or after a reset).
 */
export async function getBarPosition(): Promise<{ x: number; y: number } | null> {
  if (isPreviewMode) return mockAsync(null);
  return invoke<{ x: number; y: number } | null>("get_bar_position");
}

// --- Onboarding ---

const MOCK_ONBOARDING_STATE: OnboardingState = {
  completed: false,
  skipped: false,
  currentStep: 0,
  mode: "",
  language: "",
};

/**
 * Returns the current onboarding wizard state from config.json.
 * Defaults to a fresh state (completed=false, currentStep=0) when not set yet.
 */
export async function getOnboardingState(): Promise<OnboardingState> {
  if (isPreviewMode) return mockAsync({ ...MOCK_ONBOARDING_STATE });
  return invoke<OnboardingState>("get_onboarding_state");
}

/**
 * Persists the onboarding wizard state to config.json.
 * Called on every step transition and on skip/complete.
 */
export async function setOnboardingState(state: OnboardingState): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("set_onboarding_state", { state });
}

/**
 * Validates an API key against the provider's endpoint.
 * Returns true when the key is valid (HTTP 200), false on 401/403,
 * and throws on network errors.
 * @param provider - "groq" | "deepseek" | "openrouter" | "openai"
 * @param key      - The API key to validate
 */
export async function validateApiKey(provider: string, key: string): Promise<boolean> {
  if (isPreviewMode) return mockAsync(key.length > 10, 600);
  return invoke<boolean>("validate_api_key", { provider, key });
}

// --- Tips ---

/**
 * Returns true when the given tip has already been shown to this user
 * (stored in the tips_shown SQLite table by the backend).
 * @param tipId - Unique tip identifier, e.g. "cleanup-styles"
 */
export async function isTipShown(tipId: string): Promise<boolean> {
  if (isPreviewMode) return mockAsync(false);
  return invoke<boolean>("is_tip_shown", { tipId });
}

/**
 * Marks a tip as shown so it is never displayed again.
 * Idempotent: calling it multiple times for the same tipId is safe.
 * @param tipId - Unique tip identifier, e.g. "cleanup-styles"
 */
export async function markTipShown(tipId: string): Promise<void> {
  if (isPreviewMode) return mockAsync(undefined);
  await invoke("mark_tip_shown", { tipId });
}
