import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { isPreviewMode } from "./tauri-commands";

// In preview mode window.__TAURI_INTERNALS__ is absent, so we cannot call
// getCurrentWindow(). We default to "main" so the App component is rendered.
let label = "main";

if (!isPreviewMode) {
  // Dynamic import keeps the Tauri window API out of the module-evaluation
  // critical path when running in a plain browser.
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  label = getCurrentWindow().label;
}

// FloatingBar and PreviewPanel are Tauri-only concepts (separate overlay windows).
// In preview mode we always render the main App.
let Root: React.ComponentType;
if (label === "bar" && !isPreviewMode) {
  const { default: FloatingBar } = await import("./FloatingBar");
  Root = FloatingBar;
} else if (label === "preview" && !isPreviewMode) {
  const { default: PreviewPanel } = await import("./PreviewPanel");
  Root = PreviewPanel;
} else {
  Root = App;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
