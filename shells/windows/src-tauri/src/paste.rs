#![cfg(target_os = "windows")]
// Windows-only: depends on Win32_UI_Input_KeyboardAndMouse, no non-Windows path.

use async_trait::async_trait;
use klarvo_core::{
    error::{AppError, AppErrorKind},
    output::PasteBackend,
};
use windows::Win32::Foundation::GetLastError;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput,
    VIRTUAL_KEY, VK_CONTROL, VK_V,
};

/// Windows Ctrl+V paste implementation via Win32 `SendInput`.
///
/// Stateless: `SendInput` requires no persistent OS handle or lifecycle resource.
pub struct WinSendInputPasteBackend;

/// Internal injection abstraction — allows headless unit-testing of the VK sequence
/// without real OS input injection.
trait InputSender {
    fn send_inputs(&self, inputs: &[INPUT]) -> u32;
}

struct RealInputSender;

impl InputSender for RealInputSender {
    fn send_inputs(&self, inputs: &[INPUT]) -> u32 {
        // SAFETY:
        // 1. Array-Pointer + Länge: `inputs.len()` und `inputs.as_ptr()` sind konsistent;
        //    `inputs` ist Stack-alloziert und lebt für die Dauer des SendInput-Calls.
        // 2. KEYBDINPUT-Struct-Felder sind valide: VK_CONTROL (0x11) und VK_V (0x56)
        //    sind bekannte, stabile Virtual-Key-Codes im gültigen u16-Range.
        // 3. Kein shared-mutable-State: WinSendInputPasteBackend ist stateless;
        //    concurrent calls produzieren unabhängige Key-Sequences (OS-serialized).
        unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) }
    }
}

fn make_key_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn paste_impl(sender: &dyn InputSender) -> Result<(), AppError> {
    let inputs = [
        make_key_input(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        make_key_input(VK_V, KEYBD_EVENT_FLAGS(0)),
        make_key_input(VK_V, KEYEVENTF_KEYUP),
        make_key_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let rc = sender.send_inputs(&inputs);
    if rc == 4 {
        Ok(())
    } else {
        let last_error = unsafe { GetLastError() };
        Err(AppError {
            kind: AppErrorKind::Io,
            message: format!(
                "SendInput returned {rc} (expected 4); GetLastError: {}",
                last_error.0
            ),
            user_message: Some("error.paste.send_input_failed".to_string()),
            retryable: false,
        })
    }
}

#[async_trait]
impl PasteBackend for WinSendInputPasteBackend {
    /// Triggers Ctrl+V injection via Win32 `SendInput`.
    ///
    /// Direct-call (no `spawn_blocking`): `SendInput` for 4 KEY-events is <100μs;
    /// `spawn_blocking` thread-pool context-switch overhead (10–50μs) exceeds the gain.
    /// Phase-2 revisit: if paste() causes perceivable blocking (measured >500μs), migrate to
    /// spawn_blocking. SendInput for 4 events is consistently <100μs on WASAPI-idle desktop.
    ///
    /// Lazy-assumption: no `GetForegroundWindow` pre-check. `SendInput` returns 0 when no
    /// window is focused — falls into the Io error path (AC-D rationale).
    async fn paste(&self) -> Result<(), AppError> {
        paste_impl(&RealInputSender)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::Arc;

    struct MockInputSender {
        captured: RefCell<Vec<Vec<INPUT>>>,
        return_value: u32,
    }

    impl MockInputSender {
        fn new(return_value: u32) -> Self {
            Self {
                captured: RefCell::new(Vec::new()),
                return_value,
            }
        }
    }

    impl InputSender for MockInputSender {
        fn send_inputs(&self, inputs: &[INPUT]) -> u32 {
            self.captured.borrow_mut().push(inputs.to_vec());
            self.return_value
        }
    }

    #[test]
    fn ctrl_v_sequence_correct() {
        let mock = MockInputSender::new(4);
        let result = paste_impl(&mock);
        assert!(result.is_ok());
        let calls = mock.captured.borrow();
        assert_eq!(calls.len(), 1);
        let inputs = &calls[0];
        assert_eq!(inputs.len(), 4);
        unsafe {
            // VK_CONTROL down
            assert_eq!(inputs[0].Anonymous.ki.wVk, VK_CONTROL);
            assert_eq!(inputs[0].Anonymous.ki.dwFlags, KEYBD_EVENT_FLAGS(0));
            // VK_V down
            assert_eq!(inputs[1].Anonymous.ki.wVk, VK_V);
            assert_eq!(inputs[1].Anonymous.ki.dwFlags, KEYBD_EVENT_FLAGS(0));
            // VK_V up
            assert_eq!(inputs[2].Anonymous.ki.wVk, VK_V);
            assert_eq!(inputs[2].Anonymous.ki.dwFlags, KEYEVENTF_KEYUP);
            // VK_CONTROL up
            assert_eq!(inputs[3].Anonymous.ki.wVk, VK_CONTROL);
            assert_eq!(inputs[3].Anonymous.ki.dwFlags, KEYEVENTF_KEYUP);
        }
    }

    #[test]
    fn error_on_partial_return() {
        let mock = MockInputSender::new(0);
        let result = paste_impl(&mock);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Io));
        assert_eq!(err.user_message.as_deref(), Some("error.paste.send_input_failed"));
        assert!(!err.retryable);
    }

    #[test]
    fn paste_backend_arc_compatible() {
        // Compile-time check: Arc<dyn PasteBackend> accepts WinSendInputPasteBackend.
        let _: Arc<dyn PasteBackend> = Arc::new(WinSendInputPasteBackend);
    }
}
