use async_trait::async_trait;

use klarvo_core::PluginError;
use klarvo_core::traits::CleanupStyle;

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
impl CleanupStyle for Verbatim {
    async fn apply(&self, input: &str) -> Result<String, PluginError> {
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn identity_preserves_empty_string() {
        let v = Verbatim::new();
        assert_eq!(v.apply("").await.unwrap(), "");
    }

    #[tokio::test]
    async fn identity_preserves_plain_ascii() {
        let v = Verbatim::new();
        assert_eq!(v.apply("hello world").await.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn identity_preserves_filler_words() {
        let v = Verbatim::new();
        let input = "äh also ähm you know um das ist halt so";
        assert_eq!(v.apply(input).await.unwrap(), input);
    }

    #[tokio::test]
    async fn identity_preserves_multiline_and_whitespace() {
        let v = Verbatim::new();
        let input = "  line one\n\nline two\t\tindented\n";
        assert_eq!(v.apply(input).await.unwrap(), input);
    }

    #[tokio::test]
    async fn identity_preserves_unicode_and_punctuation() {
        let v = Verbatim::new();
        let input = "Größe: 42°C — „Zitat\" — 🎙️ déjà vu";
        assert_eq!(v.apply(input).await.unwrap(), input);
    }
}
