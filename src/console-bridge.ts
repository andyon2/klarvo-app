// console-bridge.ts — forward this window's console.* to the Rust log file.
//
// The "bar" and "preview" overlay windows are transparent / click-through / tiny,
// so their webview devtools cannot be opened interactively and their console
// output is otherwise invisible. This bridge mirrors console.log/info/warn/error
// to the `frontend_log` Tauri command, which writes them to Klarvo.log — the only
// console we can inspect for those windows (and remotely from WSL during a Windows
// smoke). The original console methods still run, so devtools are unaffected.
//
// Local-only: lines go to the on-disk log, never the network (BYOK / no-telemetry).

import { invoke } from "@tauri-apps/api/core";
import { isPreviewMode } from "./tauri-commands";

let installed = false;

type Level = "log" | "info" | "warn" | "error";

function stringifyArg(a: unknown): string {
  if (typeof a === "string") return a;
  try {
    return JSON.stringify(a);
  } catch {
    return String(a);
  }
}

/**
 * Patches console.log/info/warn/error on this window to also forward to the Rust
 * log via `frontend_log`. Idempotent; no-op in browser preview mode (no Tauri).
 *
 * @param label the window label (e.g. "main", "bar", "preview"), used to tag lines.
 */
export function installConsoleBridge(label: string): void {
  if (installed || isPreviewMode) return;
  installed = true;

  const levels: Level[] = ["log", "info", "warn", "error"];
  // Synchronous re-entrancy guard so a console call made while we are forwarding
  // does not recurse (the forwarded invoke is fire-and-forget, errors swallowed).
  let forwarding = false;

  for (const level of levels) {
    const original = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      original(...args);
      if (forwarding) return;
      forwarding = true;
      try {
        const message = args.map(stringifyArg).join(" ");
        const lvl = level === "log" ? "info" : level;
        void invoke("frontend_log", { label, level: lvl, message }).catch(() => {});
      } catch {
        // Never let logging break the app.
      } finally {
        forwarding = false;
      }
    };
  }
}
