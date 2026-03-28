import { useState, useEffect, useCallback } from "react";
import { isDesktop } from "../../platform";
import { isPreviewMode } from "../../tauri-commands";

/** Returns the app version string. Falls back to a hardcoded string in preview mode. */
async function getAppVersion(): Promise<string> {
  if (isPreviewMode) return "0.4.1-preview";
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return getVersion();
  } catch {
    return "0.4.1";
  }
}

/** Opens a URL in the system browser. Falls back to window.open in preview mode. */
async function openUrl(url: string): Promise<void> {
  try {
    const { openUrl: tauriOpenUrl } = await import("@tauri-apps/plugin-opener");
    await tauriOpenUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

/** Checks for app updates. Returns null in preview mode. */
async function checkForUpdate(): Promise<{ version: string; downloadAndInstall: () => Promise<void> } | null> {
  if (isPreviewMode) return null;
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    return check();
  } catch {
    return null;
  }
}

// --- Update Checker ----------------------------------------------------------

function UpdateChecker() {
  const [status, setStatus] = useState<"idle" | "checking" | "available" | "downloading" | "upToDate" | "error">("idle");
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [localAppVersion, setLocalAppVersion] = useState<string>("…");

  useEffect(() => {
    getAppVersion().then(setLocalAppVersion);
  }, []);

  const handleCheck = useCallback(async () => {
    setStatus("checking");
    setErrorMsg(null);
    try {
      const update = await checkForUpdate();
      if (update) {
        setUpdateVersion(update.version);
        setStatus("available");
      } else {
        setStatus("upToDate");
        setTimeout(() => setStatus("idle"), 3000);
      }
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, []);

  const handleInstall = useCallback(async () => {
    setStatus("downloading");
    try {
      const update = await checkForUpdate();
      if (update) {
        await update.downloadAndInstall();
      }
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, []);

  return (
    <div className="flex flex-col gap-2">
      <span className="text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest">Updates</span>
      <div className="flex items-center gap-2">
        {status === "available" ? (
          <button
            onClick={handleInstall}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-primary/10 border border-klarvo-primary/20 text-klarvo-primary hover:bg-klarvo-primary/15 transition-colors"
          >
            Install v{updateVersion}
          </button>
        ) : (
          <button
            onClick={handleCheck}
            disabled={status === "checking" || status === "downloading"}
            className="px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {status === "checking" ? "Checking..." : status === "downloading" ? "Downloading..." : status === "upToDate" ? "Up to date" : "Check for updates"}
          </button>
        )}
        <span className="text-[11px] text-klarvo-dim">v{localAppVersion}</span>
      </div>
      {errorMsg && <p className="text-[11px] text-klarvo-danger">{errorMsg}</p>}
    </div>
  );
}

// --- About Content -----------------------------------------------------------

interface AboutContentProps {
  appVersion: string;
  onRestartOnboarding?: () => void;
}

export function AboutContent({ appVersion, onRestartOnboarding }: AboutContentProps) {
  return (
    <div className="flex flex-col gap-5">
      {/* Updates — desktop only */}
      {isDesktop && <UpdateChecker />}

      {/* About */}
      <div className="flex flex-col gap-2">
        <span className="text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest">About</span>
        <p className="text-xs font-medium text-klarvo-muted">
          Klarvo{appVersion ? ` v${appVersion}` : ""}
        </p>
        <p className="text-[11px] text-klarvo-dim">Voice dictation you own.</p>
        <p className="text-[11px] text-klarvo-dim">by Andreas Nolte</p>
        <div className="flex items-center gap-2 mt-0.5">
          <button
            onClick={() => openUrl("https://github.com/andyon2/klarvo")}
            className="text-[11px] text-klarvo-muted hover:text-klarvo-text underline underline-offset-2 transition-colors"
          >
            GitHub
          </button>
          <span className="text-[11px] text-klarvo-dim">·</span>
          <span className="text-[11px] text-klarvo-dim">Source-Available</span>
        </div>
        {onRestartOnboarding && (
          <button
            onClick={onRestartOnboarding}
            className="mt-2 text-[11px] text-klarvo-dim hover:text-klarvo-muted underline underline-offset-2 transition-colors text-left"
          >
            Setup assistant restart
          </button>
        )}
      </div>
    </div>
  );
}
