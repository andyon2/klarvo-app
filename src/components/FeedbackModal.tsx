import { useState, useEffect, useRef, useCallback } from "react";
import { sendFeedback } from "../tauri-commands";
import { CloseIcon, SpinnerIcon } from "./icons";
import { isMobile } from "../platform";

type Category = "problem" | "idea" | "question" | "praise";

const CATEGORIES: { value: Category; label: string }[] = [
  { value: "problem", label: "Problem" },
  { value: "idea", label: "Idea" },
  { value: "question", label: "Question" },
  { value: "praise", label: "Praise" },
];

const PLACEHOLDER: Record<Category, string> = {
  problem: "What happened? What did you expect?",
  idea: "What feature or improvement would you like?",
  question: "What are you trying to do? What's unclear?",
  praise: "What do you like about Klarvo?",
};

const FEATURE_AREAS = ["Audio", "Text Cleanup", "UI", "Dictionary", "Settings", "Other"] as const;

interface FeedbackModalProps {
  isOpen: boolean;
  onClose: () => void;
  defaultArea: string;
}

export function FeedbackModal({ isOpen, onClose, defaultArea }: FeedbackModalProps) {
  const [category, setCategory] = useState<Category>("problem");
  const [featureArea, setFeatureArea] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const overlayRef = useRef<HTMLDivElement>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset form state when modal opens.
  useEffect(() => {
    if (isOpen) {
      setCategory("problem");
      setFeatureArea(null);
      setMessage("");
      setEmail("");
      setLoading(false);
      setSuccess(false);
      setError(null);
    }
    return () => {
      if (closeTimerRef.current !== null) {
        clearTimeout(closeTimerRef.current);
      }
    };
  }, [isOpen]);

  // Escape key closes modal.
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [isOpen, onClose]);

  const handleOverlayClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.target === overlayRef.current) onClose();
    },
    [onClose],
  );

  const handleSubmit = async () => {
    if (message.trim().length < 3 || loading) return;
    setLoading(true);
    setError(null);
    try {
      await sendFeedback(
        category,
        message.trim(),
        email.trim() || undefined,
        defaultArea,
        featureArea ?? undefined,
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

  const canSubmit = message.trim().length >= 3 && !loading && !success;

  return (
    <div
      ref={overlayRef}
      className="fixed inset-0 z-[9999] flex items-end sm:items-center justify-center bg-black/60 backdrop-blur-[2px] overflow-y-auto"
      onClick={handleOverlayClick}
      aria-modal="true"
      role="dialog"
      aria-label="Send feedback"
    >
      <div className="w-full max-w-sm mx-4 mb-4 sm:mb-0 bg-klarvo-surface border border-klarvo-border/60 rounded-2xl shadow-2xl shadow-black/50 overflow-hidden max-h-[90vh] overflow-y-auto">
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
          {/* Category toggle — compact row */}
          <div className="flex items-center gap-1.5">
            {CATEGORIES.map(({ value, label }) => (
              <button
                key={value}
                onClick={() => setCategory(value)}
                className={[
                  "px-2.5 py-1 rounded-md text-[11px] font-medium border transition-all duration-100",
                  category === value
                    ? "bg-klarvo-primary/20 text-klarvo-primary border-klarvo-primary/40"
                    : "bg-klarvo-elevated text-klarvo-muted border-klarvo-border/60 hover:text-klarvo-text hover:border-klarvo-border",
                ].join(" ")}
              >
                {label}
              </button>
            ))}
          </div>

          {/* Message — primary field, autofocus */}
          <div>
            <textarea
              id="feedback-message"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder={PLACEHOLDER[category]}
              rows={4}
              autoFocus={!isMobile}
              className="w-full bg-klarvo-bg border border-klarvo-border/60 rounded-lg px-3 py-2.5 text-xs text-klarvo-text placeholder:text-klarvo-dim focus:outline-none focus:border-orange-500/40 transition-colors resize-none"
            />
          </div>

          {/* Feature area chips — optional, no default */}
          <div className="flex flex-wrap items-center gap-1.5">
            {FEATURE_AREAS.map((a) => (
              <button
                key={a}
                onClick={() => setFeatureArea(featureArea === a ? null : a)}
                className={[
                  "px-2 py-0.5 rounded-md text-[10px] font-medium border transition-all duration-100",
                  featureArea === a
                    ? "bg-orange-500/15 text-orange-400 border-orange-500/40"
                    : "bg-transparent text-klarvo-dim border-klarvo-border/40 hover:text-klarvo-muted hover:border-klarvo-border/60",
                ].join(" ")}
              >
                {a}
              </button>
            ))}
          </div>

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
        </div>
      </div>
    </div>
  );
}
