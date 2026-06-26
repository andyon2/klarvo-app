import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { isPreviewMode } from "./tauri-commands";
import { installConsoleBridge } from "./console-bridge";

// In preview mode window.__TAURI_INTERNALS__ is absent, so we cannot call
// getCurrentWindow(). We default to "main" so the App component is rendered.
let label = "main";

if (!isPreviewMode) {
  // Dynamic import keeps the Tauri window API out of the module-evaluation
  // critical path when running in a plain browser.
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  label = getCurrentWindow().label;
}

// Mirror this window's console.* into Klarvo.log so overlay-window (bar/preview)
// output is inspectable — they have no reachable devtools. Must run before the
// component mounts so its first logs are captured.
installConsoleBridge(label);

// PreviewPanel is a Tauri-only concept (separate overlay window).
// The pill/bar is now a native Win32 window (Story 10-1) — no React entry point.
// In preview mode we always render the main App.
let Root: React.ComponentType;
if (label === "preview" && !isPreviewMode) {
  const { default: PreviewPanel } = await import("./PreviewPanel");
  Root = PreviewPanel;
} else {
  Root = App;
}

// The overlay-window listen() subscriptions each return an unlisten cleanup
// (`return () => unlisten.then(fn => fn())`), so StrictMode's dev-only double-mount
// is handled correctly. (The preview's "no events" bug was a capability + geometry
// issue, not a StrictMode double-subscribe — see create_preview_window.)
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
