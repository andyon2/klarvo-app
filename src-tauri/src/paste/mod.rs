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
    use ::windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_V,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, SetForegroundWindow,
    };

    /// Windows paste handler using Win32 SendInput API.
    ///
    /// Uses `arboard` for clipboard access and `SendInput` for Ctrl+V simulation.
    /// Optionally restores focus to a previously active window before pasting.
    pub struct WindowsPasteHandler {
        /// The window that was focused before Dikta started recording.
        /// If set, focus is restored to this window before simulating Ctrl+V.
        prev_hwnd: Option<isize>,
    }

    impl WindowsPasteHandler {
        pub fn new(prev_hwnd: Option<isize>) -> Self {
            Self { prev_hwnd }
        }
    }

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

            // Restore focus to the previously active window.
            if let Some(hwnd_raw) = self.prev_hwnd {
                unsafe {
                    use ::windows::Win32::Foundation::HWND;
                    let hwnd = HWND(hwnd_raw as *mut _);
                    let current = GetForegroundWindow();
                    if current != hwnd {
                        let _ = SetForegroundWindow(hwnd);
                        // Give the OS time to switch focus.
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            // Small delay to ensure clipboard is ready.
            std::thread::sleep(Duration::from_millis(50));

            // Simulate Ctrl+V via SendInput.
            simulate_ctrl_v();

            Ok(())
        }
    }

    /// Builds a keyboard INPUT event for SendInput.
    fn kbd_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if key_up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// Simulates Ctrl+V using the Win32 SendInput API.
    fn simulate_ctrl_v() {
        let inputs = [
            kbd_input(VK_CONTROL, false), // Ctrl down
            kbd_input(VK_V, false),       // V down
            kbd_input(VK_V, true),        // V up
            kbd_input(VK_CONTROL, true),  // Ctrl up
        ];

        unsafe {
            let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if sent != inputs.len() as u32 {
                log::warn!("[paste] SendInput returned {sent}, expected {}", inputs.len());
            }
        }
    }

    /// Captures the currently focused window handle (HWND) as a raw isize.
    /// Call this when the hotkey fires, before Dikta does any processing.
    pub fn get_foreground_window_handle() -> isize {
        unsafe { GetForegroundWindow().0 as isize }
    }

    /// Returns the title (caption) of the currently focused window.
    pub fn get_foreground_window_title() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() { return None; }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len == 0 { return None; }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }
}

// ---------------------------------------------------------------------------
// Public factory function -- returns the right handler for the current OS
// ---------------------------------------------------------------------------

/// Creates the platform-appropriate `PasteHandler`.
///
/// `prev_hwnd` is the handle of the window that was focused before Dikta
/// started recording. On Windows this is used to restore focus before pasting.
/// On other platforms it is ignored.
pub fn create_paste_handler(prev_hwnd: Option<isize>) -> Box<dyn PasteHandler> {
    #[cfg(target_os = "linux")]
    {
        let _ = prev_hwnd;
        Box::new(linux::LinuxPasteHandler)
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPasteHandler::new(prev_hwnd))
    }

    #[cfg(target_os = "android")]
    {
        let _ = prev_hwnd;
        Box::new(AndroidPasteHandler)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
    {
        let _ = prev_hwnd;
        Box::new(FallbackPasteHandler)
    }
}

/// Captures the currently focused window handle. Call when the hotkey fires.
///
/// Returns `Some(hwnd)` on Windows, `None` on other platforms.
pub fn capture_foreground_window() -> Option<isize> {
    #[cfg(target_os = "windows")]
    {
        Some(windows::get_foreground_window_handle())
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Returns the title of the currently focused window.
pub fn capture_foreground_window_title() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        windows::get_foreground_window_title()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Fallback for unsupported desktop platforms -- clipboard-only, no key simulation.
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
struct FallbackPasteHandler;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
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

/// Android stub -- paste happens via InputConnection in the Kotlin IME.
#[cfg(target_os = "android")]
struct AndroidPasteHandler;

#[cfg(target_os = "android")]
impl PasteHandler for AndroidPasteHandler {
    fn paste(&self, text: &str) -> Result<(), PasteError> {
        if text.is_empty() {
            return Err(PasteError::EmptyText);
        }
        log::info!("[paste] Android: text ready ({} chars), IME handles insertion", text.len());
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
        let handler = create_paste_handler(None);
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
        let handler = create_paste_handler(None);
        assert!(handler.paste("").is_err());
    }
}
