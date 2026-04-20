use async_trait::async_trait;

use klarvo_core::error::AppError;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::{CleanupInput, CleanupStyle};

pub struct Verbatim;

impl Verbatim {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Verbatim {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineStage for Verbatim {
    type Input = CleanupInput;
    type Output = String;

    async fn process(&self, input: CleanupInput) -> Result<String, AppError> {
        Ok(input.raw)
    }

    fn stage_type(&self) -> &'static str {
        "cleanup"
    }
}

#[async_trait]
impl CleanupStyle for Verbatim {}

#[cfg(test)]
mod tests {
    use super::*;
    use klarvo_core::traits::CleanupInput;

    #[tokio::test]
    async fn identity_preserves_empty_string() {
        let v = Verbatim::new();
        assert_eq!(v.apply(CleanupInput::from_raw(String::new())).await.unwrap(), "");
    }

    #[tokio::test]
    async fn identity_preserves_plain_ascii() {
        let v = Verbatim::new();
        let input = CleanupInput::from_raw("hello world".to_string());
        assert_eq!(v.apply(input).await.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn identity_preserves_filler_words() {
        let v = Verbatim::new();
        let raw = "äh also ähm you know um das ist halt so".to_string();
        let result = v.apply(CleanupInput::from_raw(raw.clone())).await.unwrap();
        assert_eq!(result, raw);
    }

    #[tokio::test]
    async fn identity_preserves_multiline_and_whitespace() {
        let v = Verbatim::new();
        let raw = "  line one\n\nline two\t\tindented\n".to_string();
        let result = v.apply(CleanupInput::from_raw(raw.clone())).await.unwrap();
        assert_eq!(result, raw);
    }

    #[tokio::test]
    async fn identity_preserves_unicode_and_punctuation() {
        let v = Verbatim::new();
        let raw = "Größe: 42°C — „Zitat\" — 🎙️ déjà vu".to_string();
        let result = v.apply(CleanupInput::from_raw(raw.clone())).await.unwrap();
        assert_eq!(result, raw);
    }
}
