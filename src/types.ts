// Cleanup style determines how the LLM processes raw transcription output.
export type CleanupStyle = "polished" | "verbatim" | "chat";

// Hotkey activation mode.
// hold   = push-to-talk: record while key is held, release to process.
// toggle = press to start, press again to stop and process.
export type HotkeyMode = "toggle" | "hold";

// Payload emitted by the backend on every state transition of the hotkey pipeline.
export interface StateChangedPayload {
  state: "recording" | "transcribing" | "cleaning" | "done" | "idle" | "error";
  text?: string;   // present when state === "done": cleaned result text
  error?: string;  // present when state === "error": human-readable message
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
}

// API key configuration status from the backend.
// Kept for backward compatibility -- prefer AppSettings where possible.
export interface ApiKeyStatus {
  groqConfigured: boolean;
  deepseekConfigured: boolean;
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

// Style display metadata.
export interface StyleMeta {
  value: CleanupStyle;
  label: string;
  description: string;
}

export const STYLE_OPTIONS: StyleMeta[] = [
  {
    value: "polished",
    label: "Polished",
    description: "Clean grammar, no filler words",
  },
  {
    value: "verbatim",
    label: "Verbatim",
    description: "Punctuation only, word-for-word",
  },
  {
    value: "chat",
    label: "Chat",
    description: "Short, casual, conversational",
  },
];
