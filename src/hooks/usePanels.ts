import { useState, useCallback } from "react";

export type PanelName = "settings" | "history" | "stats" | "notes" | "integrations" | "advanced";

interface PanelsState {
  showSettings: boolean;
  showHistory: boolean;
  showStats: boolean;
  showNotes: boolean;
  showIntegrations: boolean;
  showAdvanced: boolean;
}

// Callbacks invoked when a specific panel is first opened (for lazy data loading).
interface PanelOpenCallbacks {
  onOpenHistory?: () => void;
  onOpenStats?: () => void;
  onOpenNotes?: () => void;
  onOpenIntegrations?: () => void;
}

export function usePanels(callbacks: PanelOpenCallbacks = {}) {
  const [state, setState] = useState<PanelsState>({
    showSettings: false,
    showHistory: false,
    showStats: false,
    showNotes: false,
    showIntegrations: false,
    showAdvanced: false,
  });

  // Close all panels. Returns true if any panel was open.
  const closeAll = useCallback(() => {
    let wasOpen = false;
    setState((prev) => {
      wasOpen = Object.values(prev).some(Boolean);
      return {
        showSettings: false,
        showHistory: false,
        showStats: false,
        showNotes: false,
        showIntegrations: false,
        showAdvanced: false,
      };
    });
    return wasOpen;
  }, []);

  // Toggle a panel: open it and close all others.
  // When opening, fire the corresponding onOpen callback if provided.
  const toggle = useCallback((panel: PanelName) => {
    setState((prev) => {
      const isOpening = !prev[`show${panel.charAt(0).toUpperCase()}${panel.slice(1)}` as keyof PanelsState];

      if (isOpening) {
        if (panel === "history") callbacks.onOpenHistory?.();
        if (panel === "stats") callbacks.onOpenStats?.();
        if (panel === "notes") callbacks.onOpenNotes?.();
        if (panel === "integrations") callbacks.onOpenIntegrations?.();
      }

      return {
        showSettings: panel === "settings" ? !prev.showSettings : false,
        showHistory: panel === "history" ? !prev.showHistory : false,
        showStats: panel === "stats" ? !prev.showStats : false,
        showNotes: panel === "notes" ? !prev.showNotes : false,
        showIntegrations: panel === "integrations" ? !prev.showIntegrations : false,
        showAdvanced: panel === "advanced" ? !prev.showAdvanced : false,
      };
    });
  }, [callbacks]);

  const close = useCallback((panel: PanelName) => {
    const key = `show${panel.charAt(0).toUpperCase()}${panel.slice(1)}` as keyof PanelsState;
    setState((prev) => ({ ...prev, [key]: false }));
  }, []);

  const anyOpen = Object.values(state).some(Boolean);

  return { ...state, toggle, close, closeAll, anyOpen };
}
