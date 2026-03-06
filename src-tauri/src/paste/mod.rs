//! Text paste module.
//!
//! Provides a platform-abstracted [`PasteHandler`] trait for copying text to
//! the clipboard and simulating Ctrl+V to insert it into the focused input.
//!
//! ## Platform support
//!
//! | Platform | Clipboard | Key simulation |
//! |----------|-----------|----------------|
//! | Linux    | `arboard` | `xdotool`      |
//! | Windows  | `arboard` | TODO: `SendInput` Win32 API |
//!
//! The paste handler is synchronous -- clipboard writes and key simulation are
//! both fast, blocking operations with no benefit from async.

use std::time::Duration;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during clipboard write or key simulation.
#[derive(Debug, Error)]
pub enum PasteError {
    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Key simulation failed: {0}")]
    KeySimulation(String),

    #[error("Text is empty -- nothing to paste")]
    EmptyText,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over paste backends.
///
/// Implementations copy `text` to the system clipboard and then simulate
/// Ctrl+V (or the platform equivalent) to insert it into the focused field.
pub trait PasteHandler: Send + Sync {
    /// Copy `text` to the clipboard and simulate Ctrl+V in the focused window.
    fn paste(&self, text: &str) -> Result<(), PasteError>;
}

// ---------------------------------------------------------------------------
// Linux implementation (xdotool + arboard)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    /// Linux paste handler.
    ///
    /// Uses `arboard` for clipboard access and `xdotool` for key simulation.
    /// If `xdotool` is not installed, the text is still written to the
    /// clipboard and a warning is logged -- the user can paste manually.
    pub struct LinuxPasteHandler;

    impl PasteHandler for LinuxPasteHandler {
        fn paste(&self, text: &str) -> Result<(), PasteError> {
            if text.is_empty() {
                return Err(PasteError::EmptyText);
            }

            // Write to clipboard first.
            set_clipboard(text)?;

            // Wait briefly so the clipboard write settles before key simulation.
            std::thread::sleep(Duration::from_millis(50));

            // Simulate Ctrl+V.  On failure we log a warning but do NOT fail --
            // the text is already in the clipboard and the user can paste manually.
            simulate_ctrl_v();

            Ok(())
        }
    }

    /// Writes `text` to the system clipboard using `arboard`.
    fn set_clipboard(text: &str) -> Result<(), PasteError> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| PasteError::Clipboard(e.to_string()))?;
        clipboard
            .set_text(text)
            .map_err(|e| PasteError::Clipboard(e.to_string()))
    }

    /// Simulates Ctrl+V via `xdotool`.
    ///
    /// Logs a warning on failure instead of returning an error, because the
    /// text is already in the clipboard and the user can paste manually.
    fn simulate_ctrl_v() {
        let result = std::process::Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                // Key simulated successfully.
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!(
                    "[paste] xdotool exited with non-zero status: {stderr}. \
                     Text is still in clipboard."
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!(
                    "[paste] xdotool not found. Install it with: \
                     sudo apt install xdotool\n\
                     Text is in the clipboard -- paste manually with Ctrl+V."
                );
            }
            Err(e) => {
                log::warn!(
                    "[paste] Failed to run xdotool: {e}. \
                     Text is still in clipboard."
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Windows implementation stub
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    /// Windows paste handler.
    ///
    /// Currently uses `arboard` for clipboard access.
    /// TODO: Implement `SendInput` Win32 API for key simulation instead of
    ///       relying on external tools.
    pub struct WindowsPasteHandler;

    impl PasteHandler for WindowsPasteHandler {
        fn paste(&self, text: &str) -> Result<(), PasteError> {
            if text.is_empty() {
                return Err(PasteError::EmptyText);
            }

            // Write to clipboard.
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| PasteError::Clipboard(e.to_string()))?;
            clipboard
                .set_text(text)
                .map_err(|e| PasteError::Clipboard(e.to_string()))?;

            // TODO: Simulate Ctrl+V via SendInput Win32 API.
            // For now we only set the clipboard; the user must paste manually.
            log::warn!(
                "[paste] Windows key simulation not yet implemented. \
                 Text is in clipboard -- paste manually with Ctrl+V."
            );

            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Public factory function -- returns the right handler for the current OS
// ---------------------------------------------------------------------------

/// Creates the platform-appropriate `PasteHandler`.
///
/// Returns a `Box<dyn PasteHandler>` so callers don't need to know the
/// concrete type.
pub fn create_paste_handler() -> Box<dyn PasteHandler> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPasteHandler)
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPasteHandler)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Box::new(FallbackPasteHandler)
    }
}

/// Fallback for unsupported platforms -- clipboard-only, no key simulation.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
struct FallbackPasteHandler;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl PasteHandler for FallbackPasteHandler {
    fn paste(&self, text: &str) -> Result<(), PasteError> {
        if text.is_empty() {
            return Err(PasteError::EmptyText);
        }

        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| PasteError::Clipboard(e.to_string()))?;
        clipboard
            .set_text(text)
            .map_err(|e| PasteError::Clipboard(e.to_string()))?;

        log::warn!(
            "[paste] Key simulation not implemented for this platform. \
             Text is in clipboard -- paste manually."
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty text must be rejected before any clipboard or OS call.
    #[test]
    fn test_paste_empty_text_returns_error() {
        let handler = create_paste_handler();
        let result = handler.paste("");
        assert!(
            matches!(result, Err(PasteError::EmptyText)),
            "expected EmptyText, got: {result:?}"
        );
    }

    /// PasteError formats to human-readable strings.
    #[test]
    fn test_paste_error_display() {
        let err = PasteError::EmptyText;
        assert!(
            err.to_string().contains("empty"),
            "EmptyText error should mention 'empty'"
        );

        let err = PasteError::Clipboard("test error".to_string());
        assert!(
            err.to_string().contains("test error"),
            "Clipboard error should include the source message"
        );

        let err = PasteError::KeySimulation("xdotool not found".to_string());
        assert!(
            err.to_string().contains("xdotool"),
            "KeySimulation error should include the source message"
        );
    }

    /// create_paste_handler returns a usable (non-crashing) handler.
    ///
    /// We cannot test the actual clipboard/key-simulation in CI (no display),
    /// but we can verify that the factory function compiles and returns
    /// something that correctly rejects empty input.
    #[test]
    fn test_create_paste_handler_rejects_empty() {
        let handler = create_paste_handler();
        assert!(handler.paste("").is_err());
    }
}
