#![cfg(target_os = "windows")]

use klarvo_core::output::FocusCapture;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

pub struct WinFocusCapture;

impl FocusCapture for WinFocusCapture {
    fn capture(&self) -> Option<u64> {
        // SAFETY: GetForegroundWindow has no preconditions and is safe to call from any thread.
        // Returns a null HWND when no window has foreground focus.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd.0 as usize as u64)
        }
    }

    fn restore(&self, handle: Option<u64>) {
        if let Some(h) = handle {
            if h != 0 {
                // SAFETY: h is an HWND pointer captured from GetForegroundWindow() round-tripped
                // through u64. SetForegroundWindow is safe to call from any thread; failure is
                // best-effort (window may no longer exist; foreground-lock may reject the call).
                let _ = unsafe {
                    SetForegroundWindow(HWND(h as usize as *mut core::ffi::c_void))
                };
            }
        }
    }
}
