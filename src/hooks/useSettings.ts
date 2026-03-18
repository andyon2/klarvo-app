import { useState, useCallback, useEffect } from "react";
import type { AppSettings, CleanupStyle, HotkeyMode } from "../types";
import {
  getSettings,
  saveSettings,
  getDictionaryTerms,
  addDictionaryTerm,
  removeDictionaryTerm,
  listAudioDevices,
  setLanguage as syncLanguage,
  setCleanupStyle as syncCleanupStyle,
  setOutputLanguage as syncOutputLanguage,
} from "../tauri-commands";

export function useSettings() {
  const [loadedSettings, setLoadedSettings] = useState<AppSettings | null>(null);
  const [language, setLanguage] = useState("");
  const [cleanupStyle, setCleanupStyle] = useState<CleanupStyle>("polished");
  const [hotkey, setHotkey] = useState("ctrl+shift+d");
  const [hotkeyMode, setHotkeyMode] = useState<HotkeyMode>("hold");
  const [hotkeySlot2, setHotkeySlot2] = useState("");
  const [hotkeyModeSlot2, setHotkeyModeSlot2] = useState<HotkeyMode>("hold");
  const [audioDevice, setAudioDevice] = useState<string | null>(null);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [outputLanguage, setOutputLanguage] = useState("");
  const [dictionary, setDictionary] = useState<string[]>([]);
  const [insertAndSendSlot1, setInsertAndSendSlot1] = useState(false);
  const [insertAndSendSlot2, setInsertAndSendSlot2] = useState(false);
  const [autostopSilenceSecs, setAutostopSilenceSecs] = useState(2.0);
  const [autoModeSilenceSecs, setAutoModeSilenceSecs] = useState(2.0);

  // Load settings + dictionary + devices on mount.
  useEffect(() => {
    getSettings().then((s) => {
      setLoadedSettings(s);
      setLanguage(s.language);
      setCleanupStyle(s.cleanupStyle);
      setHotkey(s.hotkey);
      setHotkeyMode(s.hotkeyMode);
      setAudioDevice(s.audioDevice);
      setOutputLanguage(s.outputLanguage || "");
      setInsertAndSendSlot1(s.insertAndSendSlot1 ?? false);
      setInsertAndSendSlot2(s.insertAndSendSlot2 ?? false);
      setAutostopSilenceSecs(s.autostopSilenceSecs ?? 2.0);
      setAutoModeSilenceSecs(s.autoModeSilenceSecs ?? 2.0);
      setHotkeySlot2(s.hotkeySlot2 ?? "");
      setHotkeyModeSlot2(s.hotkeyModeSlot2 ?? "hold");
      syncLanguage(s.language).catch(console.error);
      syncCleanupStyle(s.cleanupStyle).catch(console.error);
    }).catch(console.error);

    getDictionaryTerms().then(setDictionary).catch(console.error);
    listAudioDevices().then(setAudioDevices).catch(console.error);
  }, []);

  const handleLanguageChange = useCallback((lang: string) => {
    setLanguage(lang);
    syncLanguage(lang).catch(console.error);
  }, []);

  const handleStyleChange = useCallback((style: CleanupStyle) => {
    setCleanupStyle(style);
    syncCleanupStyle(style).catch(console.error);
  }, []);

  const handleOutputLanguageChange = useCallback((lang: string) => {
    setOutputLanguage(lang);
    syncOutputLanguage(lang).catch(console.error);
  }, []);

  const handleSaveSettings = useCallback(async (
    groqKey: string, deepseekKey: string, lang: string, style: CleanupStyle,
    newHotkey: string, newHotkeyMode: HotkeyMode, newAudioDevice: string | null,
    sttModel: string, customPrompt: string, autostart: boolean, whisperMode: boolean,
    openaiKey: string, anthropicKey: string,
    outputLang: string, webhookUrl: string, tursoUrl: string, tursoToken: string,
    bubbleSize?: number | null, bubbleOpacity?: number | null,
    localWhisperModel?: string | null, localWhisperGpu?: boolean | null,
    sttProvider?: string | null, llmProvider?: string | null,
    newInsertAndSendSlot1?: boolean | null, newAutostopSilenceSecs?: number | null,
    newAutoModeSilenceSecs?: number | null,
    newHotkeySlot2?: string | null, newHotkeyModeSlot2?: HotkeyMode | null,
    newInsertAndSendSlot2?: boolean | null,
    newBubbleTapMode?: string | null, newBubbleTapAutoSend?: boolean | null,
    newBubbleTapSilenceSecs?: number | null, newBubbleLongPressMode?: string | null,
    newBubbleLongPressAutoSend?: boolean | null, newBubbleLongPressSilenceSecs?: number | null,
  ) => {
    await saveSettings(
      groqKey, deepseekKey, lang, style, newHotkey, newHotkeyMode, newAudioDevice,
      sttModel, customPrompt, autostart, whisperMode, openaiKey, anthropicKey,
      null, null, outputLang, webhookUrl, tursoUrl, tursoToken,
      bubbleSize, bubbleOpacity, localWhisperModel, localWhisperGpu,
      sttProvider, llmProvider,
      newInsertAndSendSlot1, newAutostopSilenceSecs, newAutoModeSilenceSecs,
      newHotkeySlot2 ?? null, newHotkeyModeSlot2 ?? null,
      newInsertAndSendSlot2 ?? null,
      newBubbleTapMode ?? null, newBubbleTapAutoSend ?? null,
      newBubbleTapSilenceSecs ?? null, newBubbleLongPressMode ?? null,
      newBubbleLongPressAutoSend ?? null, newBubbleLongPressSilenceSecs ?? null,
    );
    const updated = await getSettings();
    setLoadedSettings(updated);
    setLanguage(updated.language);
    setCleanupStyle(updated.cleanupStyle);
    setHotkey(updated.hotkey);
    setHotkeyMode(updated.hotkeyMode);
    setAudioDevice(updated.audioDevice);
    setOutputLanguage(updated.outputLanguage || "");
    setInsertAndSendSlot1(updated.insertAndSendSlot1 ?? false);
    setInsertAndSendSlot2(updated.insertAndSendSlot2 ?? false);
    setAutostopSilenceSecs(updated.autostopSilenceSecs ?? 2.0);
    setAutoModeSilenceSecs(updated.autoModeSilenceSecs ?? 2.0);
    setHotkeySlot2(updated.hotkeySlot2 ?? "");
    setHotkeyModeSlot2(updated.hotkeyModeSlot2 ?? "hold");
  }, []);

  const handleAddTerm = useCallback(async (term: string) => {
    await addDictionaryTerm(term);
    setDictionary((prev) => (prev.includes(term) ? prev : [...prev, term]));
  }, []);

  const handleRemoveTerm = useCallback(async (term: string) => {
    await removeDictionaryTerm(term);
    setDictionary((prev) => prev.filter((t) => t !== term));
  }, []);

  return {
    loadedSettings,
    language,
    cleanupStyle,
    hotkey,
    hotkeyMode,
    audioDevice,
    audioDevices,
    outputLanguage,
    dictionary,
    insertAndSendSlot1,
    insertAndSendSlot2,
    autostopSilenceSecs,
    autoModeSilenceSecs,
    hotkeySlot2,
    hotkeyModeSlot2,
    setHotkey,
    setHotkeyMode,
    setHotkeySlot2,
    setHotkeyModeSlot2,
    setAudioDevice,
    handleLanguageChange,
    handleStyleChange,
    handleOutputLanguageChange,
    handleSaveSettings,
    handleAddTerm,
    handleRemoveTerm,
    setLoadedSettings,
    setLanguage,
    setCleanupStyle,
  };
}
