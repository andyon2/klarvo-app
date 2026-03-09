/**
 * Tauri IPC command wrappers.
 *
 * Each function maps to a Rust #[tauri::command] in src-tauri/src/lib.rs.
 * Parameter keys use snake_case to match Rust struct field names.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CleanupStyle, HotkeyMode, StopRecordingResult, AppSettings, StateChangedPayload, HistoryEntry, UsageSummary, AppProfile, AdvancedSettings } from "./types";

/**
 * Starts audio capture. Backend begins buffering microphone input.
 */
export async function startRecording(): Promise<void> {
  await invoke("start_recording");
}

/**
 * Stops audio capture and saves the recorded audio internally.
 * Returns the duration of the recording in milliseconds.
 */
export async function stopRecording(): Promise<StopRecordingResult> {
  return await invoke<StopRecordingResult>("stop_recording");
}

/**
 * Cancels the active recording and discards captured audio.
 * Emits state=idle so all windows return to dormant state.
 */
export async function cancelRecording(): Promise<void> {
  await invoke("cancel_recording");
}

/**
 * Transcribes the last saved audio buffer via the configured STT engine.
 * @param language - BCP-47 language code, e.g. "de" or "en"
 */
export async function transcribeAudio(language: string): Promise<string> {
  return await invoke<string>("transcribe_audio", { language });
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
  return await invoke<string>("cleanup_text", { rawText, style });
}

/**
 * Returns current settings. API keys are masked (e.g. "****abcd").
 */
export async function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
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
  sttPriority?: string[] | null,
  llmPriority?: string[] | null,
  outputLanguage?: string | null,
  webhookUrl?: string | null,
  tursoUrl?: string | null,
  tursoToken?: string | null,
  bubbleSize?: number | null,
  bubbleOpacity?: number | null,
): Promise<void> {
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
    sttPriority: sttPriority ?? null,
    llmPriority: llmPriority ?? null,
    outputLanguage: outputLanguage ?? null,
    webhookUrl: webhookUrl ?? null,
    tursoUrl: tursoUrl ?? null,
    tursoToken: tursoToken ?? null,
    bubbleSize: bubbleSize ?? null,
    bubbleOpacity: bubbleOpacity ?? null,
  });
}

/**
 * Re-registers the global hotkey at the OS level with a new shortcut and/or mode.
 * Call after save_settings to apply the new binding immediately without restart.
 * @param shortcut - Tauri shortcut string, e.g. "ctrl+shift+d"
 * @param mode     - Activation mode: hold | toggle
 */
export async function setHotkey(shortcut: string, mode: HotkeyMode): Promise<void> {
  await invoke("set_hotkey", { shortcut, mode });
}

/**
 * Returns all custom dictionary terms.
 */
export async function getDictionaryTerms(): Promise<string[]> {
  return invoke<string[]>("get_dictionary_terms");
}

/**
 * Adds a term to the custom dictionary.
 * @param term - The word or phrase to add
 */
export async function addDictionaryTerm(term: string): Promise<void> {
  await invoke("add_dictionary_term", { term });
}

/**
 * Removes a term from the custom dictionary.
 * @param term - The exact word or phrase to remove
 */
export async function removeDictionaryTerm(term: string): Promise<void> {
  await invoke("remove_dictionary_term", { term });
}

/**
 * Subscribes to backend pipeline state changes triggered by the global hotkey.
 * Returns a promise that resolves to an unlisten function -- call it on cleanup.
 *
 * The backend emits "dikta://state-changed" at every pipeline step:
 * recording -> transcribing -> cleaning -> done | error
 */
export function onStateChanged(
  callback: (payload: StateChangedPayload) => void
): Promise<() => void> {
  return listen<StateChangedPayload>("dikta://state-changed", (event) => {
    callback(event.payload);
  });
}

/**
 * Tells the backend which language to use for the hotkey-triggered pipeline.
 * Must be called whenever the user changes the language setting.
 * @param language - BCP-47 language code, e.g. "de" or "en"
 */
export async function setLanguage(language: string): Promise<void> {
  await invoke("set_language", { language });
}

/**
 * Tells the backend which cleanup style to use for the hotkey-triggered pipeline.
 * Must be called whenever the user changes the style setting.
 * @param style - Cleanup mode: polished | verbatim | chat
 */
export async function setCleanupStyle(style: CleanupStyle): Promise<void> {
  await invoke("set_cleanup_style", { style });
}

/**
 * Returns the names of all available audio input devices.
 */
export async function listAudioDevices(): Promise<string[]> {
  return invoke<string[]>("list_audio_devices");
}

// --- History ---

export async function getHistory(limit?: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("get_history", { limit: limit ?? null });
}

export async function searchHistory(textQuery?: string, appQuery?: string, limit?: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("search_history", {
    textQuery: textQuery || null,
    appQuery: appQuery || null,
    limit: limit ?? null,
  });
}

export async function deleteHistoryEntry(id: number): Promise<void> {
  await invoke("delete_history_entry", { id });
}

export async function clearHistory(): Promise<number> {
  return invoke<number>("clear_history");
}

export async function addHistoryEntry(
  text: string,
  rawText: string | null,
  style: string,
  language: string,
): Promise<number> {
  return invoke<number>("add_history_entry", { text, rawText, style, language });
}

// --- Stats ---

export async function getUsageStats(): Promise<UsageSummary> {
  return invoke<UsageSummary>("get_usage_stats");
}

// --- Profiles ---

export async function getProfiles(): Promise<AppProfile[]> {
  return invoke<AppProfile[]>("get_profiles");
}

export async function saveProfiles(profiles: AppProfile[]): Promise<void> {
  await invoke("save_profiles", { profiles });
}

// --- Output language ---

/**
 * Tells the backend which output language to translate dictations into.
 * Empty string means no translation.
 * @param language - BCP-47 language code, e.g. "en" or "de", or "" to disable
 */
export async function setOutputLanguage(language: string): Promise<void> {
  await invoke("set_output_language", { language });
}

/**
 * Reformats an existing text into a different format via the configured LLM.
 * @param text   - The text to reformat
 * @param format - Target format: "email" | "bullets" | "summary"
 */
export async function reformatText(text: string, format: string): Promise<string> {
  return invoke<string>("reformat_text", { text, format });
}

/**
 * Returns the top filler words detected across all dictations.
 * Each entry contains the word and its occurrence count.
 */
export async function getFillerStats(): Promise<{ word: string; count: number }[]> {
  return invoke<{ word: string; count: number }[]>("get_filler_stats");
}

// --- Bar shape ---

export async function setBarShape(shape: "idle" | "pill"): Promise<void> {
  await invoke("set_bar_shape", { shape });
}

// --- Live preview ---

export async function transcribeLivePreview(): Promise<string> {
  return invoke<string>("transcribe_live_preview");
}

// --- Window context ---

/**
 * Returns the title of the currently focused window, or null when no window
 * has focus or on non-Windows platforms.
 */
export async function getActiveApp(): Promise<string | null> {
  return invoke<string | null>("get_active_app");
}

// --- Voice Notes ---

export async function getNotes(limit: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("get_notes", { limit });
}

export async function saveNote(text: string, rawText: string, style: string): Promise<number> {
  return invoke<number>("save_note", { text, rawText, style });
}

// --- Snippets ---

export interface TextSnippet {
  name: string;
  content: string;
}

export async function getSnippets(): Promise<TextSnippet[]> {
  return invoke<TextSnippet[]>("get_snippets");
}

export async function saveSnippets(snippets: TextSnippet[]): Promise<void> {
  return invoke<void>("save_snippets", { snippets });
}

export async function pasteSnippet(content: string): Promise<void> {
  return invoke<void>("paste_snippet", { content });
}

// --- Advanced Settings ---

/**
 * Returns the current advanced (power-user) settings.
 */
export async function getAdvancedSettings(): Promise<AdvancedSettings> {
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
  audioData: number[],
  language: string,
): Promise<string> {
  return invoke<string>("transcribe_audio_bytes", { audioData, language });
}

/**
 * Persists advanced settings to disk.
 * @param settings - Full AdvancedSettings object
 */
export async function saveAdvancedSettings(settings: AdvancedSettings): Promise<void> {
  await invoke("save_advanced_settings", { settings });
}

// --- Sync ---

/**
 * Syncs history with the remote Turso database.
 * Returns [pushed, pulled] counts. Returns [0, 0] if not configured.
 */
export async function syncHistory(): Promise<[number, number]> {
  return invoke<[number, number]>("sync_history");
}

// --- License ---

/**
 * Validates a license key. Returns the raw status string from the backend:
 * "licensed" | "grace_period:{timestamp}" | error string
 * @param key - License key in DIKTA-XXXX-XXXX-XXXX-XXXX format
 */
export async function validateLicense(key: string): Promise<string> {
  return invoke<string>("validate_license", { key });
}

/**
 * Returns the current license status as a raw string:
 * "licensed" | "grace_period:{timestamp}" | "unlicensed"
 */
export async function getLicenseStatus(): Promise<string> {
  return invoke<string>("get_license_status");
}

/**
 * Removes the stored license key and resets to unlicensed state.
 */
export async function removeLicense(): Promise<void> {
  await invoke("remove_license");
}
