export type SettingsCategory =
  | "home"
  | "recording-audio"
  | "ai-providers"
  | "appearance"
  | "language"
  | "shortcuts"
  | "license"
  | "dictionary"
  | "advanced"
  | "about";

export interface SettingsCategoryDef {
  id: SettingsCategory;
  label: string;
  subtitle?: string;
  iconColor: string;
  badge?: "TRIAL" | "BETA";
  desktopOnly?: boolean;
  mobileOnly?: boolean;
}

export const SETTINGS_CATEGORIES: SettingsCategoryDef[] = [
  {
    id: "recording-audio",
    label: "Recording & Audio",
    subtitle: "STT, model, microphone",
    iconColor: "#2AC3A8",
  },
  {
    id: "ai-providers",
    label: "AI & Providers",
    subtitle: "API keys, cleanup, profiles",
    iconColor: "#818CF8",
  },
  {
    id: "appearance",
    label: "Appearance",
    subtitle: "Live preview look & behavior",
    iconColor: "#34D399",
  },
  {
    id: "language",
    label: "Language",
    subtitle: "Dictation & translation language",
    iconColor: "#22D3EE",
  },
  {
    id: "shortcuts",
    label: "Shortcuts",
    subtitle: "Hotkeys, modes, paste behavior",
    iconColor: "#F59E0B",
  },
  {
    id: "license",
    label: "License",
    iconColor: "#FFA344",
  },
  {
    id: "dictionary",
    label: "Dictionary",
    iconColor: "#60A5FA",
  },
  {
    id: "advanced",
    label: "Advanced",
    subtitle: "Prompts, audio, webhooks, sync",
    iconColor: "#94A3B8",
  },
  {
    id: "about",
    label: "About",
    subtitle: "Version, updates, links",
    iconColor: "#6B7280",
  },
];
