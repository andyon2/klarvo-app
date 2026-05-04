//! Pill-Bar overlay manager (Story 9.6).
//!
//! Owns the EventBus subscriber task that drives the pill-bar window visibility
//! and waveform updates. The window itself is declared in `tauri.conf.json`
//! (`transparent: true` requires a conf entry — `WebviewWindowBuilder` alone is
//! insufficient on Windows in Tauri v2, see GitHub issue #8308).

use std::collections::VecDeque;

use klarvo_core::event::Event;
use serde::Serialize;
use tauri::{AppHandle, Emitter as _, LogicalPosition, Manager as _};
use tokio::sync::broadcast;

const WINDOW_LABEL: &str = "pill-bar";
const BIN_COUNT: usize = 64;
const WINDOW_WIDTH: f64 = 320.0;
const WINDOW_HEIGHT: f64 = 48.0;
/// Margin between bottom of pill bar and bottom of primary monitor (logical px).
const BOTTOM_MARGIN: f64 = 16.0;
/// Fade-out duration matched to CSS transition in `pill-bar.html`.
const FADE_OUT_MS: u64 = 300;

pub struct PillBar<R: tauri::Runtime> {
    app: AppHandle<R>,
}

impl<R: tauri::Runtime> PillBar<R> {
    /// Position the pill-bar window (already created by Tauri from conf) at the
    /// bottom-center of the primary monitor.
    ///
    /// Fail-soft: if the window label is missing (misconfigured conf) or
    /// monitor metadata is unavailable, returns `Ok` without crashing — the
    /// recording pipeline still works, only the overlay won't show.
    pub fn new(app: &AppHandle<R>) -> tauri::Result<Self> {
        if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
            let (x, y) = pill_bar_position(app);
            if let Err(e) = win.set_position(LogicalPosition::new(x, y)) {
                tracing::warn!(error = %e, "pill-bar set_position failed; using OS default");
            }
        } else {
            tracing::warn!("pill-bar window not found; check tauri.conf.json label");
        }
        Ok(Self { app: app.clone() })
    }

    /// Spawn the EventBus subscriber task. Runs until the broadcast channel is
    /// closed (app shutdown).
    pub fn start(self, mut receiver: broadcast::Receiver<Event>) {
        let app = self.app;
        tauri::async_runtime::spawn(async move {
            let mut ring: VecDeque<f32> = VecDeque::from(vec![0.0f32; BIN_COUNT]);

            loop {
                match receiver.recv().await {
                    Ok(event) => handle_event(&app, &mut ring, event),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "PillBar lagged; skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

/// Compute bottom-center logical position for the pill-bar window.
fn pill_bar_position<R: tauri::Runtime>(app: &AppHandle<R>) -> (f64, f64) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = (logical_w - WINDOW_WIDTH) / 2.0;
        let y = logical_h - WINDOW_HEIGHT - BOTTOM_MARGIN;
        (x, y)
    } else {
        (0.0, 0.0)
    }
}

fn handle_event<R: tauri::Runtime>(
    app: &AppHandle<R>,
    ring: &mut VecDeque<f32>,
    event: Event,
) {
    match event {
        Event::RecordingStarted { .. } => {
            ring.iter_mut().for_each(|v| *v = 0.0);
            if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
                let _ = win.show();
            }
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.show", ());
        }
        Event::RecordingCompleted { .. } => {
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.fade_out", ());
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(FADE_OUT_MS)).await;
                if let Some(win) = app_clone.get_webview_window(WINDOW_LABEL) {
                    let _ = win.hide();
                }
            });
        }
        Event::AudioLevel { rms, ts_ms } => {
            ring.pop_front();
            ring.push_back(rms.clamp(0.0, 1.0));
            let bins: Vec<f32> = ring.iter().copied().collect();
            let _ = app.emit_to(
                WINDOW_LABEL,
                "pill_bar.waveform_tick",
                WaveformPayload { bins, ts_ms },
            );
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WaveformPayload {
    bins: Vec<f32>,
    ts_ms: u64,
}
