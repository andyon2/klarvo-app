//! LLM-based text cleanup module.
//!
//! Defines the `CleanupProvider` trait and the `DeepSeekCleanup` implementation
//! that calls the DeepSeek Chat API (OpenAI-compatible).
//!
//! API docs: <https://platform.deepseek.com/api-docs>

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
#[async_trait::async_trait]
pub trait CleanupProvider: Send + Sync {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError>;
}

// ---------------------------------------------------------------------------
// DeepSeek request / response types
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

/// Token usage info -- exposed so callers can log costs if desired.
#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_cache_hit_tokens: Option<u32>,
    pub prompt_cache_miss_tokens: Option<u32>,
}

/// Error response from DeepSeek.
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// DeepSeekCleanup
// ---------------------------------------------------------------------------

/// DeepSeek Chat API client for text cleanup.
///
/// Uses `deepseek-chat` (DeepSeek-V3) at temperature 0.3 -- low enough for
/// faithful cleanup, high enough to avoid robotic output.
pub struct DeepSeekCleanup {
    api_key: String,
    client: reqwest::Client,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl DeepSeekCleanup {
    const BASE_URL: &'static str = "https://api.deepseek.com/v1/chat/completions";
    const DEFAULT_MODEL: &'static str = "deepseek-chat";
    const DEFAULT_TEMPERATURE: f32 = 0.3;
    const DEFAULT_MAX_TOKENS: u32 = 2048;

    /// Creates a new `DeepSeekCleanup` client with the given API key.
    ///
    /// The API key should come from the caller (environment variable or
    /// system keystore) -- never hard-coded.
    pub fn new(api_key: impl Into<String>) -> Self {
        DeepSeekCleanup {
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

    /// Builds a request for Command Mode: rewrite `selected_text` based on `voice_command`.
    fn build_command_request<'a>(
        &'a self,
        selected_text: &str,
        voice_command: &str,
    ) -> ChatRequest<'a> {
        let system_prompt = "You are a text editing assistant. The user has selected some text \
            and will give you a voice command describing how to change it.\n\
            - Apply the command to the selected text\n\
            - Common commands: make shorter, make longer, rephrase, make formal, make casual, \
              translate to English/German, fix grammar, turn into a list, summarize\n\
            - Preserve the language of the original text unless the command explicitly asks \
              for translation\n\
            - Return ONLY the rewritten text, no explanations or commentary\n\
            - If you don't understand the command, return the original text unchanged".to_string();

        ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: format!("Selected text:\n{selected_text}\n\nCommand: {voice_command}"),
                },
            ],
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }

    /// Command Mode: rewrites `selected_text` based on a voice `command`.
    pub async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        if selected_text.trim().is_empty() || voice_command.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let request_body = self.build_command_request(selected_text, voice_command);

        let response = self
            .client
            .post(Self::BASE_URL)
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body)
                .map(|e| e.error.message)
                .unwrap_or(body);
            return Err(LlmError::ApiError { status: status_code, message });
        }

        let result: ChatResponse = response.json().await?;
        let choice = result.choices.into_iter().next().ok_or_else(|| {
            LlmError::ResponseFormat("no choices in response".to_string())
        })?;

        Ok(CleanupResult {
            text: choice.message.content.trim().to_string(),
            prompt_tokens: result.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: result.usage.as_ref().map(|u| u.completion_tokens),
        })
    }

    /// Builds the JSON request body.
    ///
    /// Extracted so it can be tested without a network connection.
    fn build_request<'a>(
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
}

#[async_trait::async_trait]
impl CleanupProvider for DeepSeekCleanup {
    /// Sends raw transcription text to DeepSeek and returns a `CleanupResult`
    /// containing the cleaned-up text plus token usage for cost tracking.
    ///
    /// # Errors
    /// - `LlmError::EmptyInput` -- `raw_text` is blank.
    /// - `LlmError::Request` -- network or serialization failure.
    /// - `LlmError::ApiError` -- the API returned a non-2xx status.
    /// - `LlmError::OutputTruncated` -- `finish_reason` was `"length"`.
    /// - `LlmError::ResponseFormat` -- the response JSON was unexpected.
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

        let request_body = self.build_request(raw_text, style, dictionary_terms, custom_prompt);

        let response = self
            .client
            .post(Self::BASE_URL)
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body)
                .map(|e| e.error.message)
                .unwrap_or(body);
            return Err(LlmError::ApiError {
                status: status_code,
                message,
            });
        }

        let api_response: ChatResponse = response.json().await?;

        // Extract token usage before consuming choices.
        let (prompt_tokens, completion_tokens) = api_response
            .usage
            .map(|u| (Some(u.prompt_tokens), Some(u.completion_tokens)))
            .unwrap_or((None, None));

        let choice = api_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ResponseFormat("No choices in response".to_string()))?;

        // Warn caller if the output was cut off
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_cleanup_new_stores_api_key() {
        let client = DeepSeekCleanup::new("sk-test-key");
        assert_eq!(client.api_key, "sk-test-key");
        assert_eq!(client.model, DeepSeekCleanup::DEFAULT_MODEL);
        assert_eq!(client.temperature, DeepSeekCleanup::DEFAULT_TEMPERATURE);
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
}
