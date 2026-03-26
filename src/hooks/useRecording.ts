import { useState, useCallback, useEffect } from "react";
import type { RecordingState, CleanupStyle } from "../types";
import {
  startRecording,
  stopRecording,
  transcribeAudio,
  cleanupText,
  addHistoryEntry,
  onStateChanged,
  transcribeAudioBytes,
} from "../tauri-commands";
import { isMobile } from "../platform";
import { startBrowserRecording, stopBrowserRecording } from "../media-recorder";

export function useRecording(currentStyle: CleanupStyle, language: string) {
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [resultText, setResultText] = useState<string | null>(null);
  const [originalResultText, setOriginalResultText] = useState<string | null>(null);
  const [rawText, setRawText] = useState<string | null>(null);
  const [showRawText, setShowRawText] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Subscribe to backend pipeline events (hotkey-triggered recording on desktop).
  useEffect(() => {
    const unlisten = onStateChanged((p) => {
      setRecordingState(p.state as RecordingState);
      if (p.text !== undefined) { setResultText(p.text); setOriginalResultText(p.text); }
      if (p.rawText !== undefined) setRawText(p.rawText);
      if (p.error !== undefined) setErrorMessage(p.error);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const handleRecordToggle = useCallback(async () => {
    if (recordingState === "done" || recordingState === "error") {
      setRecordingState("idle");
      setErrorMessage(null);
      return;
    }

    const isRecording = recordingState === "recording";

    if (isRecording) {
      try {
        setRecordingState("transcribing");
        let transcript: string;
        if (isMobile) {
          const wavBytes = await stopBrowserRecording();
          transcript = await transcribeAudioBytes(Array.from(wavBytes), language);
        } else {
          await stopRecording();
          transcript = await transcribeAudio(language);
        }
        setRawText(transcript);
        setRecordingState("cleaning");
        const cleanedText = await cleanupText(transcript, currentStyle);
        setResultText(cleanedText);
        setOriginalResultText(cleanedText);
        setRecordingState("done");
        // Save to history (fire-and-forget).
        addHistoryEntry(cleanedText, transcript, currentStyle, language).catch(console.error);
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    } else {
      setResultText(null);
      setOriginalResultText(null);
      setRawText(null);
      setShowRawText(false);
      setErrorMessage(null);
      try {
        if (isMobile) {
          await startBrowserRecording();
        } else {
          await startRecording();
        }
        setRecordingState("recording");
      } catch (err) {
        setErrorMessage(err instanceof Error ? err.message : String(err));
        setRecordingState("error");
      }
    }
  }, [recordingState, currentStyle, language]);

  return {
    recordingState,
    resultText,
    setResultText,
    originalResultText,
    rawText,
    showRawText,
    setShowRawText,
    errorMessage,
    handleRecordToggle,
  };
}
