use std::sync::Mutex as SyncMutex;
use std::time::Duration;

use serde_json::json;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Thin wiremock wrapper that simulates the Groq Whisper transcription endpoint.
///
/// All setup methods are async. Call [`Self::endpoint`] to get the URL to pass to
/// `Groq::new_with_client`. After [`Self::with_network_failure`] is called, `endpoint()`
/// returns a guaranteed-dead port URL suitable for connection-refused tests.
pub struct GroqMockServer {
    inner: Mutex<Option<MockServer>>,
    live_url: String,
    /// Set by `with_network_failure`; `endpoint()` returns this once populated.
    dead_url: SyncMutex<Option<String>>,
}

impl GroqMockServer {
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        let live_url = format!("{}/openai/v1/audio/transcriptions", server.uri());
        Self {
            inner: Mutex::new(Some(server)),
            live_url,
            dead_url: SyncMutex::new(None),
        }
    }

    /// Returns the transcription endpoint URL.
    ///
    /// After [`Self::with_network_failure`] is called, returns the dead-port URL instead of the
    /// live server URL.
    pub fn endpoint(&self) -> String {
        self.dead_url
            .lock()
            .unwrap()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.live_url.clone())
    }

    /// Mount a mock that returns HTTP 200 with `{"text": "<text>"}`.
    pub async fn with_success_response(&self, text: &str) {
        let guard = self.inner.lock().await;
        if let Some(server) = guard.as_ref() {
            Mock::given(method("POST"))
                .and(path("/openai/v1/audio/transcriptions"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({ "text": text })),
                )
                .mount(server)
                .await;
        }
    }

    /// Mount a mock that returns `code` with `body` as response body.
    pub async fn with_status(&self, code: u16, body: &str) {
        let guard = self.inner.lock().await;
        if let Some(server) = guard.as_ref() {
            Mock::given(method("POST"))
                .and(path("/openai/v1/audio/transcriptions"))
                .respond_with(ResponseTemplate::new(code).set_body_string(body))
                .mount(server)
                .await;
        }
    }

    /// Mount a mock that delays the response by `delay` before returning 200 + transcription text.
    pub async fn with_delayed_response(&self, text: &str, delay: Duration) {
        let guard = self.inner.lock().await;
        if let Some(server) = guard.as_ref() {
            Mock::given(method("POST"))
                .and(path("/openai/v1/audio/transcriptions"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_delay(delay)
                        .set_body_json(json!({ "text": text })),
                )
                .mount(server)
                .await;
        }
    }

    /// Simulate a network failure by pointing `endpoint()` to a guaranteed-dead port.
    ///
    /// Binds a fresh TCP listener on port 0 (OS assigns), records the address, immediately drops
    /// the listener (port closed), then stores the dead URL. After this call, `endpoint()` returns
    /// the dead-port URL. This avoids the timing race where wiremock's async shutdown completes
    /// after the test request arrives.
    pub async fn with_network_failure(&self) {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind temporary listener");
        let addr = listener.local_addr().expect("get local addr");
        drop(listener); // port is now closed

        let dead = format!("http://{}/openai/v1/audio/transcriptions", addr);
        *self.dead_url.lock().unwrap() = Some(dead);

        let mut guard = self.inner.lock().await;
        *guard = None;
    }
}
