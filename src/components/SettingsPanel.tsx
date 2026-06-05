import { useState, useEffect, useCallback, useRef } from "react";
import { isPreviewMode } from "../tauri-commands";

// Plugin imports are loaded dynamically so that this module can be evaluated
// in a plain browser (preview mode) without crashing on missing Tauri globals.

/** Returns the app version string. Falls back to a hardcoded string in preview mode. */
async function getAppVersion(): Promise<string> {
  if (isPreviewMode) return "0.4.1-preview";
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return getVersion();
  } catch {
    return "0.4.1";
  }
}

import type { AppSettings, CleanupStyle, HotkeyMode, AppProfile, ParsedLicenseStatus } from "../types";
import { getProfiles, getAdvancedSettings, saveAdvancedSettings, getVoiceCommandActive, validateApiKey, clearApiKey } from "../tauri-commands";
import type { AdvancedSettings } from "../types";
import { isMobile, isDesktop } from "../platform";

// Drill-down navigation components
import type { SettingsCategory } from "./settings/types";
import { SETTINGS_CATEGORIES } from "./settings/types";
import { SettingsHome } from "./settings/SettingsHome";
import { SettingsSubPageHeader } from "./settings/SettingsSubPageHeader";
import { RecordingAudioContent } from "./settings/RecordingAudioContent";
import { AppearanceLanguageContent } from "./settings/AppearanceLanguageContent";
import { AiProvidersContent } from "./settings/AiProvidersContent";
import { ShortcutsContent } from "./settings/ShortcutsContent";
import { LicenseSection } from "./settings/LicenseSettings";
import { DictionaryContent } from "./settings/DictionaryContent";
import { AboutContent } from "./settings/AboutContent";
import { AdvancedSettingsPanel } from "./AdvancedSettingsPanel";

// === PROPS INTERFACE =========================================================

export interface SettingsPanelProps {
  onClose: () => void;
  loadedSettings: AppSettings | null;
  language: string;
  cleanupStyle: CleanupStyle;
  hotkey: string;
  hotkeyMode: HotkeyMode;
  hotkeySlot2: string;
  hotkeyModeSlot2: HotkeyMode;
  audioDevice: string | null;
  audioDevices: string[];
  dictionary: string[];
  outputLanguage: string;
  licenseStatus: ParsedLicenseStatus;
  licenseSource: string;
  licenseLoading: boolean;
  onValidateLicense: (key: string) => Promise<string | null>;
  onRemoveLicense: () => Promise<void>;
  onDeactivateLicense: () => Promise<string | null>;
  onSave: (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    hotkey: string, hotkeyMode: HotkeyMode, audioDevice: string | null,
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string, openrouterKey: string,
    outputLanguage: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
    bubbleSize?: number | null, bubbleOpacity?: number | null,
    localWhisperModel?: string | null, localWhisperGpu?: boolean | null,
    sttProvider?: string | null, llmProvider?: string | null,
    insertAndSendSlot1?: boolean | null, autostopSilenceSecs?: number | null,
    autoModeSilenceSecs?: number | null,
    hotkeySlot2?: string | null, hotkeyModeSlot2?: HotkeyMode | null,
    insertAndSendSlot2?: boolean | null,
    bubbleTapMode?: string | null, bubbleTapAutoSend?: boolean | null,
    bubbleTapSilenceSecs?: number | null, bubbleLongPressMode?: string | null,
    bubbleLongPressAutoSend?: boolean | null, bubbleLongPressSilenceSecs?: number | null,
    livePreviewEnabled?: boolean | null, previewPauseSilenceSecs?: number | null,
    previewPanelForm?: string | null,
  ) => Promise<void>;
  onLanguageChange: (lang: string) => void;
  onStyleChange: (style: CleanupStyle) => void;
  onHotkeyChange: (h: string) => void;
  onHotkeyModeChange: (m: HotkeyMode) => void;
  onAudioDeviceChange: (d: string | null) => void;
  onAddTerm: (term: string) => Promise<void>;
  onRemoveTerm: (term: string) => Promise<void>;
  onOutputLanguageChange: (lang: string) => void;
  /** Called when user clicks "Setup-Assistent erneut starten". */
  onRestartOnboarding?: () => void;
  /** Register a back-handler: returns true if Settings handled the back (sub-page → home). */
  onRegisterBack?: (fn: (() => boolean) | null) => void;
}

// === COMPONENT ===============================================================

export function SettingsPanel({
  onClose, loadedSettings, language, cleanupStyle, hotkey, hotkeyMode,
  hotkeySlot2, hotkeyModeSlot2,
  audioDevice, audioDevices, dictionary, outputLanguage,
  licenseStatus, licenseSource, licenseLoading,
  onValidateLicense, onRemoveLicense, onDeactivateLicense,
  onSave, onLanguageChange, onStyleChange: _onStyleChange, onHotkeyChange, onHotkeyModeChange,
  onAudioDeviceChange, onAddTerm, onRemoveTerm, onOutputLanguageChange,
  onRestartOnboarding,
  onRegisterBack,
}: SettingsPanelProps) {

  // --- Drill-down navigation state ---
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>("home");

  // Register back-handler so App.tsx can ask: "should I navigate sub-page → home?"
  useEffect(() => {
    onRegisterBack?.(() => {
      if (activeCategory !== "home") {
        setActiveCategory("home");
        return true;
      }
      return false;
    });
    return () => onRegisterBack?.(null);
  }, [activeCategory, onRegisterBack]);

  // --- All useState declarations ---
  const [groqKey, setGroqKey] = useState("");
  const [deepseekKey, setDeepseekKey] = useState("");
  const [localLang, setLocalLang] = useState(language);
  const [localStyle, setLocalStyle] = useState(cleanupStyle);
  const [localHotkey, setLocalHotkey] = useState(hotkey);
  const [localHotkeyMode, setLocalHotkeyMode] = useState(hotkeyMode);
  const [localHotkeySlot2, setLocalHotkeySlot2] = useState(hotkeySlot2);
  const [localHotkeyModeSlot2, setLocalHotkeyModeSlot2] = useState(hotkeyModeSlot2);
  const [localAudioDevice, setLocalAudioDevice] = useState(audioDevice);
  const [localSttModel, setLocalSttModel] = useState(loadedSettings?.sttModel ?? "whisper-large-v3-turbo");
  const [localCustomPrompt, setLocalCustomPrompt] = useState(loadedSettings?.customPrompt ?? "");
  const [localAutostart, setLocalAutostart] = useState(loadedSettings?.autostart ?? false);
  const [localWhisperMode, setLocalWhisperMode] = useState(loadedSettings?.whisperMode ?? false);
  const [openaiKey, setOpenaiKey] = useState("");
  const [anthropicKey, setAnthropicKey] = useState("");
  const [openrouterKey, setOpenrouterKey] = useState("");
  // API key validation errors: null = no error, string = error message.
  const [apiKeyErrors, setApiKeyErrors] = useState<Record<string, string | null>>({});
  // Which keys are currently being validated (spinner state).
  const [apiKeyValidating, setApiKeyValidating] = useState<Record<string, boolean>>({});
  // Confirm-remove state per provider. When truthy, shows "Remove?" confirmation.
  const [apiKeyConfirmRemove, setApiKeyConfirmRemove] = useState<Record<string, boolean>>({});
  const apiKeyConfirmTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const [localSttProvider, setLocalSttProvider] = useState<string>(loadedSettings?.sttProvider ?? "groq");
  const [localLlmProvider, setLocalLlmProvider] = useState<string>(loadedSettings?.llmProvider ?? "deepseek");
  const [localOutputLanguage, setLocalOutputLanguage] = useState(outputLanguage);
  useEffect(() => { setLocalOutputLanguage(outputLanguage); }, [outputLanguage]);
  const [localWebhookUrl, setLocalWebhookUrl] = useState(loadedSettings?.webhookUrl ?? "");
  const [localTursoUrl, setLocalTursoUrl] = useState(loadedSettings?.tursoUrl ?? "");
  const [tursoToken, setTursoToken] = useState("");
  const [localBubbleSize, setLocalBubbleSize] = useState(loadedSettings?.bubbleSize ?? 1.0);
  const [localBubbleOpacity, setLocalBubbleOpacity] = useState(loadedSettings?.bubbleOpacity ?? 0.85);
  const [localWhisperModel, setLocalWhisperModel] = useState(loadedSettings?.localWhisperModel ?? "small");
  const [localWhisperGpu, setLocalWhisperGpu] = useState(loadedSettings?.localWhisperGpu ?? true);
  const [localInsertAndSendSlot1, setLocalInsertAndSendSlot1] = useState(loadedSettings?.insertAndSendSlot1 ?? false);
  const [localInsertAndSendSlot2, setLocalInsertAndSendSlot2] = useState(loadedSettings?.insertAndSendSlot2 ?? false);
  const [localSendStopPauseSecs, setLocalSendStopPauseSecs] = useState(() => {
    const autostop = loadedSettings?.autostopSilenceSecs ?? 2.0;
    const auto = loadedSettings?.autoModeSilenceSecs ?? 2.0;
    return Math.max(autostop, auto);  // AC-4: display the larger of two diverged values
  });
  const [bubbleTab, setBubbleTab] = useState<0 | 1>(0);
  const [localBubbleTapMode, setLocalBubbleTapMode] = useState<HotkeyMode>((loadedSettings?.bubbleTapMode ?? "toggle") as HotkeyMode);
  const [localBubbleTapAutoSend, setLocalBubbleTapAutoSend] = useState(loadedSettings?.bubbleTapAutoSend ?? false);
  const [localBubbleTapSilenceSecs, setLocalBubbleTapSilenceSecs] = useState(loadedSettings?.bubbleTapSilenceSecs ?? 2.0);
  const [localBubbleLongPressMode, setLocalBubbleLongPressMode] = useState<HotkeyMode>((loadedSettings?.bubbleLongPressMode ?? "hold") as HotkeyMode);
  const [localBubbleLongPressAutoSend, setLocalBubbleLongPressAutoSend] = useState(loadedSettings?.bubbleLongPressAutoSend ?? false);
  const [localBubbleLongPressSilenceSecs, setLocalBubbleLongPressSilenceSecs] = useState(loadedSettings?.bubbleLongPressSilenceSecs ?? 2.0);
  // Voice Command Mode: reflects what the backend monitor is currently doing.
  // Toggling this calls toggle_voice_command_mode and syncs the backend directly.
  const [localVoiceCommandEnabled, setLocalVoiceCommandEnabled] = useState(loadedSettings?.voiceCommandEnabled ?? false);
  // Live Preview toggle (desktop only — opt-in, default false)
  const [localLivePreviewEnabled, setLocalLivePreviewEnabled] = useState(
    loadedSettings?.livePreviewEnabled ?? false
  );
  const [localPreviewPauseSilenceSecs, setLocalPreviewPauseSilenceSecs] = useState(
    loadedSettings?.previewPauseSilenceSecs ?? 2.0
  );
  const [localPreviewPanelForm, setLocalPreviewPanelForm] = useState(
    loadedSettings?.previewPanelForm ?? "comfortable"
  );
  // Silence threshold: lives in AdvancedSettings, loaded separately on mount.
  const [localSilenceThreshold, setLocalSilenceThreshold] = useState(0.005);
  const [localAutoPaste, setLocalAutoPaste] = useState(true);
  const [localPasteDelayMs, setLocalPasteDelayMs] = useState(80);
  const [localAutoCapitalize, setLocalAutoCapitalize] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [newTerm, setNewTerm] = useState("");
  const [appVersion, setAppVersion] = useState<string>("");
  // isDirty: true when any local state differs from the persisted loadedSettings.
  // License activation must NOT set this flag (it auto-saves immediately).
  const [isDirty, setIsDirty] = useState(false);
  // Active tab inside the combined Hotkey section: 0 = Hotkey 1, 1 = Hotkey 2
  const [hotkeyTab, setHotkeyTab] = useState<0 | 1>(0);

  // Load profiles on mount.
  useEffect(() => { getProfiles().then(setProfiles).catch(console.error); }, []);

  // Load advanced settings on mount to initialise localSilenceThreshold.
  const [advancedSettings, setAdvancedSettings] = useState<AdvancedSettings | null>(null);
  useEffect(() => {
    getAdvancedSettings()
      .then((adv) => {
        setAdvancedSettings(adv);
        setLocalSilenceThreshold(adv.silenceThreshold);
        setLocalAutoPaste(adv.autoPaste);
        setLocalPasteDelayMs(adv.pasteDelayMs);
        setLocalAutoCapitalize(adv.autoCapitalize);
      })
      .catch(console.error);
  }, []);

  // Sync Voice Command toggle with the actual backend runtime state on mount,
  // and keep it in sync via events (e.g. when monitor stops itself via "Klarvo off").
  useEffect(() => {
    if (!isDesktop) return;

    getVoiceCommandActive()
      .then(setLocalVoiceCommandEnabled)
      .catch(console.error);

    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<{ active: boolean }>("klarvo://voice-command-state-changed", (event) => {
        setLocalVoiceCommandEnabled(event.payload.active);
      }).then((fn) => { unlisten = fn; }).catch(console.error);
    });

    return () => { unlisten?.(); };
  }, []);

  // Load app version on mount.
  useEffect(() => { getAppVersion().then(setAppVersion).catch(() => setAppVersion("0.4.1")); }, []);

  useEffect(() => { setLocalLang(language); }, [language]);
  useEffect(() => { setLocalStyle(cleanupStyle); }, [cleanupStyle]);
  useEffect(() => { setLocalHotkey(hotkey); }, [hotkey]);
  useEffect(() => { setLocalHotkeyMode(hotkeyMode); }, [hotkeyMode]);
  useEffect(() => { setLocalHotkeySlot2(hotkeySlot2); }, [hotkeySlot2]);
  useEffect(() => { setLocalHotkeyModeSlot2(hotkeyModeSlot2); }, [hotkeyModeSlot2]);
  useEffect(() => { setLocalAudioDevice(audioDevice); }, [audioDevice]);
  useEffect(() => {
    if (loadedSettings) {
      setLocalSttModel(loadedSettings.sttModel);
      setLocalCustomPrompt(loadedSettings.customPrompt);
      setLocalAutostart(loadedSettings.autostart);
      setLocalWhisperMode(loadedSettings.whisperMode);
      setLocalSttProvider(loadedSettings.sttProvider ?? "groq");
      const llmProv = loadedSettings.llmProvider ?? "deepseek";
      const llmKeyMap: Record<string, string | undefined> = {
        deepseek: loadedSettings.deepseekApiKeyMasked,
        openai: loadedSettings.openaiApiKeyMasked,
        anthropic: loadedSettings.anthropicApiKeyMasked,
        groq: loadedSettings.groqApiKeyMasked,
        openrouter: loadedSettings.openrouterApiKeyMasked,
      };
      if (!llmKeyMap[llmProv]) {
        const fallback = ["deepseek", "openai", "groq", "anthropic", "openrouter"].find(p => llmKeyMap[p]);
        setLocalLlmProvider(fallback ?? llmProv);
      } else {
        setLocalLlmProvider(llmProv);
      }
      setLocalOutputLanguage(loadedSettings.outputLanguage ?? "");
      setLocalWebhookUrl(loadedSettings.webhookUrl ?? "");
      setLocalTursoUrl(loadedSettings.tursoUrl ?? "");
      setLocalBubbleSize(loadedSettings.bubbleSize ?? 1.0);
      setLocalBubbleOpacity(loadedSettings.bubbleOpacity ?? 0.85);
      setLocalWhisperModel(loadedSettings.localWhisperModel ?? "small");
      setLocalWhisperGpu(loadedSettings.localWhisperGpu ?? true);
      setLocalInsertAndSendSlot1(loadedSettings.insertAndSendSlot1 ?? false);
      setLocalInsertAndSendSlot2(loadedSettings.insertAndSendSlot2 ?? false);
      const autostop = loadedSettings.autostopSilenceSecs ?? 2.0;
      const auto = loadedSettings.autoModeSilenceSecs ?? 2.0;
      setLocalSendStopPauseSecs(Math.max(autostop, auto));
      setLocalHotkeySlot2(loadedSettings.hotkeySlot2 ?? "");
      setLocalHotkeyModeSlot2(loadedSettings.hotkeyModeSlot2 ?? "hold");
      setLocalBubbleTapMode((loadedSettings.bubbleTapMode ?? "toggle") as HotkeyMode);
      setLocalBubbleTapAutoSend(loadedSettings.bubbleTapAutoSend ?? false);
      setLocalBubbleTapSilenceSecs(loadedSettings.bubbleTapSilenceSecs ?? 2.0);
      setLocalBubbleLongPressMode((loadedSettings.bubbleLongPressMode ?? "hold") as HotkeyMode);
      setLocalBubbleLongPressAutoSend(loadedSettings.bubbleLongPressAutoSend ?? false);
      setLocalBubbleLongPressSilenceSecs(loadedSettings.bubbleLongPressSilenceSecs ?? 2.0);
      setLocalVoiceCommandEnabled(loadedSettings.voiceCommandEnabled ?? false);
      setLocalLivePreviewEnabled(loadedSettings.livePreviewEnabled ?? false);
      setLocalPreviewPauseSilenceSecs(loadedSettings.previewPauseSilenceSecs ?? 2.0);
      setLocalPreviewPanelForm(loadedSettings.previewPanelForm ?? "comfortable");
    }
  }, [loadedSettings]);

  // Auto-switch STT provider when the currently selected provider has no API key
  // but another cloud provider does. Prevents the user from accidentally saving
  // a provider config that will produce a 401 on first use.
  // Does NOT touch "local" provider -- that needs no cloud key.
  useEffect(() => {
    if (!loadedSettings) return;
    if (localSttProvider === "local") return;

    const groqAvailable = !!loadedSettings.groqApiKeyMasked;
    const openaiAvailable = !!loadedSettings.openaiApiKeyMasked;

    if (localSttProvider === "groq" && !groqAvailable && openaiAvailable) {
      setLocalSttProvider("openai");
      setLocalSttModel("whisper-1");
    } else if (localSttProvider === "openai" && !openaiAvailable && groqAvailable) {
      setLocalSttProvider("groq");
      setLocalSttModel("whisper-large-v3-turbo");
    }
    // If neither provider has a key: leave state as-is -- both are equally broken.
  }, [loadedSettings, localSttProvider, localSttModel]);

  // Track dirty state: compare local values against the last saved settings.
  // API key fields: any non-empty input counts as dirty (new key to save).
  // License activation is excluded -- it triggers auto-save and must not set dirty.
  useEffect(() => {
    if (!loadedSettings) return;
    const dirty =
      localLang !== (loadedSettings.language ?? "") ||
      localStyle !== (loadedSettings.cleanupStyle ?? "polished") ||
      localHotkey !== (loadedSettings.hotkey ?? "") ||
      localHotkeyMode !== (loadedSettings.hotkeyMode ?? "hold") ||
      localAudioDevice !== (loadedSettings.audioDevice ?? null) ||
      localSttModel !== (loadedSettings.sttModel ?? "whisper-large-v3-turbo") ||
      localCustomPrompt !== (loadedSettings.customPrompt ?? "") ||
      localAutostart !== (loadedSettings.autostart ?? false) ||
      localWhisperMode !== (loadedSettings.whisperMode ?? false) ||
      localSttProvider !== (loadedSettings.sttProvider ?? "groq") ||
      localLlmProvider !== (loadedSettings.llmProvider ?? "deepseek") ||
      localOutputLanguage !== (loadedSettings.outputLanguage ?? "") ||
      localWebhookUrl !== (loadedSettings.webhookUrl ?? "") ||
      localTursoUrl !== (loadedSettings.tursoUrl ?? "") ||
      localBubbleSize !== (loadedSettings.bubbleSize ?? 1.0) ||
      localBubbleOpacity !== (loadedSettings.bubbleOpacity ?? 0.85) ||
      localWhisperModel !== (loadedSettings.localWhisperModel ?? "small") ||
      localWhisperGpu !== (loadedSettings.localWhisperGpu ?? true) ||
      localInsertAndSendSlot1 !== (loadedSettings.insertAndSendSlot1 ?? false) ||
      localInsertAndSendSlot2 !== (loadedSettings.insertAndSendSlot2 ?? false) ||
      localSendStopPauseSecs !== Math.max(loadedSettings.autostopSilenceSecs ?? 2.0, loadedSettings.autoModeSilenceSecs ?? 2.0) ||
      localHotkeySlot2 !== (loadedSettings.hotkeySlot2 ?? "") ||
      localHotkeyModeSlot2 !== (loadedSettings.hotkeyModeSlot2 ?? "hold") ||
      groqKey.trim() !== "" ||
      deepseekKey.trim() !== "" ||
      openaiKey.trim() !== "" ||
      anthropicKey.trim() !== "" ||
      tursoToken.trim() !== "" ||
      (!isDesktop && (
        localBubbleTapMode !== (loadedSettings.bubbleTapMode ?? "toggle") ||
        localBubbleTapAutoSend !== (loadedSettings.bubbleTapAutoSend ?? false) ||
        localBubbleTapSilenceSecs !== (loadedSettings.bubbleTapSilenceSecs ?? 2.0) ||
        localBubbleLongPressMode !== (loadedSettings.bubbleLongPressMode ?? "hold") ||
        localBubbleLongPressAutoSend !== (loadedSettings.bubbleLongPressAutoSend ?? false) ||
        localBubbleLongPressSilenceSecs !== (loadedSettings.bubbleLongPressSilenceSecs ?? 2.0)
      ))
    || (advancedSettings !== null && advancedSettings.silenceThreshold !== localSilenceThreshold)
    || (advancedSettings !== null && advancedSettings.autoPaste !== localAutoPaste)
    || (advancedSettings !== null && advancedSettings.pasteDelayMs !== localPasteDelayMs)
    || (advancedSettings !== null && advancedSettings.autoCapitalize !== localAutoCapitalize)
    || (loadedSettings?.livePreviewEnabled ?? false) !== localLivePreviewEnabled
    || (loadedSettings?.previewPauseSilenceSecs ?? 2.0) !== localPreviewPauseSilenceSecs
    || (loadedSettings?.previewPanelForm ?? "comfortable") !== localPreviewPanelForm;
    setIsDirty(dirty);
  }, [
    loadedSettings, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, localSttProvider,
    localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl, localBubbleSize,
    localBubbleOpacity, localWhisperModel, localWhisperGpu,
    localInsertAndSendSlot1, localInsertAndSendSlot2, localSendStopPauseSecs, localHotkeySlot2, localHotkeyModeSlot2,
    localBubbleTapMode, localBubbleTapAutoSend, localBubbleTapSilenceSecs,
    localBubbleLongPressMode, localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
    groqKey, deepseekKey, openaiKey, anthropicKey, tursoToken,
    advancedSettings, localSilenceThreshold, localAutoPaste, localPasteDelayMs, localAutoCapitalize,
    localLivePreviewEnabled, localPreviewPauseSilenceSecs, localPreviewPanelForm,
  ]);

  // --- useCallback handlers ---

  const handleLangChange = useCallback((lang: string) => {
    setLocalLang(lang);
    onLanguageChange(lang);
  }, [onLanguageChange]);

  const handleOutputLanguageChange = useCallback((lang: string) => {
    setLocalOutputLanguage(lang);
    onOutputLanguageChange(lang);
  }, [onOutputLanguageChange]);

  // handleStyleChange removed — style picker moved to main screen only.
  // localStyle + onStyleChange are still used in save/dirty logic.

  const handleHotkeyChange = useCallback((h: string) => {
    setLocalHotkey(h);
    onHotkeyChange(h);
  }, [onHotkeyChange]);

  const handleHotkeyModeChange = useCallback((m: HotkeyMode) => {
    setLocalHotkeyMode(m);
    onHotkeyModeChange(m);
  }, [onHotkeyModeChange]);

  const handleAudioDeviceChange = useCallback((d: string | null) => {
    setLocalAudioDevice(d);
    onAudioDeviceChange(d);
  }, [onAudioDeviceChange]);

  // When switching to offline STT, auto-select local LLM cleanup.
  // When switching back to cloud STT, restore the first available cloud provider.
  const handleSttProviderChange = useCallback((provider: string) => {
    setLocalSttProvider(provider);
    if (provider === "local") {
      setLocalLlmProvider("local");
    } else {
      // Restore to first available cloud provider, or deepseek as default.
      const hasDeepseek = !!loadedSettings?.deepseekApiKeyMasked;
      const hasOpenai = !!loadedSettings?.openaiApiKeyMasked;
      const hasGroq = !!loadedSettings?.groqApiKeyMasked;
      const hasOpenrouter = !!loadedSettings?.openrouterApiKeyMasked;
      const fallback =
        (hasDeepseek && "deepseek") ||
        (hasOpenai && "openai") ||
        (hasGroq && "groq") ||
        (hasOpenrouter && "openrouter") ||
        "deepseek";
      setLocalLlmProvider(fallback);
    }
  }, [loadedSettings]);

  // Internal helper: calls onSave with all current values. Used by both the
  // explicit Save button and the auto-save after license activation.
  const saveCurrentSettings = useCallback(async (opts?: { silent?: boolean }) => {
    // Validate any newly entered API keys before persisting.
    // Keys that are empty (no change) are skipped.
    const keysToValidate: Array<{ provider: string; key: string }> = [
      { provider: "groq", key: groqKey.trim() },
      { provider: "deepseek", key: deepseekKey.trim() },
      { provider: "openai", key: openaiKey.trim() },
      { provider: "anthropic", key: anthropicKey.trim() },
      { provider: "openrouter", key: openrouterKey.trim() },
    ].filter((e) => e.key !== "");

    if (keysToValidate.length > 0) {
      // Mark all pending keys as validating.
      const validatingState: Record<string, boolean> = {};
      keysToValidate.forEach(({ provider }) => { validatingState[provider] = true; });
      setApiKeyValidating(validatingState);

      const errors: Record<string, string | null> = {};
      await Promise.all(
        keysToValidate.map(async ({ provider, key }) => {
          try {
            const valid = await validateApiKey(provider, key);
            errors[provider] = valid ? null : "Invalid API key";
          } catch {
            // Network error: treat as invalid so the user can retry.
            errors[provider] = "Validation failed — check your network";
          }
        }),
      );

      setApiKeyValidating({});
      setApiKeyErrors((prev) => ({ ...prev, ...errors }));

      const hasErrors = Object.values(errors).some((e) => e !== null);
      if (hasErrors) return; // Abort save — errors shown inline.
    }

    setSaving(true);
    if (!opts?.silent) setSaveMsg(null);
    try {
      // AC-1: Regler B writes BOTH keys unconditionally to the same value
      const autostopSecs = localSendStopPauseSecs;
      const autoModeSecs = localSendStopPauseSecs;
      await onSave(
        groqKey.trim(), deepseekKey.trim(), localLang, localStyle, localHotkey, localHotkeyMode,
        localAudioDevice, localSttModel, localCustomPrompt, localAutostart, localWhisperMode,
        openaiKey.trim(), anthropicKey.trim(), openrouterKey.trim(),
        localOutputLanguage, localWebhookUrl.trim(), localTursoUrl.trim(), tursoToken.trim(),
        localBubbleSize, localBubbleOpacity,
        localWhisperModel, localWhisperGpu,
        localSttProvider, localLlmProvider,
        localInsertAndSendSlot1, autostopSecs, autoModeSecs,
        localHotkeySlot2, localHotkeyModeSlot2,
        localInsertAndSendSlot2,
        localBubbleTapMode, localBubbleTapAutoSend,
        localBubbleTapSilenceSecs, localBubbleLongPressMode,
        localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
        localLivePreviewEnabled, localPreviewPauseSilenceSecs,
        localPreviewPanelForm,
      );
      // Save AdvancedSettings fields when any have changed.
      if (advancedSettings !== null && (
        advancedSettings.silenceThreshold !== localSilenceThreshold ||
        advancedSettings.autoPaste !== localAutoPaste ||
        advancedSettings.pasteDelayMs !== localPasteDelayMs ||
        advancedSettings.autoCapitalize !== localAutoCapitalize
      )) {
        const updatedAdv: AdvancedSettings = {
          ...advancedSettings,
          silenceThreshold: localSilenceThreshold,
          autoPaste: localAutoPaste,
          pasteDelayMs: localPasteDelayMs,
          autoCapitalize: localAutoCapitalize,
        };
        await saveAdvancedSettings(updatedAdv);
        setAdvancedSettings(updatedAdv);
      }
      setGroqKey("");
      setDeepseekKey("");
      setOpenaiKey("");
      setAnthropicKey("");
      setTursoToken("");
      // Clear validation errors after a successful save.
      setApiKeyErrors({});
      if (!opts?.silent) {
        setSaveMsg("Saved");
        setTimeout(() => setSaveMsg(null), 2000);
      }
    } catch (err) {
      if (!opts?.silent) setSaveMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [
    groqKey, deepseekKey, localLang, localStyle, localHotkey, localHotkeyMode, localAudioDevice,
    localSttModel, localCustomPrompt, localAutostart, localWhisperMode, openaiKey, anthropicKey,
    localSttProvider, localLlmProvider, localOutputLanguage, localWebhookUrl, localTursoUrl, tursoToken,
    localBubbleSize, localBubbleOpacity, localWhisperModel, localWhisperGpu,
    localInsertAndSendSlot1, localInsertAndSendSlot2, localSendStopPauseSecs, localHotkeySlot2, localHotkeyModeSlot2,
    localBubbleTapMode, localBubbleTapAutoSend, localBubbleTapSilenceSecs,
    localBubbleLongPressMode, localBubbleLongPressAutoSend, localBubbleLongPressSilenceSecs,
    advancedSettings, localSilenceThreshold,
    openrouterKey,
    localLivePreviewEnabled, localPreviewPauseSilenceSecs, localPreviewPanelForm,
    onSave,
  ]);

  const handleSave = useCallback(async () => {
    await saveCurrentSettings();
  }, [saveCurrentSettings]);

  // Handles the two-step confirm-then-remove flow for API keys.
  // First click: shows "Remove?" confirmation for 4 s.
  // Second click within 4 s: calls clearApiKey and reloads settings.
  const handleApiKeyRemoveClick = useCallback((provider: string) => {
    if (!apiKeyConfirmRemove[provider]) {
      setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: true }));
      apiKeyConfirmTimers.current[provider] = setTimeout(() => {
        setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: false }));
      }, 4000);
      return;
    }
    // Second click: execute removal.
    if (apiKeyConfirmTimers.current[provider]) clearTimeout(apiKeyConfirmTimers.current[provider]);
    setApiKeyConfirmRemove((prev) => ({ ...prev, [provider]: false }));
    clearApiKey(provider)
      .then(() => saveCurrentSettings({ silent: true }))
      .catch((err) => console.error("clearApiKey failed:", err));
  }, [apiKeyConfirmRemove, saveCurrentSettings]);

  // Clear confirm timers on unmount.
  useEffect(() => {
    const timers = apiKeyConfirmTimers.current;
    return () => { Object.values(timers).forEach(clearTimeout); };
  }, []);

  const handleAddTerm = useCallback(async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    try {
      await onAddTerm(trimmed);
      setNewTerm("");
    } catch (err) {
      console.error(err);
    }
  }, [newTerm, onAddTerm]);

  // --- Derived values ---

  const groqOk = !!loadedSettings?.groqApiKeyMasked;
  const deepseekOk = !!loadedSettings?.deepseekApiKeyMasked;
  const openaiOk = !!loadedSettings?.openaiApiKeyMasked;
  const anthropicOk = !!loadedSettings?.anthropicApiKeyMasked;
  const openrouterOk = !!loadedSettings?.openrouterApiKeyMasked;

  // Feature gate: user has an active paid license (licensed, active trial, or valid grace period).
  const isPaid =
    licenseStatus.type === "licensed" ||
    (licenseStatus.type === "trial" &&
      licenseStatus.trialUntil !== undefined &&
      licenseStatus.trialUntil > Date.now() / 1000) ||
    (licenseStatus.type === "grace_period" &&
      licenseStatus.graceUntil !== undefined &&
      licenseStatus.graceUntil > Date.now() / 1000);

  const isTrial = licenseStatus.type === "trial";

  // On Android the system nav bar (~48 px) overlaps the WebView bottom edge.
  // env(safe-area-inset-bottom) is unreliable in Android WebView so we use a
  // fixed 48 px deduction on mobile to keep the sticky Save footer visible.
  const panelMaxH = isMobile ? "max-h-[calc(100vh-168px)]" : "max-h-[calc(100vh-120px)]";

  // Suppress unused variable warnings for state that exists for dirty-tracking
  // or future use but is not yet wired into the new drill-down render.
  void localAutostart;
  void localWhisperMode;
  void localBubbleSize;
  void localBubbleOpacity;
  void localWebhookUrl;
  void localTursoUrl;
  void localVoiceCommandEnabled;
  void syncing;
  void syncMsg;
  void setSyncing;
  void setSyncMsg;

  // === NEW RENDER ==============================================================
  return (
    <div className={`w-full bg-klarvo-surface border border-klarvo-border/60 rounded-2xl overflow-hidden shadow-xl shadow-black/30 flex flex-col ${panelMaxH}`}>
      {activeCategory === "home" ? (
        <SettingsHome
          onSelectCategory={setActiveCategory}
          onClose={onClose}
          isTrial={isTrial}
        />
      ) : (
        <>
          <SettingsSubPageHeader
            title={SETTINGS_CATEGORIES.find(c => c.id === activeCategory)?.label ?? "Settings"}
            onBack={() => setActiveCategory("home")}
            onClose={onClose}
          />
          <div className="overflow-y-auto flex-1 min-h-0 p-4 flex flex-col gap-5">
            {activeCategory === "recording-audio" && (
              <RecordingAudioContent
                localSttProvider={localSttProvider} setLocalSttProvider={handleSttProviderChange}
                localSttModel={localSttModel} setLocalSttModel={setLocalSttModel}
                localLlmProvider={localLlmProvider} setLocalLlmProvider={setLocalLlmProvider}
                localAudioDevice={localAudioDevice} handleAudioDeviceChange={handleAudioDeviceChange}
                audioDevices={audioDevices}
                localWhisperModel={localWhisperModel} setLocalWhisperModel={setLocalWhisperModel}
                localWhisperGpu={localWhisperGpu} setLocalWhisperGpu={setLocalWhisperGpu}
                isPaid={isPaid} groqOk={groqOk} deepseekOk={deepseekOk} openaiOk={openaiOk} openrouterOk={openrouterOk}
                loadedSettings={loadedSettings}
              />
            )}
            {activeCategory === "ai-providers" && (
              <AiProvidersContent
                groqKey={groqKey} setGroqKey={setGroqKey}
                deepseekKey={deepseekKey} setDeepseekKey={setDeepseekKey}
                openaiKey={openaiKey} setOpenaiKey={setOpenaiKey}
                anthropicKey={anthropicKey} setAnthropicKey={setAnthropicKey}
                openrouterKey={openrouterKey} setOpenrouterKey={setOpenrouterKey}
                apiKeyErrors={apiKeyErrors} setApiKeyErrors={setApiKeyErrors}
                apiKeyValidating={apiKeyValidating}
                apiKeyConfirmRemove={apiKeyConfirmRemove}
                onApiKeyRemoveClick={handleApiKeyRemoveClick}
                localCustomPrompt={localCustomPrompt} setLocalCustomPrompt={setLocalCustomPrompt}
                profiles={profiles} setProfiles={setProfiles}
                loadedSettings={loadedSettings}
                isPaid={isPaid}
                isTrial={isTrial}
                groqOk={groqOk} deepseekOk={deepseekOk} openaiOk={openaiOk}
                anthropicOk={anthropicOk} openrouterOk={openrouterOk}
                setSaveMsg={setSaveMsg}
              />
            )}
            {activeCategory === "shortcuts" && (
              <ShortcutsContent
                localHotkey={localHotkey} setLocalHotkey={handleHotkeyChange}
                localHotkeyMode={localHotkeyMode} setLocalHotkeyMode={handleHotkeyModeChange}
                localHotkeySlot2={localHotkeySlot2} setLocalHotkeySlot2={setLocalHotkeySlot2}
                localHotkeyModeSlot2={localHotkeyModeSlot2} setLocalHotkeyModeSlot2={setLocalHotkeyModeSlot2}
                localSendStopPauseSecs={localSendStopPauseSecs} setLocalSendStopPauseSecs={setLocalSendStopPauseSecs}
                localInsertAndSendSlot1={localInsertAndSendSlot1} setLocalInsertAndSendSlot1={setLocalInsertAndSendSlot1}
                localInsertAndSendSlot2={localInsertAndSendSlot2} setLocalInsertAndSendSlot2={setLocalInsertAndSendSlot2}
                hotkeyTab={hotkeyTab} setHotkeyTab={setHotkeyTab}
                bubbleTab={bubbleTab} setBubbleTab={setBubbleTab}
                localBubbleTapMode={localBubbleTapMode} setLocalBubbleTapMode={setLocalBubbleTapMode}
                localBubbleTapAutoSend={localBubbleTapAutoSend} setLocalBubbleTapAutoSend={setLocalBubbleTapAutoSend}
                localBubbleTapSilenceSecs={localBubbleTapSilenceSecs} setLocalBubbleTapSilenceSecs={setLocalBubbleTapSilenceSecs}
                localBubbleLongPressMode={localBubbleLongPressMode} setLocalBubbleLongPressMode={setLocalBubbleLongPressMode}
                localBubbleLongPressAutoSend={localBubbleLongPressAutoSend} setLocalBubbleLongPressAutoSend={setLocalBubbleLongPressAutoSend}
                localBubbleLongPressSilenceSecs={localBubbleLongPressSilenceSecs} setLocalBubbleLongPressSilenceSecs={setLocalBubbleLongPressSilenceSecs}
                localSilenceThreshold={localSilenceThreshold}
                loadedSettings={loadedSettings}
                onHotkeyChange={onHotkeyChange}
                onHotkeyModeChange={onHotkeyModeChange}
                isPaid={isPaid}
                localAutoPaste={localAutoPaste}
                setLocalAutoPaste={setLocalAutoPaste}
                localPasteDelayMs={localPasteDelayMs}
                setLocalPasteDelayMs={setLocalPasteDelayMs}
                localAutoCapitalize={localAutoCapitalize}
                setLocalAutoCapitalize={setLocalAutoCapitalize}
                localLivePreviewEnabled={localLivePreviewEnabled}
                setLocalLivePreviewEnabled={setLocalLivePreviewEnabled}
                localPreviewPauseSilenceSecs={localPreviewPauseSilenceSecs}
                setLocalPreviewPauseSilenceSecs={setLocalPreviewPauseSilenceSecs}
                localPreviewPanelForm={localPreviewPanelForm}
                setLocalPreviewPanelForm={setLocalPreviewPanelForm}
              />
            )}
            {activeCategory === "license" && (
              <LicenseSection
                licenseStatus={licenseStatus}
                licenseSource={licenseSource}
                onValidate={onValidateLicense}
                onRemove={onRemoveLicense}
                onDeactivate={onDeactivateLicense}
                licenseLoading={licenseLoading}
              />
            )}
            {activeCategory === "dictionary" && (
              <DictionaryContent
                dictionary={dictionary}
                newTerm={newTerm} setNewTerm={setNewTerm}
                onAddTerm={handleAddTerm}
                onRemoveTerm={onRemoveTerm}
                isPaid={isPaid}
                isTrial={isTrial}
              />
            )}
            {activeCategory === "about" && (
              <AboutContent
                appVersion={appVersion}
                onRestartOnboarding={onRestartOnboarding}
              />
            )}
            {activeCategory === "appearance-language" && (
              <AppearanceLanguageContent
                localLang={localLang}
                handleLangChange={handleLangChange}
                localOutputLanguage={localOutputLanguage}
                handleOutputLanguageChange={handleOutputLanguageChange}
              />
            )}
            {activeCategory === "advanced" && (
              <AdvancedSettingsPanel isPaid={isPaid} isTrial={isTrial} embedded />
            )}
          </div>
        </>
      )}

      {/* Save button -- sticky footer */}
      {(isDirty || saveMsg) && (
        <div className={`px-4 py-3 border-t border-klarvo-border/40 ${isMobile ? "mobile-safe-bottom" : ""}`}>
          <button onClick={handleSave} disabled={saving}
            className={[
              "w-full py-2.5 rounded-xl text-sm font-medium transition-all duration-150 border",
              saveMsg === "Saved" ? "bg-klarvo-primary/15 border-klarvo-primary/30 text-klarvo-primary"
                : saveMsg && saveMsg !== "Saved" ? "bg-klarvo-danger/10 border-klarvo-danger/20 text-klarvo-danger"
                : "bg-klarvo-primary/10 border-klarvo-primary/30 text-klarvo-primary hover:bg-klarvo-primary/15 hover:border-klarvo-primary/40 animate-pulse",
              "disabled:opacity-50 disabled:cursor-not-allowed",
            ].join(" ")}
          >
            {saving ? "Saving..." : saveMsg ?? "Save Settings"}
          </button>
        </div>
      )}
    </div>
  );
}
