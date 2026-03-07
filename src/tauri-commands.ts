/**
 * Tauri IPC command wrappers.
 *
 * Each function maps to a Rust #[tauri::command] in src-tauri/src/lib.rs.
 * Parameter keys use snake_case to match Rust struct field names.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CleanupStyle, HotkeyMode, StopRecordingResult, AppSettings, StateChangedPayload, HistoryEntry, UsageSummary, AppProfile } from "./types";

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

export async function searchHistory(query: string, limit?: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("search_history", { query, limit: limit ?? null });
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

// --- Bar shape ---

export async function setBarShape(shape: "circle" | "pill"): Promise<void> {
  await invoke("set_bar_shape", { shape });
}

// --- Live preview ---

export async function transcribeLivePreview(): Promise<string> {
  return invoke<string>("transcribe_live_preview");
}
