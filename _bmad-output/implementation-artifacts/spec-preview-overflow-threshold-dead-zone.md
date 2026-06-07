---
title: 'Fix overflow threshold dead-zone in preview scroll detection'
type: 'bugfix'
created: '2026-06-08'
status: 'done'
route: 'one-shot'
---

# Fix overflow threshold dead-zone in preview scroll detection

## Intent

**Problem:** The Task 7.2 `useLayoutEffect` in `PreviewPanel.tsx` compared `scrollHeight` against `clampedMaxHeightRef.current` — the *requested* window height (e.g. 818 px) computed in `runShowSequence`. When the OS clamps the actual window smaller (e.g. 661 px clientHeight), the threshold is ~150 px too high, so `panelScrolls` stays false and the card becomes non-scrollable before content first clips at the real height cap.

**Approach:** Replace the threshold with the card element's actual rendered height (`previewPanelRef.current.clientHeight + 1`), which is the canonical overflow check and is always consistent with what the browser has rendered.

## Suggested Review Order

- [`src/PreviewPanel.tsx:361`](../../src/PreviewPanel.tsx) — single-line threshold change in Task 7.2 `useLayoutEffect`
