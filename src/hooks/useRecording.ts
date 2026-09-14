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
  const [warningMessage, setWarningMessage] = useState<string | null>(null);

  // Subscribe to backend pipeline events (hotkey-triggered recording on desktop).
  useEffect(() => {
    const unlisten = onStateChanged((p) => {
      // A warning can arrive two ways (story 7-10): as its own transient
      // "warning" event (the STT fallback ladder), or riding on the terminal
      // "done" + clipboardOnly event when cleanup failed and the raw text was
      // left in the clipboard. Capture it before the state branch, otherwise
      // the degrade cause — including the model ID — is dropped here.
      //
      // The `else` is not cosmetic (review round 2): a hotkey-driven run never
      // passes through `handleRecordToggle`, which holds the only other
      // `setWarningMessage(null)` sites. Without it the message outlives its own
      // run and the NEXT successful run wears the previous run's amber degrade
      // text instead of "Done". So `warningMessage` means "the warning carried
      // by the latest state event", nothing longer-lived.
      if (p.warning) setWarningMessage(p.warning);
      else setWarningMessage(null);
      // Warning is transient: don't update recordingState (the pipeline
      // continues and will send "done" next).
      if (p.state === "warning") return;
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
      setWarningMessage(null);
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
      setWarningMessage(null);
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
    warningMessage,
    handleRecordToggle,
  };
}
