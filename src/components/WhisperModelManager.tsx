import { useState, useEffect, useCallback } from "react";
import {
  getWhisperModels,
  downloadWhisperModel,
  deleteWhisperModel,
  onModelDownloadProgress,
  onModelDownloadComplete,
  onModelDownloadError,
  type WhisperModelWithStatus,
} from "../tauri-commands";
import { LockIcon } from "./icons";

// Static metadata for all supported models.
// tiny/base removed — quality too low to represent Dikta.
const MODEL_LABELS: Record<string, string> = {
  small: "small (488 MB) — Recommended",
  medium: "medium (1.5 GB)",
  "large-v3": "large-v3 (3.1 GB)",
};

// Only small is free. medium and large-v3 require a paid license.
const PAID_MODELS = new Set(["medium", "large-v3"]);

interface DownloadState {
  bytesReceived: number;
  totalBytes: number;
}

interface WhisperModelManagerProps {
  /** Currently selected model ID from settings */
  selectedModel: string;
  /** Whether GPU acceleration is enabled */
  gpuEnabled: boolean;
  /** Called when user picks a different model */
  onModelChange: (modelId: string) => void;
  /** Called when user toggles GPU setting */
  onGpuChange: (enabled: boolean) => void;
  /** Whether user has a paid license */
  isPaid: boolean;
}

export function WhisperModelManager({
  selectedModel,
  gpuEnabled,
  onModelChange,
  onGpuChange,
  isPaid,
}: WhisperModelManagerProps) {
  const [models, setModels] = useState<WhisperModelWithStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Map from modelId -> active download progress
  const [downloading, setDownloading] = useState<Record<string, DownloadState>>({});
  // Map from modelId -> error string
  const [downloadErrors, setDownloadErrors] = useState<Record<string, string>>({});
  // Confirmation state for delete: tracks modelId awaiting confirm
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const fetchModels = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const result = await getWhisperModels();
      setModels(result);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchModels();
  }, [fetchModels]);

  // Subscribe to download events.
  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    onModelDownloadProgress((payload) => {
      setDownloading((prev) => ({
        ...prev,
        [payload.modelId]: {
          bytesReceived: payload.bytesReceived,
          totalBytes: payload.totalBytes,
        },
      }));
    }).then((fn) => { unlistenProgress = fn; });

    onModelDownloadComplete((payload) => {
      // Remove from downloading state and refresh model list.
      setDownloading((prev) => {
        const next = { ...prev };
        delete next[payload.modelId];
        return next;
      });
      setDownloadErrors((prev) => {
        const next = { ...prev };
        delete next[payload.modelId];
        return next;
      });
      fetchModels();
    }).then((fn) => { unlistenComplete = fn; });

    onModelDownloadError((payload) => {
      setDownloading((prev) => {
        const next = { ...prev };
        delete next[payload.modelId];
        return next;
      });
      setDownloadErrors((prev) => ({ ...prev, [payload.modelId]: payload.error }));
    }).then((fn) => { unlistenError = fn; });

    return () => {
      unlistenProgress?.();
      unlistenComplete?.();
      unlistenError?.();
    };
  }, [fetchModels]);

  const handleDownload = useCallback(async (modelId: string) => {
    setDownloadErrors((prev) => {
      const next = { ...prev };
      delete next[modelId];
      return next;
    });
    try {
      await downloadWhisperModel(modelId);
      // Progress events will update state -- optimistically mark as starting.
      setDownloading((prev) => ({
        ...prev,
        [modelId]: { bytesReceived: 0, totalBytes: 0 },
      }));
    } catch (err) {
      setDownloadErrors((prev) => ({
        ...prev,
        [modelId]: err instanceof Error ? err.message : String(err),
      }));
    }
  }, []);

  const handleDelete = useCallback(async (modelId: string) => {
    if (confirmDelete !== modelId) {
      setConfirmDelete(modelId);
      // Auto-reset confirm after 4 seconds.
      setTimeout(() => setConfirmDelete((cur) => cur === modelId ? null : cur), 4000);
      return;
    }
    setConfirmDelete(null);
    try {
      await deleteWhisperModel(modelId);
      fetchModels();
    } catch (err) {
      setDownloadErrors((prev) => ({
        ...prev,
        [modelId]: err instanceof Error ? err.message : String(err),
      }));
    }
  }, [confirmDelete, fetchModels]);

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    return mb >= 1 ? `${mb.toFixed(0)} MB` : `${(bytes / 1024).toFixed(0)} KB`;
  }

  const hintCls = "text-[11px] text-zinc-500 leading-relaxed";
  const LABEL_CLS = "text-[11px] font-medium text-zinc-400 uppercase tracking-widest";

  if (loading) {
    return (
      <div className="flex items-center gap-2 py-2">
        <svg className="w-4 h-4 text-zinc-500 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
        </svg>
        <span className={hintCls}>Loading models...</span>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex flex-col gap-1.5">
        <p className="text-xs text-red-400">{loadError}</p>
        <button
          onClick={fetchModels}
          className="self-start px-3 py-1.5 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 text-zinc-300 hover:bg-zinc-800/60 transition-colors"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Model selection dropdown */}
      <div className="flex items-center justify-between gap-3">
        <span className={LABEL_CLS}>Model</span>
        <select
          value={selectedModel}
          onChange={(e) => {
            const id = e.target.value;
            if (!isPaid && PAID_MODELS.has(id)) return;
            onModelChange(id);
          }}
          className="bg-[#111113] border border-zinc-800/60 rounded-lg px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-emerald-500/40 transition-colors cursor-pointer max-w-[220px]"
        >
          {Object.entries(MODEL_LABELS).map(([id, label]) => {
            const locked = !isPaid && PAID_MODELS.has(id);
            return (
              <option key={id} value={id} disabled={locked}>
                {locked ? `${label} (Paid)` : label}
              </option>
            );
          })}
        </select>
      </div>

      {/* Per-model status rows -- show only the selected model's row for compactness */}
      {models.map((model) => {
        if (model.id !== selectedModel) return null;

        const isDownloading = model.id in downloading;
        const progress = downloading[model.id];
        const hasError = !!downloadErrors[model.id];
        const isReady = model.status === "downloaded";

        const progressPct =
          isDownloading && progress && progress.totalBytes > 0
            ? Math.round((progress.bytesReceived / progress.totalBytes) * 100)
            : 0;

        return (
          <div key={model.id} className="flex flex-col gap-2">
            {/* Status + action */}
            <div className="flex items-center gap-2 flex-wrap">
              {isReady && !isDownloading && (
                <>
                  <span className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-emerald-500/15 text-emerald-400">
                    <svg className="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                    Ready
                  </span>
                  <button
                    onClick={() => handleDelete(model.id)}
                    className={[
                      "text-[11px] transition-colors",
                      confirmDelete === model.id
                        ? "text-red-400 hover:text-red-300"
                        : "text-zinc-600 hover:text-zinc-400",
                    ].join(" ")}
                  >
                    {confirmDelete === model.id ? "Confirm delete?" : "Delete"}
                  </button>
                </>
              )}

              {!isReady && !isDownloading && (
                <button
                  onClick={() => { if (!isPaid && PAID_MODELS.has(model.id)) return; handleDownload(model.id); }}
                  disabled={!isPaid && PAID_MODELS.has(model.id)}
                  title={!isPaid && PAID_MODELS.has(model.id) ? "Requires Dikta License" : undefined}
                  className={[
                    "px-3 py-1.5 rounded-lg text-xs font-medium bg-[#111113] border border-zinc-800/60 transition-colors",
                    !isPaid && PAID_MODELS.has(model.id)
                      ? "text-zinc-600 cursor-not-allowed opacity-50"
                      : "text-zinc-300 hover:bg-zinc-800/60",
                  ].join(" ")}
                >
                  {!isPaid && PAID_MODELS.has(model.id) ? (
                    <span className="flex items-center gap-1"><LockIcon className="w-3 h-3" /> Paid</span>
                  ) : (
                    <>Download ({formatBytes(model.sizeBytes)})</>
                  )}
                </button>
              )}

              {isDownloading && (
                <span className={hintCls}>Downloading...</span>
              )}
            </div>

            {/* Progress bar */}
            {isDownloading && (
              <div className="flex flex-col gap-1.5">
                <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-emerald-500/60 rounded-full transition-all duration-300"
                    style={{ width: `${progressPct}%` }}
                  />
                </div>
                {progress && progress.totalBytes > 0 && (
                  <span className={hintCls}>
                    {formatBytes(progress.bytesReceived)} / {formatBytes(progress.totalBytes)}
                  </span>
                )}
              </div>
            )}

            {/* Error */}
            {hasError && (
              <p className="text-xs text-red-400">{downloadErrors[model.id]}</p>
            )}
          </div>
        );
      })}

      {/* GPU toggle */}
      <div className="flex items-center justify-between gap-3">
        <div className="flex flex-col gap-0.5">
          <span className={LABEL_CLS}>GPU Acceleration (CUDA)</span>
          <span className={hintCls}>Requires NVIDIA GPU + CUDA Toolkit</span>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={gpuEnabled}
          onClick={() => onGpuChange(!gpuEnabled)}
          className={[
            "relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0",
            gpuEnabled ? "bg-emerald-500/40" : "bg-zinc-700",
          ].join(" ")}
        >
          <span
            className={[
              "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
              gpuEnabled ? "translate-x-4" : "",
            ].join(" ")}
          />
        </button>
      </div>
    </div>
  );
}
