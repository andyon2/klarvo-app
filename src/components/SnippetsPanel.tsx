import { useState } from "react";
import type { TextSnippet } from "../tauri-commands";
import { saveSnippets, pasteSnippet } from "../tauri-commands";
import { CloseIcon } from "./icons";
import { INPUT_CLS } from "./ui";

interface SnippetsPanelProps {
  snippets: TextSnippet[];
  onUpdate: (s: TextSnippet[]) => void;
  onClose: () => void;
}

export function SnippetsPanel({ snippets, onUpdate, onClose }: SnippetsPanelProps) {
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  const handlePaste = async (content: string) => {
    try {
      await pasteSnippet(content);
    } catch (err) {
      console.error("paste_snippet failed:", err);
    }
  };

  const handleSave = async () => {
    try {
      const clean = snippets.filter((s) => s.name.trim() || s.content.trim());
      await saveSnippets(clean);
      onUpdate(clean);
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(null), 2000);
    } catch (err) {
      setSaveMsg(String(err));
    }
  };

  return (
    <div className="w-full bg-[#0e0e11] border border-zinc-800/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30">
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/40">
        <span className="text-[10px] font-semibold text-zinc-500 uppercase tracking-widest">Text Snippets</span>
        <button
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-200 transition-colors p-1 rounded-lg hover:bg-zinc-800/50"
        >
          <CloseIcon />
        </button>
      </div>

      <div className="overflow-y-auto max-h-[400px] p-4 flex flex-col gap-2">
        <p className="text-[10px] text-zinc-500">Click "Paste" to insert a snippet into the active window.</p>

        {snippets.length === 0 ? (
          <p className="text-xs text-zinc-500 italic text-center py-4">No snippets yet. Add your first one below.</p>
        ) : (
          snippets.map((s, i) => (
            <div key={i} className="bg-[#111113] border border-zinc-800/60 rounded-xl p-3 group hover:border-zinc-700/60 transition-colors">
              <div className="flex items-center justify-between gap-2 mb-2">
                <input
                  type="text"
                  placeholder="Name"
                  value={s.name}
                  onChange={(e) => {
                    const next = [...snippets];
                    next[i] = { ...next[i], name: e.target.value };
                    onUpdate(next);
                  }}
                  className={`flex-1 ${INPUT_CLS}`}
                />
                <button
                  onClick={() => onUpdate(snippets.filter((_, j) => j !== i))}
                  className="text-zinc-500 hover:text-red-400 transition-colors p-1"
                >
                  <CloseIcon />
                </button>
              </div>
              <textarea
                placeholder="Content to paste..."
                value={s.content}
                onChange={(e) => {
                  const next = [...snippets];
                  next[i] = { ...next[i], content: e.target.value };
                  onUpdate(next);
                }}
                rows={2}
                className={`${INPUT_CLS} resize-none`}
              />
              <div className="flex justify-end mt-2">
                <button
                  onClick={() => handlePaste(s.content)}
                  disabled={!s.content.trim()}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-[10px] font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-all disabled:opacity-30 disabled:cursor-not-allowed"
                >
                  Paste
                </button>
              </div>
            </div>
          ))
        )}

        <div className="flex gap-2 pt-1">
          <button
            onClick={() => onUpdate([...snippets, { name: "", content: "" }])}
            className="px-3 py-2 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 transition-colors"
          >
            + Add Snippet
          </button>
          {snippets.length > 0 && (
            <button
              onClick={handleSave}
              className="px-3 py-2 rounded-lg text-xs font-medium bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 hover:bg-emerald-500/15 transition-colors"
            >
              {saveMsg ?? "Save"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
