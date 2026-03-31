use klarvo_lib::llm::{CleanupProvider, CleanupResult, CleanupStyle, LlmError, OpenAiCompatibleCleanup};

// ---------------------------------------------------------------------------
// Provider factory
// ---------------------------------------------------------------------------

/// Supported LLM provider names for testing.
pub enum Provider {
    Groq,
    DeepSeek,
    OpenAi,
}

impl Provider {
    /// Read from PI_PROVIDER env var, default to Groq.
    pub fn from_env() -> Self {
        match std::env::var("PI_PROVIDER")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "deepseek" => Provider::DeepSeek,
            "openai" => Provider::OpenAi,
            _ => Provider::Groq,
        }
    }

    /// Create a `Box<dyn CleanupProvider>` for this provider.
    ///
    /// Panics if the required API key env var is not set.
    pub fn make(&self) -> Box<dyn CleanupProvider> {
        match self {
            Provider::Groq => {
                let key = std::env::var("GROQ_API_KEY")
                    .expect("GROQ_API_KEY must be set for PI security tests");
                Box::new(OpenAiCompatibleCleanup::new(
                    key,
                    "https://api.groq.com/openai/v1/chat/completions",
                    "llama-3.3-70b-versatile",
                ))
            }
            Provider::DeepSeek => {
                let key = std::env::var("DEEPSEEK_API_KEY")
                    .expect("DEEPSEEK_API_KEY must be set for PI security tests");
                Box::new(OpenAiCompatibleCleanup::new(
                    key,
                    "https://api.deepseek.com/v1/chat/completions",
                    "deepseek-chat",
                ))
            }
            Provider::OpenAi => {
                let key = std::env::var("OPENAI_API_KEY")
                    .expect("OPENAI_API_KEY must be set for PI security tests");
                Box::new(OpenAiCompatibleCleanup::new(
                    key,
                    "https://api.openai.com/v1/chat/completions",
                    "gpt-4o-mini",
                ))
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Provider::Groq => "groq",
            Provider::DeepSeek => "deepseek",
            Provider::OpenAi => "openai",
        }
    }
}

// ---------------------------------------------------------------------------
// Injection adapters
// ---------------------------------------------------------------------------

/// Inject a payload via the `custom_prompt` surface.
///
/// The payload is passed as the `custom_prompt` parameter, which gets
/// interpolated into the system prompt as "Additional user instructions: {payload}".
pub async fn inject_via_custom_prompt(
    provider: &dyn CleanupProvider,
    input_text: &str,
    payload: &str,
) -> Result<CleanupResult, LlmError> {
    provider
        .cleanup(input_text, CleanupStyle::Polished, None, Some(payload))
        .await
}

/// Inject a payload via the `raw_text` surface (user message).
///
/// The payload IS the raw text sent as the user message. The system prompt
/// is clean (no custom prompt, no dictionary).
pub async fn inject_via_raw_text(
    provider: &dyn CleanupProvider,
    payload: &str,
) -> Result<CleanupResult, LlmError> {
    provider
        .cleanup(payload, CleanupStyle::Polished, None, None)
        .await
}

/// Inject a payload via the `dictionary_terms` surface.
///
/// The payload is passed as dictionary terms, which get interpolated into the
/// system prompt as "The user's custom dictionary terms (preserve these exactly): {payload}".
pub async fn inject_via_dictionary(
    provider: &dyn CleanupProvider,
    input_text: &str,
    payload: &str,
) -> Result<CleanupResult, LlmError> {
    provider
        .cleanup(input_text, CleanupStyle::Polished, Some(payload), None)
        .await
}

/// Inject a payload via the `voice_command` surface (Command Mode).
///
/// The payload is used as the voice command, with a benign selected text.
pub async fn inject_via_voice_command(
    provider: &dyn CleanupProvider,
    selected_text: &str,
    payload: &str,
) -> Result<CleanupResult, LlmError> {
    provider.rewrite(selected_text, payload).await
}

/// Inject a payload via the `selected_text` surface (Command Mode).
///
/// The payload is the selected text (e.g. a malicious document), with a
/// benign voice command.
pub async fn inject_via_selected_text(
    provider: &dyn CleanupProvider,
    payload: &str,
    voice_command: &str,
) -> Result<CleanupResult, LlmError> {
    provider.rewrite(payload, voice_command).await
}

// ---------------------------------------------------------------------------
// Golden path (baseline)
// ---------------------------------------------------------------------------

/// Run a normal cleanup to verify the provider works correctly.
/// Returns the cleaned text. Panics on error.
pub async fn golden_path(provider: &dyn CleanupProvider) -> String {
    let input = "I went to the um store yesterday and uh bought some groceries";
    let result = provider
        .cleanup(input, CleanupStyle::Polished, None, None)
        .await
        .expect("Golden path cleanup failed — is the API key valid?");

    assert!(
        !result.text.is_empty(),
        "Golden path returned empty text"
    );
    assert!(
        result.text.len() < input.len() * 3,
        "Golden path output suspiciously long: {} chars",
        result.text.len()
    );

    result.text
}
