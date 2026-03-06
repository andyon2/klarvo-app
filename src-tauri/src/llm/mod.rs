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
    /// Minimal cleanup: punctuation + capitalization only, keep speech patterns.
    Verbatim,
    /// Chat-ready: short, casual, emojis allowed.
    Chat,
}

impl CleanupStyle {
    /// Returns the system prompt for this cleanup style.
    ///
    /// `dictionary_terms` is an optional comma-separated list of custom terms
    /// the LLM should preserve verbatim (from the user's dictionary).
    pub fn system_prompt(&self, dictionary_terms: Option<&str>) -> String {
        let dict_section = match dictionary_terms {
            Some(terms) if !terms.is_empty() => {
                format!("\n\nThe user's custom dictionary terms (preserve these exactly): {terms}")
            }
            _ => String::new(),
        };

        match self {
            CleanupStyle::Polished => format!(
                "You are a text cleanup assistant. The user will give you raw speech-to-text \
                output. Clean it up:\n\
                - Remove filler words (um, uh, like, you know / äh, ähm, also)\n\
                - Remove false starts and self-corrections (keep only the final version)\n\
                - Fix grammar and punctuation\n\
                - Format professionally (proper capitalization, paragraphs where appropriate)\n\
                - Preserve the speaker's meaning exactly -- do not add or change content\n\
                - Language: respond in the same language as the input\
                {dict_section}"
            ),
            CleanupStyle::Verbatim => format!(
                "You are a text cleanup assistant. The user will give you raw speech-to-text \
                output. Minimal cleanup only:\n\
                - Add punctuation and capitalization\n\
                - Fix obvious transcription errors\n\
                - Keep filler words and speech patterns intact\n\
                - Language: respond in the same language as the input\
                {dict_section}"
            ),
            CleanupStyle::Chat => {
                // Chat style has no dictionary context -- keeps it short
                "You are a text cleanup assistant. The user will give you raw speech-to-text \
                output. Make it chat-ready:\n\
                - Remove all filler words\n\
                - Make it concise and casual\n\
                - Keep it short -- this is for messaging apps\n\
                - Emojis are okay if they fit naturally\n\
                - Language: respond in the same language as the input"
                    .to_string()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over LLM text-cleanup backends.
///
/// Implementations receive raw transcription text and return cleaned-up text.
#[async_trait::async_trait]
pub trait CleanupProvider: Send + Sync {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
    ) -> Result<String, LlmError>;
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
    #[allow(dead_code)]
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

    /// Builds the JSON request body.
    ///
    /// Extracted so it can be tested without a network connection.
    fn build_request<'a>(
        &'a self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
    ) -> ChatRequest<'a> {
        let system_prompt = style.system_prompt(dictionary_terms);

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
    /// Sends raw transcription text to DeepSeek and returns the cleaned-up version.
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
    ) -> Result<String, LlmError> {
        if raw_text.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let request_body = self.build_request(raw_text, style, dictionary_terms);

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

        let result: ChatResponse = response.json().await?;

        let choice = result
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

        Ok(content)
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
        let req = client.build_request("hello world", CleanupStyle::Polished, None);

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
        let req = client.build_request("test", CleanupStyle::Verbatim, None);
        assert!(
            req.messages[0].content.contains("Minimal cleanup"),
            "Verbatim prompt should say 'Minimal cleanup'"
        );
    }

    #[test]
    fn test_build_request_chat_style() {
        let client = DeepSeekCleanup::new("key");
        let req = client.build_request("test", CleanupStyle::Chat, None);
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
        let req = client.build_request("some text", CleanupStyle::Polished, None);
        let json = serde_json::to_string(&req).expect("should serialize to JSON");
        assert!(json.contains("deepseek-chat"));
        assert!(json.contains("some text"));
        assert!(json.contains("\"temperature\":0.3") || json.contains("\"temperature\": 0.3"));
    }

    /// Verifies that empty input is rejected before hitting the network.
    #[tokio::test]
    async fn test_cleanup_empty_input_returns_error() {
        let client = DeepSeekCleanup::new("dummy-key");
        let result = client.cleanup("   ", CleanupStyle::Polished, None).await;
        assert!(
            matches!(result, Err(LlmError::EmptyInput)),
            "expected EmptyInput error, got: {result:?}"
        );
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
        let prompt_with = style.system_prompt(Some("Kubernetes"));
        let prompt_without = style.system_prompt(None);
        // Chat style intentionally omits dictionary context to keep prompts short
        assert_eq!(
            prompt_with, prompt_without,
            "Chat style should ignore dictionary terms"
        );
    }
}
