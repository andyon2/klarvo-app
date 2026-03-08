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
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">Voice Notes</span>
        <button
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
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
              ? "bg-red-500/15 border-red-500/30 text-red-400"
              : noteState === "processing"
              ? "bg-amber-500/10 border-amber-500/20 text-amber-400 opacity-60 cursor-not-allowed"
              : "bg-emerald-500/10 border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15",
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
        {noteError && <span className="text-[10px] text-red-400">{noteError}</span>}
        <p className="text-[10px] text-zinc-500 ml-auto">Notes are saved, not pasted.</p>
      </div>

      {/* Notes list */}
      <div className="overflow-y-auto max-h-[300px] p-4 flex flex-col gap-2">
        {notes.length === 0 ? (
          <p className="text-xs text-zinc-500 italic text-center py-4">No voice notes yet. Record your first one!</p>
        ) : (
          notes.map((note) => (
            <div
              key={note.id}
              className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors"
            >
              <p className="text-xs text-zinc-300 whitespace-pre-wrap line-clamp-3">{note.text}</p>
              <div className="flex items-center justify-between mt-2">
                <span className="text-[10px] text-zinc-500">
                  {new Date(note.createdAt + "Z").toLocaleString()}
                </span>
                <button
                  onClick={() => navigator.clipboard.writeText(note.text).catch(console.error)}
                  className="text-[10px] text-zinc-500 hover:text-emerald-400 opacity-0 group-hover:opacity-100 transition-all"
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
