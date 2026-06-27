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

// Mirror this window's console.* into Klarvo.log so overlay-window output
// is inspectable. Must run before the component mounts so its first logs are captured.
installConsoleBridge(label);

// Story 10-1: bar window is native Win32 (no React entry point).
// Story 10-2: preview is native Win32 (PreviewPanel.tsx removed).
// Only the "main" window remains here; always render App.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
