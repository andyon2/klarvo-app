/**
 * Tauri IPC command wrappers.
 *
 * Each function maps to a Rust #[tauri::command] in src-tauri/src/lib.rs.
 * Parameter keys use snake_case to match Rust struct field names.
 */
import { invoke } from "@tauri-apps/api/core";
import type { CleanupStyle, StopRecordingResult, ApiKeyStatus } from "./types";

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
  return await invoke<string>("cleanup_text", { raw_text: rawText, style });
}

/**
 * Returns whether each API key is currently configured in the backend.
 */
export async function getApiKeyStatus(): Promise<ApiKeyStatus> {
  return await invoke<ApiKeyStatus>("get_api_key_status");
}

/**
 * Persists API keys in the backend (stored in system keystore, never on disk).
 * Pass undefined to leave an existing key unchanged.
 * @param groqApiKey     - Groq API key (optional)
 * @param deepseekApiKey - DeepSeek API key (optional)
 */
export async function updateApiKeys(
  groqApiKey?: string,
  deepseekApiKey?: string
): Promise<void> {
  await invoke("update_api_keys", {
    groq_api_key: groqApiKey ?? null,
    deepseek_api_key: deepseekApiKey ?? null,
  });
}
