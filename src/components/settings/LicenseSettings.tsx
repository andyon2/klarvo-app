import { useState, useEffect, useCallback, useRef } from "react";
import { isPreviewMode } from "../../tauri-commands";
import type { ParsedLicenseStatus } from "../../types";
import { isMobile } from "../../platform";
import { INPUT_CLS, INPUT_CLS_M } from "../ui";

// --- Helpers ------------------------------------------------------------------

/** Opens a URL in the system browser. Falls back to window.open in preview mode. */
async function openUrl(url: string): Promise<void> {
  try {
    const { openUrl: tauriOpenUrl } = await import("@tauri-apps/plugin-opener");
    await tauriOpenUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

// Auto-formats a license key input: uppercase, inserts dashes after every 4 chars
// in the payload section (after "KLARVO-").
export function formatLicenseKeyInput(raw: string): string {
  const trimmed = raw.trim().toUpperCase();
  if (trimmed.length === 0) return "";

  // Detect HMAC key prefix: KLARVO / VOXLIT / DIKTA → format as PREFIX-XXXX-XXXX-XXXX-XXXX.
  const stripped = trimmed.replace(/[^A-Z0-9]/g, "");
  const isHmacKey =
    stripped.startsWith("KLARVO") ||
    stripped.startsWith("VOXLIT") ||
    stripped.startsWith("DIKTA");

  if (!isHmacKey) {
    // Lemon Squeezy keys (UUIDs) or unknown formats: pass through as-is.
    return trimmed;
  }

  // HMAC key formatting: PREFIX-XXXX-XXXX-XXXX-XXXX
  let prefix = "";
  let prefixLen = 0;
  if (stripped.startsWith("KLARVO")) { prefix = "KLARVO"; prefixLen = 6; }
  else if (stripped.startsWith("VOXLIT")) { prefix = "VOXLIT"; prefixLen = 6; }
  else if (stripped.startsWith("DIKTA")) { prefix = "DIKTA"; prefixLen = 5; }

  const body = stripped.slice(prefixLen);
  const chunks: string[] = [];
  for (let i = 0; i < body.length && i < 16; i += 4) {
    chunks.push(body.slice(i, i + 4));
  }
  return prefix + (chunks.length > 0 ? "-" + chunks.join("-") : "");
}

export function formatGraceDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
}

export const LOCKED_FEATURES = [
  "Offline HD Models",
  "Command Mode",
  "Snippets",
  "Unlimited Dictionary",
  "Voice Notes",
  "Live Transcription",
  "Cleanup Instructions",
  "Cross-Device Sync",
  "Advanced Statistics",
];

// --- LicenseKeyInput ----------------------------------------------------------

function LicenseKeyInput({
  value, onChange, onActivate, loading, error,
}: {
  value: string;
  onChange: (v: string) => void;
  onActivate: () => void;
  loading: boolean;
  error: string | null;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex gap-2">
        <input
          type="text"
          spellCheck={false}
          autoComplete="off"
          placeholder="Enter your license key"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !loading && onActivate()}
          maxLength={50} // HMAC: 26 chars, LS UUID: 36 chars
          className={[
            "flex-1 font-mono tracking-widest",
            isMobile ? INPUT_CLS_M : INPUT_CLS,
          ].join(" ")}
        />
        <button
          onClick={onActivate}
          disabled={loading || !value.trim()}
          className={[
            "rounded-lg font-medium bg-klarvo-teal/10 border border-klarvo-teal/20 text-klarvo-teal",
            "hover:bg-klarvo-teal/15 disabled:opacity-40 disabled:cursor-not-allowed transition-colors",
            isMobile ? "px-4 py-2.5 text-sm" : "px-3 py-2 text-xs",
          ].join(" ")}
        >
          {loading ? "..." : "Activate"}
        </button>
      </div>
      {error && (
        <p className={["text-klarvo-danger", isMobile ? "text-sm" : "text-xs"].join(" ")}>
          {error}
        </p>
      )}
    </div>
  );
}

// --- LicenseSection -----------------------------------------------------------

export interface LicenseSectionProps {
  licenseStatus: ParsedLicenseStatus;
  licenseSource: string;
  onValidate: (key: string) => Promise<string | null>;
  onRemove: () => Promise<void>;
  onDeactivate: () => Promise<string | null>;
  licenseLoading: boolean;
}

export function LicenseSection({ licenseStatus, licenseSource, onValidate, onRemove, onDeactivate, licenseLoading }: LicenseSectionProps) {
  const [keyInput, setKeyInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [confirmDeactivate, setConfirmDeactivate] = useState(false);
  const deactivateTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const confirmTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleKeyChange = useCallback((raw: string) => {
    setKeyInput(formatLicenseKeyInput(raw));
    setError(null);
  }, []);

  const handleActivate = useCallback(async () => {
    const trimmed = keyInput.trim();
    if (!trimmed) return;
    setError(null);
    const err = await onValidate(trimmed);
    if (err) {
      setError(err);
    } else {
      setKeyInput("");
    }
  }, [keyInput, onValidate]);

  const handleRemoveClick = useCallback(() => {
    if (!confirmRemove) {
      setConfirmRemove(true);
      confirmTimerRef.current = setTimeout(() => setConfirmRemove(false), 4000);
      return;
    }
    if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current);
    setConfirmRemove(false);
    onRemove();
  }, [confirmRemove, onRemove]);

  const handleDeactivateClick = useCallback(async () => {
    if (!confirmDeactivate) {
      setConfirmDeactivate(true);
      deactivateTimerRef.current = setTimeout(() => setConfirmDeactivate(false), 4000);
      return;
    }
    if (deactivateTimerRef.current) clearTimeout(deactivateTimerRef.current);
    setConfirmDeactivate(false);
    const err = await onDeactivate();
    if (err) setError(err);
  }, [confirmDeactivate, onDeactivate]);

  // Cleanup timers on unmount.
  useEffect(() => {
    return () => {
      if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current);
      if (deactivateTimerRef.current) clearTimeout(deactivateTimerRef.current);
    };
  }, []);

  const isLicensed = licenseStatus.type === "licensed";
  const isTrial = licenseStatus.type === "trial";
  const isGrace = licenseStatus.type === "grace_period";
  const isUnlicensed = licenseStatus.type === "unlicensed";

  // Shared trial-badge label used in both trial and grace period states.
  const trialBadgeLabel = isPreviewMode
    ? "Trial — Preview Mode"
    : (() => {
        if (!licenseStatus.trialUntil) return "Trial";
        const daysLeft = Math.max(0, Math.ceil((licenseStatus.trialUntil - Date.now() / 1000) / 86400));
        return daysLeft === 0 ? "Trial — expires today" : `Trial — ${daysLeft} day${daysLeft === 1 ? "" : "s"} left`;
      })();

  // Shared remove/deactivate buttons used in licensed and grace period states.
  const removeDeactivateButtons = (
    <div className="flex flex-col gap-1">
      <button
        onClick={handleRemoveClick}
        disabled={licenseLoading}
        className={[
          "self-start transition-colors disabled:opacity-40",
          isMobile ? "text-sm" : "text-[11px]",
          confirmRemove ? "text-klarvo-danger hover:text-klarvo-danger-hi" : "text-klarvo-amber/80 hover:text-klarvo-amber",
        ].join(" ")}
      >
        {confirmRemove ? "Click again to confirm removal" : "Remove License"}
      </button>
      {licenseSource === "lemon_squeezy" && (
        <button
          onClick={handleDeactivateClick}
          disabled={licenseLoading}
          className={[
            "self-start transition-colors disabled:opacity-40",
            isMobile ? "text-sm" : "text-[11px]",
            confirmDeactivate ? "text-klarvo-danger hover:text-klarvo-danger-hi" : "text-klarvo-amber/80 hover:text-klarvo-amber",
          ].join(" ")}
        >
          {licenseLoading ? "Deactivating..." : confirmDeactivate ? "Click again to confirm" : "Deactivate License (free device slot)"}
        </button>
      )}
    </div>
  );

  return (
    <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">

      {/* Licensed state */}
      {isLicensed && (
        <>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-success/20 text-klarvo-success">
              <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M20 6L9 17l-5-5" />
              </svg>
              Licensed
            </span>
          </div>
          <p className={isMobile ? "text-sm text-klarvo-muted" : "text-xs text-klarvo-muted"}>All features unlocked.</p>
          {removeDeactivateButtons}
        </>
      )}

      {/* Trial state — no remove button, show key input and upsell link */}
      {isTrial && (
        <>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-teal/15 text-klarvo-teal">
              {trialBadgeLabel}
            </span>
          </div>
          <LicenseKeyInput
            value={keyInput}
            onChange={handleKeyChange}
            onActivate={handleActivate}
            loading={licenseLoading}
            error={error}
          />
          <button
            onClick={() => openUrl("https://klarvo.app")}
            className={[
              "self-start transition-colors",
              isMobile ? "text-sm" : "text-[11px]",
              "text-klarvo-muted hover:text-klarvo-text underline underline-offset-2",
            ].join(" ")}
          >
            Unlock permanently for €29 &rarr; klarvo.app
          </button>
        </>
      )}

      {/* Grace period state — had a key, cache expired, needs revalidation */}
      {isGrace && (
        <>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-amber/20 text-klarvo-amber">
              Grace Period
            </span>
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-teal/15 text-klarvo-teal">
              {trialBadgeLabel}
            </span>
          </div>
          {licenseStatus.graceUntil && (
            <p className={isMobile ? "text-sm text-klarvo-amber/80" : "text-xs text-klarvo-amber/80"}>
              License expires on {formatGraceDate(licenseStatus.graceUntil)}
            </p>
          )}
          <p className={isMobile ? "text-sm text-klarvo-muted" : "text-[11px] text-klarvo-muted"}>
            Your license key needs to be revalidated soon. Connect to the internet or enter your key again.
          </p>
          {removeDeactivateButtons}
        </>
      )}

      {/* Free Tier (unlicensed) state */}
      {isUnlicensed && (
        <>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-elevated text-klarvo-muted">
              Free Tier
            </span>
          </div>
          <LicenseKeyInput
            value={keyInput}
            onChange={handleKeyChange}
            onActivate={handleActivate}
            loading={licenseLoading}
            error={error}
          />
          <div className="flex flex-wrap gap-1.5 mt-0.5">
            {LOCKED_FEATURES.map((f) => (
              <span key={f} className="rounded-full px-2 py-0.5 text-[11px] font-medium bg-klarvo-elevated text-klarvo-muted">
                {f}
              </span>
            ))}
          </div>
          <button
            onClick={() => openUrl("https://klarvo.app")}
            className={[
              "self-start transition-colors",
              isMobile ? "text-sm" : "text-[11px]",
              "text-klarvo-muted hover:text-klarvo-text underline underline-offset-2",
            ].join(" ")}
          >
            Get a license at klarvo.app
          </button>
        </>
      )}
    </div>
  );
}
