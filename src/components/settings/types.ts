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
    iconColor: "#29C7AC",
  },
  {
    id: "ai-providers",
    label: "AI & Providers",
    subtitle: "API keys, cleanup, profiles",
    iconColor: "#57DDC7",
  },
  {
    id: "appearance",
    label: "Appearance",
    subtitle: "Live preview look & behavior",
    iconColor: "#4FC58A",
  },
  {
    id: "language",
    label: "Language",
    subtitle: "Dictation & translation language",
    iconColor: "#57DDC7",
  },
  {
    id: "shortcuts",
    label: "Shortcuts",
    subtitle: "Hotkeys, modes, paste behavior",
    iconColor: "#E9A24C",
  },
  {
    id: "license",
    label: "License",
    iconColor: "#E9A24C",
  },
  {
    id: "dictionary",
    label: "Dictionary",
    iconColor: "#29C7AC",
  },
  {
    id: "advanced",
    label: "Advanced",
    subtitle: "Prompts, audio, webhooks, sync",
    iconColor: "#A4A9AC",
  },
  {
    id: "about",
    label: "About",
    subtitle: "Version, updates, links",
    iconColor: "#6F7479",
  },
];
