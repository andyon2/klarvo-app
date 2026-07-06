import type { AppSettings, CleanupStyle, AppProfile } from "../../types";
import { STYLE_OPTIONS } from "../../types";
import { isMobile } from "../../platform";
import { CloseIcon, LockIcon } from "../icons";
import { StatusDot, INPUT_CLS, INPUT_CLS_M, LABEL_CLS_M } from "../ui";
import { MobileTextarea } from "../MobileTextarea";
import { saveProfiles } from "../../tauri-commands";
import { KSelect } from "./FormControls";

// --- Props -------------------------------------------------------------------

export interface AiProvidersContentProps {
  // API key values (new input, empty = no change)
  groqKey: string;
  setGroqKey: (v: string) => void;
  deepseekKey: string;
  setDeepseekKey: (v: string) => void;
  openaiKey: string;
  setOpenaiKey: (v: string) => void;
  anthropicKey: string;
  setAnthropicKey: (v: string) => void;
  openrouterKey: string;
  setOpenrouterKey: (v: string) => void;
  // Validation state
  apiKeyErrors: Record<string, string | null>;
  setApiKeyErrors: (fn: (prev: Record<string, string | null>) => Record<string, string | null>) => void;
  apiKeyValidating: Record<string, boolean>;
  apiKeyConfirmRemove: Record<string, boolean>;
  onApiKeyRemoveClick: (provider: string) => void;
  // Custom prompt
  localCustomPrompt: string;
  setLocalCustomPrompt: (v: string) => void;
  // App profiles
  profiles: AppProfile[];
  setProfiles: (v: AppProfile[]) => void;
  // Loaded settings (for masked key placeholders)
  loadedSettings: AppSettings | null;
  // Feature gate
  isPaid: boolean;
  isTrial: boolean;
  // Provider status (key present in backend)
  groqOk: boolean;
  deepseekOk: boolean;
  openaiOk: boolean;
  anthropicOk: boolean;
  openrouterOk: boolean;
  // Save feedback (used by "Save Profiles" button)
  setSaveMsg: (v: string | null) => void;
}

// --- Component ---------------------------------------------------------------

export function AiProvidersContent({
  groqKey, setGroqKey, deepseekKey, setDeepseekKey,
  openaiKey, setOpenaiKey, anthropicKey, setAnthropicKey,
  openrouterKey, setOpenrouterKey,
  apiKeyErrors, setApiKeyErrors, apiKeyValidating, apiKeyConfirmRemove, onApiKeyRemoveClick,
  localCustomPrompt, setLocalCustomPrompt,
  profiles, setProfiles,
  loadedSettings,
  isPaid,
  isTrial,
  groqOk, deepseekOk, openaiOk, anthropicOk, openrouterOk,
  setSaveMsg,
}: AiProvidersContentProps) {
  const TrialBadge = () => (
    <span className="text-[10px] font-semibold uppercase tracking-wider bg-klarvo-teal/15 text-klarvo-teal px-1.5 py-0.5 rounded border border-klarvo-teal/25">
      Trial
    </span>
  );

  return (
    <div className="flex flex-col gap-5">

      {/* --- API Keys --- */}
      <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span className={LABEL_CLS_M}>Groq</span>
            <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Speech + Cleanup)</span>
            <StatusDot active={groqOk} />
          </div>
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={groqOk ? loadedSettings!.groqApiKeyMasked : "gsk_..."}
            value={groqKey}
            onChange={(e) => { setGroqKey(e.target.value); setApiKeyErrors((p) => ({ ...p, groq: null })); }}
            className={`${INPUT_CLS_M} font-geist-mono`}
          />
          {apiKeyValidating["groq"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
          {apiKeyErrors["groq"] && <span className="text-[11px] text-klarvo-amber">{apiKeyErrors["groq"]}</span>}
          {groqOk && (
            <button type="button" onClick={() => onApiKeyRemoveClick("groq")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["groq"] ? "text-klarvo-danger hover:text-klarvo-danger" : "text-klarvo-amber/80 hover:text-klarvo-amber"}`}>
              {apiKeyConfirmRemove["groq"] ? "Click again to confirm removal" : "Remove Key"}
            </button>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span className={LABEL_CLS_M}>DeepSeek</span>
            <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
            <StatusDot active={deepseekOk} />
          </div>
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={deepseekOk ? loadedSettings!.deepseekApiKeyMasked : "sk-..."}
            value={deepseekKey}
            onChange={(e) => { setDeepseekKey(e.target.value); setApiKeyErrors((p) => ({ ...p, deepseek: null })); }}
            className={`${INPUT_CLS_M} font-geist-mono`}
          />
          {apiKeyValidating["deepseek"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
          {apiKeyErrors["deepseek"] && <span className="text-[11px] text-klarvo-amber">{apiKeyErrors["deepseek"]}</span>}
          {deepseekOk && (
            <button type="button" onClick={() => onApiKeyRemoveClick("deepseek")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["deepseek"] ? "text-klarvo-danger hover:text-klarvo-danger" : "text-klarvo-amber/80 hover:text-klarvo-amber"}`}>
              {apiKeyConfirmRemove["deepseek"] ? "Click again to confirm removal" : "Remove Key"}
            </button>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span className={LABEL_CLS_M}>OpenAI</span>
            <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Speech + Cleanup)</span>
            <StatusDot active={openaiOk} />
          </div>
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={openaiOk ? loadedSettings!.openaiApiKeyMasked : "sk-..."}
            value={openaiKey}
            onChange={(e) => { setOpenaiKey(e.target.value); setApiKeyErrors((p) => ({ ...p, openai: null })); }}
            className={`${INPUT_CLS_M} font-geist-mono`}
          />
          {apiKeyValidating["openai"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
          {apiKeyErrors["openai"] && <span className="text-[11px] text-klarvo-amber">{apiKeyErrors["openai"]}</span>}
          {openaiOk && (
            <button type="button" onClick={() => onApiKeyRemoveClick("openai")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["openai"] ? "text-klarvo-danger hover:text-klarvo-danger" : "text-klarvo-amber/80 hover:text-klarvo-amber"}`}>
              {apiKeyConfirmRemove["openai"] ? "Click again to confirm removal" : "Remove Key"}
            </button>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span className={LABEL_CLS_M}>Anthropic</span>
            <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
            <StatusDot active={anthropicOk} />
          </div>
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={anthropicOk ? loadedSettings!.anthropicApiKeyMasked : "sk-ant-..."}
            value={anthropicKey}
            onChange={(e) => { setAnthropicKey(e.target.value); setApiKeyErrors((p) => ({ ...p, anthropic: null })); }}
            className={`${INPUT_CLS_M} font-geist-mono`}
          />
          {apiKeyValidating["anthropic"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
          {apiKeyErrors["anthropic"] && <span className="text-[11px] text-klarvo-amber">{apiKeyErrors["anthropic"]}</span>}
          {anthropicOk && (
            <button type="button" onClick={() => onApiKeyRemoveClick("anthropic")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["anthropic"] ? "text-klarvo-danger hover:text-klarvo-danger" : "text-klarvo-amber/80 hover:text-klarvo-amber"}`}>
              {apiKeyConfirmRemove["anthropic"] ? "Click again to confirm removal" : "Remove Key"}
            </button>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <span className={LABEL_CLS_M}>OpenRouter</span>
            <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>(Cleanup)</span>
            <StatusDot active={openrouterOk} />
          </div>
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder={openrouterOk ? loadedSettings!.openrouterApiKeyMasked : "sk-or-..."}
            value={openrouterKey}
            onChange={(e) => { setOpenrouterKey(e.target.value); setApiKeyErrors((p) => ({ ...p, openrouter: null })); }}
            className={`${INPUT_CLS_M} font-geist-mono`}
          />
          {apiKeyValidating["openrouter"] && <span className="text-[11px] text-klarvo-muted">Validating...</span>}
          {apiKeyErrors["openrouter"] && <span className="text-[11px] text-klarvo-amber">{apiKeyErrors["openrouter"]}</span>}
          {openrouterOk && (
            <button type="button" onClick={() => onApiKeyRemoveClick("openrouter")} className={`self-start transition-colors ${isMobile ? "text-sm" : "text-[11px]"} ${apiKeyConfirmRemove["openrouter"] ? "text-klarvo-danger hover:text-klarvo-danger" : "text-klarvo-amber/80 hover:text-klarvo-amber"}`}>
              {apiKeyConfirmRemove["openrouter"] ? "Click again to confirm removal" : "Remove Key"}
            </button>
          )}
        </div>
      </div>

      {/* --- Cleanup Instructions --- */}
      <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
        <div className="flex items-center gap-2">
          <span className={isMobile ? "text-sm font-semibold text-klarvo-muted uppercase tracking-widest" : "text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest"}>Cleanup Instructions</span>
          {isPaid && isTrial && <TrialBadge />}
        </div>
        <MobileTextarea
          label="Cleanup Instructions"
          hint="Appended to the system prompt during LLM cleanup."
          value={localCustomPrompt}
          onChange={isPaid ? setLocalCustomPrompt : () => {}}
          placeholder={isPaid ? "Extra instructions for the LLM, e.g. 'Always use formal German' or 'Keep technical terms in English'" : "Requires Klarvo License"}
          rows={3}
          className={`${INPUT_CLS_M} resize-none${!isPaid ? " opacity-50 cursor-not-allowed" : ""}`}
          disabled={!isPaid}
        />
        {/* Preset buttons -- one click replaces the entire custom prompt */}
        <div className="flex items-center gap-2 flex-wrap">
          <span className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Presets:</span>
          {([
            { label: "Formal", prompt: "Always use formal language. Avoid colloquialisms and slang." },
            { label: "Technical", prompt: "Keep technical terms in English. Use precise, professional language." },
            { label: "Casual", prompt: "Keep it casual and conversational. Use natural, relaxed language." },
          ] as const).map(({ label, prompt }) => (
            <button
              key={label}
              type="button"
              onClick={() => setLocalCustomPrompt(prompt)}
              className={[
                "border rounded-lg font-medium transition-colors",
                "bg-transparent border-klarvo-border/60 text-klarvo-muted",
                "hover:border-klarvo-border-2 hover:text-klarvo-text",
                isMobile ? "px-4 min-h-[44px] text-sm" : "px-3 py-1.5 text-xs",
              ].join(" ")}
            >
              {label}
            </button>
          ))}
          <button
            type="button"
            onClick={() => setLocalCustomPrompt("")}
            className={[
              "transition-colors",
              "text-klarvo-dim hover:text-klarvo-muted",
              isMobile ? "px-3 min-h-[44px] text-sm" : "px-2 py-1.5 text-xs",
            ].join(" ")}
          >
            Clear
          </button>
        </div>
        <p className={isMobile ? "text-xs text-klarvo-dim" : "text-[11px] text-klarvo-dim"}>Appended to the system prompt during LLM cleanup.</p>
      </div>

      {/* --- App Profiles (paid feature) --- */}
      <div className="flex flex-col gap-3 pl-4 pb-3 pt-1">
        {!isPaid ? (
          // Free-tier paygate: show lock message, no profile editing allowed.
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2 text-klarvo-dim">
              <LockIcon className="w-3.5 h-3.5 text-klarvo-dim flex-shrink-0" />
              <p className="text-xs">App Profiles require a Klarvo license.</p>
            </div>
            <p className="text-[11px] text-klarvo-dim">Override style and language per app based on window title.</p>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-2">
              <span className={isMobile ? "text-sm font-semibold text-klarvo-muted uppercase tracking-widest" : "text-[11px] font-semibold text-klarvo-muted uppercase tracking-widest"}>App Profiles</span>
              {isTrial && <TrialBadge />}
            </div>
            <p className="text-[11px] text-klarvo-dim">Override style/language per app. Matches window title substring.</p>

            {profiles.map((p, i) => (
              <div key={i} className="bg-klarvo-bg border border-klarvo-border/60 rounded-xl p-3 flex flex-col gap-2">
                <div className="flex items-center justify-between gap-2">
                  <input
                    type="text"
                    placeholder="Profile name"
                    value={p.name}
                    onChange={(e) => {
                      const next = [...profiles];
                      next[i] = { ...next[i], name: e.target.value };
                      setProfiles(next);
                    }}
                    className={`flex-1 ${INPUT_CLS}`}
                  />
                  <button
                    onClick={() => {
                      const next = profiles.filter((_, j) => j !== i);
                      setProfiles(next);
                      saveProfiles(next).catch(console.error);
                    }}
                    className="text-klarvo-dim hover:text-klarvo-danger transition-colors p-1"
                  >
                    <CloseIcon />
                  </button>
                </div>
                <input
                  type="text"
                  placeholder="Window title pattern, e.g. 'Slack' or 'Visual Studio'"
                  value={p.appPattern}
                  onChange={(e) => {
                    const next = [...profiles];
                    next[i] = { ...next[i], appPattern: e.target.value };
                    setProfiles(next);
                  }}
                  className={INPUT_CLS}
                />
                <div className="flex gap-2">
                  <KSelect
                    value={p.cleanupStyle}
                    onChange={(v) => {
                      const next = [...profiles];
                      next[i] = { ...next[i], cleanupStyle: v as CleanupStyle };
                      setProfiles(next);
                    }}
                    options={STYLE_OPTIONS.map((opt) => ({ value: opt.value, label: opt.label }))}
                  />
                  <KSelect
                    value={p.language}
                    onChange={(v) => {
                      const next = [...profiles];
                      next[i] = { ...next[i], language: v };
                      setProfiles(next);
                    }}
                    options={[
                      { value: "", label: "Auto" },
                      { value: "de", label: "DE" },
                      { value: "en", label: "EN" },
                    ]}
                  />
                </div>
                <input
                  type="text"
                  placeholder="Custom prompt for this app (optional)"
                  value={p.customPrompt}
                  onChange={(e) => {
                    const next = [...profiles];
                    next[i] = { ...next[i], customPrompt: e.target.value };
                    setProfiles(next);
                  }}
                  className={INPUT_CLS}
                />
              </div>
            ))}

            <div className="flex gap-2">
              <button
                onClick={() => setProfiles([...profiles, { name: "", appPattern: "", cleanupStyle: "polished", language: "", customPrompt: "" }])}
                className="px-3 py-2 rounded-lg text-xs font-medium bg-klarvo-bg border border-klarvo-border/60 text-klarvo-muted hover:bg-klarvo-surface/60 transition-colors"
              >
                + Add Profile
              </button>
              {profiles.length > 0 && (
                <button
                  onClick={() => saveProfiles(profiles).then(() => setSaveMsg("Profiles saved")).catch((e) => setSaveMsg(String(e)))}
                  className="px-3 py-2 rounded-lg text-xs font-medium bg-klarvo-teal/10 border border-klarvo-teal/20 text-klarvo-teal hover:bg-klarvo-teal/15 transition-colors"
                >
                  Save Profiles
                </button>
              )}
            </div>
          </>
        )}
      </div>

    </div>
  );
}
