// Cleanup style determines how the LLM processes raw transcription output.
export type CleanupStyle = "polished" | "verbatim" | "chat";

// Hotkey activation mode.
// hold     = push-to-talk: record while key is held, release to process.
// toggle   = press to start, press again to stop and process.
// autostop = like toggle, but stops automatically after configurable silence.
// auto     = continuous dictation: each silence gap triggers a transcription cycle.
export type HotkeyMode = "toggle" | "hold" | "autostop" | "auto";

// Payload emitted by the backend on every state transition of the hotkey pipeline.
export interface StateChangedPayload {
  state: "recording" | "transcribing" | "cleaning" | "done" | "idle" | "error";
  text?: string;           // present when state === "done": cleaned result text
  rawText?: string;        // present when state === "done": raw transcript before cleanup
  error?: string;          // present when state === "error": human-readable message
  clipboardOnly?: boolean; // present when state === "done": true when focus-restore failed and only clipboard was written
}

// Recording state machine states.
export type RecordingState = "idle" | "recording" | "transcribing" | "cleaning" | "done" | "error";

// Result returned from stop_recording Tauri command.
export interface StopRecordingResult {
  durationMs: number;
}

// Full settings object returned by the backend (API keys are masked: "****abcd").
export interface AppSettings {
  groqApiKeyMasked: string;
  deepseekApiKeyMasked: string;
  language: string;
  cleanupStyle: CleanupStyle;
  hotkey: string;
  hotkeyMode: HotkeyMode;
  audioDevice: string | null;
  sttModel: string;
  customPrompt: string;
  autostart: boolean;
  whisperMode: boolean;
  openaiApiKeyMasked: string;
  anthropicApiKeyMasked: string;
  sttProvider: string;
  llmProvider: string;
  // Deprecated: use sttProvider / llmProvider instead.
  sttPriority?: string[];
  llmPriority?: string[];
  outputLanguage: string;
  webhookUrl: string;
  tursoUrl: string;
  tursoTokenMasked: string;
  deviceId: string;
  bubbleSize: number;
  bubbleOpacity: number;
  localWhisperModel: string;
  localWhisperGpu: boolean;
  // Recording behaviour extensions.
  // insertAndSend is now per-slot. These fields map to insert_and_send_slot1 / slot2 in Rust.
  insertAndSendSlot1: boolean;
  insertAndSendSlot2: boolean;
  autostopSilenceSecs: number;
  autoModeSilenceSecs: number;
  // Second hotkey slot (optional — empty string means disabled).
  hotkeySlot2: string;
  hotkeyModeSlot2: HotkeyMode;
  // Bubble touch controls (Android only).
  // Tap and long-press each have their own mode, auto-send, and silence config.
  bubbleTapMode: string;
  bubbleTapAutoSend: boolean;
  bubbleTapSilenceSecs: number;
  bubbleLongPressMode: string;
  bubbleLongPressAutoSend: boolean;
  bubbleLongPressSilenceSecs: number;
}

// A per-application recording profile.
export interface AppProfile {
  name: string;
  appPattern: string;
  cleanupStyle: CleanupStyle;
  language: string;
  customPrompt: string;
}

// API key configuration status from the backend.
// Kept for backward compatibility -- prefer AppSettings where possible.
export interface ApiKeyStatus {
  groqConfigured: boolean;
  deepseekConfigured: boolean;
}

// A single dictation history entry from the SQLite database.
export interface HistoryEntry {
  id: number;
  text: string;
  rawText: string | null;
  style: string;
  language: string;
  isNote?: boolean;
  appName: string | null;
  createdAt: string;
}

// Aggregated usage/cost statistics from the backend.
export interface UsageSummary {
  totalDictations: number;
  totalWords: number;
  totalCostUsd: number;
  totalAudioSeconds: number;
  totalSttCostUsd: number;
  totalLlmCostUsd: number;
  dictationsToday: number;
  costTodayUsd: number;
}

// App-level state shape.
export interface AppState {
  recordingState: RecordingState;
  currentStyle: CleanupStyle;
  resultText: string | null;
  errorMessage: string | null;
}

// Status bar label map -- keeps component logic clean.
export const STATUS_LABELS: Record<RecordingState, string> = {
  idle: "Ready",
  recording: "Recording...",
  transcribing: "Transcribing...",
  cleaning: "Cleaning up...",
  done: "Done",
  error: "Error",
};

// Fine-grained advanced settings for power users.
export interface AdvancedSettings {
  sttPromptDe: string;
  sttPromptEn: string;
  sttPromptAuto: string;
  sttTemperature: number;
  llmSystemPromptPolished: string;
  llmSystemPromptVerbatim: string;
  llmSystemPromptChat: string;
  llmCommandModePrompt: string;
  llmTemperature: number;
  llmMaxTokens: number;
  llmModelDeepseek: string;
  llmModelOpenai: string;
  llmModelAnthropic: string;
  llmModelGroq: string;
  chunkThreshold: number;
  chunkTargetSize: number;
  silenceThreshold: number;
  whisperModeThreshold: number;
  minRecordingMs: number;
  whisperModeGain: number;
  autoPaste: boolean;
  pasteDelayMs: number;
  autoCapitalize: boolean;
  webhookHeaders: string;
  webhookTimeoutSecs: number;
  logLevel: string;
  uiScale: string;
}

// Style display metadata.
export interface StyleMeta {
  value: CleanupStyle;
  label: string;
  description: string;
}

// License status types.
export type LicenseStatus = "licensed" | "trial" | "grace_period" | "unlicensed";

export interface ParsedLicenseStatus {
  type: LicenseStatus;
  trialUntil?: number;  // Unix timestamp seconds, only present for trial
  graceUntil?: number;  // Unix timestamp seconds, only present for grace_period
}

export function parseLicenseStatus(raw: string): ParsedLicenseStatus {
  if (raw === "licensed") return { type: "licensed" };
  if (raw === "unlicensed") return { type: "unlicensed" };
  if (raw.startsWith("trial:")) {
    const until = parseInt(raw.split(":")[1], 10);
    return { type: "trial", trialUntil: isNaN(until) ? undefined : until };
  }
  if (raw.startsWith("grace_period:")) {
    const until = parseInt(raw.split(":")[1], 10);
    return { type: "grace_period", graceUntil: isNaN(until) ? undefined : until };
  }
  return { type: "unlicensed" };
}

export const STYLE_OPTIONS: StyleMeta[] = [
  {
    value: "polished",
    label: "Polished",
    description: "Fix grammar, smooth flow",
  },
  {
    value: "verbatim",
    label: "Verbatim",
    description: "Your words, just clean",
  },
  {
    value: "chat",
    label: "Chat",
    description: "Short, casual, conversational",
  },
];
