// PreviewPanel.tsx — Story 6.1 scaffold (no content yet; 6.2 adds the live preview)
// This window is transparent, click-through, and always-on-top.
// It MUST NOT mount any subscriptions here — 6.2 wires the event listeners.
import React from "react";

const RESET_CSS: React.CSSProperties = {
  margin: 0,
  padding: 0,
  background: "transparent",
  overflow: "hidden",
  width: "100vw",
  height: "100vh",
};

export default function PreviewPanel(): React.ReactElement {
  return <div style={RESET_CSS} />;
}
