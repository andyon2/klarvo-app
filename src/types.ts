// Cleanup style determines how the LLM processes raw transcription output.
export type CleanupStyle = "polished" | "verbatim" | "chat";

// Recording state machine states.
export type RecordingState = "idle" | "recording" | "transcribing" | "cleaning" | "done" | "error";

// Result returned from stop_recording / transcribe_audio Tauri commands.
export interface TranscriptionResult {
  rawText: string;
}

// Result returned from cleanup_text Tauri command.
export interface CleanupResult {
  cleanedText: string;
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
