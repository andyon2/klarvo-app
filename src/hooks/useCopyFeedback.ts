// useCopyFeedback — per-control "Copied" / "Copy failed" label state for Copy buttons.
// State is keyed by a caller-supplied id so multiple Copy controls on screen never share state (AC2).

import { useCallback, useEffect, useRef, useState } from "react";

// Andi's 2026-08-19 decision (sprint-change-proposal-2026-08-19.md): the Copy confirmation
// holds for 1500 ms, the delete-undo window holds for 6000 ms.
export const COPY_FEEDBACK_MS = 1500;
export const UNDO_WINDOW_MS = 6000;

export type CopyStatus = "idle" | "copied" | "failed";

export function copyLabel(status: CopyStatus, restingLabel: string): string {
  if (status === "copied") return "Copied";
  if (status === "failed") return "Copy failed";
  return restingLabel;
}

export function copyColorClassName(status: CopyStatus, restingClassName: string): string {
  if (status === "copied") return "text-klarvo-success";
  if (status === "failed") return "text-klarvo-danger";
  return restingClassName;
}

export function useCopyFeedback() {
  const [statuses, setStatuses] = useState<Record<string, CopyStatus>>({});
  const timers = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    const timersAtMount = timers.current;
    return () => {
      timersAtMount.forEach((timer) => window.clearTimeout(timer));
      timersAtMount.clear();
    };
  }, []);

  const copy = useCallback(async (id: string, text: string) => {
    const existing = timers.current.get(id);
    if (existing !== undefined) window.clearTimeout(existing);

    try {
      await navigator.clipboard.writeText(text);
      setStatuses((prev) => ({ ...prev, [id]: "copied" }));
    } catch {
      setStatuses((prev) => ({ ...prev, [id]: "failed" }));
    }

    const timer = window.setTimeout(() => {
      setStatuses((prev) => ({ ...prev, [id]: "idle" }));
      timers.current.delete(id);
    }, COPY_FEEDBACK_MS);
    timers.current.set(id, timer);
  }, []);

  const statusOf = useCallback((id: string): CopyStatus => statuses[id] ?? "idle", [statuses]);

  return { copy, statusOf };
}
