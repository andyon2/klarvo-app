/**
 * Tauri IPC command stubs.
 *
 * Each function maps to a Rust #[tauri::command] in src-tauri/src/lib.rs.
 * The invoke calls are ready; the Rust implementations will be wired up
 * by the rust-core agent in a later phase.
 */
import { invoke } from "@tauri-apps/api/core";
import type { CleanupStyle, TranscriptionResult, CleanupResult } from "./types";

/**
 * Starts audio capture. Backend begins buffering microphone input.
 */
export async function startRecording(): Promise<void> {
  await invoke("start_recording");
}

/**
 * Stops audio capture and returns raw audio path or buffer handle.
 * Returns the raw transcription result after STT processing.
 */
export async function stopRecording(): Promise<TranscriptionResult> {
  return await invoke<TranscriptionResult>("stop_recording");
}

/**
 * Sends raw audio buffer to the configured STT engine (Groq or local Whisper).
 * Returns the raw transcript text.
 */
export async function transcribeAudio(): Promise<TranscriptionResult> {
  return await invoke<TranscriptionResult>("transcribe_audio");
}

/**
 * Sends raw transcript to the configured LLM (DeepSeek) for style-based cleanup.
 * @param rawText  - Raw transcript string from STT
 * @param style    - Cleanup mode: polished | verbatim | chat
 */
export async function cleanupText(
  rawText: string,
  style: CleanupStyle
): Promise<CleanupResult> {
  return await invoke<CleanupResult>("cleanup_text", { rawText, style });
}
