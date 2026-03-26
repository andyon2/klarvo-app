import { useState, useCallback } from "react";
import type { HistoryEntry } from "../types";
import { transcribeAudio, cleanupText, saveNote, transcribeAudioBytes } from "../tauri-commands";
import { startRecording, stopRecording } from "../tauri-commands";
import { isMobile } from "../platform";
import { startBrowserRecording, stopBrowserRecording } from "../media-recorder";
import { MicIcon, StopIcon, SpinnerIcon, CloseIcon } from "./icons";

interface VoiceNotesPanelProps {
  notes: HistoryEntry[];
  onRefresh: () => void;
  onClose: () => void;
}

export function VoiceNotesPanel({ notes, onRefresh, onClose }: VoiceNotesPanelProps) {
  const [noteState, setNoteState] = useState<"idle" | "recording" | "processing">("idle");
  const [noteError, setNoteError] = useState<string | null>(null);

  const handleRecordNote = useCallback(async () => {
    if (noteState === "recording") {
      // Stop and save as note.
      setNoteState("processing");
      try {
        let transcript: string;
        if (isMobile) {
          const wavBytes = await stopBrowserRecording();
          transcript = await transcribeAudioBytes(Array.from(wavBytes), "");
        } else {
          await stopRecording();
          transcript = await transcribeAudio("");
        }
        const cleaned = await cleanupText(transcript, "polished");
        await saveNote(cleaned, transcript, "polished");
        onRefresh();
        setNoteState("idle");
      } catch (err) {
        setNoteError(err instanceof Error ? err.message : String(err));
        setNoteState("idle");
      }
    } else {
      setNoteError(null);
      try {
        if (isMobile) {
          await startBrowserRecording();
        } else {
          await startRecording();
        }
        setNoteState("recording");
      } catch (err) {
        setNoteError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [noteState, onRefresh]);

  return (
    <div className="w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      <div className="flex items-center justify-between px-4 py-3 border-b border-klarvo-border/40">
        <span className="text-[10px] font-semibold text-klarvo-dim uppercase tracking-widest">Voice Notes</span>
        <button
          onClick={onClose}
          className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Record note button */}
      <div className="px-4 pt-3 flex items-center gap-3">
        <button
          onClick={handleRecordNote}
          disabled={noteState === "processing"}
          className={[
            "flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-medium border transition-all duration-150",
            noteState === "recording"
              ? "bg-klarvo-danger/15 border-klarvo-danger/30 text-klarvo-danger"
              : noteState === "processing"
              ? "bg-klarvo-warning/10 border-klarvo-warning/20 text-klarvo-warning opacity-60 cursor-not-allowed"
              : "bg-klarvo-primary/10 border-klarvo-primary/20 text-klarvo-primary hover:bg-klarvo-primary/15",
          ].join(" ")}
        >
          {noteState === "recording" ? (
            <><StopIcon className="w-3.5 h-3.5" /> Stop & Save</>
          ) : noteState === "processing" ? (
            <><SpinnerIcon className="w-3.5 h-3.5" /> Processing...</>
          ) : (
            <><MicIcon className="w-3.5 h-3.5" /> Record Note</>
          )}
        </button>
        {noteError && <span className="text-[10px] text-klarvo-danger">{noteError}</span>}
        <p className="text-[10px] text-klarvo-dim ml-auto">Notes are saved, not pasted.</p>
      </div>

      {/* Notes list */}
      <div className="overflow-y-auto max-h-[300px] p-4 flex flex-col gap-2">
        {notes.length === 0 ? (
          <p className="text-xs text-klarvo-dim italic text-center py-4">No voice notes yet. Record your first one!</p>
        ) : (
          notes.map((note) => (
            <div
              key={note.id}
              className="bg-klarvo-bg border border-klarvo-border/60 rounded-xl p-3 group hover:border-klarvo-border/60 transition-colors"
            >
              <p className="text-xs text-klarvo-muted whitespace-pre-wrap line-clamp-3">{note.text}</p>
              <div className="flex items-center justify-between mt-2">
                <span className="text-[10px] text-klarvo-dim">
                  {new Date(note.createdAt + "Z").toLocaleString()}
                </span>
                <button
                  onClick={() => navigator.clipboard.writeText(note.text).catch(console.error)}
                  className="text-[10px] text-klarvo-dim hover:text-klarvo-primary opacity-0 group-hover:opacity-100 transition-all"
                >
                  Copy
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
