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
    stt_priority: Option<Vec<String>>,
    llm_priority: Option<Vec<String>>,
    output_language: Option<String>,
    webhook_url: Option<String>,
    turso_url: Option<String>,
    turso_token: Option<String>,
    bubble_size: Option<f32>,
    bubble_opacity: Option<f32>,
) -> Result<(), String> {
    let inner = state.inner();

    // License gate: Whisper Mode requires a paid license.
    if whisper_mode.unwrap_or(false) {
        require_license!(state, LicensedFeature::WhisperMode);
    }

    // Validate the hotkey string before writing anything to disk (desktop only).
    println!("[save_settings] hotkey={hotkey:?} mode={hotkey_mode:?}");
    #[cfg(desktop)]
    let parsed_shortcut = hotkey
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map_err(|e| {
            println!("[save_settings] Invalid shortcut: {e}");
            format!("Invalid shortcut string: {e}")
        })?;

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
        hotkey,
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
        stt_priority: stt_priority.unwrap_or(existing.stt_priority),
        llm_priority: llm_priority.unwrap_or(existing.llm_priority),
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
        license_key: existing.license_key,
        license_validated_at: existing.license_validated_at,
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

    // Re-register the global shortcut with the (possibly new) hotkey + mode (desktop only).
    #[cfg(desktop)]
    crate::pipeline::register_hotkey(&handle, parsed_shortcut, hotkey_mode)?;

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

    Ok(SettingsView {
        groq_api_key_masked: mask_api_key(&cfg.groq_api_key),
        deepseek_api_key_masked: mask_api_key(&cfg.deepseek_api_key),
        language: cfg.language,
        cleanup_style: cfg.cleanup_style,
        hotkey: cfg.hotkey,
        hotkey_mode: cfg.hotkey_mode,
        audio_device: cfg.audio_device,
        stt_model: cfg.stt_model,
        custom_prompt: cfg.custom_prompt,
        autostart: cfg.autostart,
        whisper_mode: cfg.whisper_mode,
        openai_api_key_masked: mask_api_key(&cfg.openai_api_key),
        anthropic_api_key_masked: mask_api_key(&cfg.anthropic_api_key),
        stt_priority: cfg.stt_priority,
        llm_priority: cfg.llm_priority,
        output_language: cfg.output_language,
        webhook_url: cfg.webhook_url,
        turso_url: cfg.turso_url,
        turso_token_masked: mask_api_key(&cfg.turso_token),
        device_id: cfg.device_id,
        bubble_size: cfg.bubble_size,
        bubble_opacity: cfg.bubble_opacity,
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
///
/// Returns an error if the shortcut string is invalid or registration fails.
/// Persists both the new shortcut and mode to config.
#[tauri::command]
pub async fn set_hotkey(
    handle: AppHandle,
    state: State<'_, AppState>,
    shortcut: String,
    mode: HotkeyMode,
) -> Result<(), String> {
    // Validate and register the shortcut (desktop only).
    #[cfg(desktop)]
    {
        let parsed = shortcut
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .map_err(|e| format!("Invalid shortcut string: {e}"))?;
        crate::pipeline::register_hotkey(&handle, parsed, mode)?;
    }

    // Persist both fields to config.
    let inner = state.inner();
    let mut cfg = crate::lock!(inner.config)?;
    cfg.hotkey = shortcut;
    cfg.hotkey_mode = mode;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist hotkey setting: {e}"))
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
