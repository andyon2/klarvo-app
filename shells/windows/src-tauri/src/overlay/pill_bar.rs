//! Pill-Bar overlay manager (Story 9.6).
//!
//! Owns the EventBus subscriber task that drives the pill-bar window visibility
//! and waveform updates. The window itself is declared in `tauri.conf.json`
//! (`transparent: true` requires a conf entry — `WebviewWindowBuilder` alone is
//! insufficient on Windows in Tauri v2, see GitHub issue #8308).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use klarvo_core::event::Event;
use klarvo_core::settings::Settings;
use serde::Serialize;
use tauri::{AppHandle, Emitter as _, LogicalPosition, LogicalSize, Manager as _};
use tokio::sync::broadcast;

const WINDOW_LABEL: &str = "pill-bar";
const BIN_COUNT: usize = 64;
const WINDOW_WIDTH: f64 = 320.0;
const WINDOW_HEIGHT: f64 = 48.0;
const LIVE_PREVIEW_WIDTH: f64 = 480.0;
const LIVE_PREVIEW_HEIGHT: f64 = 84.0;
/// Margin between bottom of pill bar and bottom of primary monitor (logical px).
const BOTTOM_MARGIN: f64 = 16.0;
/// Fade-out duration matched to CSS transition in `pill-bar.html`. Drift between
/// these two values produces either a visible-pop (Rust hides before CSS fade
/// completes) or a transparent click-blocker (CSS fade completes but Rust never
/// hides). Keep the two in sync.
const FADE_OUT_MS: u64 = 300;
/// Debounce window for position saves triggered by WindowEvent::Moved.
const DRAG_DEBOUNCE_MS: u64 = 300;

pub struct PillBar<R: tauri::Runtime> {
    app: AppHandle<R>,
}

impl<R: tauri::Runtime> PillBar<R> {
    /// Position the pill-bar window (already created by Tauri from conf) at the
    /// bottom-center of the primary monitor, and register the drag-position
    /// listener for Story 11.3.
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

            // Story 11.3: debounced position-save on drag
            let app_for_drag = app.clone();
            let pending_pos: Arc<Mutex<Option<(f64, f64)>>> = Arc::new(Mutex::new(None));
            let save_pending = Arc::new(AtomicBool::new(false));

            let pending_pos_clone = Arc::clone(&pending_pos);
            let save_pending_clone = Arc::clone(&save_pending);

            win.on_window_event(move |event| {
                if let tauri::WindowEvent::Moved(phys_pos) = event {
                    let scale = app_for_drag
                        .primary_monitor()
                        .ok()
                        .flatten()
                        .map(|m| m.scale_factor())
                        .unwrap_or(1.0);
                    let lx = phys_pos.x as f64 / scale;
                    let ly = phys_pos.y as f64 / scale;
                    {
                        let mut guard = pending_pos_clone.lock().unwrap();
                        *guard = Some((lx, ly));
                    }
                    if save_pending_clone
                        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        let app2 = app_for_drag.clone();
                        let pp = Arc::clone(&pending_pos_clone);
                        let flag = Arc::clone(&save_pending_clone);
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(DRAG_DEBOUNCE_MS))
                                .await;
                            flag.store(false, Ordering::SeqCst);
                            let pos = pp.lock().unwrap().take();
                            if let Some((x, y)) = pos {
                                if let Err(e) =
                                    app2.state::<Settings>().set_pill_bar_position(x, y)
                                {
                                    tracing::warn!(error = %e, "pill-bar position save failed");
                                }
                            }
                        });
                    }
                }
            });
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
            // Hide-task generation counter: each `RecordingStarted` increments
            // this; pending hide-tasks compare their captured snapshot before
            // calling `win.hide()`. A hide-task whose generation no longer
            // matches the current epoch is stale (a new recording started
            // within the 300 ms fade window) and must no-op.
            let fade_epoch = Arc::new(AtomicU64::new(0));

            loop {
                match receiver.recv().await {
                    Ok(event) => handle_event(&app, &mut ring, &fade_epoch, event),
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
///
/// Negative coordinates are clamped to 0 to keep the pill on-screen on
/// portrait/small-monitor configurations where `logical_w < WINDOW_WIDTH`.
fn pill_bar_position<R: tauri::Runtime>(app: &AppHandle<R>) -> (f64, f64) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = ((logical_w - WINDOW_WIDTH) / 2.0).max(0.0);
        let y = (logical_h - WINDOW_HEIGHT - BOTTOM_MARGIN).max(0.0);
        (x, y)
    } else {
        (0.0, 0.0)
    }
}

fn handle_event<R: tauri::Runtime>(
    app: &AppHandle<R>,
    ring: &mut VecDeque<f32>,
    fade_epoch: &Arc<AtomicU64>,
    event: Event,
) {
    match event {
        Event::RecordingStarted { .. } => {
            // Bump the fade-epoch so any pending hide-task from a previous
            // session no-ops when its delay elapses.
            fade_epoch.fetch_add(1, Ordering::SeqCst);
            ring.iter_mut().for_each(|v| *v = 0.0);
            if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
                // 1. Reset to recording size (in case previous session was in LP mode)
                if let Err(e) = win.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)) {
                    tracing::warn!(error = %e, "pill-bar size reset failed");
                }
                // 2. Restore last dragged position, or fall back to bottom-center
                let (x, y) = match app.state::<Settings>().pill_bar_position() {
                    Ok(Some((x, y))) => (x, y),
                    _ => pill_bar_position(app),
                };
                if let Err(e) = win.set_position(LogicalPosition::new(x, y)) {
                    tracing::warn!(error = %e, "pill-bar set_position failed");
                }
                // 3. Show
                if let Err(e) = win.show() {
                    tracing::warn!(error = %e, "pill-bar show failed");
                }
            }
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.show", ());
        }
        Event::RecordingCompleted { .. } => {
            schedule_fade_and_hide(app, fade_epoch, "natural");
        }
        Event::AudioLevel { rms, ts_ms } => {
            // NaN/Inf at the source would propagate through `clamp` and serialize
            // to JSON as `null`, which the JS-side now zero-fills, but cleaner to
            // sanitize here too so the ring buffer never holds non-finite floats.
            let v = if rms.is_finite() { rms.clamp(0.0, 1.0) } else { 0.0 };
            ring.pop_front();
            ring.push_back(v);
            let bins: Vec<f32> = ring.iter().copied().collect();
            let _ = app.emit_to(
                WINDOW_LABEL,
                "pill_bar.waveform_tick",
                WaveformPayload { bins, ts_ms },
            );
        }
        Event::RecordingAborted { .. } => {
            // Fade out identically to RecordingCompleted. No epoch bump — abort
            // ends the current session, does not start a new one.
            schedule_fade_and_hide(app, fade_epoch, "abort");
        }
        _ => {}
    }
}

/// Emit `pill_bar.fade_out` and schedule a `win.hide()` after `FADE_OUT_MS`.
///
/// The hide is gated on a `fade_epoch` snapshot: if a new recording starts within
/// the fade window (which bumps the epoch), the scheduled hide no-ops so the
/// freshly-shown pill-bar is not torn down. `path_label` is included in any
/// hide-failure warning to distinguish natural-completion from abort paths.
fn schedule_fade_and_hide<R: tauri::Runtime>(
    app: &AppHandle<R>,
    fade_epoch: &Arc<AtomicU64>,
    path_label: &'static str,
) {
    let _ = app.emit_to(WINDOW_LABEL, "pill_bar.fade_out", ());
    let app_clone = app.clone();
    let epoch_snapshot = fade_epoch.load(Ordering::SeqCst);
    let epoch_clone = Arc::clone(fade_epoch);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(FADE_OUT_MS)).await;
        if epoch_clone.load(Ordering::SeqCst) != epoch_snapshot {
            return;
        }
        if let Some(win) = app_clone.get_webview_window(WINDOW_LABEL) {
            if let Err(e) = win.hide() {
                tracing::warn!(error = %e, path = path_label, "pill-bar hide failed");
            }
        }
    });
}

/// Story 11.3: LP-resize test trigger; wired to pipeline in 11.4.
#[tauri::command]
#[specta::specta]
pub async fn dev_pill_bar_enter_live_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        win.set_size(LogicalSize::new(LIVE_PREVIEW_WIDTH, LIVE_PREVIEW_HEIGHT))
            .map_err(|e| e.to_string())?;
        app.emit_to(WINDOW_LABEL, "pill_bar.enter_live_preview", ())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct WaveformPayload {
    bins: Vec<f32>,
    ts_ms: u64,
}
