//! LLM-based text cleanup module.
//!
//! Defines the `CleanupProvider` trait and concrete implementations:
//!
//! - `DeepSeekCleanup`: DeepSeek Chat API (OpenAI-compatible, cheapest)
//! - `OpenAiCleanup`: OpenAI Chat API (OpenAI-compatible, `gpt-4o-mini`)
//! - `GroqCleanup`: Groq Chat API (OpenAI-compatible, `llama-3.3-70b-versatile`)
//! - `AnthropicCleanup`: Anthropic Messages API (different format, `claude-haiku-4-5-20251001`)
//!
//! DeepSeek, OpenAI and Groq all share the generic `OpenAiCompatibleCleanup`
//! struct. Anthropic requires its own implementation because the API format
//! differs (different headers, top-level system field, different response shape).
//!
//! The `CleanupProvider` trait also exposes a `rewrite()` method for Command
//! Mode (rewrite selected text based on a voice command) with a default
//! implementation that works for all OpenAI-compatible providers.
//!
//! API docs:
//! - DeepSeek: <https://platform.deepseek.com/api-docs>
//! - OpenAI: <https://platform.openai.com/docs/api-reference/chat/create>
//! - Groq: <https://console.groq.com/docs/openai>
//! - Anthropic: <https://docs.anthropic.com/en/api/messages>

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during LLM text cleanup.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Unexpected response format: {0}")]
    ResponseFormat(String),

    #[error("Input text is empty")]
    EmptyInput,

    #[error("Output was truncated: max_tokens limit reached")]
    OutputTruncated,
}

// ---------------------------------------------------------------------------
// Cleanup style
// ---------------------------------------------------------------------------

/// Controls how aggressively the LLM cleans up the raw transcription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CleanupStyle {
    /// Full cleanup: remove fillers, fix grammar, professional formatting.
    Polished,
    /// Light cleanup: remove fillers and duplications, keep speaker's words.
    Verbatim,
    /// Chat-ready: short, casual, emojis allowed.
    Chat,
}

impl CleanupStyle {
    /// Returns the system prompt for this cleanup style.
    ///
    /// `dictionary_terms` is an optional comma-separated list of custom terms
    /// the LLM should preserve verbatim (from the user's dictionary).
    ///
    /// `custom_prompt` is an optional string of additional user instructions
    /// appended at the end of the system prompt.
    pub fn system_prompt(&self, dictionary_terms: Option<&str>, custom_prompt: Option<&str>) -> String {
        let dict_section = match dictionary_terms {
            Some(terms) if !terms.is_empty() => {
                format!("\n\nThe user's custom dictionary terms (preserve these exactly): {terms}")
            }
            _ => String::new(),
        };

        let custom_section = match custom_prompt {
            Some(p) if !p.trim().is_empty() => {
                format!("\n\nAdditional user instructions: {}", p.trim())
            }
            _ => String::new(),
        };

        match self {
            CleanupStyle::Polished => format!(
                "You are a text cleanup assistant. The user will give you raw speech-to-text \
                output. Clean it up:\n\
                - Remove filler words (um, uh, like, you know / äh, ähm, also)\n\
                - Handle mid-speech corrections: when the speaker backtracks or corrects \
                  themselves (e.g. 'tomorrow, no wait, Friday' → 'Friday', \
                  'ich meine eigentlich' → keep only the correction), output ONLY the \
                  final intended version\n\
                - Fix grammar and punctuation\n\
                - Format for readability: use line breaks between distinct thoughts, \
                  paragraph breaks for topic changes, and blank lines to separate sections\n\
                - Use proper capitalization\n\
                - For lists or enumerations, use bullet points or numbered lists\n\
                - Preserve the speaker's meaning exactly -- do not add or change content\n\
                - Language: respond in the same language as the input. If the input mixes \
                  German and English, keep each part in its original language\n\
                - Return ONLY the cleaned text, no explanations or commentary\
                {dict_section}{custom_section}"
            ),
            CleanupStyle::Verbatim => format!(
                "You are a text cleanup assistant. The user will give you raw speech-to-text \
                output. Light cleanup -- keep the original wording:\n\
                - Remove filler words (um, uh, like, you know / äh, ähm, also, halt, sozusagen)\n\
                - Handle mid-speech corrections: when the speaker backtracks or corrects \
                  themselves (e.g. 'tomorrow, no wait, Friday' → 'Friday', \
                  'das heißt, nein, ich meine' → keep only the correction), output ONLY the \
                  final intended version\n\
                - Add punctuation and capitalization\n\
                - Fix obvious transcription errors\n\
                - Add line breaks between sentences for readability\n\
                - Do NOT rephrase, summarize, or change the speaker's words\n\
                - Keep the speaker's style, tone, and sentence structure\n\
                - Language: respond in the same language as the input\n\
                - Return ONLY the cleaned text, no explanations or commentary\
                {dict_section}{custom_section}"
            ),
            CleanupStyle::Chat => {
                // Chat style has no dictionary context -- keeps it short
                format!(
                    "You are a text cleanup assistant. The user will give you raw speech-to-text \
                    output. Make it chat-ready:\n\
                    - Remove all filler words\n\
                    - Handle mid-speech corrections: when the speaker backtracks, keep only \
                      the final intended version\n\
                    - Make it concise and casual\n\
                    - Keep it short -- this is for messaging apps\n\
                    - Use line breaks where natural in longer messages\n\
                    - Emojis are okay if they fit naturally\n\
                    - Language: respond in the same language as the input\n\
                    - Return ONLY the cleaned text, no explanations or commentary\
                    {custom_section}"
                )
            }
        }
    }

    /// System prompt for Command Mode (rewrite selected text based on voice command).
    pub fn command_mode_system_prompt() -> &'static str {
        "You are a text editing assistant. The user has selected some text \
            and will give you a voice command describing how to change it.\n\
            - Apply the command to the selected text\n\
            - Common commands: make shorter, make longer, rephrase, make formal, make casual, \
              translate to English/German, fix grammar, turn into a list, summarize\n\
            - Preserve the language of the original text unless the command explicitly asks \
              for translation\n\
            - Return ONLY the rewritten text, no explanations or commentary\n\
            - If you don't understand the command, return the original text unchanged"
    }
}

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// The output of an LLM cleanup call, including token usage for cost tracking.
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// The cleaned-up text returned by the LLM.
    pub text: String,
    /// Number of prompt tokens consumed (if the API reported it).
    pub prompt_tokens: Option<u32>,
    /// Number of completion tokens consumed (if the API reported it).
    pub completion_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over LLM text-cleanup backends.
///
/// Implementations receive raw transcription text and return a `CleanupResult`
/// that includes the cleaned text plus token usage for cost tracking.
///
/// The `rewrite()` method supports Command Mode (voice-edit selected text).
/// It has a default implementation that returns `EmptyInput` -- providers
/// that support rewrite should override it. The concrete implementations
/// in this module all delegate through their own `rewrite()` logic.
#[async_trait::async_trait]
pub trait CleanupProvider: Send + Sync {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError>;

    /// Rewrites `selected_text` according to a `voice_command`.
    ///
    /// Used in Command Mode. The default implementation returns an error so
    /// providers that don't support rewrite fail gracefully. All concrete
    /// providers in this crate provide a full implementation.
    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        let _ = (selected_text, voice_command);
        Err(LlmError::ResponseFormat(
            "rewrite() not implemented for this provider".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

/// Token usage info from OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_cache_hit_tokens: Option<u32>,
    pub prompt_cache_miss_tokens: Option<u32>,
}

/// Error response from OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// OpenAiCompatibleCleanup -- generic OpenAI Chat API client
// ---------------------------------------------------------------------------

/// Generic OpenAI-compatible Chat API client for text cleanup.
///
/// Works with any endpoint that speaks the OpenAI Chat Completions protocol:
/// DeepSeek, OpenAI, Groq, and many others.
pub struct OpenAiCompatibleCleanup {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl OpenAiCompatibleCleanup {
    const DEFAULT_TEMPERATURE: f32 = 0.3;
    const DEFAULT_MAX_TOKENS: u32 = 2048;

    /// Creates a new client.
    ///
    /// - `api_key`: Bearer token for the API.
    /// - `base_url`: Full URL of the chat completions endpoint.
    /// - `model`: Model identifier.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        OpenAiCompatibleCleanup {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            temperature: Self::DEFAULT_TEMPERATURE,
            max_tokens: Self::DEFAULT_MAX_TOKENS,
        }
    }

    /// Builds the cleanup request body.
    pub fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> ChatRequest<'a> {
        let system_prompt = style.system_prompt(dictionary_terms, custom_prompt);

        ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: raw_text.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }

    /// Builds the Command Mode rewrite request body.
    pub fn build_command_request<'a>(
        &'a self,
        selected_text: &str,
        voice_command: &str,
    ) -> ChatRequest<'a> {
        ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: CleanupStyle::command_mode_system_prompt().to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: format!(
                        "Selected text:\n{selected_text}\n\nCommand: {voice_command}"
                    ),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }

    /// Sends a `ChatRequest` to the endpoint and parses the response.
    async fn send_request(&self, body: &ChatRequest<'_>) -> Result<CleanupResult, LlmError> {
        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let body_text = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body_text)
                .map(|e| e.error.message)
                .unwrap_or(body_text);
            return Err(LlmError::ApiError {
                status: status_code,
                message,
            });
        }

        let api_response: ChatResponse = response.json().await?;

        let (prompt_tokens, completion_tokens) = api_response
            .usage
            .map(|u| (Some(u.prompt_tokens), Some(u.completion_tokens)))
            .unwrap_or((None, None));

        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ResponseFormat("No choices in response".to_string()))?;

        if choice.finish_reason.as_deref() == Some("length") {
            return Err(LlmError::OutputTruncated);
        }

        let content = choice.message.content;
        if content.is_empty() {
            return Err(LlmError::ResponseFormat(
                "Empty content in response".to_string(),
            ));
        }

        Ok(CleanupResult {
            text: content,
            prompt_tokens,
            completion_tokens,
        })
    }
}

#[async_trait::async_trait]
impl CleanupProvider for OpenAiCompatibleCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        if raw_text.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let body = self.build_request(raw_text, style, dictionary_terms, custom_prompt);
        self.send_request(&body).await
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        if selected_text.trim().is_empty() || voice_command.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let body = self.build_command_request(selected_text, voice_command);
        let mut result = self.send_request(&body).await?;
        // Trim whitespace from Command Mode output
        result.text = result.text.trim().to_string();
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// DeepSeekCleanup
// ---------------------------------------------------------------------------

/// DeepSeek Chat API client for text cleanup.
///
/// Uses `deepseek-chat` (DeepSeek-V3) at temperature 0.3 -- low enough for
/// faithful cleanup, high enough to avoid robotic output.
///
/// This is a thin wrapper around `OpenAiCompatibleCleanup` with DeepSeek's
/// endpoint and model pre-configured.
pub struct DeepSeekCleanup {
    inner: OpenAiCompatibleCleanup,
}

impl DeepSeekCleanup {
    pub(crate) const BASE_URL: &'static str = "https://api.deepseek.com/v1/chat/completions";
    pub(crate) const DEFAULT_MODEL: &'static str = "deepseek-chat";

    /// Creates a new `DeepSeekCleanup` client with the given API key.
    ///
    /// The API key should come from the caller (environment variable or
    /// system keystore) -- never hard-coded.
    pub fn new(api_key: impl Into<String>) -> Self {
        DeepSeekCleanup {
            inner: OpenAiCompatibleCleanup::new(api_key, Self::BASE_URL, Self::DEFAULT_MODEL),
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner.model = model.into();
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Builds the JSON request body (for testing without network).
    #[cfg(test)]
    pub fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> ChatRequest<'a> {
        self.inner.build_request(raw_text, style, dictionary_terms, custom_prompt)
    }
}

#[async_trait::async_trait]
impl CleanupProvider for DeepSeekCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.cleanup(raw_text, style, dictionary_terms, custom_prompt).await
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.rewrite(selected_text, voice_command).await
    }
}

// ---------------------------------------------------------------------------
// OpenAiCleanup
// ---------------------------------------------------------------------------

/// OpenAI Chat API client for text cleanup.
///
/// Uses `gpt-4o-mini` -- a good balance of quality and cost.
pub struct OpenAiCleanup {
    inner: OpenAiCompatibleCleanup,
}

impl OpenAiCleanup {
    const BASE_URL: &'static str = "https://api.openai.com/v1/chat/completions";
    const DEFAULT_MODEL: &'static str = "gpt-4o-mini";

    /// Creates a new `OpenAiCleanup` client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        OpenAiCleanup {
            inner: OpenAiCompatibleCleanup::new(api_key, Self::BASE_URL, Self::DEFAULT_MODEL),
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner.model = model.into();
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Builds the JSON request body (for testing without network).
    #[cfg(test)]
    pub fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> ChatRequest<'a> {
        self.inner.build_request(raw_text, style, dictionary_terms, custom_prompt)
    }
}

#[async_trait::async_trait]
impl CleanupProvider for OpenAiCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.cleanup(raw_text, style, dictionary_terms, custom_prompt).await
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.rewrite(selected_text, voice_command).await
    }
}

// ---------------------------------------------------------------------------
// GroqCleanup
// ---------------------------------------------------------------------------

/// Groq Chat API client for text cleanup (OpenAI-compatible).
///
/// Uses `llama-3.3-70b-versatile` -- fast LPU inference, good quality,
/// uses the same Groq API key as the STT provider.
pub struct GroqCleanup {
    inner: OpenAiCompatibleCleanup,
}

impl GroqCleanup {
    const BASE_URL: &'static str = "https://api.groq.com/openai/v1/chat/completions";
    const DEFAULT_MODEL: &'static str = "llama-3.3-70b-versatile";

    /// Creates a new `GroqCleanup` client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        GroqCleanup {
            inner: OpenAiCompatibleCleanup::new(api_key, Self::BASE_URL, Self::DEFAULT_MODEL),
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner.model = model.into();
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Builds the JSON request body (for testing without network).
    #[cfg(test)]
    pub fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> ChatRequest<'a> {
        self.inner.build_request(raw_text, style, dictionary_terms, custom_prompt)
    }
}

#[async_trait::async_trait]
impl CleanupProvider for GroqCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.cleanup(raw_text, style, dictionary_terms, custom_prompt).await
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        self.inner.rewrite(selected_text, voice_command).await
    }
}

// ---------------------------------------------------------------------------
// AnthropicCleanup -- Anthropic Messages API (different format from OpenAI)
// ---------------------------------------------------------------------------

/// Anthropic-specific request format.
///
/// The Anthropic Messages API differs from OpenAI's Chat Completions:
/// - `system` is a top-level field (not a message with role="system")
/// - `messages` only contains user/assistant turns
/// - `max_tokens` is required (not optional)
/// - Auth uses `x-api-key` header instead of `Authorization: Bearer`
/// - Response `content` is an array of typed blocks, not a single string
#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    system: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API response.
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic API error response.
#[derive(Debug, Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

/// Anthropic Messages API client for text cleanup.
///
/// Uses `claude-haiku-4-5-20251001` -- fast and cheap for text cleanup tasks.
/// Auth uses `x-api-key` header (not Bearer token).
pub struct AnthropicCleanup {
    api_key: String,
    client: reqwest::Client,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl AnthropicCleanup {
    const BASE_URL: &'static str = "https://api.anthropic.com/v1/messages";
    const API_VERSION: &'static str = "2023-06-01";
    const DEFAULT_MODEL: &'static str = "claude-haiku-4-5-20251001";
    const DEFAULT_TEMPERATURE: f32 = 0.3;
    const DEFAULT_MAX_TOKENS: u32 = 2048;

    /// Creates a new `AnthropicCleanup` client with the given API key.
    ///
    /// The API key should come from the caller (environment variable or
    /// system keystore) -- never hard-coded.
    pub fn new(api_key: impl Into<String>) -> Self {
        AnthropicCleanup {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            model: Self::DEFAULT_MODEL.to_string(),
            temperature: Self::DEFAULT_TEMPERATURE,
            max_tokens: Self::DEFAULT_MAX_TOKENS,
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Builds the cleanup request body (for testing without network).
    pub fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> AnthropicRequest<'a> {
        AnthropicRequest {
            model: &self.model,
            system: style.system_prompt(dictionary_terms, custom_prompt),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: raw_text.to_string(),
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
        }
    }

    /// Builds the Command Mode rewrite request body.
    pub fn build_command_request<'a>(
        &'a self,
        selected_text: &str,
        voice_command: &str,
    ) -> AnthropicRequest<'a> {
        AnthropicRequest {
            model: &self.model,
            system: CleanupStyle::command_mode_system_prompt().to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: format!(
                    "Selected text:\n{selected_text}\n\nCommand: {voice_command}"
                ),
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
        }
    }

    /// Sends an `AnthropicRequest` and parses the response.
    async fn send_request(&self, body: &AnthropicRequest<'_>) -> Result<CleanupResult, LlmError> {
        let response = self
            .client
            .post(Self::BASE_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", Self::API_VERSION)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let body_text = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<AnthropicErrorResponse>(&body_text)
                .map(|e| e.error.message)
                .unwrap_or(body_text);
            return Err(LlmError::ApiError {
                status: status_code,
                message,
            });
        }

        let api_response: AnthropicResponse = response.json().await?;

        // "max_tokens" is Anthropic's equivalent of OpenAI's "length" finish_reason
        if api_response.stop_reason.as_deref() == Some("max_tokens") {
            return Err(LlmError::OutputTruncated);
        }

        // Extract text from the first text block in the content array
        let text = api_response
            .content
            .into_iter()
            .find(|block| block.block_type == "text")
            .and_then(|block| block.text)
            .ok_or_else(|| LlmError::ResponseFormat("No text block in Anthropic response".to_string()))?;

        if text.is_empty() {
            return Err(LlmError::ResponseFormat(
                "Empty text in Anthropic response".to_string(),
            ));
        }

        let (prompt_tokens, completion_tokens) = api_response
            .usage
            .map(|u| (Some(u.input_tokens), Some(u.output_tokens)))
            .unwrap_or((None, None));

        Ok(CleanupResult {
            text,
            prompt_tokens,
            completion_tokens,
        })
    }
}

#[async_trait::async_trait]
impl CleanupProvider for AnthropicCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        if raw_text.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let body = self.build_request(raw_text, style, dictionary_terms, custom_prompt);
        self.send_request(&body).await
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        if selected_text.trim().is_empty() || voice_command.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let body = self.build_command_request(selected_text, voice_command);
        let mut result = self.send_request(&body).await?;
        result.text = result.text.trim().to_string();
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- DeepSeekCleanup tests (preserved from original) ---

    #[test]
    fn test_deepseek_cleanup_new_stores_api_key() {
        let client = DeepSeekCleanup::new("sk-test-key");
        assert_eq!(client.api_key(), "sk-test-key");
        assert_eq!(client.inner.model, DeepSeekCleanup::DEFAULT_MODEL);
        assert_eq!(client.inner.temperature, OpenAiCompatibleCleanup::DEFAULT_TEMPERATURE);
    }

    /// Verifies that the request body has the correct structure for all styles.
    #[test]
    fn test_build_request_polished_contains_system_prompt() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request("hello world", CleanupStyle::Polished, None, None);

        assert_eq!(req.model, "deepseek-chat");
        assert_eq!(req.temperature, 0.3);
        assert_eq!(req.max_tokens, 2048);
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[1].role, "user");
        assert_eq!(req.messages[1].content, "hello world");
        assert!(
            req.messages[0].content.contains("filler words"),
            "Polished prompt should mention filler words"
        );
    }

    #[test]
    fn test_build_request_verbatim_style() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request("test", CleanupStyle::Verbatim, None, None);
        assert!(
            req.messages[0].content.contains("Light cleanup"),
            "Verbatim prompt should say 'Light cleanup'"
        );
    }

    #[test]
    fn test_build_request_chat_style() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request("test", CleanupStyle::Chat, None, None);
        assert!(
            req.messages[0].content.contains("chat-ready"),
            "Chat prompt should say 'chat-ready'"
        );
    }

    #[test]
    fn test_build_request_with_dictionary_terms() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request(
            "text with Kubernetes",
            CleanupStyle::Polished,
            Some("Kubernetes, DeepSeek, Tauri"),
            None,
        );
        assert!(
            req.messages[0]
                .content
                .contains("Kubernetes, DeepSeek, Tauri"),
            "System prompt should include dictionary terms"
        );
    }

    #[test]
    fn test_build_request_serializes_to_valid_json() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request("some text", CleanupStyle::Polished, None, None);
        let json = serde_json::to_string(&req).expect("should serialize to JSON");
        assert!(json.contains("deepseek-chat"));
        assert!(json.contains("some text"));
        assert!(json.contains("\"temperature\":0.3") || json.contains("\"temperature\": 0.3"));
    }

    /// Verifies that empty input is rejected before hitting the network.
    #[tokio::test]
    async fn test_cleanup_empty_input_returns_error() {
        let client = DeepSeekCleanup::new("dummy-key");
        let result = client.cleanup("   ", CleanupStyle::Polished, None, None).await;
        assert!(
            matches!(result, Err(LlmError::EmptyInput)),
            "expected EmptyInput error, got: {result:?}"
        );
    }

    /// Verifies that `CleanupResult` exposes the expected fields.
    #[test]
    fn test_cleanup_result_fields() {
        let r = CleanupResult {
            text: "Hello world".to_string(),
            prompt_tokens: Some(10),
            completion_tokens: Some(5),
        };
        assert_eq!(r.text, "Hello world");
        assert_eq!(r.prompt_tokens, Some(10));
        assert_eq!(r.completion_tokens, Some(5));
    }

    /// Verifies that `CleanupResult` can be constructed without token info.
    #[test]
    fn test_cleanup_result_no_tokens() {
        let r = CleanupResult {
            text: "text".to_string(),
            prompt_tokens: None,
            completion_tokens: None,
        };
        assert!(r.prompt_tokens.is_none());
        assert!(r.completion_tokens.is_none());
    }

    /// Verifies that CleanupStyle serializes correctly (used for Tauri commands).
    #[test]
    fn test_cleanup_style_serialization() {
        let polished = serde_json::to_string(&CleanupStyle::Polished).unwrap();
        let verbatim = serde_json::to_string(&CleanupStyle::Verbatim).unwrap();
        let chat = serde_json::to_string(&CleanupStyle::Chat).unwrap();

        assert_eq!(polished, r#""polished""#);
        assert_eq!(verbatim, r#""verbatim""#);
        assert_eq!(chat, r#""chat""#);
    }

    #[test]
    fn test_cleanup_style_chat_ignores_dictionary() {
        let style = CleanupStyle::Chat;
        let prompt_with = style.system_prompt(Some("Kubernetes"), None);
        let prompt_without = style.system_prompt(None, None);
        // Chat style intentionally omits dictionary context to keep prompts short
        assert_eq!(
            prompt_with, prompt_without,
            "Chat style should ignore dictionary terms"
        );
    }

    /// Custom prompt is appended to the system prompt when non-empty.
    #[test]
    fn test_system_prompt_with_custom_prompt() {
        let style = CleanupStyle::Polished;
        let prompt = style.system_prompt(None, Some("Always use formal German."));
        assert!(
            prompt.contains("Additional user instructions: Always use formal German."),
            "Custom prompt should be appended to the system prompt"
        );
    }

    /// Empty or whitespace-only custom prompt is not appended.
    #[test]
    fn test_system_prompt_empty_custom_prompt_is_ignored() {
        let style = CleanupStyle::Polished;
        let with_empty = style.system_prompt(None, Some("   "));
        let without = style.system_prompt(None, None);
        assert_eq!(
            with_empty, without,
            "Whitespace-only custom prompt should not change the system prompt"
        );
    }

    /// Custom prompt works for Chat style too.
    #[test]
    fn test_system_prompt_chat_with_custom_prompt() {
        let style = CleanupStyle::Chat;
        let prompt = style.system_prompt(None, Some("No emojis please."));
        assert!(
            prompt.contains("Additional user instructions: No emojis please."),
            "Chat style should include custom prompt"
        );
    }

    /// Both dictionary terms and custom prompt appear together.
    #[test]
    fn test_system_prompt_dict_and_custom_prompt() {
        let style = CleanupStyle::Verbatim;
        let prompt = style.system_prompt(Some("Kubernetes"), Some("Use bullet points."));
        assert!(
            prompt.contains("Kubernetes"),
            "Dictionary terms should be present"
        );
        assert!(
            prompt.contains("Additional user instructions: Use bullet points."),
            "Custom prompt should be present"
        );
    }

    /// build_request passes custom_prompt through to the system prompt.
    #[test]
    fn test_build_request_with_custom_prompt() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request(
            "some text",
            CleanupStyle::Polished,
            None,
            Some("Always use Sie-form in German."),
        );
        assert!(
            req.messages[0]
                .content
                .contains("Always use Sie-form in German."),
            "Custom prompt should appear in system message"
        );
    }

    // --- OpenAiCleanup tests ---

    #[test]
    fn test_openai_cleanup_new_stores_api_key() {
        let client = OpenAiCleanup::new("sk-openai-test-key");
        assert_eq!(client.api_key(), "sk-openai-test-key");
        assert_eq!(client.inner.model, OpenAiCleanup::DEFAULT_MODEL);
    }

    #[test]
    fn test_openai_cleanup_default_model_is_gpt4o_mini() {
        let client = OpenAiCleanup::new("key");
        assert_eq!(client.inner.model, "gpt-4o-mini");
    }

    #[test]
    fn test_openai_cleanup_build_request_correct_model() {
        let client = OpenAiCleanup::new("key");
        let req = client.build_request("hello", CleanupStyle::Polished, None, None);
        assert_eq!(req.model, "gpt-4o-mini");
    }

    #[test]
    fn test_openai_cleanup_build_request_serializes_to_json() {
        let client = OpenAiCleanup::new("key");
        let req = client.build_request("hello world", CleanupStyle::Chat, None, None);
        let json = serde_json::to_string(&req).expect("serialization should succeed");
        assert!(json.contains("gpt-4o-mini"));
        assert!(json.contains("hello world"));
    }

    #[tokio::test]
    async fn test_openai_cleanup_empty_input_returns_error() {
        let client = OpenAiCleanup::new("dummy-key");
        let result = client.cleanup("  ", CleanupStyle::Polished, None, None).await;
        assert!(matches!(result, Err(LlmError::EmptyInput)));
    }

    #[tokio::test]
    async fn test_openai_cleanup_rewrite_empty_input_returns_error() {
        let client = OpenAiCleanup::new("dummy-key");
        let result = client.rewrite("", "make it shorter").await;
        assert!(matches!(result, Err(LlmError::EmptyInput)));
    }

    // --- GroqCleanup tests ---

    #[test]
    fn test_groq_cleanup_new_stores_api_key() {
        let client = GroqCleanup::new("gsk-groq-test-key");
        assert_eq!(client.api_key(), "gsk-groq-test-key");
        assert_eq!(client.inner.model, GroqCleanup::DEFAULT_MODEL);
    }

    #[test]
    fn test_groq_cleanup_default_model_is_llama() {
        let client = GroqCleanup::new("key");
        assert_eq!(client.inner.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_groq_cleanup_build_request_correct_model() {
        let client = GroqCleanup::new("key");
        let req = client.build_request("hello", CleanupStyle::Polished, None, None);
        assert_eq!(req.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_groq_cleanup_build_request_serializes_to_json() {
        let client = GroqCleanup::new("key");
        let req = client.build_request("test text", CleanupStyle::Verbatim, None, None);
        let json = serde_json::to_string(&req).expect("serialization should succeed");
        assert!(json.contains("llama-3.3-70b-versatile"));
        assert!(json.contains("test text"));
    }

    #[tokio::test]
    async fn test_groq_cleanup_empty_input_returns_error() {
        let client = GroqCleanup::new("dummy-key");
        let result = client.cleanup("", CleanupStyle::Chat, None, None).await;
        assert!(matches!(result, Err(LlmError::EmptyInput)));
    }

    // --- AnthropicCleanup tests ---

    #[test]
    fn test_anthropic_cleanup_new_stores_api_key() {
        let client = AnthropicCleanup::new("sk-ant-test-key");
        assert_eq!(client.api_key(), "sk-ant-test-key");
        assert_eq!(client.model, AnthropicCleanup::DEFAULT_MODEL);
    }

    #[test]
    fn test_anthropic_cleanup_default_model() {
        let client = AnthropicCleanup::new("key");
        assert_eq!(client.model, "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_anthropic_cleanup_build_request_has_system_field() {
        let client = AnthropicCleanup::new("key");
        let req = client.build_request("some text", CleanupStyle::Polished, None, None);
        // System is a top-level field, not a message
        assert!(!req.system.is_empty(), "system prompt should be non-empty");
        assert!(req.system.contains("filler words"), "Polished system prompt expected");
        assert_eq!(req.messages.len(), 1, "only the user message, no system message");
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[0].content, "some text");
    }

    #[test]
    fn test_anthropic_cleanup_build_request_correct_model() {
        let client = AnthropicCleanup::new("key");
        let req = client.build_request("text", CleanupStyle::Chat, None, None);
        assert_eq!(req.model, "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_anthropic_cleanup_build_request_serializes_to_json() {
        let client = AnthropicCleanup::new("key");
        let req = client.build_request("hello world", CleanupStyle::Polished, None, None);
        let json = serde_json::to_string(&req).expect("should serialize");
        assert!(json.contains("claude-haiku-4-5-20251001"));
        assert!(json.contains("hello world"));
        // System should be a top-level key, not inside messages
        assert!(json.contains("\"system\""));
    }

    #[test]
    fn test_anthropic_cleanup_build_request_with_dictionary() {
        let client = AnthropicCleanup::new("key");
        let req = client.build_request(
            "text",
            CleanupStyle::Polished,
            Some("Kubernetes, Rust"),
            None,
        );
        assert!(req.system.contains("Kubernetes, Rust"));
    }

    #[test]
    fn test_anthropic_cleanup_build_command_request() {
        let client = AnthropicCleanup::new("key");
        let req = client.build_command_request("Hello world", "make it formal");
        assert_eq!(req.messages[0].role, "user");
        assert!(req.messages[0].content.contains("Hello world"));
        assert!(req.messages[0].content.contains("make it formal"));
        assert!(req.system.contains("text editing assistant"));
    }

    #[tokio::test]
    async fn test_anthropic_cleanup_empty_input_returns_error() {
        let client = AnthropicCleanup::new("dummy-key");
        let result = client.cleanup("   ", CleanupStyle::Polished, None, None).await;
        assert!(matches!(result, Err(LlmError::EmptyInput)));
    }

    #[tokio::test]
    async fn test_anthropic_cleanup_rewrite_empty_returns_error() {
        let client = AnthropicCleanup::new("dummy-key");
        let result = client.rewrite("", "").await;
        assert!(matches!(result, Err(LlmError::EmptyInput)));
    }

    // --- CleanupProvider trait default rewrite ---

    /// A minimal provider that only implements cleanup and relies on the
    /// default rewrite() to verify the default returns an error.
    struct MinimalProvider;

    #[async_trait::async_trait]
    impl CleanupProvider for MinimalProvider {
        async fn cleanup(
            &self,
            _raw_text: &str,
            _style: CleanupStyle,
            _dictionary_terms: Option<&str>,
            _custom_prompt: Option<&str>,
        ) -> Result<CleanupResult, LlmError> {
            Ok(CleanupResult {
                text: "cleaned".to_string(),
                prompt_tokens: None,
                completion_tokens: None,
            })
        }
    }

    #[tokio::test]
    async fn test_default_rewrite_returns_not_implemented_error() {
        let provider = MinimalProvider;
        let result = provider.rewrite("some text", "make shorter").await;
        assert!(
            matches!(result, Err(LlmError::ResponseFormat(_))),
            "default rewrite() should return ResponseFormat error"
        );
    }

    // --- Command Mode system prompt ---

    #[test]
    fn test_command_mode_system_prompt_mentions_text_editing() {
        let prompt = CleanupStyle::command_mode_system_prompt();
        assert!(prompt.contains("text editing assistant"));
        assert!(prompt.contains("voice command"));
    }
}
