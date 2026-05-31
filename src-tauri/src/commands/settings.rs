//! Tauri commands for settings, API keys and configuration management.

use std::sync::Arc;

use tauri::{AppHandle, State};
use reqwest;

use crate::config::{self, AppConfig, HotkeyMode};
use crate::license::LicensedFeature;
use crate::require_license;
use crate::llm::{self, CleanupStyle};
use crate::pipeline::resolve_providers;
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
    let value_name: Vec<u16> = "Klarvo\0".encode_utf16().collect();

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
// SettingsPatch + merge_settings (seam for unit testing)
// ---------------------------------------------------------------------------

/// All fields that `save_settings` can write.  Fields that are always
/// preserved from `existing` (e.g. `command_hotkey`, license fields) are NOT
/// included here.
///
/// `Default` is implemented manually (see below) to make partial fixtures
/// ergonomic in tests:
/// `SettingsPatch { groq_api_key: "key".into(), ..SettingsPatch::default() }`.
pub struct SettingsPatch {
    pub groq_api_key: String,
    pub deepseek_api_key: String,
    pub language: String,
    pub cleanup_style: crate::llm::CleanupStyle,
    pub hotkey: String,
    pub hotkey_mode: crate::config::HotkeyMode,
    pub audio_device: Option<String>,
    pub stt_model: Option<String>,
    pub custom_prompt: Option<String>,
    pub autostart: Option<bool>,
    pub whisper_mode: Option<bool>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub output_language: Option<String>,
    pub webhook_url: Option<String>,
    pub turso_url: Option<String>,
    pub turso_token: Option<String>,
    pub bubble_size: Option<f32>,
    pub bubble_opacity: Option<f32>,
    pub local_whisper_model: Option<String>,
    pub local_whisper_gpu: Option<bool>,
    pub stt_provider: Option<String>,
    pub llm_provider: Option<String>,
    pub insert_and_send: Option<bool>,
    pub autostop_silence_secs: Option<f32>,
    pub auto_mode_silence_secs: Option<f32>,
    pub hotkey_slot2: Option<String>,
    pub hotkey_mode_slot2: Option<String>,
    pub insert_and_send_slot1: Option<bool>,
    pub insert_and_send_slot2: Option<bool>,
    pub bubble_recording_mode: Option<String>,
    pub bubble_tap_mode: Option<String>,
    pub bubble_tap_auto_send: Option<bool>,
    pub bubble_tap_silence_secs: Option<f32>,
    pub bubble_long_press_mode: Option<String>,
    pub bubble_long_press_auto_send: Option<bool>,
    pub bubble_long_press_silence_secs: Option<f32>,
    pub openrouter_api_key: Option<String>,
}

impl Default for SettingsPatch {
    fn default() -> Self {
        SettingsPatch {
            groq_api_key: String::new(),
            deepseek_api_key: String::new(),
            language: String::new(),
            cleanup_style: crate::llm::CleanupStyle::Polished,
            hotkey: String::new(),
            hotkey_mode: crate::config::HotkeyMode::Hold,
            audio_device: None,
            stt_model: None,
            custom_prompt: None,
            autostart: None,
            whisper_mode: None,
            openai_api_key: None,
            anthropic_api_key: None,
            output_language: None,
            webhook_url: None,
            turso_url: None,
            turso_token: None,
            bubble_size: None,
            bubble_opacity: None,
            local_whisper_model: None,
            local_whisper_gpu: None,
            stt_provider: None,
            llm_provider: None,
            insert_and_send: None,
            autostop_silence_secs: None,
            auto_mode_silence_secs: None,
            hotkey_slot2: None,
            hotkey_mode_slot2: None,
            insert_and_send_slot1: None,
            insert_and_send_slot2: None,
            bubble_recording_mode: None,
            bubble_tap_mode: None,
            bubble_tap_auto_send: None,
            bubble_tap_silence_secs: None,
            bubble_long_press_mode: None,
            bubble_long_press_auto_send: None,
            bubble_long_press_silence_secs: None,
            openrouter_api_key: None,
        }
    }
}

/// Pure function: merges `patch` into `existing` and returns the resulting
/// `AppConfig`.
///
/// This is the **production** merge logic extracted from `save_settings`
/// (lines 218-338 of the original).  The body is copied verbatim — no logic
/// changes.  `save_settings` delegates to this function.
///
/// Deliberately not `pub(crate)` so test modules in this file can access it
/// directly without a `super::` import.
pub fn merge_settings(existing: AppConfig, patch: SettingsPatch) -> AppConfig {
    AppConfig {
        groq_api_key: if patch.groq_api_key.is_empty() {
            existing.groq_api_key
        } else {
            patch.groq_api_key.clone()
        },
        deepseek_api_key: if patch.deepseek_api_key.is_empty() {
            existing.deepseek_api_key
        } else {
            patch.deepseek_api_key.clone()
        },
        language: patch.language,
        cleanup_style: patch.cleanup_style,
        hotkey: patch.hotkey.clone(),
        hotkey_mode: patch.hotkey_mode,
        audio_device: patch.audio_device,
        stt_model: patch.stt_model.unwrap_or(existing.stt_model),
        custom_prompt: patch.custom_prompt.unwrap_or(existing.custom_prompt),
        profiles: existing.profiles,
        autostart: patch.autostart.unwrap_or(existing.autostart),
        whisper_mode: patch.whisper_mode.unwrap_or(existing.whisper_mode),
        command_hotkey: existing.command_hotkey,
        openai_api_key: match patch.openai_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.openai_api_key,
        },
        anthropic_api_key: match patch.anthropic_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.anthropic_api_key,
        },
        openrouter_api_key: match patch.openrouter_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.openrouter_api_key,
        },
        stt_provider: patch.stt_provider.unwrap_or(existing.stt_provider),
        llm_provider: patch.llm_provider.unwrap_or(existing.llm_provider),
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
            slots[0].hotkey = patch.hotkey.clone();
            slots[0].mode = patch.hotkey_mode;
            if let Some(v) = patch.insert_and_send_slot1 {
                slots[0].insert_and_send = v;
            }

            // Slot 1 -- updated only when the caller explicitly passes a value.
            if let Some(ref h2) = patch.hotkey_slot2 {
                slots[1].hotkey = h2.clone();
            }
            if let Some(ref m2_str) = patch.hotkey_mode_slot2 {
                slots[1].mode = m2_str.parse().unwrap_or(crate::config::HotkeyMode::Hold);
            }
            if let Some(v) = patch.insert_and_send_slot2 {
                slots[1].insert_and_send = v;
            }

            slots
        },
        output_language: patch.output_language.unwrap_or(existing.output_language),
        snippets: existing.snippets,
        voice_notes_hotkey: existing.voice_notes_hotkey,
        webhook_url: patch.webhook_url.unwrap_or(existing.webhook_url),
        turso_url: match patch.turso_url {
            Some(ref u) if !u.is_empty() => u.clone(),
            Some(ref u) if u.is_empty() => String::new(), // explicitly cleared
            _ => existing.turso_url,
        },
        turso_token: match patch.turso_token {
            Some(ref t) if !t.is_empty() => t.clone(),
            _ => existing.turso_token,
        },
        device_id: existing.device_id,
        bubble_size: patch.bubble_size.unwrap_or(existing.bubble_size),
        bubble_opacity: patch.bubble_opacity.unwrap_or(existing.bubble_opacity),
        advanced: existing.advanced,
        local_whisper_model: patch.local_whisper_model.unwrap_or(existing.local_whisper_model),
        local_whisper_gpu: patch.local_whisper_gpu.unwrap_or(existing.local_whisper_gpu),
        license_key: existing.license_key,
        license_validated_at: existing.license_validated_at,
        license_source: existing.license_source,
        ls_instance_id: existing.ls_instance_id,
        ls_last_validated_at: existing.ls_last_validated_at,
        bar_x: existing.bar_x,
        bar_y: existing.bar_y,
        insert_and_send: patch.insert_and_send.unwrap_or(existing.insert_and_send),
        autostop_silence_secs: patch.autostop_silence_secs.unwrap_or(existing.autostop_silence_secs),
        auto_mode_silence_secs: patch.auto_mode_silence_secs.unwrap_or(existing.auto_mode_silence_secs),
        bubble_recording_mode: patch.bubble_recording_mode.unwrap_or(existing.bubble_recording_mode),
        bubble_tap_mode: patch.bubble_tap_mode.unwrap_or(existing.bubble_tap_mode),
        bubble_tap_auto_send: patch.bubble_tap_auto_send.unwrap_or(existing.bubble_tap_auto_send),
        bubble_tap_silence_secs: patch.bubble_tap_silence_secs
            .unwrap_or(existing.bubble_tap_silence_secs),
        bubble_long_press_mode: patch.bubble_long_press_mode.unwrap_or(existing.bubble_long_press_mode),
        bubble_long_press_auto_send: patch.bubble_long_press_auto_send
            .unwrap_or(existing.bubble_long_press_auto_send),
        bubble_long_press_silence_secs: patch.bubble_long_press_silence_secs
            .unwrap_or(existing.bubble_long_press_silence_secs),
        onboarding: existing.onboarding,
        voice_command_enabled: existing.voice_command_enabled,
        first_install_at: existing.first_install_at,
        feedback_webhook_url: existing.feedback_webhook_url,
    }
}

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
#[allow(clippy::too_many_arguments)]
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
    _stt_priority: Option<Vec<String>>,
    // deprecated: ignored -- kept for backwards compatibility with older frontend versions
    _llm_priority: Option<Vec<String>>,
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
    openrouter_api_key: Option<String>,
) -> Result<(), String> {
    let inner = state.inner();

    // Trim whitespace from all API keys so that whitespace-only strings are
    // treated the same as empty strings (preserves existing key).
    let groq_api_key = groq_api_key.trim().to_string();
    let deepseek_api_key = deepseek_api_key.trim().to_string();
    let openai_api_key = openai_api_key.map(|k| k.trim().to_string());
    let anthropic_api_key = anthropic_api_key.map(|k| k.trim().to_string());
    let openrouter_api_key = openrouter_api_key.map(|k| k.trim().to_string());

    // License gate: Whisper Mode requires a paid license.
    if whisper_mode.unwrap_or(false) {
        require_license!(state, LicensedFeature::WhisperMode);
    }

    // Validate hotkey strings before writing anything to disk (desktop only).
    // Slot 0 (`hotkey` param) is always validated. Slot 1 (`hotkey_slot2`) is
    // only validated when non-empty -- empty string means "disable the slot".
    log::info!("[save_settings] hotkey={hotkey:?} mode={hotkey_mode:?}");
    #[cfg(desktop)]
    {
        hotkey
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| {
                log::warn!("[save_settings] Invalid shortcut: {e}");
                format!("Invalid shortcut string: {e}")
            })?;

        if let Some(ref h2) = hotkey_slot2 {
            if !h2.is_empty() {
                h2.parse::<tauri_plugin_global_shortcut::Shortcut>()
                    .map_err(|e| {
                        log::warn!("[save_settings] Invalid slot-2 shortcut: {e}");
                        format!("Invalid slot-2 shortcut string: {e}")
                    })?;
            }
        }
    }

    // Build updated config by delegating to the pure merge function.
    // Empty API key strings preserve the existing key so the user can change
    // other settings without re-entering keys.
    let patch = SettingsPatch {
        groq_api_key,
        deepseek_api_key,
        language,
        cleanup_style,
        hotkey,
        hotkey_mode,
        audio_device,
        stt_model,
        custom_prompt,
        autostart,
        whisper_mode,
        openai_api_key,
        anthropic_api_key,
        output_language,
        webhook_url,
        turso_url,
        turso_token,
        bubble_size,
        bubble_opacity,
        local_whisper_model,
        local_whisper_gpu,
        stt_provider,
        llm_provider,
        insert_and_send,
        autostop_silence_secs,
        auto_mode_silence_secs,
        hotkey_slot2,
        hotkey_mode_slot2,
        insert_and_send_slot1,
        insert_and_send_slot2,
        bubble_recording_mode,
        bubble_tap_mode,
        bubble_tap_auto_send,
        bubble_tap_silence_secs,
        bubble_long_press_mode,
        bubble_long_press_auto_send,
        bubble_long_press_silence_secs,
        openrouter_api_key,
    };
    let new_cfg = inner.save_config_locked("settings", |cfg| {
        *cfg = merge_settings(cfg.clone(), patch);
    })?;

    // Resolve providers from the persisted config and hot-reload them.
    let (new_stt, new_cleanup) = resolve_providers(&new_cfg, &inner.app_data_dir);
    *crate::write_lock!(inner.stt_provider)? = new_stt;
    *crate::write_lock!(inner.cleanup_provider)? = new_cleanup;

    // Re-register all hotkey slots from the (now-updated) in-memory config (desktop only).
    // Non-fatal: hotkey re-registration is a side-effect. API key changes and other
    // settings are already saved at this point -- don't roll back because of a
    // hotkey conflict (e.g. "already registered" race or duplicate slots).
    #[cfg(desktop)]
    if let Err(e) = crate::pipeline::register_hotkey(&handle) {
        log::warn!("[settings] Hotkey re-registration failed (settings saved anyway): {e}");
    }

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
        .hotkey_slots.first()
        .map(|s| s.hotkey.clone())
        .unwrap_or_else(|| cfg.hotkey.clone());
    let slot0_mode = cfg
        .hotkey_slots.first()
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
        openrouter_api_key_masked: mask_api_key(&cfg.openrouter_api_key),
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
        insert_and_send_slot1: cfg.hotkey_slots.first().map(|s| s.insert_and_send).unwrap_or(false),
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
        voice_command_enabled: cfg.voice_command_enabled,
        feedback_webhook_url: cfg.feedback_webhook_url,
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

    state
        .inner()
        .save_config_locked("advanced settings", |cfg| cfg.advanced = settings)?;
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

    inner.save_config_locked("API keys", |cfg| {
        if let Some(ref key) = groq_api_key {
            cfg.groq_api_key = key.clone();
        }
        if let Some(ref key) = deepseek_api_key {
            cfg.deepseek_api_key = key.clone();
        }
    })?;

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
    state
        .inner()
        .save_config_locked("language setting", |cfg| cfg.language = language)?;
    Ok(())
}

/// Sets the cleanup style used by the hotkey pipeline and persists the change.
#[tauri::command]
pub fn set_cleanup_style(state: State<'_, AppState>, style: CleanupStyle) -> Result<(), String> {
    state
        .inner()
        .save_config_locked("cleanup style", |cfg| cfg.cleanup_style = style)?;
    Ok(())
}

/// Sets the output language for translation and persists the change.
///
/// `language`: ISO-639-1 code, e.g. `"en"` to translate to English.
/// Empty string = no translation (dictation stays in original language).
#[tauri::command]
pub fn set_output_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    state
        .inner()
        .save_config_locked("output language setting", |cfg| cfg.output_language = language)?;
    Ok(())
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
    inner.save_config_locked("hotkey setting", |cfg| {
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
    })?;

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

/// Returns the title of the last window that was active before Klarvo received
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
// Onboarding commands
// ---------------------------------------------------------------------------

/// Returns the current onboarding wizard state.
///
/// The frontend reads this on startup to decide whether to show the wizard,
/// resume from a saved step, or skip it entirely.
#[tauri::command]
pub fn get_onboarding_state(
    state: State<'_, AppState>,
) -> Result<crate::config::OnboardingState, String> {
    let cfg = crate::lock!(state.inner().config)?;
    Ok(cfg.onboarding.clone())
}

/// Updates the onboarding wizard state in-memory and persists it to disk.
///
/// The frontend calls this after each wizard step so progress survives a
/// restart. Overwrites the entire `onboarding` sub-object -- the frontend
/// always passes the complete current state.
#[tauri::command]
pub fn set_onboarding_state(
    state: State<'_, AppState>,
    onboarding_state: crate::config::OnboardingState,
) -> Result<(), String> {
    state
        .inner()
        .save_config_locked("onboarding state", |cfg| cfg.onboarding = onboarding_state)?;
    Ok(())
}

/// Validates an API key by making a minimal authenticated request to the
/// provider's models endpoint.
///
/// Returns:
/// - `Ok(true)`  — HTTP 200: key is valid and accepted.
/// - `Ok(false)` — HTTP 401/403: key is invalid or lacks permissions.
/// - `Err(msg)`  — Network error, timeout, or unexpected HTTP status.
///
/// Supported providers: `"groq"`, `"openai"`, `"deepseek"`, `"openrouter"`.
/// Unknown provider strings return `Err`.
///
/// This command takes the provider name and key directly as parameters so
/// it can be called before any key is persisted (e.g. while the user is
/// still typing in the wizard).
#[tauri::command]
pub async fn validate_api_key(provider: String, key: String) -> Result<bool, String> {
    let url = match provider.as_str() {
        "groq"       => "https://api.groq.com/openai/v1/models",
        "openai"     => "https://api.openai.com/v1/models",
        "deepseek"   => "https://api.deepseek.com/v1/models",
        "openrouter" => "https://openrouter.ai/api/v1/models",
        other => {
            return Err(format!("Unknown provider: {other:?}. Supported: groq, openai, deepseek, openrouter"));
        }
    };

    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .bearer_auth(&key)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error while validating {provider} key: {e}"))?;

    match response.status().as_u16() {
        200 => Ok(true),
        401 | 403 => Ok(false),
        other => Err(format!(
            "Unexpected HTTP {other} from {provider} validation endpoint"
        )),
    }
}

/// Clears (deletes) the stored API key for a given provider.
///
/// After clearing, the key is set to an empty string in memory and on disk.
/// If the cleared key was used by the active STT or LLM provider, the
/// provider is hot-reloaded (typically falls back to another configured key
/// or becomes unavailable).
///
/// Supported providers: `"groq"`, `"deepseek"`, `"openai"`, `"anthropic"`,
/// `"openrouter"`.
///
/// Returns `Err` for unknown provider strings.
#[tauri::command]
pub async fn clear_api_key(
    handle: AppHandle,
    state: State<'_, AppState>,
    provider: String,
) -> Result<(), String> {
    let inner = state.inner();

    // Validate the provider up front so we fail before acquiring any lock.
    let provider = provider.as_str();
    if !matches!(provider, "groq" | "deepseek" | "openai" | "anthropic" | "openrouter") {
        return Err(format!(
            "Unknown provider: {provider:?}. Supported: groq, deepseek, openai, anthropic, openrouter"
        ));
    }

    let new_cfg = inner.save_config_locked("config after clearing API key", |cfg| match provider {
        "groq" => cfg.groq_api_key = String::new(),
        "deepseek" => cfg.deepseek_api_key = String::new(),
        "openai" => cfg.openai_api_key = String::new(),
        "anthropic" => cfg.anthropic_api_key = String::new(),
        "openrouter" => cfg.openrouter_api_key = String::new(),
        _ => unreachable!("provider validated above"),
    })?;

    // Resolve new providers from the persisted config and hot-reload.
    let (new_stt, new_cleanup) = resolve_providers(&new_cfg, &inner.app_data_dir);
    *crate::write_lock!(inner.stt_provider)?     = new_stt;
    *crate::write_lock!(inner.cleanup_provider)? = new_cleanup;

    log::info!("[clear_api_key] Cleared {provider} API key; providers hot-reloaded");

    // Silence the "unused variable" warning on non-desktop builds where the
    // handle is only used for hotkey re-registration (which lives in desktop
    // code paths).
    let _ = &handle;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::config::{load_config, save_config, AppConfig, HotkeyMode, HotkeySlot};
    use crate::llm::CleanupStyle;
    use std::sync::Arc;

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

    // -----------------------------------------------------------------------
    // mask_api_key: whitespace-only key produces empty string (not "****")
    // -----------------------------------------------------------------------

    /// Whitespace-only keys are treated as empty by `mask_api_key`.
    #[test]
    fn test_mask_api_key_whitespace_only_returns_empty() {
        assert_eq!(
            crate::mask_api_key("   "),
            "",
            "whitespace-only key must produce an empty mask, not \"****\""
        );
        assert_eq!(
            crate::mask_api_key("\t\n"),
            "",
            "tab/newline key must produce an empty mask"
        );
    }

    /// A real key is still masked correctly after the trim change.
    #[test]
    fn test_mask_api_key_trims_surrounding_whitespace() {
        // The key itself has surrounding spaces -- after trim the last-4 is "5678"
        let masked = crate::mask_api_key("  sk-12345678  ");
        assert_eq!(masked, "****5678");
    }

    /// Empty string produces an empty mask (unchanged behaviour).
    #[test]
    fn test_mask_api_key_empty_produces_empty() {
        assert_eq!(crate::mask_api_key(""), "");
    }

    // -----------------------------------------------------------------------
    // clear_api_key: config-level key clearing via save_config / load_config
    // -----------------------------------------------------------------------

    /// Clearing a key via save_config → load_config produces an empty key field.
    ///
    /// This tests the config-layer half of `clear_api_key` (the Tauri command
    /// itself requires a live AppState and is integration-tested manually).
    #[test]
    fn test_clear_api_key_persists_empty_string() {
        let dir = temp_dir();

        // Start with a config that has all five keys set.
        let cfg_before = AppConfig {
            groq_api_key: "gsk_testkey".to_string(),
            deepseek_api_key: "dsk_testkey".to_string(),
            openai_api_key: "oai_testkey".to_string(),
            anthropic_api_key: "ant_testkey".to_string(),
            openrouter_api_key: "or_testkey".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg_before).expect("initial save_config");

        // Simulate clearing the groq key.
        let mut cfg_after = load_config(dir.path());
        cfg_after.groq_api_key = String::new();
        save_config(dir.path(), &cfg_after).expect("save after clear");

        let loaded = load_config(dir.path());
        assert_eq!(
            loaded.groq_api_key, "",
            "groq_api_key must be empty after explicit clear"
        );
        // Other keys must remain untouched.
        assert_eq!(loaded.deepseek_api_key, "dsk_testkey");
        assert_eq!(loaded.openai_api_key, "oai_testkey");
        assert_eq!(loaded.anthropic_api_key, "ant_testkey");
        assert_eq!(loaded.openrouter_api_key, "or_testkey");
    }

    // -----------------------------------------------------------------------
    // Characterization tests for merge_settings (Task 2.1, Phase 2)
    //
    // These tests call the PRODUCTION `merge_settings` function, not a copy.
    // They characterize the CURRENT behaviour -- anomalies are intentionally
    // preserved as-is. Each anomaly is named A1-A5 matching the briefing.
    //
    // DO NOT correct anomalies here. The purpose is to nail down the existing
    // behaviour so future refactoring can detect regressions.
    // -----------------------------------------------------------------------

    use super::{merge_settings, SettingsPatch};

    /// Helper: a non-default `existing` config with all common fields set to
    /// distinct sentinel values so we can verify which ones survive a patch.
    fn existing_with_sentinels() -> AppConfig {
        AppConfig {
            groq_api_key: "existing-groq".to_string(),
            deepseek_api_key: "existing-deepseek".to_string(),
            openai_api_key: "existing-openai".to_string(),
            anthropic_api_key: "existing-anthropic".to_string(),
            openrouter_api_key: "existing-openrouter".to_string(),
            language: "de".to_string(),
            cleanup_style: CleanupStyle::Verbatim,
            hotkey: "ctrl+shift+x".to_string(),
            hotkey_mode: HotkeyMode::Toggle,
            stt_model: Some("whisper-large-v3".to_string()).unwrap_or_default(),
            custom_prompt: "existing-prompt".to_string(),
            autostart: true,
            whisper_mode: true,
            output_language: "en".to_string(),
            webhook_url: "https://existing.webhook".to_string(),
            turso_url: "libsql://existing.turso.io".to_string(),
            turso_token: "existing-turso-token".to_string(),
            bubble_size: 1.5,
            bubble_opacity: 0.5,
            local_whisper_model: "base".to_string(),
            local_whisper_gpu: false,
            stt_provider: "openai".to_string(),
            llm_provider: "anthropic".to_string(),
            insert_and_send: true,
            autostop_silence_secs: 5.0,
            auto_mode_silence_secs: 6.0,
            bubble_recording_mode: "toggle".to_string(),
            bubble_tap_mode: "autostop".to_string(),
            bubble_tap_auto_send: true,
            bubble_tap_silence_secs: 3.0,
            bubble_long_press_mode: "auto".to_string(),
            bubble_long_press_auto_send: true,
            bubble_long_press_silence_secs: 4.0,
            // Fields never touched by merge_settings:
            command_hotkey: "ctrl+shift+e".to_string(),
            voice_notes_hotkey: "ctrl+shift+n".to_string(),
            device_id: "existing-device-id".to_string(),
            license_key: "existing-license".to_string(),
            license_validated_at: 9999,
            license_source: "hmac".to_string(),
            ls_instance_id: "existing-ls-id".to_string(),
            ls_last_validated_at: 8888,
            bar_x: Some(100.0),
            bar_y: Some(200.0),
            hotkey_slots: vec![
                HotkeySlot { hotkey: "ctrl+shift+x".to_string(), mode: HotkeyMode::Toggle, insert_and_send: false },
                HotkeySlot { hotkey: "ctrl+shift+y".to_string(), mode: HotkeyMode::Hold, insert_and_send: true },
            ],
            ..AppConfig::default()
        }
    }

    // -----------------------------------------------------------------------
    // Happy Path: fully populated patch replaces all settable fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_settings_happy_path_full_patch() {
        let existing = existing_with_sentinels();
        let patch = SettingsPatch {
            groq_api_key: "new-groq".to_string(),
            deepseek_api_key: "new-deepseek".to_string(),
            language: "en".to_string(),
            cleanup_style: CleanupStyle::Chat,
            hotkey: "ctrl+shift+q".to_string(),
            hotkey_mode: HotkeyMode::AutoStop,
            audio_device: Some("Microphone XYZ".to_string()),
            stt_model: Some("whisper-small".to_string()),
            custom_prompt: Some("new-prompt".to_string()),
            autostart: Some(false),
            whisper_mode: Some(false),
            openai_api_key: Some("new-openai".to_string()),
            anthropic_api_key: Some("new-anthropic".to_string()),
            output_language: Some("de".to_string()),
            webhook_url: Some("https://new.webhook".to_string()),
            turso_url: Some("libsql://new.turso.io".to_string()),
            turso_token: Some("new-turso-token".to_string()),
            bubble_size: Some(2.0),
            bubble_opacity: Some(0.9),
            local_whisper_model: Some("small".to_string()),
            local_whisper_gpu: Some(true),
            stt_provider: Some("local".to_string()),
            llm_provider: Some("deepseek".to_string()),
            insert_and_send: Some(false),
            autostop_silence_secs: Some(1.5),
            auto_mode_silence_secs: Some(2.5),
            hotkey_slot2: Some("ctrl+shift+z".to_string()),
            hotkey_mode_slot2: Some("toggle".to_string()),
            insert_and_send_slot1: Some(true),
            insert_and_send_slot2: Some(false),
            bubble_recording_mode: Some("auto".to_string()),
            bubble_tap_mode: Some("hold".to_string()),
            bubble_tap_auto_send: Some(false),
            bubble_tap_silence_secs: Some(1.0),
            bubble_long_press_mode: Some("toggle".to_string()),
            bubble_long_press_auto_send: Some(false),
            bubble_long_press_silence_secs: Some(1.5),
            openrouter_api_key: Some("new-openrouter".to_string()),
        };

        let result = merge_settings(existing, patch);

        assert_eq!(result.groq_api_key, "new-groq");
        assert_eq!(result.deepseek_api_key, "new-deepseek");
        assert_eq!(result.language, "en");
        assert_eq!(result.cleanup_style, CleanupStyle::Chat);
        assert_eq!(result.hotkey, "ctrl+shift+q");
        assert_eq!(result.hotkey_mode, HotkeyMode::AutoStop);
        assert_eq!(result.audio_device, Some("Microphone XYZ".to_string()));
        assert_eq!(result.stt_model, "whisper-small");
        assert_eq!(result.custom_prompt, "new-prompt");
        assert!(!result.autostart);
        assert!(!result.whisper_mode);
        assert_eq!(result.openai_api_key, "new-openai");
        assert_eq!(result.anthropic_api_key, "new-anthropic");
        assert_eq!(result.openrouter_api_key, "new-openrouter");
        assert_eq!(result.output_language, "de");
        assert_eq!(result.webhook_url, "https://new.webhook");
        assert_eq!(result.turso_url, "libsql://new.turso.io");
        assert_eq!(result.turso_token, "new-turso-token");
        assert!((result.bubble_size - 2.0).abs() < f32::EPSILON);
        assert!((result.bubble_opacity - 0.9).abs() < f32::EPSILON);
        assert_eq!(result.local_whisper_model, "small");
        assert!(result.local_whisper_gpu);
        assert_eq!(result.stt_provider, "local");
        assert_eq!(result.llm_provider, "deepseek");
        assert!(!result.insert_and_send);
        assert!((result.autostop_silence_secs - 1.5).abs() < f32::EPSILON);
        assert!((result.auto_mode_silence_secs - 2.5).abs() < f32::EPSILON);
        assert_eq!(result.hotkey_slots[0].hotkey, "ctrl+shift+q");
        assert_eq!(result.hotkey_slots[0].mode, HotkeyMode::AutoStop);
        assert!(result.hotkey_slots[0].insert_and_send);
        assert_eq!(result.hotkey_slots[1].hotkey, "ctrl+shift+z");
        assert_eq!(result.hotkey_slots[1].mode, HotkeyMode::Toggle);
        assert!(!result.hotkey_slots[1].insert_and_send);
        assert_eq!(result.bubble_recording_mode, "auto");
        assert_eq!(result.bubble_tap_mode, "hold");
        assert!(!result.bubble_tap_auto_send);
        assert!((result.bubble_tap_silence_secs - 1.0).abs() < f32::EPSILON);
        assert_eq!(result.bubble_long_press_mode, "toggle");
        assert!(!result.bubble_long_press_auto_send);
        assert!((result.bubble_long_press_silence_secs - 1.5).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Empty-Key-Preserve: empty string preserves existing groq/deepseek keys
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_settings_empty_groq_key_preserves_existing() {
        let existing = AppConfig {
            groq_api_key: "existing-groq".to_string(),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            groq_api_key: String::new(), // empty → preserve existing
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        assert_eq!(result.groq_api_key, "existing-groq",
            "empty groq_api_key in patch must preserve existing key");
    }

    #[test]
    fn test_merge_settings_empty_deepseek_key_preserves_existing() {
        let existing = AppConfig {
            deepseek_api_key: "existing-deepseek".to_string(),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            deepseek_api_key: String::new(),
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        assert_eq!(result.deepseek_api_key, "existing-deepseek",
            "empty deepseek_api_key in patch must preserve existing key");
    }

    /// `Some("")` for openai/anthropic/openrouter preserves existing key
    /// (match-guard `if !k.is_empty()` falls through to the `_` arm).
    #[test]
    fn test_merge_settings_some_empty_optional_key_preserves_existing() {
        let existing = AppConfig {
            openai_api_key: "existing-openai".to_string(),
            anthropic_api_key: "existing-anthropic".to_string(),
            openrouter_api_key: "existing-openrouter".to_string(),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            openai_api_key: Some(String::new()),
            anthropic_api_key: Some(String::new()),
            openrouter_api_key: Some(String::new()),
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        assert_eq!(result.openai_api_key, "existing-openai",
            "Some(\"\") for openai_api_key must preserve existing key");
        assert_eq!(result.anthropic_api_key, "existing-anthropic",
            "Some(\"\") for anthropic_api_key must preserve existing key");
        assert_eq!(result.openrouter_api_key, "existing-openrouter",
            "Some(\"\") for openrouter_api_key must preserve existing key");
    }

    // -----------------------------------------------------------------------
    // A1: turso_token NOT clearable via Some(""), but turso_url IS
    // -----------------------------------------------------------------------

    /// A1a: `turso_token: Some("")` falls through to `_ => existing.turso_token`
    /// (match arm `Some(ref t) if !t.is_empty()` does not match; there is no
    /// explicit empty-clear arm unlike turso_url). Token is NOT cleared.
    #[test]
    fn test_anomaly_a1_turso_token_some_empty_not_clearable() {
        let existing = AppConfig {
            turso_token: "existing-token".to_string(),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            turso_token: Some(String::new()), // intended to clear, but won't
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        // ANOMALY: token is NOT cleared; existing value survives.
        assert_eq!(result.turso_token, "existing-token",
            "A1: turso_token Some(\"\") does NOT clear existing token (anomaly, not a bug fix)");
    }

    /// A1b: `turso_url: Some("")` IS cleared because there is an explicit
    /// `Some(ref u) if u.is_empty() => String::new()` arm.
    #[test]
    fn test_anomaly_a1_turso_url_some_empty_is_clearable() {
        let existing = AppConfig {
            turso_url: "libsql://existing.turso.io".to_string(),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            turso_url: Some(String::new()), // explicitly cleared
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        // Correct (asymmetric from token): url IS cleared.
        assert_eq!(result.turso_url, "",
            "A1: turso_url Some(\"\") clears the URL (asymmetric with turso_token)");
    }

    // -----------------------------------------------------------------------
    // A2: audio_device overwrites unconditionally (None sets to None)
    // -----------------------------------------------------------------------

    /// A2: `audio_device: None` in patch sets result to `None`, discarding any
    /// existing device selection. Unlike `.unwrap_or(existing.X)` fields, this
    /// field uses direct assignment without a fallback.
    #[test]
    fn test_anomaly_a2_audio_device_none_overwrites_existing() {
        let existing = AppConfig {
            audio_device: Some("Existing Microphone".to_string()),
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            audio_device: None, // no device in patch → wipes existing selection
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        // ANOMALY: existing device is discarded; result is None (system default).
        assert_eq!(result.audio_device, None,
            "A2: audio_device=None in patch overwrites existing (no unwrap_or fallback)");
    }

    // -----------------------------------------------------------------------
    // A3: hotkey_mode_slot2 unknown string → HotkeyMode::Hold (silent)
    // -----------------------------------------------------------------------

    /// A3: An unrecognised `hotkey_mode_slot2` string is silently mapped to
    /// `HotkeyMode::Hold` via `.parse().unwrap_or(HotkeyMode::Hold)`.
    /// No error is returned to the caller.
    #[test]
    fn test_anomaly_a3_hotkey_mode_slot2_unknown_string_becomes_hold() {
        let existing = AppConfig::default();
        let patch = SettingsPatch {
            hotkey: "ctrl+shift+d".to_string(),
            hotkey_slot2: Some("ctrl+shift+e".to_string()),
            hotkey_mode_slot2: Some("nonexistent_mode".to_string()), // unknown
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        // ANOMALY: unknown mode is silently swallowed; slot 1 mode = Hold.
        assert_eq!(result.hotkey_slots[1].mode, HotkeyMode::Hold,
            "A3: unknown hotkey_mode_slot2 string maps silently to Hold (no error propagated)");
    }

    // -----------------------------------------------------------------------
    // A4: fields never touched by merge_settings (always from existing)
    // -----------------------------------------------------------------------

    /// A4: A fully-populated patch cannot change any of the "read-only-in-merge"
    /// fields. They always come from `existing`.
    #[test]
    fn test_anomaly_a4_non_patchable_fields_always_from_existing() {
        let existing = existing_with_sentinels();
        // Patch with all settable fields populated -- does not matter what values
        // we pick because the fields below are never read from patch.
        let patch = SettingsPatch::default();

        let result = merge_settings(existing.clone(), patch);

        // These fields must always equal their `existing` values.
        assert_eq!(result.command_hotkey, existing.command_hotkey,
            "command_hotkey must come from existing");
        assert_eq!(result.voice_notes_hotkey, existing.voice_notes_hotkey,
            "voice_notes_hotkey must come from existing");
        assert_eq!(result.snippets, existing.snippets,
            "snippets must come from existing");
        assert_eq!(result.profiles, existing.profiles,
            "profiles must come from existing");
        assert_eq!(result.advanced, existing.advanced,
            "advanced must come from existing");
        assert_eq!(result.license_key, existing.license_key,
            "license_key must come from existing");
        assert_eq!(result.license_validated_at, existing.license_validated_at,
            "license_validated_at must come from existing");
        assert_eq!(result.license_source, existing.license_source,
            "license_source must come from existing");
        assert_eq!(result.ls_instance_id, existing.ls_instance_id,
            "ls_instance_id must come from existing");
        assert_eq!(result.ls_last_validated_at, existing.ls_last_validated_at,
            "ls_last_validated_at must come from existing");
        assert_eq!(result.bar_x, existing.bar_x,
            "bar_x must come from existing");
        assert_eq!(result.bar_y, existing.bar_y,
            "bar_y must come from existing");
        assert_eq!(result.onboarding, existing.onboarding,
            "onboarding must come from existing");
        assert_eq!(result.voice_command_enabled, existing.voice_command_enabled,
            "voice_command_enabled must come from existing");
        assert_eq!(result.first_install_at, existing.first_install_at,
            "first_install_at must come from existing");
        assert_eq!(result.feedback_webhook_url, existing.feedback_webhook_url,
            "feedback_webhook_url must come from existing");
        assert_eq!(result.device_id, existing.device_id,
            "device_id must come from existing");
        assert_eq!(result.stt_priority, existing.stt_priority,
            "stt_priority (deprecated) must come from existing");
        assert_eq!(result.llm_priority, existing.llm_priority,
            "llm_priority (deprecated) must come from existing");
    }

    // -----------------------------------------------------------------------
    // A5: deprecated global `insert_and_send` is written by merge_settings
    // -----------------------------------------------------------------------

    /// A5: `merge_settings` still writes the deprecated global `insert_and_send`
    /// field (Z.321 of the original) via `patch.insert_and_send.unwrap_or(existing)`.
    ///
    /// Note: `load_config` has a SEPARATE migration that moves
    /// `global insert_and_send=true` into slots. That migration is NOT tested
    /// here (it belongs to config/mod.rs tests). What we test is that the merge
    /// step itself writes the global flag as-is.
    #[test]
    fn test_anomaly_a5_deprecated_global_insert_and_send_is_written() {
        let existing = AppConfig { insert_and_send: false, ..AppConfig::default() };
        let patch = SettingsPatch {
            insert_and_send: Some(true),
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        // ANOMALY: deprecated global flag is written by merge_settings.
        // The load_config migration (out of scope here) will later move it to slots.
        assert!(result.insert_and_send,
            "A5: merge_settings writes deprecated global insert_and_send (migration handled by load_config separately)");
    }

    // -----------------------------------------------------------------------
    // Slot behaviour: slot 1 only updated when hotkey_slot2 is Some
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_settings_slot1_unchanged_when_hotkey_slot2_is_none() {
        let existing = AppConfig {
            hotkey_slots: vec![
                HotkeySlot { hotkey: "ctrl+shift+d".to_string(), mode: HotkeyMode::Hold, insert_and_send: false },
                HotkeySlot { hotkey: "ctrl+shift+e".to_string(), mode: HotkeyMode::Toggle, insert_and_send: true },
            ],
            ..AppConfig::default()
        };
        let patch = SettingsPatch {
            hotkey: "ctrl+shift+d".to_string(),
            hotkey_slot2: None, // not provided → slot 1 unchanged
            hotkey_mode_slot2: None,
            insert_and_send_slot2: None,
            ..SettingsPatch::default()
        };
        let result = merge_settings(existing, patch);
        assert_eq!(result.hotkey_slots[1].hotkey, "ctrl+shift+e",
            "slot 1 hotkey must be unchanged when hotkey_slot2 is None");
        assert_eq!(result.hotkey_slots[1].mode, HotkeyMode::Toggle,
            "slot 1 mode must be unchanged when hotkey_mode_slot2 is None");
        assert!(result.hotkey_slots[1].insert_and_send,
            "slot 1 insert_and_send must be unchanged when insert_and_send_slot2 is None");
    }

    // -----------------------------------------------------------------------
    // ROB-04 / Story 1.3 + Story 4.3: disk-write serialization via the
    // `AppState::save_config_locked` choke-point.
    //
    // These specs drive the REAL production helper (not a hand-rolled copy of the
    // lock dance), so dropping the lock discipline from `save_config_locked` makes
    // them fail.
    // -----------------------------------------------------------------------

    /// Concurrent API-key save and bar-drag save, both through `save_config_locked`.
    /// Because the helper serializes the whole read-modify-write cycle behind
    /// `config_disk_write`, the later writer reads the earlier writer's update and
    /// preserves it: the final on-disk config contains BOTH the new key AND the new
    /// bar position. Without the lock the two read-modify-writes could interleave and
    /// the last whole-file write would clobber the other's field.
    #[test]
    fn test_save_config_locked_serializes_concurrent_saves() {
        use crate::dictionary::Dictionary;
        use crate::AppState;

        let dir = temp_dir();
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let cfg = AppConfig {
            groq_api_key: "initial-key".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();

        let state = Arc::new(AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db));
        let barrier = Arc::new(std::sync::Barrier::new(2));

        // Thread A: new API key. Thread B: bar position. The barrier maximises the race
        // window; the helper's internal lock is what makes the outcome safe.
        let state_a = Arc::clone(&state);
        let barrier_a = Arc::clone(&barrier);
        let h_a = std::thread::spawn(move || {
            barrier_a.wait();
            state_a
                .save_config_locked("test-key", |cfg| cfg.groq_api_key = "new-key".to_string())
                .unwrap();
        });

        let state_b = Arc::clone(&state);
        let barrier_b = Arc::clone(&barrier);
        let h_b = std::thread::spawn(move || {
            barrier_b.wait();
            state_b
                .save_config_locked("test-bar", |cfg| {
                    cfg.bar_x = Some(123.0);
                    cfg.bar_y = Some(456.0);
                })
                .unwrap();
        });

        h_a.join().unwrap();
        h_b.join().unwrap();

        let final_cfg = load_config(dir.path());
        assert_eq!(final_cfg.groq_api_key, "new-key",
            "API key must survive: save_config_locked serializes the read-modify-write cycle");
        assert_eq!(final_cfg.bar_x, Some(123.0), "bar_x must survive");
        assert_eq!(final_cfg.bar_y, Some(456.0), "bar_y must survive");
    }

    /// Three concurrent savers hammering distinct fields through `save_config_locked`,
    /// repeated over many rounds. Every field set by every saver must be present at the end.
    /// This is a *probabilistic* negative control, not a deterministic one: the `config`
    /// mutex already serializes the in-memory mutations, so a single round could pass even
    /// with `config_disk_write` removed — but across 16 rounds of 3-way racing, an
    /// unserialized helper would very likely let one whole-file write clobber another's field
    /// in some round. Sound by construction (barrier-of-3 matches 3 threads; consistent lock
    /// order; no false failures while the lock is present).
    #[test]
    fn test_save_config_locked_serializes_concurrent_saves_three_way() {
        use crate::dictionary::Dictionary;
        use crate::AppState;

        for _round in 0..16 {
            let dir = temp_dir();
            let db = rusqlite::Connection::open_in_memory().unwrap();
            save_config(dir.path(), &AppConfig::default()).unwrap();
            let state = Arc::new(AppState::new(
                AppConfig::default(),
                Dictionary::new(),
                dir.path().to_path_buf(),
                db,
            ));
            let barrier = Arc::new(std::sync::Barrier::new(3));

            let mut handles = Vec::new();
            for which in [0u8, 1u8, 2u8] {
                let st = Arc::clone(&state);
                let bar = Arc::clone(&barrier);
                handles.push(std::thread::spawn(move || {
                    bar.wait();
                    st.save_config_locked("test", |cfg| match which {
                        0 => cfg.groq_api_key = "k".to_string(),
                        1 => {
                            cfg.bar_x = Some(7.0);
                            cfg.bar_y = Some(8.0);
                        }
                        _ => cfg.language = "de".to_string(),
                    })
                    .unwrap();
                }));
            }
            for h in handles {
                h.join().unwrap();
            }

            let final_cfg = load_config(dir.path());
            assert_eq!(final_cfg.groq_api_key, "k", "key survived the round");
            assert_eq!(final_cfg.bar_x, Some(7.0), "bar_x survived the round");
            assert_eq!(final_cfg.language, "de", "language survived the round");
        }
    }

    /// `save_config_locked` updates the in-memory config AND the on-disk file to the
    /// same snapshot, and returns that snapshot. This is the helper's core contract and
    /// binds the tests to the real production path: a memory-update or disk-write
    /// regression inside the helper fails here. (Replaces Story 1.3's Spec B, whose
    /// "config free during I/O" property is now structural — the helper drops `config`
    /// in an inner block before the disk write, by construction.)
    #[test]
    fn test_save_config_locked_updates_memory_and_disk_coherently() {
        use crate::dictionary::Dictionary;
        use crate::AppState;

        let dir = temp_dir();
        let db = rusqlite::Connection::open_in_memory().unwrap();
        save_config(dir.path(), &AppConfig::default()).unwrap();
        let state = AppState::new(
            AppConfig::default(),
            Dictionary::new(),
            dir.path().to_path_buf(),
            db,
        );

        let returned = state
            .save_config_locked("coherence", |cfg| {
                cfg.groq_api_key = "k".to_string();
                cfg.output_language = "en".to_string();
            })
            .unwrap();

        // Returned snapshot reflects the mutation.
        assert_eq!(returned.groq_api_key, "k");
        assert_eq!(returned.output_language, "en");
        // In-memory config updated.
        {
            let mem = state.config.lock().unwrap();
            assert_eq!(mem.groq_api_key, "k", "in-memory config must be updated");
            assert_eq!(mem.output_language, "en");
        }
        // On-disk config updated to the same values.
        let on_disk = load_config(dir.path());
        assert_eq!(on_disk.groq_api_key, "k", "on-disk config must match memory");
        assert_eq!(on_disk.output_language, "en");
    }
}
