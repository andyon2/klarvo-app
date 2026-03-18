//! Tauri commands for settings, API keys and configuration management.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::config::{self, save_config, AppConfig, HotkeyMode};
use crate::license::LicensedFeature;
use crate::require_license;
use crate::llm::{self, CleanupStyle};
use crate::pipeline::{resolve_cleanup_provider, resolve_stt_provider};
use crate::stt::{self};
use crate::{ApiKeyStatus, AppState, SettingsView};
use crate::mask_api_key;

// ---------------------------------------------------------------------------
// Autostart helper (Windows only)
// ---------------------------------------------------------------------------

/// Writes or removes the autostart registry entry under
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
///
/// On non-Windows platforms this is a no-op (the config field is still
/// persisted, but OS-level startup is not wired up).
#[cfg(target_os = "windows")]
pub fn apply_autostart(enabled: bool) {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, KEY_SET_VALUE,
        HKEY_CURRENT_USER, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::PCWSTR;

    // Encode the registry key path as a null-terminated wide string.
    let key_path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "Dikta\0".encode_utf16().collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            Some(0),
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        );

        if result != ERROR_SUCCESS {
            log::warn!("[autostart] Failed to open registry key: {:?}", result);
            return;
        }

        if enabled {
            // Determine path to the current executable.
            match std::env::current_exe() {
                Ok(exe_path) => {
                    let exe_str = exe_path.to_string_lossy();
                    // Quote the path in case it contains spaces.
                    let quoted = format!("\"{exe_str}\"\0");
                    let wide: Vec<u16> = quoted.encode_utf16().collect();
                    let byte_len = (wide.len() * 2) as u32;
                    let bytes =
                        std::slice::from_raw_parts(wide.as_ptr() as *const u8, byte_len as usize);

                    let set_result = RegSetValueExW(
                        hkey,
                        PCWSTR(value_name.as_ptr()),
                        Some(0),
                        REG_SZ,
                        Some(bytes),
                    );
                    if set_result != ERROR_SUCCESS {
                        log::warn!("[autostart] Failed to write registry value: {:?}", set_result);
                    } else {
                        log::info!("[autostart] Autostart enabled: {exe_str}");
                    }
                }
                Err(e) => {
                    log::warn!("[autostart] Could not determine exe path: {e}");
                }
            }
        } else {
            // Delete the value (ignore error if it doesn't exist).
            let _ = RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()));
            log::info!("[autostart] Autostart disabled (registry entry removed)");
        }

        let _ = RegCloseKey(hkey);
    }
}

/// No-op stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn apply_autostart(_enabled: bool) {}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Persists new settings and hot-reloads the affected providers.
///
/// After saving to disk:
/// - STT and LLM providers are replaced with new instances (API key changes
///   take effect immediately without a restart).
/// - The global shortcut is re-registered with the new hotkey string and mode.
///
/// Passing an empty string for an API key disables that provider (requests
/// will fail with an auth error from the API until a valid key is supplied).
#[tauri::command]
pub async fn save_settings(
    handle: AppHandle,
    state: State<'_, AppState>,
    groq_api_key: String,
    deepseek_api_key: String,
    language: String,
    cleanup_style: CleanupStyle,
    hotkey: String,
    hotkey_mode: HotkeyMode,
    audio_device: Option<String>,
    stt_model: Option<String>,
    custom_prompt: Option<String>,
    autostart: Option<bool>,
    whisper_mode: Option<bool>,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    // deprecated: ignored -- kept for backwards compatibility with older frontend versions
    stt_priority: Option<Vec<String>>,
    // deprecated: ignored -- kept for backwards compatibility with older frontend versions
    llm_priority: Option<Vec<String>>,
    output_language: Option<String>,
    webhook_url: Option<String>,
    turso_url: Option<String>,
    turso_token: Option<String>,
    bubble_size: Option<f32>,
    bubble_opacity: Option<f32>,
    local_whisper_model: Option<String>,
    local_whisper_gpu: Option<bool>,
    stt_provider: Option<String>,
    llm_provider: Option<String>,
    insert_and_send: Option<bool>,
    autostop_silence_secs: Option<f32>,
    auto_mode_silence_secs: Option<f32>,
    // Optional secondary hotkey string (slot 1). Empty string = disable slot.
    // None = leave slot 1 unchanged.
    hotkey_slot2: Option<String>,
    // Optional recording mode for the secondary hotkey slot (slot 1).
    // Passed as a string ("hold", "toggle", "autoStop", "auto") -- same
    // encoding as the existing `hotkey_mode` parameter.
    // None = leave slot 1 mode unchanged.
    hotkey_mode_slot2: Option<String>,
    // Whether to press Enter after pasting for slot 0. None = leave unchanged.
    insert_and_send_slot1: Option<bool>,
    // Whether to press Enter after pasting for slot 1. None = leave unchanged.
    insert_and_send_slot2: Option<bool>,
    // Recording mode for the Android floating bubble.
    // Valid values: "hold", "toggle", "autostop", "auto".
    // None = leave unchanged (backward-compatible with older frontend versions).
    bubble_recording_mode: Option<String>,
    // Per-gesture bubble controls. None = leave existing value unchanged.
    bubble_tap_mode: Option<String>,
    bubble_tap_auto_send: Option<bool>,
    bubble_tap_silence_secs: Option<f32>,
    bubble_long_press_mode: Option<String>,
    bubble_long_press_auto_send: Option<bool>,
    bubble_long_press_silence_secs: Option<f32>,
) -> Result<(), String> {
    let inner = state.inner();

    // License gate: Whisper Mode requires a paid license.
    if whisper_mode.unwrap_or(false) {
        require_license!(state, LicensedFeature::WhisperMode);
    }

    // Validate hotkey strings before writing anything to disk (desktop only).
    // Slot 0 (`hotkey` param) is always validated. Slot 1 (`hotkey_slot2`) is
    // only validated when non-empty -- empty string means "disable the slot".
    println!("[save_settings] hotkey={hotkey:?} mode={hotkey_mode:?}");
    #[cfg(desktop)]
    {
        hotkey
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| {
                println!("[save_settings] Invalid shortcut: {e}");
                format!("Invalid shortcut string: {e}")
            })?;

        if let Some(ref h2) = hotkey_slot2 {
            if !h2.is_empty() {
                h2.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    .map_err(|e| {
                        println!("[save_settings] Invalid slot-2 shortcut: {e}");
                        format!("Invalid slot-2 shortcut string: {e}")
                    })?;
            }
        }
    }

    // Build updated config. Empty API key strings preserve the existing key
    // so the user can change other settings without re-entering keys.
    let existing = crate::lock!(inner.config)?.clone();
    let new_cfg = AppConfig {
        groq_api_key: if groq_api_key.is_empty() {
            existing.groq_api_key
        } else {
            groq_api_key.clone()
        },
        deepseek_api_key: if deepseek_api_key.is_empty() {
            existing.deepseek_api_key
        } else {
            deepseek_api_key.clone()
        },
        language,
        cleanup_style,
        hotkey: hotkey.clone(),
        hotkey_mode,
        audio_device,
        stt_model: stt_model.unwrap_or(existing.stt_model),
        custom_prompt: custom_prompt.unwrap_or(existing.custom_prompt),
        profiles: existing.profiles,
        autostart: autostart.unwrap_or(existing.autostart),
        whisper_mode: whisper_mode.unwrap_or(existing.whisper_mode),
        command_hotkey: existing.command_hotkey,
        openai_api_key: match openai_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.openai_api_key,
        },
        anthropic_api_key: match anthropic_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.anthropic_api_key,
        },
        stt_provider: stt_provider.unwrap_or(existing.stt_provider),
        llm_provider: llm_provider.unwrap_or(existing.llm_provider),
        // deprecated fields: ignore the incoming values, preserve what was on disk
        // so old config.json files round-trip cleanly
        stt_priority: existing.stt_priority,
        llm_priority: existing.llm_priority,
        // Build the updated hotkey_slots:
        // - Slot 0 is always updated from the `hotkey` / `hotkey_mode` parameters
        //   (backward-compatible with any frontend that doesn't know about slots).
        // - Slot 1 is updated only when `hotkey_slot2` is supplied; otherwise the
        //   existing value is preserved so a settings save never silently wipes it.
        hotkey_slots: {
            let mut slots = existing.hotkey_slots.clone();

            // Ensure the Vec is at least 2 elements long.
            while slots.len() < 2 {
                slots.push(crate::config::HotkeySlot {
                    hotkey: String::new(),
                    mode: crate::config::HotkeyMode::Hold,
                    insert_and_send: false,
                });
            }

            // Slot 0 -- always updated from the `hotkey` / `hotkey_mode` params.
            slots[0].hotkey = hotkey.clone();
            slots[0].mode = hotkey_mode;
            if let Some(v) = insert_and_send_slot1 {
                slots[0].insert_and_send = v;
            }

            // Slot 1 -- updated only when the caller explicitly passes a value.
            if let Some(ref h2) = hotkey_slot2 {
                slots[1].hotkey = h2.clone();
            }
            if let Some(ref m2_str) = hotkey_mode_slot2 {
                slots[1].mode = m2_str.parse().unwrap_or(crate::config::HotkeyMode::Hold);
            }
            if let Some(v) = insert_and_send_slot2 {
                slots[1].insert_and_send = v;
            }

            slots
        },
        output_language: output_language.unwrap_or(existing.output_language),
        snippets: existing.snippets,
        voice_notes_hotkey: existing.voice_notes_hotkey,
        webhook_url: webhook_url.unwrap_or(existing.webhook_url),
        turso_url: match turso_url {
            Some(ref u) if !u.is_empty() => u.clone(),
            Some(ref u) if u.is_empty() => String::new(), // explicitly cleared
            _ => existing.turso_url,
        },
        turso_token: match turso_token {
            Some(ref t) if !t.is_empty() => t.clone(),
            _ => existing.turso_token,
        },
        device_id: existing.device_id,
        bubble_size: bubble_size.unwrap_or(existing.bubble_size),
        bubble_opacity: bubble_opacity.unwrap_or(existing.bubble_opacity),
        advanced: existing.advanced,
        local_whisper_model: local_whisper_model.unwrap_or(existing.local_whisper_model),
        local_whisper_gpu: local_whisper_gpu.unwrap_or(existing.local_whisper_gpu),
        license_key: existing.license_key,
        license_validated_at: existing.license_validated_at,
        bar_x: existing.bar_x,
        bar_y: existing.bar_y,
        insert_and_send: insert_and_send.unwrap_or(existing.insert_and_send),
        autostop_silence_secs: autostop_silence_secs.unwrap_or(existing.autostop_silence_secs),
        auto_mode_silence_secs: auto_mode_silence_secs.unwrap_or(existing.auto_mode_silence_secs),
        bubble_recording_mode: bubble_recording_mode.unwrap_or(existing.bubble_recording_mode),
        bubble_tap_mode: bubble_tap_mode.unwrap_or(existing.bubble_tap_mode),
        bubble_tap_auto_send: bubble_tap_auto_send.unwrap_or(existing.bubble_tap_auto_send),
        bubble_tap_silence_secs: bubble_tap_silence_secs
            .unwrap_or(existing.bubble_tap_silence_secs),
        bubble_long_press_mode: bubble_long_press_mode.unwrap_or(existing.bubble_long_press_mode),
        bubble_long_press_auto_send: bubble_long_press_auto_send
            .unwrap_or(existing.bubble_long_press_auto_send),
        bubble_long_press_silence_secs: bubble_long_press_silence_secs
            .unwrap_or(existing.bubble_long_press_silence_secs),
    };

    // Resolve providers from the new config before persisting.
    let new_stt = resolve_stt_provider(&new_cfg);
    let new_cleanup = resolve_cleanup_provider(&new_cfg);

    // Persist to disk.
    save_config(&inner.app_data_dir, &new_cfg)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Update in-memory config.
    *crate::lock!(inner.config)? = new_cfg;

    // Hot-reload providers based on priority lists.
    *crate::write_lock!(inner.stt_provider)? = new_stt;
    *crate::write_lock!(inner.cleanup_provider)? = new_cleanup;

    // Re-register all hotkey slots from the (now-updated) in-memory config (desktop only).
    #[cfg(desktop)]
    crate::pipeline::register_hotkey(&handle)?;

    // Apply autostart: write or remove the OS startup entry.
    let autostart_enabled = crate::lock!(inner.config)?.autostart;
    apply_autostart(autostart_enabled);

    Ok(())
}

/// Returns the current settings for display in the frontend.
///
/// API keys are masked (only last 4 characters visible) so this can be sent
/// to the frontend without exposing the full secrets.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    let cfg = crate::lock!(state.inner().config)?.clone();

    // Pull slot 0 (primary) and slot 1 (secondary) out of hotkey_slots.
    // Fall back to the legacy flat fields for slot 0 in case the Vec is empty
    // (should not happen after migration, but be defensive).
    let slot0_hotkey = cfg
        .hotkey_slots
        .get(0)
        .map(|s| s.hotkey.clone())
        .unwrap_or_else(|| cfg.hotkey.clone());
    let slot0_mode = cfg
        .hotkey_slots
        .get(0)
        .map(|s| s.mode)
        .unwrap_or(cfg.hotkey_mode);
    let slot1_hotkey = cfg
        .hotkey_slots
        .get(1)
        .map(|s| s.hotkey.clone())
        .unwrap_or_default();
    let slot1_mode = cfg
        .hotkey_slots
        .get(1)
        .map(|s| s.mode)
        .unwrap_or(config::HotkeyMode::Hold);

    Ok(SettingsView {
        groq_api_key_masked: mask_api_key(&cfg.groq_api_key),
        deepseek_api_key_masked: mask_api_key(&cfg.deepseek_api_key),
        language: cfg.language,
        cleanup_style: cfg.cleanup_style,
        hotkey: slot0_hotkey,
        hotkey_mode: slot0_mode,
        audio_device: cfg.audio_device,
        stt_model: cfg.stt_model,
        custom_prompt: cfg.custom_prompt,
        autostart: cfg.autostart,
        whisper_mode: cfg.whisper_mode,
        openai_api_key_masked: mask_api_key(&cfg.openai_api_key),
        anthropic_api_key_masked: mask_api_key(&cfg.anthropic_api_key),
        stt_provider: cfg.stt_provider,
        llm_provider: cfg.llm_provider,
        output_language: cfg.output_language,
        webhook_url: cfg.webhook_url,
        turso_url: cfg.turso_url,
        turso_token_masked: mask_api_key(&cfg.turso_token),
        device_id: cfg.device_id,
        bubble_size: cfg.bubble_size,
        bubble_opacity: cfg.bubble_opacity,
        local_whisper_model: cfg.local_whisper_model,
        local_whisper_gpu: cfg.local_whisper_gpu,
        insert_and_send_slot1: cfg.hotkey_slots.get(0).map(|s| s.insert_and_send).unwrap_or(false),
        insert_and_send_slot2: cfg.hotkey_slots.get(1).map(|s| s.insert_and_send).unwrap_or(false),
        autostop_silence_secs: cfg.autostop_silence_secs,
        auto_mode_silence_secs: cfg.auto_mode_silence_secs,
        hotkey_slot2: slot1_hotkey,
        hotkey_mode_slot2: slot1_mode,
        bubble_recording_mode: cfg.bubble_recording_mode,
        bubble_tap_mode: cfg.bubble_tap_mode,
        bubble_tap_auto_send: cfg.bubble_tap_auto_send,
        bubble_tap_silence_secs: cfg.bubble_tap_silence_secs,
        bubble_long_press_mode: cfg.bubble_long_press_mode,
        bubble_long_press_auto_send: cfg.bubble_long_press_auto_send,
        bubble_long_press_silence_secs: cfg.bubble_long_press_silence_secs,
    })
}

/// Returns the current advanced settings.
#[tauri::command]
pub fn get_advanced_settings(
    state: State<'_, AppState>,
) -> Result<config::AdvancedSettings, String> {
    let cfg = crate::lock!(state.inner().config)?;
    Ok(cfg.advanced.clone())
}

/// Saves updated advanced settings. Replaces the entire advanced block.
///
/// If any custom LLM system prompt field is non-empty (i.e. the user is
/// overriding built-in prompts), a paid license is required.
#[tauri::command]
pub fn save_advanced_settings(
    state: State<'_, AppState>,
    settings: config::AdvancedSettings,
) -> Result<(), String> {
    // License gate: custom LLM system prompts require a paid license.
    let has_custom_prompt = !settings.llm_system_prompt_polished.is_empty()
        || !settings.llm_system_prompt_verbatim.is_empty()
        || !settings.llm_system_prompt_chat.is_empty()
        || !settings.llm_command_mode_prompt.is_empty();
    if has_custom_prompt {
        require_license!(state, LicensedFeature::CustomPrompts);
    }

    let inner = state.inner();
    let mut cfg = crate::lock!(inner.config)?;
    cfg.advanced = settings;
    save_config(&inner.app_data_dir, &cfg)
        .map_err(|e| format!("Failed to save advanced settings: {e}"))?;
    Ok(())
}

/// Returns which API keys are currently configured (non-empty).
///
/// Does NOT return the key values themselves -- only booleans indicating
/// presence. The frontend uses this to show configuration status.
#[tauri::command]
pub fn get_api_key_status(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    let cfg = crate::lock!(state.inner().config)?.clone();

    Ok(ApiKeyStatus {
        groq_configured: !cfg.groq_api_key.is_empty(),
        deepseek_configured: !cfg.deepseek_api_key.is_empty(),
    })
}

/// Replaces the STT and/or LLM provider with a new instance using the supplied
/// API keys. Settings are also persisted to disk.
///
/// Passing `None` for a key leaves that provider unchanged.
/// Passing `Some("")` effectively disables the provider.
///
/// Kept for backward compatibility with existing frontend code.
/// New code should prefer `save_settings`.
#[tauri::command]
pub async fn update_api_keys(
    state: State<'_, AppState>,
    groq_api_key: Option<String>,
    deepseek_api_key: Option<String>,
) -> Result<(), String> {
    let inner = state.inner();

    {
        let mut cfg = crate::lock!(inner.config)?;

        if let Some(ref key) = groq_api_key {
            cfg.groq_api_key = key.clone();
        }
        if let Some(ref key) = deepseek_api_key {
            cfg.deepseek_api_key = key.clone();
        }

        // Persist updated config.
        let cfg_clone = cfg.clone();
        drop(cfg); // release lock before I/O
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist API keys: {e}"))?;
    }

    if let Some(key) = groq_api_key {
        let model = crate::lock!(inner.config)?.stt_model.clone();
        *crate::write_lock!(inner.stt_provider)? =
            Arc::new(stt::GroqWhisper::new(key).with_model(model));
    }

    if let Some(key) = deepseek_api_key {
        *crate::write_lock!(inner.cleanup_provider)? =
            Arc::new(llm::DeepSeekCleanup::new(key));
    }

    Ok(())
}

/// Sets the language used by the hotkey pipeline and persists the change.
///
/// `language`: ISO-639-1 code, e.g. `"de"` or `"en"`. Empty string = auto-detect.
#[tauri::command]
pub fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = crate::lock!(inner.config)?;
    cfg.language = language;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist language setting: {e}"))
}

/// Sets the cleanup style used by the hotkey pipeline and persists the change.
#[tauri::command]
pub fn set_cleanup_style(state: State<'_, AppState>, style: CleanupStyle) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = crate::lock!(inner.config)?;
    cfg.cleanup_style = style;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist cleanup style: {e}"))
}

/// Sets the output language for translation and persists the change.
///
/// `language`: ISO-639-1 code, e.g. `"en"` to translate to English.
/// Empty string = no translation (dictation stays in original language).
#[tauri::command]
pub fn set_output_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = crate::lock!(inner.config)?;
    cfg.output_language = language;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist output language setting: {e}"))
}

/// Changes the registered global hotkey and/or mode at runtime.
///
/// `shortcut`: a Tauri shortcut string, e.g. `"ctrl+shift+d"`.
/// `mode`: `HotkeyMode::Hold` or `HotkeyMode::Toggle`.
/// `slot_index`: which slot to update (0 = primary, 1 = secondary). Defaults
///   to 0 when `None` -- so existing callers remain backward-compatible.
///
/// Returns an error if the shortcut string is invalid or registration fails.
/// Persists both the new shortcut and mode to config.
#[tauri::command]
pub async fn set_hotkey(
    handle: AppHandle,
    state: State<'_, AppState>,
    shortcut: String,
    mode: HotkeyMode,
    slot_index: Option<u8>,
) -> Result<(), String> {
    let idx = slot_index.unwrap_or(0) as usize;

    // Only validate non-empty shortcuts; an empty string for slot 1 means
    // "disable this slot" and does not need to parse as a valid shortcut.
    #[cfg(desktop)]
    if !shortcut.is_empty() {
        shortcut
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| format!("Invalid shortcut string: {e}"))?;
    }

    let inner = state.inner();
    {
        let mut cfg = crate::lock!(inner.config)?;

        // Ensure the Vec is at least (idx + 1) elements long.
        while cfg.hotkey_slots.len() <= idx {
            cfg.hotkey_slots.push(crate::config::HotkeySlot {
                hotkey: String::new(),
                mode: crate::config::HotkeyMode::Hold,
                insert_and_send: false,
            });
        }

        // Update the target slot.
        cfg.hotkey_slots[idx].hotkey = shortcut.clone();
        cfg.hotkey_slots[idx].mode = mode;

        // Keep the legacy flat fields in sync for slot 0 (config.json round-trip).
        if idx == 0 {
            cfg.hotkey = shortcut.clone();
            cfg.hotkey_mode = mode;
        }

        let cfg_clone = cfg.clone();
        drop(cfg);
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist hotkey setting: {e}"))?;
    }

    // Re-register all hotkey slots from the updated config (desktop only).
    #[cfg(desktop)]
    crate::pipeline::register_hotkey(&handle)?;

    Ok(())
}

/// Reformats text into a specific output format (email, bullets, summary).
///
/// Uses the currently configured LLM provider to transform the text.
#[tauri::command]
pub async fn reformat_text(
    state: State<'_, AppState>,
    text: String,
    format: String,
) -> Result<String, String> {
    let inner = state.inner();
    let provider = crate::read_lock!(inner.cleanup_provider)?.clone();
    provider
        .reformat(&text, &format)
        .await
        .map(|r| r.text)
        .map_err(|e| format!("Reformat failed: {e}"))
}

/// Returns `true` if no API keys have been configured yet.
///
/// Used by the frontend to decide whether to show the onboarding wizard on
/// startup. Treated as "first run" when all provider keys are empty.
#[tauri::command]
pub fn is_first_run(state: State<'_, AppState>) -> bool {
    let inner = state.inner();
    match inner.config.lock() {
        Ok(g) => {
            g.groq_api_key.is_empty()
                && g.deepseek_api_key.is_empty()
                && g.openai_api_key.is_empty()
                && g.anthropic_api_key.is_empty()
        }
        Err(_) => true,
    }
}

/// Returns the title of the last window that was active before Dikta received
/// focus (captured at hotkey press time), or `None` when no title was captured.
#[tauri::command]
pub fn get_active_app(state: State<'_, AppState>) -> Option<String> {
    state.prev_window_title.lock().ok().and_then(|t| t.clone())
}

/// Pauses or resumes the global hotkey handler.
///
/// Called by the frontend ShortcutRecorder when it enters/exits listening mode.
/// While paused, all global hotkey events are silently swallowed so the user
/// can press the current shortcut without triggering the pipeline.
#[tauri::command]
pub fn set_hotkey_paused(state: State<'_, AppState>, paused: bool) {
    state
        .hotkey_paused
        .store(paused, std::sync::atomic::Ordering::SeqCst);
    log::debug!("[settings] hotkey_paused = {paused}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::config::{load_config, save_config, AppConfig};

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir creation failed")
    }

    /// The three new recording-mode fields (`insert_and_send`,
    /// `autostop_silence_secs`, `auto_mode_silence_secs`) survive a
    /// save → load round-trip through `config.json`.
    ///
    /// This validates that `save_settings` can persist these fields and
    /// `get_settings` will return them correctly (both delegate to
    /// `AppConfig`/`save_config`/`load_config`).
    #[test]
    fn test_save_settings_persists_recording_mode_fields() {
        let dir = temp_dir();

        let cfg = AppConfig {
            insert_and_send: true,
            autostop_silence_secs: 1.5,
            auto_mode_silence_secs: 3.5,
            ..AppConfig::default()
        };

        save_config(dir.path(), &cfg).expect("save_config should succeed");
        let loaded = load_config(dir.path());

        // Migration: global insert_and_send=true is moved to slots, global reset to false
        assert!(
            !loaded.insert_and_send,
            "global insert_and_send should be false after migration to slots"
        );
        assert!(
            loaded.hotkey_slots.iter().all(|s| s.insert_and_send),
            "all slots should have insert_and_send=true after migration"
        );
        assert!(
            (loaded.autostop_silence_secs - 1.5).abs() < f32::EPSILON,
            "autostop_silence_secs should round-trip to 1.5, got {}",
            loaded.autostop_silence_secs
        );
        assert!(
            (loaded.auto_mode_silence_secs - 3.5).abs() < f32::EPSILON,
            "auto_mode_silence_secs should round-trip to 3.5, got {}",
            loaded.auto_mode_silence_secs
        );
    }

    /// `bubble_recording_mode` defaults to `"hold"` when the field is absent
    /// from an old config.json (backward-compatibility).
    #[test]
    fn test_bubble_recording_mode_default_value() {
        assert_eq!(
            AppConfig::default().bubble_recording_mode,
            "hold",
            "bubble_recording_mode must default to \"hold\""
        );
    }

    /// `bubble_recording_mode` survives a save → load round-trip intact.
    #[test]
    fn test_bubble_recording_mode_roundtrip() {
        let dir = temp_dir();

        let cfg = AppConfig {
            bubble_recording_mode: "toggle".to_string(),
            ..AppConfig::default()
        };

        save_config(dir.path(), &cfg).expect("save_config should succeed");
        let loaded = load_config(dir.path());

        assert_eq!(
            loaded.bubble_recording_mode, "toggle",
            "bubble_recording_mode should round-trip as \"toggle\""
        );
    }

    /// Old config.json without the six new bubble gesture fields loads correctly
    /// and returns the documented defaults for each field.
    #[test]
    fn test_bubble_gesture_fields_default_when_absent_from_json() {
        let dir = temp_dir();

        // Minimal config -- none of the new bubble gesture fields are present.
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes())
            .expect("write partial config");

        let loaded = load_config(dir.path());

        assert_eq!(
            loaded.bubble_tap_mode, "toggle",
            "bubble_tap_mode must default to \"toggle\""
        );
        assert!(
            !loaded.bubble_tap_auto_send,
            "bubble_tap_auto_send must default to false"
        );
        assert!(
            (loaded.bubble_tap_silence_secs - 2.0).abs() < f32::EPSILON,
            "bubble_tap_silence_secs must default to 2.0, got {}",
            loaded.bubble_tap_silence_secs
        );
        assert_eq!(
            loaded.bubble_long_press_mode, "hold",
            "bubble_long_press_mode must default to \"hold\""
        );
        assert!(
            !loaded.bubble_long_press_auto_send,
            "bubble_long_press_auto_send must default to false"
        );
        assert!(
            (loaded.bubble_long_press_silence_secs - 2.0).abs() < f32::EPSILON,
            "bubble_long_press_silence_secs must default to 2.0, got {}",
            loaded.bubble_long_press_silence_secs
        );
    }

    /// Round-trip: serialize AppConfig with non-default bubble gesture values,
    /// then reload from disk -- all six fields must survive intact.
    #[test]
    fn test_bubble_gesture_fields_roundtrip() {
        let dir = temp_dir();

        let cfg = AppConfig {
            bubble_tap_mode: "autostop".to_string(),
            bubble_tap_auto_send: true,
            bubble_tap_silence_secs: 1.5,
            bubble_long_press_mode: "auto".to_string(),
            bubble_long_press_auto_send: true,
            bubble_long_press_silence_secs: 3.5,
            ..AppConfig::default()
        };

        save_config(dir.path(), &cfg).expect("save_config should succeed");
        let loaded = load_config(dir.path());

        assert_eq!(loaded.bubble_tap_mode, "autostop");
        assert!(loaded.bubble_tap_auto_send);
        assert!(
            (loaded.bubble_tap_silence_secs - 1.5).abs() < f32::EPSILON,
            "bubble_tap_silence_secs should be 1.5, got {}",
            loaded.bubble_tap_silence_secs
        );
        assert_eq!(loaded.bubble_long_press_mode, "auto");
        assert!(loaded.bubble_long_press_auto_send);
        assert!(
            (loaded.bubble_long_press_silence_secs - 3.5).abs() < f32::EPSILON,
            "bubble_long_press_silence_secs should be 3.5, got {}",
            loaded.bubble_long_press_silence_secs
        );
    }

    /// When `bubble_recording_mode` is absent from JSON (old config file),
    /// `load_config` returns `"hold"` as the default -- no crash, no data loss.
    #[test]
    fn test_bubble_recording_mode_defaults_when_absent_from_json() {
        let dir = temp_dir();

        // Write a minimal config that does not contain the new field.
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes())
            .expect("write partial config");

        let loaded = load_config(dir.path());

        assert_eq!(
            loaded.bubble_recording_mode, "hold",
            "bubble_recording_mode should default to \"hold\" when absent from config"
        );
    }

    /// `insert_and_send` is stored per-slot. Verify that per-slot values
    /// survive a save → load round-trip independently.
    #[test]
    fn test_insert_and_send_per_slot_roundtrip() {
        use crate::config::{HotkeyMode, HotkeySlot};

        let dir = temp_dir();

        let mut cfg = AppConfig::default();
        // Slot 0: insert_and_send = true, slot 1: insert_and_send = false.
        cfg.hotkey_slots = vec![
            HotkeySlot { hotkey: "ctrl+shift+d".to_string(), mode: HotkeyMode::Hold, insert_and_send: true },
            HotkeySlot { hotkey: "ctrl+shift+e".to_string(), mode: HotkeyMode::Toggle, insert_and_send: false },
        ];

        save_config(dir.path(), &cfg).expect("save_config should succeed");
        let loaded = load_config(dir.path());

        assert!(
            loaded.hotkey_slots[0].insert_and_send,
            "slot 0 insert_and_send should be true after round-trip"
        );
        assert!(
            !loaded.hotkey_slots[1].insert_and_send,
            "slot 1 insert_and_send should be false after round-trip"
        );
    }

    /// When `insert_and_send` is omitted from the saved JSON (old config),
    /// it defaults to `false` -- no migration step needed.
    #[test]
    fn test_save_settings_recording_mode_defaults_when_absent() {
        let dir = temp_dir();

        // Write a minimal config without the new fields.
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes())
            .expect("write partial config");

        let loaded = load_config(dir.path());

        assert!(
            !loaded.insert_and_send,
            "insert_and_send should default to false when absent from config"
        );
        assert!(
            (loaded.autostop_silence_secs - 2.0).abs() < f32::EPSILON,
            "autostop_silence_secs should default to 2.0 when absent"
        );
        assert!(
            (loaded.auto_mode_silence_secs - 2.0).abs() < f32::EPSILON,
            "auto_mode_silence_secs should default to 2.0 when absent"
        );
    }
}
