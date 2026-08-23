/**
 * PreviewComments — Annotation overlay for UX feedback in preview mode.
 *
 * Active only when isPreviewMode is true. Toggle "comment mode" via the
 * floating button (bottom-left). While comment mode is ON:
 *   - A translucent overlay intercepts ALL clicks (app buttons don't fire)
 *   - Clicking any element opens a comment popup
 *   - Hover highlights the element under the cursor
 * While comment mode is OFF: the app works normally.
 *
 * Comments are stored in localStorage and can be exported as Markdown.
 */

import { useState, useEffect, useRef, useCallback } from "react";
import { COPY_FEEDBACK_MS } from "../hooks/useCopyFeedback";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Comment {
  id: number;
  elementText: string;
  elementTag: string;
  stepContext: string;
  comment: string;
  timestamp: string;
  x: number;
  y: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STORAGE_KEY = "klarvo-preview-comments";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function loadComments(): Comment[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Comment[]) : [];
  } catch {
    return [];
  }
}

function saveComments(comments: Comment[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(comments));
}

/** Truncate a string to maxLen chars, appending ellipsis if truncated. */
function truncate(str: string, maxLen: number): string {
  const cleaned = str.replace(/\s+/g, " ").trim();
  return cleaned.length > maxLen ? cleaned.slice(0, maxLen) + "…" : cleaned;
}

/** Walk up the DOM to find the nearest data-step attribute value. */
function findStepContext(el: Element): string {
  let node: Element | null = el;
  while (node) {
    const step = node.getAttribute("data-step");
    if (step) return step;
    node = node.parentElement;
  }
  // Fallback: look for the first visible h2 text on the page
  const h2 = document.querySelector("h2");
  if (h2?.textContent) return truncate(h2.textContent, 60);
  return "";
}

/**
 * Find the most specific visible element at coordinates.
 * Uses document.elementsFromPoint to get all elements, then picks the
 * topmost non-overlay element (skipping our own UI).
 */
function findTargetElement(x: number, y: number): Element | null {
  const elements = document.elementsFromPoint(x, y);
  for (const el of elements) {
    // Skip our own overlay and UI elements
    if (el.closest("[data-preview-ui]")) continue;
    if (el.id === "comment-mode-overlay") continue;
    return el;
  }
  return null;
}

/** Build the Markdown export string from a list of comments. */
function buildMarkdown(comments: Comment[]): string {
  const date = new Date().toISOString().slice(0, 10);
  const lines: string[] = [
    "# UX-Feedback — Preview Session",
    "",
    `Datum: ${date}`,
    "",
    "## Kommentare",
  ];

  for (const c of comments) {
    const stepLine = c.stepContext ? `Step: "${c.stepContext}"` : "Step: unbekannt";
    lines.push(
      "",
      `### #${c.id} — ${stepLine}`,
      `**Element:** \`<${c.elementTag}>\` "${c.elementText}"`,
      `**Kommentar:** ${c.comment}`,
    );
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Popup sub-component
// ---------------------------------------------------------------------------

interface PopupProps {
  x: number;
  y: number;
  elementTag: string;
  elementText: string;
  stepContext: string;
  onSave: (comment: string) => void;
  onCancel: () => void;
}

function CommentPopup({ x, y, elementTag, elementText, stepContext, onSave, onCancel }: PopupProps) {
  const [text, setText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    // Small delay to avoid the click event that opened the popup from
    // interfering with focus
    const timer = setTimeout(() => textareaRef.current?.focus(), 50);
    return () => clearTimeout(timer);
  }, []);

  // Keep popup inside viewport
  const popupWidth = 320;
  const popupEstHeight = 220;
  const left = Math.min(Math.max(8, x - popupWidth / 2), window.innerWidth - popupWidth - 8);
  const top = y + 20 + popupEstHeight > window.innerHeight
    ? Math.max(8, y - popupEstHeight - 8)
    : y + 20;

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Escape") {
      onCancel();
    } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      if (text.trim()) onSave(text.trim());
    }
  }

  return (
    <div
      data-preview-ui
      className="fixed z-[9998] bg-zinc-800 border border-zinc-600 rounded-xl shadow-2xl shadow-black/60 p-3 flex flex-col gap-2"
      style={{ left, top, width: popupWidth }}
      onClick={(e) => e.stopPropagation()}
      onMouseDown={(e) => e.stopPropagation()}
    >
      {/* Element info */}
      <div className="bg-zinc-900/70 rounded-lg px-2.5 py-1.5">
        {stepContext && (
          <p className="text-[10px] text-amber-400/80 font-mono mb-0.5">Step: {stepContext}</p>
        )}
        <p className="text-[11px] text-zinc-400 font-mono leading-relaxed">
          <span className="text-zinc-300">&lt;{elementTag}&gt;</span>{" "}
          <span className="text-zinc-500 italic">"{elementText}"</span>
        </p>
      </div>

      {/* Comment textarea */}
      <textarea
        ref={textareaRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Kommentar eingeben… (Ctrl+Enter speichern, Esc abbrechen)"
        rows={3}
        className="w-full bg-zinc-900 border border-zinc-600/60 rounded-lg px-2.5 py-2 text-sm text-zinc-200 placeholder:text-zinc-600 resize-none focus:outline-none focus:border-amber-500/50 transition-colors"
      />

      {/* Actions */}
      <div className="flex items-center justify-end gap-2">
        <button
          onClick={onCancel}
          className="px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200 transition-colors rounded-lg hover:bg-zinc-700/50"
        >
          Abbrechen
        </button>
        <button
          onClick={() => { if (text.trim()) onSave(text.trim()); }}
          disabled={!text.trim()}
          className="px-3 py-1.5 text-xs bg-amber-500/20 border border-amber-500/40 text-amber-400 hover:bg-amber-500/30 hover:text-amber-300 rounded-lg transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Speichern
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sidebar panel sub-component
// ---------------------------------------------------------------------------

interface SidebarProps {
  comments: Comment[];
  onDelete: (id: number) => void;
  onClearAll: () => void;
  onClose: () => void;
  onExport: () => void;
  copied: boolean;
}

function CommentSidebar({ comments, onDelete, onClearAll, onClose, onExport, copied }: SidebarProps) {
  return (
    <div
      data-preview-ui
      className="fixed left-0 top-0 h-full z-[9997] flex flex-col bg-zinc-900/95 backdrop-blur-sm border-r border-zinc-700/60 shadow-2xl shadow-black/60"
      style={{ width: 320 }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-700/60 flex-shrink-0">
        <span className="text-sm font-semibold text-zinc-200">
          Kommentare
          {comments.length > 0 && (
            <span className="ml-2 text-[11px] text-zinc-500">({comments.length})</span>
          )}
        </span>
        <button
          onClick={onClose}
          className="text-zinc-500 hover:text-zinc-300 transition-colors p-1 rounded-lg hover:bg-zinc-800"
          aria-label="Schliessen"
        >
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Comment list */}
      <div className="flex-1 overflow-y-auto px-3 py-2 flex flex-col gap-2">
        {comments.length === 0 ? (
          <p className="text-xs text-zinc-600 italic text-center mt-8">
            Noch keine Kommentare.<br />Klick auf ein Element im Kommentar-Modus.
          </p>
        ) : (
          comments.map((c) => (
            <div
              key={c.id}
              className="bg-zinc-800/60 border border-zinc-700/50 rounded-xl p-3 group"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <span className="flex-shrink-0 w-5 h-5 rounded-full bg-amber-500 text-black text-[10px] font-bold flex items-center justify-center">
                    {c.id}
                  </span>
                  <span className="text-[11px] text-zinc-500 font-mono truncate">
                    &lt;{c.elementTag}&gt; "{c.elementText}"
                  </span>
                </div>
                <button
                  onClick={() => onDelete(c.id)}
                  className="flex-shrink-0 text-zinc-600 hover:text-red-400 transition-colors opacity-0 group-hover:opacity-100 p-0.5 rounded"
                  aria-label="Kommentar loeschen"
                >
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                    <path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6" />
                  </svg>
                </button>
              </div>
              {c.stepContext && (
                <p className="text-[10px] text-amber-400/70 mt-1.5 font-mono">
                  Step: {c.stepContext}
                </p>
              )}
              <p className="text-xs text-zinc-300 mt-1.5 leading-relaxed">{c.comment}</p>
            </div>
          ))
        )}
      </div>

      {/* Footer actions */}
      {comments.length > 0 && (
        <div className="flex flex-col gap-2 px-3 py-3 border-t border-zinc-700/60 flex-shrink-0">
          <button
            onClick={onExport}
            className="w-full px-3 py-2 text-xs bg-amber-500/15 border border-amber-500/30 text-amber-400 hover:bg-amber-500/25 hover:text-amber-300 rounded-lg transition-colors font-medium"
          >
            {copied ? "✓ Kopiert!" : "Als Markdown kopieren"}
          </button>
          <button
            onClick={onClearAll}
            className="w-full px-3 py-2 text-xs text-zinc-500 hover:text-red-400 hover:bg-red-500/10 border border-transparent hover:border-red-500/20 rounded-lg transition-colors"
          >
            Alle löschen
          </button>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PreviewComments() {
  const [comments, setComments] = useState<Comment[]>(() => loadComments());
  const [commentMode, setCommentMode] = useState(false);
  const [popup, setPopup] = useState<{
    x: number;
    y: number;
    elementTag: string;
    elementText: string;
    stepContext: string;
  } | null>(null);
  const [hoverRect, setHoverRect] = useState<DOMRect | null>(null);
  const [showSidebar, setShowSidebar] = useState(false);
  const [showMarkers, setShowMarkers] = useState(true);
  const [copied, setCopied] = useState(false);
  const nextIdRef = useRef<number>(
    comments.length > 0 ? Math.max(...comments.map((c) => c.id)) + 1 : 1,
  );

  // Persist whenever comments change
  useEffect(() => {
    saveComments(comments);
  }, [comments]);

  // Warn before losing comments on reload/close
  useEffect(() => {
    if (comments.length === 0) return;
    function handleBeforeUnload(e: BeforeUnloadEvent) {
      e.preventDefault();
      // Modern browsers show a generic message, but setting returnValue is required
      e.returnValue = "Du hast nicht-exportierte Kommentare. Wirklich verlassen?";
    }
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [comments.length]);

  // In comment mode: track hover to highlight elements
  useEffect(() => {
    if (!commentMode || popup) return;

    function handleMouseMove(e: MouseEvent) {
      const target = findTargetElement(e.clientX, e.clientY);
      if (target) {
        setHoverRect(target.getBoundingClientRect());
      } else {
        setHoverRect(null);
      }
    }

    document.addEventListener("mousemove", handleMouseMove, { passive: true });
    return () => document.removeEventListener("mousemove", handleMouseMove);
  }, [commentMode, popup]);

  // Clear hover when popup opens
  useEffect(() => {
    if (popup) setHoverRect(null);
  }, [popup]);

  // Handle click on overlay in comment mode
  const handleOverlayClick = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    // Find the actual element under the click (beneath our overlay)
    const target = findTargetElement(e.clientX, e.clientY);
    if (!target) return;

    const tag = target.tagName.toLowerCase();
    const elementText = truncate(target.textContent ?? "", 80);
    const stepContext = findStepContext(target);

    setPopup({
      x: e.clientX,
      y: e.clientY,
      elementTag: tag,
      elementText,
      stepContext,
    });
  }, []);

  // Escape exits comment mode or closes popup
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (popup) {
          setPopup(null);
        } else if (commentMode) {
          setCommentMode(false);
          setHoverRect(null);
        }
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [popup, commentMode]);

  function handleSave(commentText: string) {
    if (!popup) return;
    const newComment: Comment = {
      id: nextIdRef.current++,
      elementText: popup.elementText,
      elementTag: popup.elementTag,
      stepContext: popup.stepContext,
      comment: commentText,
      timestamp: new Date().toISOString(),
      x: popup.x,
      y: popup.y,
    };
    setComments((prev) => [...prev, newComment]);
    setPopup(null);
  }

  function handleDelete(id: number) {
    setComments((prev) => prev.filter((c) => c.id !== id));
  }

  function handleClearAll() {
    setComments([]);
    nextIdRef.current = 1;
  }

  async function handleExport() {
    const md = buildMarkdown(comments);
    try {
      await navigator.clipboard.writeText(md);
      setCopied(true);
      setTimeout(() => setCopied(false), COPY_FEEDBACK_MS);
    } catch {
      // Fallback: create a temporary textarea
      const ta = document.createElement("textarea");
      ta.value = md;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(true);
      setTimeout(() => setCopied(false), COPY_FEEDBACK_MS);
    }
  }

  return (
    <>
      {/* ── Comment mode overlay ── */}
      {/* When active: covers entire screen, intercepts all clicks, shows hover highlight */}
      {commentMode && !popup && (
        <div
          id="comment-mode-overlay"
          data-preview-ui
          className="fixed inset-0 z-[9995]"
          style={{ cursor: "crosshair" }}
          onClick={handleOverlayClick}
        >
          {/* Top bar indicating comment mode is active */}
          <div className="absolute top-0 left-0 right-0 bg-amber-500/15 border-b border-amber-500/40 px-4 py-1.5 flex items-center justify-center gap-2">
            <span className="text-xs font-medium text-amber-400">
              Kommentar-Modus — Klicke auf ein Element zum Kommentieren
            </span>
            <span className="text-[10px] text-amber-400/60">(Esc zum Beenden)</span>
          </div>

          {/* Hover highlight rectangle */}
          {hoverRect && (
            <div
              className="absolute border-2 border-amber-400/70 bg-amber-400/5 rounded pointer-events-none transition-all duration-75"
              style={{
                left: hoverRect.left - 2,
                top: hoverRect.top - 2,
                width: hoverRect.width + 4,
                height: hoverRect.height + 4,
              }}
            />
          )}
        </div>
      )}

      {/* ── Numbered markers for saved comments ── */}
      {showMarkers && comments.map((c) => (
        <div
          key={c.id}
          data-preview-ui
          className="fixed z-[9996] pointer-events-none"
          style={{
            left: c.x - 10,
            top: c.y - 10,
          }}
        >
          <span className="w-5 h-5 rounded-full bg-amber-500 text-black text-[10px] font-bold flex items-center justify-center shadow-lg shadow-black/40">
            {c.id}
          </span>
        </div>
      ))}

      {/* ── Popup for new comment ── */}
      {popup && (
        <div data-preview-ui className="fixed inset-0 z-[9997]" onClick={() => setPopup(null)}>
          <div onClick={(e) => e.stopPropagation()}>
            <CommentPopup
              x={popup.x}
              y={popup.y}
              elementTag={popup.elementTag}
              elementText={popup.elementText}
              stepContext={popup.stepContext}
              onSave={handleSave}
              onCancel={() => setPopup(null)}
            />
          </div>
        </div>
      )}

      {/* ── Sidebar ── */}
      {showSidebar && (
        <CommentSidebar
          comments={comments}
          onDelete={handleDelete}
          onClearAll={handleClearAll}
          onClose={() => setShowSidebar(false)}
          onExport={handleExport}
          copied={copied}
        />
      )}

      {/* ── Floating controls — bottom left ── */}
      <div data-preview-ui className="fixed bottom-4 left-4 z-[9999] flex items-center gap-2">
        {/* Comment mode toggle */}
        <button
          onClick={() => {
            setCommentMode((v) => !v);
            setPopup(null);
            setHoverRect(null);
          }}
          className={[
            "flex items-center gap-2 px-3.5 py-2 rounded-full",
            "border backdrop-blur-sm shadow-lg shadow-black/40",
            "text-xs font-medium transition-all duration-150",
            commentMode
              ? "bg-amber-500/20 border-amber-500/60 text-amber-400"
              : "bg-zinc-800/90 border-zinc-700/60 text-zinc-400 hover:text-zinc-200 hover:border-zinc-600",
          ].join(" ")}
        >
          {/* Crosshair / annotation icon */}
          {commentMode ? (
            <svg className="w-3.5 h-3.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          ) : (
            <svg className="w-3.5 h-3.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
            </svg>
          )}
          {commentMode ? "Beenden" : "Kommentieren"}
        </button>

        {/* Comment count / sidebar toggle */}
        {comments.length > 0 && (
          <>
            <button
              onClick={() => setShowSidebar((v) => !v)}
              className={[
                "flex items-center gap-1.5 px-3 py-2 rounded-full",
                "border backdrop-blur-sm shadow-lg shadow-black/40",
                "text-xs font-medium transition-all duration-150",
                showSidebar
                  ? "bg-zinc-800/90 border-amber-500/40 text-amber-400"
                  : "bg-zinc-800/90 border-zinc-700/60 text-zinc-400 hover:text-zinc-200 hover:border-zinc-600",
              ].join(" ")}
            >
              <span className="text-amber-400 font-bold">{comments.length}</span>
              <span className="text-zinc-500">Kommentar{comments.length !== 1 ? "e" : ""}</span>
            </button>

            {/* Marker visibility toggle */}
            <button
              onClick={() => setShowMarkers((v) => !v)}
              className={[
                "flex items-center gap-1.5 px-2.5 py-2 rounded-full",
                "border backdrop-blur-sm shadow-lg shadow-black/40",
                "text-xs font-medium transition-all duration-150",
                "bg-zinc-800/90 border-zinc-700/60",
                showMarkers
                  ? "text-amber-400 hover:text-amber-300"
                  : "text-zinc-500 hover:text-zinc-300",
              ].join(" ")}
              title={showMarkers ? "Marker ausblenden" : "Marker einblenden"}
            >
              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                {showMarkers ? (
                  <>
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </>
                ) : (
                  <>
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                    <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                    <line x1="1" y1="1" x2="23" y2="23" />
                  </>
                )}
              </svg>
            </button>
          </>
        )}
      </div>
    </>
  );
}
