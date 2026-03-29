import { useState, useEffect, useCallback } from "react";
import {
  getLlmModelStatus,
  downloadLlmModel,
  deleteLlmModel,
  onLlmModelDownloadProgress,
  onLlmModelDownloadComplete,
  onLlmModelDownloadError,
  type LlmModelStatus,
} from "../tauri-commands";

const MODEL_NAME = "Qwen2.5-1.5B (Offline Cleanup)";
const MODEL_SIZE_BYTES = 1_100_000_000; // ~1.1 GB
const MODEL_SIZE_LABEL = "~1.1 GB";

interface DownloadProgress {
  bytesReceived: number;
  totalBytes: number;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 MB";
  const mb = bytes / (1024 * 1024);
  return mb >= 1 ? `${mb.toFixed(0)} MB` : `${(bytes / 1024).toFixed(0)} KB`;
}

export function LlmModelManager() {
  const [status, setStatus] = useState<LlmModelStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<DownloadProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const fetchStatus = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const result = await getLlmModelStatus();
      setStatus(result);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  // Subscribe to download events.
  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    onLlmModelDownloadProgress((payload) => {
      setDownloading({
        bytesReceived: payload.bytesReceived,
        totalBytes: payload.totalBytes,
      });
    }).then((fn) => { unlistenProgress = fn; });

    onLlmModelDownloadComplete(() => {
      setDownloading(null);
      setDownloadError(null);
      fetchStatus();
    }).then((fn) => { unlistenComplete = fn; });

    onLlmModelDownloadError((payload) => {
      setDownloading(null);
      setDownloadError(payload.error);
    }).then((fn) => { unlistenError = fn; });

    return () => {
      unlistenProgress?.();
      unlistenComplete?.();
      unlistenError?.();
    };
  }, [fetchStatus]);

  const handleDownload = useCallback(async () => {
    setDownloadError(null);
    try {
      await downloadLlmModel();
      // Optimistically mark as starting -- progress events will update.
      setDownloading({ bytesReceived: 0, totalBytes: MODEL_SIZE_BYTES });
    } catch (err) {
      setDownloadError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const handleDelete = useCallback(async () => {
    if (!confirmDelete) {
      setConfirmDelete(true);
      setTimeout(() => setConfirmDelete(false), 4000);
      return;
    }
    setConfirmDelete(false);
    try {
      await deleteLlmModel();
      fetchStatus();
    } catch (err) {
      setDownloadError(err instanceof Error ? err.message : String(err));
    }
  }, [confirmDelete, fetchStatus]);

  const hintCls = "text-[11px] text-klarvo-muted leading-relaxed";

  if (loading) {
    return (
      <div className="flex items-center gap-2 py-1">
        <svg className="w-3.5 h-3.5 text-klarvo-dim animate-spin flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
        </svg>
        <span className={hintCls}>Checking model status...</span>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex flex-col gap-1.5">
        <p className="text-xs text-klarvo-danger">{loadError}</p>
        <button
          onClick={fetchStatus}
          className="self-start px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 transition-colors"
        >
          Retry
        </button>
      </div>
    );
  }

  const isReady = status?.downloaded ?? false;
  const isDownloading = downloading !== null;

  const progressPct =
    isDownloading && downloading.totalBytes > 0
      ? Math.round((downloading.bytesReceived / downloading.totalBytes) * 100)
      : 0;

  const readySizeLabel = status?.sizeBytes
    ? formatBytes(status.sizeBytes)
    : MODEL_SIZE_LABEL;

  return (
    <div className="flex flex-col gap-2 mt-1 px-3 py-2.5 rounded-lg bg-klarvo-surface/30 border border-klarvo-border/30">
      {/* Model name row with status */}
      <div className="flex items-center justify-between gap-3 flex-wrap">
        <span className="text-xs font-medium text-klarvo-muted">{MODEL_NAME}</span>

        {/* Not downloaded, not downloading */}
        {!isReady && !isDownloading && (
          <div className="flex items-center gap-2 flex-wrap">
            <span className={hintCls}>{MODEL_SIZE_LABEL}</span>
            <button
              onClick={handleDownload}
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 transition-colors"
            >
              Download
            </button>
          </div>
        )}

        {/* Ready */}
        {isReady && !isDownloading && (
          <div className="flex items-center gap-2 flex-wrap">
            <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-klarvo-primary/15 text-klarvo-primary">
              <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M20 6L9 17l-5-5" />
              </svg>
              Ready ({readySizeLabel})
            </span>
            <button
              onClick={handleDelete}
              className={[
                "text-[11px] transition-colors",
                confirmDelete
                  ? "text-klarvo-danger hover:text-red-300"
                  : "text-klarvo-dim hover:text-klarvo-muted",
              ].join(" ")}
            >
              {confirmDelete ? "Confirm delete?" : "Delete Model"}
            </button>
          </div>
        )}

        {/* Downloading -- label on same row as name */}
        {isDownloading && (
          <span className={hintCls}>
            {progressPct}% ({formatBytes(downloading.bytesReceived)} / {formatBytes(downloading.totalBytes > 0 ? downloading.totalBytes : MODEL_SIZE_BYTES)})
          </span>
        )}
      </div>

      {/* Progress bar (only while downloading) */}
      {isDownloading && (
        <div className="w-full h-1.5 bg-klarvo-elevated rounded-full overflow-hidden">
          <div
            className="h-full bg-klarvo-primary/70 rounded-full transition-all duration-300"
            style={{ width: `${progressPct}%` }}
          />
        </div>
      )}

      {/* Download error */}
      {downloadError && (
        <div className="flex items-center justify-between gap-2">
          <p className="text-xs text-klarvo-danger">{downloadError}</p>
          <button
            onClick={handleDownload}
            className="text-[11px] text-klarvo-muted hover:text-klarvo-text transition-colors"
          >
            Retry
          </button>
        </div>
      )}
    </div>
  );
}
