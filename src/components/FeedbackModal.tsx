import { useState, useEffect, useRef } from "react";
import { sendFeedback, getFeedbackMetrics, type FeedbackMetrics } from "../tauri-commands";
import { CloseIcon, SpinnerIcon } from "./icons";
import { isMobile } from "../platform";

type Category = "bug" | "text_quality" | "too_slow" | "feature_request" | "praise_other";

const CATEGORIES: { value: Category; emoji: string; label: string }[] = [
  { value: "bug",             emoji: "🐛", label: "Bug / Fehler" },
  { value: "text_quality",   emoji: "📝", label: "Text-Qualität (KI hat Mist gebaut)" },
  { value: "too_slow",       emoji: "🐌", label: "Zu langsam" },
  { value: "feature_request", emoji: "💡", label: "Feature-Wunsch" },
  { value: "praise_other",   emoji: "👍", label: "Lob / Sonstiges" },
];

const PLACEHOLDER: Record<Category, string> = {
  bug:             "What happened? What did you expect?",
  text_quality:    "What did the AI get wrong? Share the original and the bad result.",
  too_slow:        "Where does it feel slow? Recording start, transcription, or paste?",
  feature_request: "What would you like Klarvo to do?",
  praise_other:    "What do you like about Klarvo?",
};


interface FeedbackModalProps {
  isOpen: boolean;
  onClose: () => void;
  defaultArea: string;
}

export function FeedbackModal({ isOpen, onClose, defaultArea }: FeedbackModalProps) {
  const [category, setCategory] = useState<Category | null>(null);
  const [message, setMessage] = useState("");
  const [includeDictation, setIncludeDictation] = useState(false);
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<FeedbackMetrics | null>(null);

  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset form state and fetch metrics when modal opens.
  useEffect(() => {
    if (isOpen) {
      setCategory(null);
      setMessage("");
      setIncludeDictation(false);
      setEmail("");
      setLoading(false);
      setSuccess(false);
      setError(null);
      getFeedbackMetrics().then(setMetrics).catch(() => setMetrics(null));
    }
    return () => {
      if (closeTimerRef.current !== null) {
        clearTimeout(closeTimerRef.current);
      }
    };
  }, [isOpen]);

  // Close on Escape key.
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  const handleSubmit = async () => {
    if (!category || message.trim().length < 3 || loading) return;
    setLoading(true);
    setError(null);
    try {
      await sendFeedback(
        category,
        message.trim(),
        email.trim() || undefined,
        defaultArea,
        undefined,
        includeDictation,
      );
      setSuccess(true);
      closeTimerRef.current = setTimeout(() => {
        onClose();
      }, 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  const canSubmit = category !== null && message.trim().length >= 3 && !loading && !success;
  const textareaPlaceholder = category ? PLACEHOLDER[category] : "Describe your feedback...";

  return (
    <div className="w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl shadow-xl shadow-black/30 overflow-hidden max-h-[80vh] overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-orange-500/20">
        <span className="text-[11px] font-semibold text-orange-400/80 uppercase tracking-widest">
          Send Feedback
        </span>
        <button
          onClick={onClose}
          aria-label="Close feedback"
          className="text-klarvo-dim hover:text-klarvo-text transition-colors p-1 rounded-lg hover:bg-klarvo-surface/50"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Body */}
      <div className="px-4 py-4 flex flex-col gap-3">
        {/* Category dropdown */}
        <div>
          <select
            value={category ?? ""}
            onChange={(e) => setCategory((e.target.value as Category) || null)}
            className={[
              "w-full bg-klarvo-bg border rounded-lg px-3 py-2 text-xs transition-colors",
              "focus:outline-none appearance-none cursor-pointer",
              "bg-[url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E\")] bg-no-repeat bg-[right_0.75rem_center]",
              category
                ? "border-orange-500/40 text-klarvo-text focus:border-orange-500/60"
                : "border-klarvo-border/60 text-klarvo-dim focus:border-orange-500/40",
            ].join(" ")}
          >
            <option value="" disabled className="bg-klarvo-bg text-klarvo-dim">
              What's this about?
            </option>
            {CATEGORIES.map(({ value, emoji, label }) => (
              <option key={value} value={value} className="bg-klarvo-bg text-klarvo-text">
                {emoji} {label}
              </option>
            ))}
          </select>
        </div>

        {/* Message — primary field, autofocus */}
        <div>
          <textarea
            id="feedback-message"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder={textareaPlaceholder}
            rows={4}
            autoFocus={!isMobile}
            className="w-full bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-3 py-2.5 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-orange-500/40 transition-colors resize-none"
          />
        </div>

        {/* Opt-in: include last dictation */}
        <label className="flex items-start gap-2.5 cursor-pointer group">
          <div className="relative flex-shrink-0 mt-0.5">
            <input
              type="checkbox"
              checked={includeDictation}
              onChange={(e) => setIncludeDictation(e.target.checked)}
              className="sr-only"
            />
            <div
              className={[
                "w-3.5 h-3.5 rounded border transition-all duration-100",
                includeDictation
                  ? "bg-orange-500/30 border-orange-500/60"
                  : "bg-klarvo-bg border-klarvo-border/60 group-hover:border-klarvo-border",
              ].join(" ")}
            >
              {includeDictation && (
                <svg
                  className="w-full h-full text-orange-400"
                  viewBox="0 0 14 14"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <polyline points="2 7 6 11 12 3" />
                </svg>
              )}
            </div>
          </div>
          <span className="text-[11px] text-klarvo-muted group-hover:text-klarvo-text transition-colors leading-tight">
            Include last dictation{" "}
            <span className="text-klarvo-dim">(helps me reproduce the issue)</span>
          </span>
        </label>

        {/* Email (optional) */}
        <div>
          <input
            id="feedback-email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="Your email (optional, for follow-up)"
            className="w-full bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-3 py-2 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-orange-500/40 transition-colors"
          />
        </div>

        {/* Success message */}
        {success && (
          <p className="text-xs text-orange-400 font-medium text-center py-1">
            Thanks! Your feedback has been sent.
          </p>
        )}

        {/* Error message */}
        {error && !success && (
          <p className="text-xs text-klarvo-danger font-medium">{error}</p>
        )}

        {/* Submit */}
        <button
          onClick={handleSubmit}
          disabled={!canSubmit}
          className={[
            "w-full min-h-[44px] px-4 py-2.5 rounded-lg text-xs font-semibold transition-all duration-150",
            "flex items-center justify-center gap-2",
            canSubmit
              ? "bg-orange-500/20 text-orange-400 border border-orange-500/30 hover:bg-orange-500/30"
              : "bg-klarvo-elevated text-klarvo-dim border border-klarvo-border/40 cursor-not-allowed",
          ].join(" ")}
        >
          {loading && <SpinnerIcon className="w-3.5 h-3.5" />}
          Send Feedback
        </button>

        {/* What gets sent — trust signal, always visible */}
        <div className="text-[10px] text-klarvo-dim leading-relaxed text-center px-2 space-y-0.5">
          <p>
            Automatically included: app version, OS, language, dictation mode
          </p>
          {metrics && (metrics.lastSttLatencyMs !== null || metrics.sttErrorCount > 0) && (
            <p>
              {metrics.lastSttLatencyMs !== null && (
                <span>Last dictation: STT {metrics.lastSttLatencyMs}ms</span>
              )}
              {metrics.lastLlmLatencyMs !== null && (
                <span> · LLM {metrics.lastLlmLatencyMs}ms</span>
              )}
              {metrics.lastTotalLatencyMs !== null && (
                <span> · Total {metrics.lastTotalLatencyMs}ms</span>
              )}
              {(metrics.sttErrorCount > 0 || metrics.llmErrorCount > 0 || metrics.pasteErrorCount > 0) && (
                <span>
                  {" · Errors: "}
                  {metrics.sttErrorCount > 0 && `STT×${metrics.sttErrorCount} `}
                  {metrics.llmErrorCount > 0 && `LLM×${metrics.llmErrorCount} `}
                  {metrics.pasteErrorCount > 0 && `Paste×${metrics.pasteErrorCount}`}
                </span>
              )}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
