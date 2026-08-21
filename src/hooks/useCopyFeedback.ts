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
  const generations = useRef<Map<string, number>>(new Map());
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    const timersAtMount = timers.current;
    const generationsAtMount = generations.current;
    return () => {
      alive.current = false;
      timersAtMount.forEach((timer) => window.clearTimeout(timer));
      timersAtMount.clear();
      generationsAtMount.clear();
    };
  }, []);

  const copy = useCallback(async (id: string, text: string) => {
    const existingTimer = timers.current.get(id);
    if (existingTimer !== undefined) window.clearTimeout(existingTimer);
    timers.current.delete(id);

    // A generation counter guards against a second click on the same id landing while the
    // first click's clipboard write is still pending (Task 1.3): the stale settlement below
    // recognizes it was superseded and skips registering an orphaned timer.
    const generation = (generations.current.get(id) ?? 0) + 1;
    generations.current.set(id, generation);

    let status: CopyStatus;
    try {
      await navigator.clipboard.writeText(text);
      status = "copied";
    } catch (err) {
      console.error(err);
      status = "failed";
    }

    if (!alive.current || generations.current.get(id) !== generation) return;

    setStatuses((prev) => ({ ...prev, [id]: status }));
    const timer = window.setTimeout(() => {
      setStatuses((prev) => ({ ...prev, [id]: "idle" }));
      timers.current.delete(id);
      generations.current.delete(id);
    }, COPY_FEEDBACK_MS);
    timers.current.set(id, timer);
  }, []);

  const statusOf = useCallback((id: string): CopyStatus => statuses[id] ?? "idle", [statuses]);

  return { copy, statusOf };
}
