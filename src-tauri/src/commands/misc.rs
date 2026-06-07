//! Tauri commands that don't fit neatly into a single category:
//! profiles, snippets, sync, paste and window-management helpers.

use tauri::{AppHandle, Manager, State};

use crate::config::{AppProfile, TextSnippet};
use crate::license::LicensedFeature;
use crate::require_license;
use crate::paste::{capture_foreground_window, create_paste_handler};
use crate::sync;
use crate::AppState;

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// Returns the app-specific profiles list.
///
/// Requires a paid license (App Profiles is a paid feature).
#[tauri::command]
pub fn get_profiles(state: State<'_, AppState>) -> Result<Vec<AppProfile>, String> {
    require_license!(state, LicensedFeature::AppProfiles);
    let cfg = crate::lock!(state.inner().config)?;
    Ok(cfg.profiles.clone())
}

/// Replaces the full profiles list and persists to disk.
///
/// Requires a paid license (App Profiles is a paid feature).
#[tauri::command]
pub fn save_profiles(
    state: State<'_, AppState>,
    profiles: Vec<AppProfile>,
) -> Result<(), String> {
    require_license!(state, LicensedFeature::AppProfiles);
    state
        .inner()
        .save_config_locked("profiles", |cfg| cfg.profiles = profiles)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Snippets
// ---------------------------------------------------------------------------

/// Returns all user-defined text snippets from config.
///
/// Requires a paid license (Text Snippets is a paid feature).
#[tauri::command]
pub fn get_snippets(state: State<'_, AppState>) -> Result<Vec<TextSnippet>, String> {
    require_license!(state, LicensedFeature::Snippets);
    let cfg = crate::lock!(state.inner().config)?;
    Ok(cfg.snippets.clone())
}

/// Replaces the full snippet list and persists to disk.
///
/// The caller supplies the complete list; individual add/remove operations
/// are handled on the frontend and the resulting list is sent here.
///
/// Requires a paid license (Text Snippets is a paid feature).
#[tauri::command]
pub fn save_snippets(
    state: State<'_, AppState>,
    snippets: Vec<TextSnippet>,
) -> Result<(), String> {
    require_license!(state, LicensedFeature::Snippets);
    state
        .inner()
        .save_config_locked("snippets", |cfg| cfg.snippets = snippets)?;
    Ok(())
}

/// Pastes snippet content into the previously focused window.
///
/// Reuses the same foreground-window capture and paste infrastructure as the
/// dictation pipeline. The caller is responsible for supplying the content
/// string -- no look-up by name happens here.
///
/// If `prev_foreground_hwnd` is `None` (e.g. no dictation has run yet),
/// the paste handler falls back to clipboard-based pasting into whatever
/// window currently has focus.
#[tauri::command]
pub async fn paste_snippet(state: State<'_, AppState>, content: String) -> Result<(), String> {
    let prev_hwnd = *state
        .inner()
        .prev_foreground_hwnd
        .lock()
        .map_err(|_| "Internal state lock poisoned".to_string())?;

    // Capture current foreground window if we have no stored one from recording.
    let hwnd = prev_hwnd.or_else(capture_foreground_window);

    let paste_handler = create_paste_handler(hwnd);
    paste_handler
        .paste(&content)
        .map_err(|e| format!("Failed to paste snippet: {e}"))
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// Syncs history with the remote Turso database.
///
/// Returns `(pushed, pulled)` counts. If Turso is not configured (empty URL
/// or token), returns `(0, 0)` without error.
///
/// Steps are interleaved so the Mutex is never held across an await:
/// 1. Read config + unsynced entries (sync, lock held briefly)
/// 2. Ensure remote table + push to Turso (async, no lock)
/// 3. Mark synced + pull from Turso (async, no lock)
/// 4. Insert pulled entries (sync, lock held briefly)
///
/// Requires a paid license (Cross-device Sync is a paid feature).
#[tauri::command]
pub async fn sync_history(state: State<'_, AppState>) -> Result<(u32, u32), String> {
    require_license!(state, LicensedFeature::Sync);
    let inner = state.inner();
    let cfg = crate::lock!(inner.config)?.clone();

    if cfg.turso_url.is_empty() || cfg.turso_token.is_empty() {
        return Ok((0, 0));
    }

    // Step 1: Read unsynced entries (lock DB briefly, then release).
    let unsynced = {
        let db = crate::lock!(inner.history_db)?;
        sync::read_unsynced_entries(&db).map_err(|e| format!("Sync failed (read): {e}"))?
    };

    // Step 2: Ensure remote table + push (async, no DB lock).
    let (pushed, uuids) = sync::ensure_and_push(&cfg.turso_url, &cfg.turso_token, unsynced)
        .await
        .map_err(|e| format!("Sync failed (push): {e}"))?;

    // Step 3: Mark pushed entries as synced (lock DB briefly).
    if pushed > 0 {
        let db = crate::lock!(inner.history_db)?;
        sync::mark_entries_synced(&db, &uuids)
            .map_err(|e| format!("Sync failed (mark): {e}"))?;
    }

    // Step 4: Pull remote entries (async, no DB lock).
    let remote = sync::pull_remote_entries(&cfg.turso_url, &cfg.turso_token, &cfg.device_id)
        .await
        .map_err(|e| format!("Sync failed (pull): {e}"))?;

    // Step 5: Insert pulled entries into local DB (lock DB briefly).
    let pulled = if !remote.is_empty() {
        let db = crate::lock!(inner.history_db)?;
        sync::insert_pulled_entries(&db, &remote)
            .map_err(|e| format!("Sync failed (insert): {e}"))?
    } else {
        0
    };

    Ok((pushed, pulled))
}

// ---------------------------------------------------------------------------
// Bar position persistence
// ---------------------------------------------------------------------------

/// Persists the floating bar window position (logical pixels) to config.
///
/// Called by the frontend after the user drags the bar to a new position.
/// Both `x` and `y` are always saved together -- a partial update is not
/// supported because a single coordinate is not useful on its own.
#[tauri::command]
pub fn save_bar_position(state: State<'_, AppState>, x: f64, y: f64) -> Result<(), String> {
    state.inner().save_config_locked("bar position", |cfg| {
        cfg.bar_x = Some(x);
        cfg.bar_y = Some(y);
    })?;
    Ok(())
}

/// Saved floating-bar position. Serialized as a JSON object `{ "x", "y" }`
/// so the frontend can read `.x` / `.y` directly. (A bare tuple would
/// serialize to a JSON array `[x, y]`, where `.x` is `undefined` -> the
/// preview show-sequence then computed NaN and `set_position` rejected null.)
#[derive(serde::Serialize)]
pub struct BarPosition {
    pub x: f64,
    pub y: f64,
}

/// Returns the last saved floating bar position, or `None` if no position
/// has been saved yet (first run or after a config reset).
///
/// The frontend uses this to restore the bar to its previous position on
/// startup instead of the default center-bottom placement.
#[tauri::command]
pub fn get_bar_position(state: State<'_, AppState>) -> Result<Option<BarPosition>, String> {
    let cfg = crate::lock!(state.inner().config)?;
    Ok(match (cfg.bar_x, cfg.bar_y) {
        (Some(x), Some(y)) => Some(BarPosition { x, y }),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Window / UI helpers
// ---------------------------------------------------------------------------

/// Ensures the floating bar window exists and is responsive.
///
/// Returns `true` if the window had to be recreated, `false` if it was already
/// alive and responding. The frontend should call this command before starting
/// a recording session to recover from the rare case where the bar window
/// silently vanished after hours in the background.
///
/// Desktop-only: on mobile the bar concept does not apply.
#[cfg(desktop)]
#[tauri::command]
pub async fn ensure_bar_window(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    // Check if the bar window handle exists and responds to is_visible().
    if let Some(bar) = app.get_webview_window("bar") {
        match bar.is_visible() {
            Ok(_) => {
                log::debug!("[bar] ensure_bar_window: window exists and responds");
                return Ok(false); // No recreation needed
            }
            Err(e) => {
                log::warn!("[bar] ensure_bar_window: window exists but not responding: {e}");
                // Fall through to recreation
            }
        }
    } else {
        log::warn!("[bar] ensure_bar_window: window not found, recreating");
    }

    // Read the saved position from config before recreating.
    let (saved_x, saved_y) = {
        let cfg = crate::lock!(state.inner().config)?;
        (cfg.bar_x, cfg.bar_y)
    };

    match crate::create_bar_window(&app, saved_x, saved_y) {
        Ok(_) => {
            log::info!("[bar] ensure_bar_window: successfully recreated bar window");
            Ok(true)
        }
        Err(e) => {
            log::error!("[bar] ensure_bar_window: failed to recreate: {e}");
            Err(format!("Failed to recreate bar window: {e}"))
        }
    }
}

/// Ensures the standalone preview window exists and is responsive.
///
/// Returns `true` if the window had to be recreated, `false` if it was already
/// alive and responding. Unlike `ensure_bar_window`, no saved position is
/// needed: the preview position is derived from the pill anchor at show-time
/// (Story 6.2), not persisted in config.
///
/// Desktop-only: the preview window concept does not apply to mobile.
#[cfg(desktop)]
#[tauri::command]
pub async fn ensure_preview_window(
    app: tauri::AppHandle,
    _state: State<'_, AppState>,
) -> Result<bool, String> {
    // Check if the preview window handle exists and responds to is_visible().
    if let Some(preview) = app.get_webview_window("preview") {
        match preview.is_visible() {
            Ok(_) => {
                log::debug!("[preview] ensure_preview_window: window exists and responds");
                return Ok(false); // No recreation needed
            }
            Err(e) => {
                log::warn!("[preview] ensure_preview_window: window exists but not responding: {e}");
                // Fall through to recreation
            }
        }
    } else {
        log::warn!("[preview] ensure_preview_window: window not found, recreating");
    }

    match crate::create_preview_window(&app) {
        Ok(_) => {
            log::info!("[preview] ensure_preview_window: recreated");
            Ok(true)
        }
        Err(e) => {
            log::error!("[preview] ensure_preview_window: failed to recreate: {e}");
            Err(format!("Failed to recreate preview window: {e}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Build provenance
// ---------------------------------------------------------------------------

/// Build provenance for the About screen. Used to verify a rebuild actually
/// shipped (stale-binary detection): `built_epoch` changes on every recompile,
/// even from an uncommitted working tree, so a stale binary shows an old stamp.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,
    pub git_hash: String,
    pub built_epoch: u64,
}

/// Returns version + git short hash + build timestamp.
///
/// The timestamp is the mtime of THIS running executable, read at runtime —
/// the true write/link time of the binary. Unlike a build-script-captured time
/// (which Cargo caches, and which a mtime-preserving sync like robocopy can
/// leave stale), a binary that was never re-linked honestly reports its old
/// mtime. That is exactly the stale-rebuild case we want to surface.
#[tauri::command]
pub fn get_build_info() -> BuildInfo {
    let built_epoch = std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: env!("KLARVO_BUILD_HASH").to_string(),
        built_epoch,
    }
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

/// Returns the absolute path to the app log directory.
///
/// Used by the frontend to open the folder (desktop) or display the path.
#[tauri::command]
pub fn get_log_dir_path(handle: AppHandle) -> Result<String, String> {
    let log_dir = handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to resolve log dir: {e}"))?;
    Ok(log_dir.to_string_lossy().to_string())
}

/// Reads the most recent log file content (up to ~200 KB).
///
/// Collects all `*.log` files from both the Tauri app log directory
/// (tauri-plugin-log / Rust logs) and the app data directory's `logs/`
/// subfolder (KlarvoLogger / Kotlin logs on Android). Returns their
/// concatenated content, sorted by name.
#[tauri::command]
pub fn read_recent_logs(handle: AppHandle) -> Result<String, String> {
    let mut dirs_to_scan: Vec<std::path::PathBuf> = Vec::new();

    // 1. Tauri app log dir (tauri-plugin-log writes here)
    if let Ok(log_dir) = handle.path().app_log_dir() {
        if log_dir.exists() {
            dirs_to_scan.push(log_dir);
        }
    }

    // 2. {app_data_dir}/logs/ (KlarvoLogger on Android writes here)
    if let Ok(data_dir) = handle.path().app_data_dir() {
        let kotlin_log_dir = data_dir.join("logs");
        // Avoid scanning the same dir twice
        if kotlin_log_dir.exists() && !dirs_to_scan.contains(&kotlin_log_dir) {
            dirs_to_scan.push(kotlin_log_dir);
        }
    }

    if dirs_to_scan.is_empty() {
        return Ok("No log directories found.".into());
    }

    let mut files: Vec<_> = Vec::new();
    for dir in &dirs_to_scan {
        if let Ok(entries) = std::fs::read_dir(dir) {
            files.extend(
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map_or(false, |ext| ext == "log")
                    }),
            );
        }
    }
    files.sort_by_key(|e| e.file_name());

    let mut content = String::new();
    let max_bytes: usize = 200_000; // ~200 KB cap

    for entry in files {
        if content.len() >= max_bytes {
            break;
        }
        if let Ok(text) = std::fs::read_to_string(entry.path()) {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!(
                "=== {} ===\n",
                entry.file_name().to_string_lossy()
            ));
            let remaining = max_bytes.saturating_sub(content.len());
            if text.len() > remaining {
                content.push_str(&text[text.len() - remaining..]);
                content.push_str("\n[... truncated ...]");
            } else {
                content.push_str(&text);
            }
        }
    }

    if content.is_empty() {
        Ok("No log files found.".into())
    } else {
        Ok(content)
    }
}

// ---------------------------------------------------------------------------
// Window / UI helpers
// ---------------------------------------------------------------------------

/// Updates the floating bar window region (thin idle pill vs. expanded active pill).
/// Called by the frontend whenever the bar state changes.
#[tauri::command]
pub fn set_bar_shape(handle: AppHandle, shape: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        if let Some(bar) = handle.get_webview_window("bar") {
            let scale = bar.scale_factor().unwrap_or(1.0);
            if let Ok(hwnd) = bar.hwnd() {
                let h = hwnd.0 as isize;
                if shape == "idle" {
                    let w = (80.0 * scale) as i32;
                    let ht = (10.0 * scale) as i32;
                    crate::set_window_region_pill(h, w, ht);
                } else if shape == "panel" {
                    // Live-preview expanded card (Story 5.2). The panel height is
                    // DYNAMIC — it auto-grows upward with the accumulated preview
                    // text (5.2 cosmetic revision) — so the region must match the
                    // window's ACTUAL size rather than a hardcoded constant. The
                    // frontend always awaits setSize before calling this, so
                    // inner_size() already reflects the new dimensions. The radius
                    // MUST equal the wrapper's CSS borderRadius when isPanelOpen
                    // (14) or the OS-region vs CSS-shape gap shows as the white line.
                    if let Ok(size) = bar.inner_size() {
                        let w = size.width as i32;
                        let ht = size.height as i32;
                        let r = (14.0 * scale) as i32; // card corner radius
                        crate::set_window_region_round_rect(h, w, ht, r);
                    }
                } else {
                    let w = (200.0 * scale) as i32;
                    let ht = (36.0 * scale) as i32;
                    crate::set_window_region_pill(h, w, ht);
                }
            }
        }
    }
    let _ = (handle, shape); // suppress unused warnings on non-Windows
    Ok(())
}

/// Applies a rounded-rect OS window region to the "preview" window.
/// Called once per show (size is already set before this is called).
/// `radius` is the CSS border-radius in logical px (Story 6.6: AC-4 / R11).
/// It MUST equal the CSS `borderRadius` applied to the preview card or a
/// white-line corner artifact appears on Windows.
///
/// Inversion (smoke-time): pass radius=8 while CSS uses borderRadius=14
/// → white-line gap at corners → RED.
///
/// `#[cfg(target_os = "windows")]` body only; no-op on other platforms.
#[tauri::command]
pub fn set_preview_shape(handle: AppHandle, radius: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        if let Some(preview) = handle.get_webview_window("preview") {
            let scale = preview.scale_factor().unwrap_or(1.0);
            if let Ok(hwnd) = preview.hwnd() {
                let h = hwnd.0 as isize;
                if let Ok(size) = preview.inner_size() {
                    let w = size.width as i32;
                    let ht = size.height as i32;
                    let r = (radius as f64 * scale) as i32;
                    crate::set_window_region_round_rect(h, w, ht, r);
                }
            }
        }
    }
    let _ = (handle, radius); // suppress unused on non-Windows
    Ok(())
}

/// Receives a `console.*` line forwarded from any frontend window and writes it
/// to the Rust log file (`Klarvo.log`).
///
/// The `"bar"` and `"preview"` overlay windows are transparent / click-through /
/// tiny, so their webview devtools cannot be opened interactively — their
/// `console.*` output is otherwise invisible. This bridge makes it land in the
/// same log file as the Rust side, the only console we can inspect for those
/// windows (and remotely from WSL during Windows smokes).
///
/// Local-only: writes to the on-disk log, never the network (BYOK / no-telemetry).
#[tauri::command]
pub fn frontend_log(label: String, level: String, message: String) {
    match level.as_str() {
        "error" => log::error!("[fe:{label}] {message}"),
        "warn" => log::warn!("[fe:{label}] {message}"),
        "debug" => log::debug!("[fe:{label}] {message}"),
        _ => log::info!("[fe:{label}] {message}"),
    }
}
