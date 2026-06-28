//! Native Win32 layered-window pill overlay.
//!
//! Replaces the WebView2 "bar" window with a `WS_EX_LAYERED | WS_EX_TOPMOST`
//! window that stays fully composited even when a foreground app covers it.
//! Content rendered in-process via a tiny-skia Pixmap (shapes) + GDI (text),
//! then presented with `UpdateLayeredWindow(ULW_ALPHA)`.
//!
//! ## Threading
//! The window and its message loop run on a dedicated OS thread.
//! The public [`NativePill`] handle communicates via `PostMessageW`.
//!
//! ## Rendering
//! 1. Tiny-skia draws all shapes into a premultiplied RGBA [`Pixmap`].
//! 2. The Pixmap is converted RGBA→BGRA and written into the Win32 DIB section.
//! 3. GDI text is drawn (white-on-black) into a temp DIB, then alpha-composited
//!    (coverage from the B channel) onto the BGRA DIB.
//! 4. `UpdateLayeredWindow` presents the final BGRA DIB to the DWM.

#![cfg(target_os = "windows")]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::mem::size_of;
use std::sync::mpsc;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tiny_skia::{Color, FillRule, LineCap, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

// ---------------------------------------------------------------------------
// Dimensions & layout constants (logical pixels, scale-independent)
// ---------------------------------------------------------------------------
const PILL_W: f32 = 200.0;
const PILL_H: f32 = 36.0;
const PAD: f32 = 10.0; // horizontal padding
const GAP: f32 = 6.0;  // gap between elements

// Logo: 24×24 rounded rect at left edge + padding
const LOGO_SIZE: f32 = 24.0;
const LOGO_X: f32 = PAD;
const LOGO_Y: f32 = (PILL_H - LOGO_SIZE) / 2.0; // 6.0

// Stop button: 14×14 outer, 8×8 inner (red square)
const STOP_X: f32 = LOGO_X + LOGO_SIZE + GAP;
const STOP_OUTER: f32 = 14.0;
const STOP_INNER: f32 = 8.0;
const STOP_Y: f32 = (PILL_H - STOP_OUTER) / 2.0;

// Waveform: 5 bars, 3px gap, flex after stop button
const WAVE_X: f32 = STOP_X + STOP_OUTER + GAP;
const WAVE_BARS: usize = 5;
const WAVE_H_MAX: f32 = 20.0;
const WAVE_BAR_W: f32 = 4.0; // approximate (flex fills remaining space)

// Spinner: 13×13 just after logo
const SPIN_X: f32 = LOGO_X + LOGO_SIZE + GAP;
const SPIN_SIZE: f32 = 13.0;
const SPIN_Y: f32 = (PILL_H - SPIN_SIZE) / 2.0;

// Label text X (after spinner or K or check)
const LABEL_X_AFTER_SPIN: f32 = SPIN_X + SPIN_SIZE + GAP;

// Check icon: 11×11
const CHECK_SIZE: f32 = 11.0;

// ---------------------------------------------------------------------------
// Custom messages (WM_APP range: 0x8000-0xBFFF)
// ---------------------------------------------------------------------------
const WM_PILL_SET_STATE: u32 = 0x8001; // WPARAM=state_code, LPARAM=clipboard_only
const WM_PILL_SET_RMS: u32 = 0x8002;   // WPARAM=f32::to_bits()
const WM_PILL_SET_MODE: u32 = 0x8003;  // WPARAM=ptr to Box<String> (caller Box::into_raw)
const WM_PILL_SHUTDOWN: u32 = 0x8010;  // Request orderly teardown; handler calls DestroyWindow

// Timer for spinner animation and done/error timeout
const TIMER_ANIMATE: usize = 1;
const TIMER_MS: u32 = 33; // ~30 fps

// Done flash durations (ms)
const DONE_NORMAL_MS: u128 = 1500;
const DONE_CLIPBOARD_MS: u128 = 4000;
const ERROR_IDLE_MS: u128 = 2500;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Local copy of pipeline state for the pill renderer.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NativePillState {
    Idle,
    Recording,
    Transcribing,
    Cleaning,
    Done,
    DoneClipboard,
    Error,
}

impl NativePillState {
    fn is_visible(self) -> bool {
        !matches!(self, NativePillState::Idle)
    }
    fn needs_animation(self) -> bool {
        matches!(
            self,
            NativePillState::Transcribing | NativePillState::Cleaning | NativePillState::Done | NativePillState::DoneClipboard | NativePillState::Error
        )
    }
    fn accent(self) -> (f32, f32, f32) {
        // (R, G, B) in 0.0-1.0
        match self {
            NativePillState::Recording => (42.0 / 255.0, 195.0 / 255.0, 168.0 / 255.0),
            NativePillState::Transcribing | NativePillState::Cleaning => {
                (255.0 / 255.0, 163.0 / 255.0, 68.0 / 255.0)
            }
            NativePillState::Done => (74.0 / 255.0, 222.0 / 255.0, 128.0 / 255.0),
            NativePillState::DoneClipboard => (255.0 / 255.0, 163.0 / 255.0, 68.0 / 255.0),
            NativePillState::Error => (255.0 / 255.0, 115.0 / 255.0, 105.0 / 255.0),
            NativePillState::Idle => (0.0, 0.0, 0.0),
        }
    }
    fn from_code(code: u8, clipboard_only: bool) -> Self {
        match code {
            1 => NativePillState::Recording,
            2 => NativePillState::Transcribing,
            3 => NativePillState::Cleaning,
            4 => {
                if clipboard_only {
                    NativePillState::DoneClipboard
                } else {
                    NativePillState::Done
                }
            }
            5 => NativePillState::Error,
            _ => NativePillState::Idle,
        }
    }
}

struct Drag {
    start_cur_x: i32,
    start_cur_y: i32,
    start_win_x: i32,
    start_win_y: i32,
}

/// Per-window data stored in GWLP_USERDATA.
struct PillWindowState {
    display: NativePillState,
    waveform: [f32; 20],
    waveform_pos: usize,
    spinner_deg: f32,
    done_at: Option<Instant>,
    error_at: Option<Instant>,
    drag: Option<Drag>,
    last_bar_moved_emit: Option<Instant>,
    // GDI resources
    main_dc: HDC,
    main_bmp: HBITMAP,
    main_bits: *mut core::ffi::c_void,
    tmp_dc: HDC,
    tmp_bmp: HBITMAP,
    tmp_bits: *mut core::ffi::c_void,
    font_k: HFONT,
    font_label: HFONT,
    font_mode: HFONT,
    font_label_lg: HFONT,
    // Geometry
    phys_w: i32,
    phys_h: i32,
    win_x: i32,
    win_y: i32,
    scale: f64,
    // Tauri
    app_handle: AppHandle,
    hotkey_mode: String,
    timer_active: bool,
    // Tracks the previous visibility so render_frame can gate HWND_TOPMOST
    // re-assertion to the hidden→visible edge rather than every frame (10-3 review).
    was_visible: bool,
}

// SAFETY: PillWindowState is only ever touched from the pill thread (WndProc).
// AppHandle is Clone+Send+Sync in Tauri 2.
unsafe impl Send for PillWindowState {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Handle to the native pill window. Cheap to clone (just an HWND + thread handle).
pub struct NativePill {
    hwnd: isize, // HWND stored as isize for Send
    _thread: std::thread::JoinHandle<()>,
}

// SAFETY: hwnd is only used via PostMessageW (thread-safe) and IsWindow (read-only).
unsafe impl Send for NativePill {}

impl NativePill {
    /// Spawn the pill window on a dedicated thread and return a handle.
    /// `saved_x` / `saved_y` are logical-pixel positions from config.
    pub fn create(
        app_handle: AppHandle,
        saved_x: Option<f64>,
        saved_y: Option<f64>,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Result<isize, String>>();
        let thread = std::thread::spawn(move || {
            pill_thread(app_handle, saved_x, saved_y, tx);
        });
        let hwnd = rx
            .recv()
            .map_err(|_| "pill thread died before sending HWND".to_string())??;
        Ok(NativePill { hwnd, _thread: thread })
    }

    /// Update pill state from a `PipelineState`.
    /// Skips `Warning` (transient toast — pill state unchanged per FloatingBar parity).
    pub fn set_state(
        &self,
        state: &crate::hotkey::PipelineState,
        clipboard_only: bool,
    ) {
        use crate::hotkey::PipelineState;
        let code: u8 = match state {
            PipelineState::Idle => 0,
            PipelineState::Recording => 1,
            PipelineState::Transcribing => 2,
            PipelineState::Cleaning => 3,
            PipelineState::Done => 4,
            PipelineState::Error => 5,
            PipelineState::Warning => return, // ignored
        };
        let lparam = if clipboard_only { 1isize } else { 0isize };
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PILL_SET_STATE,
                WPARAM(code as usize),
                LPARAM(lparam),
            );
        }
    }

    /// Feed an RMS level (~15 Hz from the audio thread).
    pub fn feed_rms(&self, level: f32) {
        let bits = level.to_bits() as usize;
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PILL_SET_RMS,
                WPARAM(bits),
                LPARAM(0),
            );
        }
    }

    /// Update the hotkey mode badge (e.g. "Hold", "Toggle").
    pub fn set_hotkey_mode(&self, mode: String) {
        let boxed = Box::into_raw(Box::new(mode));
        unsafe {
            // If PostMessage fails the box leaks (window may be gone) — acceptable.
            if PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PILL_SET_MODE,
                WPARAM(boxed as usize),
                LPARAM(0),
            )
            .is_err()
            {
                drop(Box::from_raw(boxed));
            }
        }
    }

    /// Check if the native pill window still exists.
    pub fn is_alive(&self) -> bool {
        unsafe { IsWindow(Some(HWND(self.hwnd as *mut _))).as_bool() }
    }
}

impl Drop for NativePill {
    fn drop(&mut self) {
        // Post a custom shutdown message; the WndProc calls DestroyWindow which
        // triggers the real WM_DESTROY + WM_NCDESTROY teardown and frees state.
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PILL_SHUTDOWN,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel helpers (BGRA premultiplied)
// ---------------------------------------------------------------------------

/// Write a premultiplied BGRA pixel to the main DIB at logical (x, y) × scale.
#[inline]
unsafe fn write_pixel_bgra(bits: *mut u8, w: i32, px: i32, py: i32, b: u8, g: u8, r: u8, a: u8) {
    if px < 0 || py < 0 { return; }
    let idx = (py * w + px) as usize * 4;
    let ptr = bits.add(idx);
    *ptr = b;
    *ptr.add(1) = g;
    *ptr.add(2) = r;
    *ptr.add(3) = a;
}

/// Porter-Duff "src over dst" blend in premultiplied BGRA space.
/// `src_*_pm` values are already premultiplied.
#[inline]
unsafe fn blend_over_bgra(
    bits: *mut u8,
    w: i32,
    px: i32,
    py: i32,
    src_pm_b: u8,
    src_pm_g: u8,
    src_pm_r: u8,
    src_a: u8,
) {
    if px < 0 || py < 0 { return; }
    let idx = (py * w + px) as usize * 4;
    let ptr = bits.add(idx);
    let dst_b = *ptr;
    let dst_g = *ptr.add(1);
    let dst_r = *ptr.add(2);
    let dst_a = *ptr.add(3);
    let inv = 255u32 - src_a as u32;
    *ptr        = (src_pm_b as u32 + dst_b as u32 * inv / 255).min(255) as u8;
    *ptr.add(1) = (src_pm_g as u32 + dst_g as u32 * inv / 255).min(255) as u8;
    *ptr.add(2) = (src_pm_r as u32 + dst_r as u32 * inv / 255).min(255) as u8;
    *ptr.add(3) = (src_a as u32 + dst_a as u32 * inv / 255).min(255) as u8;
}

// ---------------------------------------------------------------------------
// DIB creation
// ---------------------------------------------------------------------------

unsafe fn create_dib(
    w: i32,
    h: i32,
    bits_out: &mut *mut core::ffi::c_void,
) -> Result<(HDC, HBITMAP), String> {
    let screen_dc = GetDC(None);
    if screen_dc.is_invalid() {
        return Err("GetDC failed".into());
    }
    let dc = CreateCompatibleDC(Some(screen_dc));
    ReleaseDC(None, screen_dc);
    if dc.is_invalid() {
        return Err("CreateCompatibleDC failed".into());
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default()],
    };

    let bmp = CreateDIBSection(
        Some(dc),
        &bmi,
        DIB_RGB_COLORS,
        bits_out,
        None,
        0,
    )
    .map_err(|e| format!("CreateDIBSection failed: {e}"))?;

    SelectObject(dc, bmp.into());
    Ok((dc, bmp))
}

/// Register an in-memory font file (.ttf) with this process's GDI font table.
/// The font lives for the process lifetime (not removed — app-wide UI font).
unsafe fn load_embedded_font(bytes: &'static [u8]) {
    let mut num_fonts: u32 = 0;
    let h = AddFontMemResourceEx(
        bytes.as_ptr() as *const core::ffi::c_void,
        bytes.len() as u32,
        None,
        &mut num_fonts as *mut u32 as *const u32,
    );
    if h.is_invalid() {
        log::warn!("[native_pill] AddFontMemResourceEx failed — falling back to default font");
    }
}

unsafe fn create_font(name: PCWSTR, height_px: i32, weight: i32) -> HFONT {
    CreateFontW(
        -height_px, // negative = character height
        0, 0, 0,
        weight,
        0, 0, 0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        ANTIALIASED_QUALITY,
        (FF_DONTCARE.0 | VARIABLE_PITCH.0) as u32,
        name,
    )
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Copy tiny-skia RGBA pixmap into BGRA main DIB (swap R↔B).
unsafe fn copy_rgba_to_bgra(pixmap: &Pixmap, main_bits: *mut u8, byte_count: usize) {
    let src = pixmap.data();
    for i in 0..byte_count / 4 {
        let base = i * 4;
        let r = src[base];
        let g = src[base + 1];
        let b = src[base + 2];
        let a = src[base + 3];
        // BGRA layout
        *main_bits.add(base)     = b;
        *main_bits.add(base + 1) = g;
        *main_bits.add(base + 2) = r;
        *main_bits.add(base + 3) = a;
    }
}

/// Composite GDI-rendered white-on-black text from tmp_bits (BGRA) onto main_bits (BGRA).
/// `text_r/g/b` is the desired text color in straight-alpha.
unsafe fn composite_text_mask(
    tmp_bits: *const u8,
    main_bits: *mut u8,
    w: i32,
    h: i32,
    text_r: u8,
    text_g: u8,
    text_b: u8,
) {
    let total = (w * h) as usize;
    for i in 0..total {
        let base = i * 4;
        // tmp DIB is BGRA; GDI draws white text → B=255,G=255,R=255 at text pixels
        let coverage = *tmp_bits.add(base) as u32; // B channel = coverage
        if coverage == 0 {
            continue;
        }
        // Premultiply source
        let src_a = coverage as u8;
        let pm_b = (text_b as u32 * coverage / 255) as u8;
        let pm_g = (text_g as u32 * coverage / 255) as u8;
        let pm_r = (text_r as u32 * coverage / 255) as u8;
        // Blend over existing
        let inv = 255u32 - src_a as u32;
        let dst = main_bits.add(base);
        *dst        = (pm_b as u32 + *dst        as u32 * inv / 255).min(255) as u8;
        *dst.add(1) = (pm_g as u32 + *dst.add(1) as u32 * inv / 255).min(255) as u8;
        *dst.add(2) = (pm_r as u32 + *dst.add(2) as u32 * inv / 255).min(255) as u8;
        *dst.add(3) = (src_a as u32 + *dst.add(3) as u32 * inv / 255).min(255) as u8;
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Main render: build the frame and call UpdateLayeredWindow.
unsafe fn render_frame(hwnd: HWND, s: &mut PillWindowState) {
    let pw = s.phys_w;
    let ph = s.phys_h;
    let sc = s.scale as f32;

    // Hide window when idle
    if !s.display.is_visible() {
        ShowWindow(hwnd, SW_HIDE);
        s.was_visible = false;
        return;
    }

    // --- 1. tiny-skia shape rendering into RGBA pixmap ---
    let Some(mut pixmap) = Pixmap::new(pw as u32, ph as u32) else {
        log::warn!("[native_pill] Pixmap::new({pw},{ph}) failed — skipping frame");
        return;
    };

    let (ar, ag, ab) = s.display.accent();
    let accent_color = Color::from_rgba(ar, ag, ab, 1.0).unwrap_or(Color::WHITE);

    // Stadium background: rgba(25,25,25,0.96)
    {
        let r = ph as f32 / 2.0; // full-height radius → stadium
        let k = r * 0.5522847498_f32;
        let w = pw as f32;
        let h = ph as f32;
        let mut pb = PathBuilder::new();
        pb.move_to(r, 0.0);
        pb.line_to(w - r, 0.0);
        pb.cubic_to(w - r + k, 0.0, w, r - k, w, r);
        pb.cubic_to(w, r + k, w - r + k, h, w - r, h);
        pb.line_to(r, h);
        pb.cubic_to(r - k, h, 0.0, r + k, 0.0, r);
        pb.cubic_to(0.0, r - k, r - k, 0.0, r, 0.0);
        pb.close();
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.shader = Shader::SolidColor(
                Color::from_rgba(25.0/255.0, 25.0/255.0, 25.0/255.0, 0.96).unwrap(),
            );
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    // K logo background: teal rounded rect 24×24 at LOGO_X, LOGO_Y
    {
        let lx = LOGO_X * sc;
        let ly = LOGO_Y * sc;
        let ls = LOGO_SIZE * sc;
        let lr = 6.0 * sc; // corner radius
        let k = lr * 0.5522847498_f32;
        let mut pb = PathBuilder::new();
        pb.move_to(lx + lr, ly);
        pb.line_to(lx + ls - lr, ly);
        pb.cubic_to(lx + ls - lr + k, ly, lx + ls, ly + lr - k, lx + ls, ly + lr);
        pb.line_to(lx + ls, ly + ls - lr);
        pb.cubic_to(lx + ls, ly + ls - lr + k, lx + ls - lr + k, ly + ls, lx + ls - lr, ly + ls);
        pb.line_to(lx + lr, ly + ls);
        pb.cubic_to(lx + lr - k, ly + ls, lx, ly + ls - lr + k, lx, ly + ls - lr);
        pb.line_to(lx, ly + lr);
        pb.cubic_to(lx, ly + lr - k, lx + lr - k, ly, lx + lr, ly);
        pb.close();
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.shader = Shader::SolidColor(
                Color::from_rgba(20.0/255.0, 184.0/255.0, 166.0/255.0, 1.0).unwrap(),
            );
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    match s.display {
        NativePillState::Recording => {
            // Stop button inner square: rgba(248,113,113,0.9)
            {
                let ox = (STOP_X + (STOP_OUTER - STOP_INNER) / 2.0) * sc;
                let oy = (STOP_Y + (STOP_OUTER - STOP_INNER) / 2.0) * sc;
                let side = STOP_INNER * sc;
                if let Some(rect) = tiny_skia::Rect::from_xywh(ox, oy, side, side) {
                    let path = PathBuilder::from_rect(rect);
                    let mut paint = Paint::default();
                    paint.shader = Shader::SolidColor(
                        Color::from_rgba(248.0/255.0, 113.0/255.0, 113.0/255.0, 0.9).unwrap(),
                    );
                    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
            // Measure the mode-badge text so the waveform reserves space for it
            // on the right (SOLL: badge is flexShrink:0, the waveform flex:1 fills
            // only the space that remains). font_mode is already physical-scaled.
            let badge_w_phys = {
                SelectObject(s.tmp_dc, s.font_mode.into());
                let wide = to_wide(mode_label(&s.hotkey_mode));
                let slice = &wide[..wide.len().saturating_sub(1)]; // drop trailing NUL
                let mut size = SIZE { cx: 0, cy: 0 };
                if GetTextExtentPoint32W(s.tmp_dc, slice, &mut size).as_bool() {
                    size.cx as f32
                } else {
                    30.0 * sc
                }
            };
            // Waveform: 5 bars
            render_waveform(&mut pixmap, &s.waveform, s.waveform_pos, sc, badge_w_phys);
        }

        NativePillState::Transcribing | NativePillState::Cleaning => {
            // Spinner arc
            render_spinner(&mut pixmap, sc, s.spinner_deg, accent_color);
        }

        NativePillState::Done | NativePillState::DoneClipboard => {
            // Check mark (green) or clipboard box (amber)
            let check_x = (SPIN_X + (SPIN_SIZE - CHECK_SIZE) / 2.0) * sc;
            let check_y = (SPIN_Y + (SPIN_SIZE - CHECK_SIZE) / 2.0) * sc;
            if matches!(s.display, NativePillState::Done) {
                render_check(&mut pixmap, check_x, check_y, CHECK_SIZE * sc, accent_color);
            } else {
                // Amber clipboard: simple filled rect with white inner
                render_clipboard_icon(&mut pixmap, check_x, check_y, CHECK_SIZE * sc);
            }
        }

        NativePillState::Error | NativePillState::Idle => {}
    }

    // --- 2. Copy RGBA→BGRA into main DIB ---
    let byte_count = (pw * ph) as usize * 4;
    copy_rgba_to_bgra(&pixmap, s.main_bits as *mut u8, byte_count);

    // --- 3. GDI text compositing onto BGRA main DIB ---
    {
        // Clear tmp DIB
        core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
        // White text, transparent background
        let white = COLORREF(0x00FFFFFF);
        SetTextColor(s.tmp_dc, white);
        SetBkMode(s.tmp_dc, TRANSPARENT);

        // "K" in logo
        {
            SelectObject(s.tmp_dc, s.font_k.into());
            let text = to_wide("K");
            // Center the glyph in the 24×24 logo box from its measured extent
            // (SOLL uses flex center; a fixed offset drifts per font/DPI).
            let mut ksz = SIZE { cx: 0, cy: 0 };
            let _ = GetTextExtentPoint32W(s.tmp_dc, &text[..1], &mut ksz);
            let lx = (LOGO_X * sc) + ((LOGO_SIZE * sc) - ksz.cx as f32) / 2.0;
            let ly = (LOGO_Y * sc) + ((LOGO_SIZE * sc) - ksz.cy as f32) / 2.0;
            TextOutW(s.tmp_dc, lx as i32, ly as i32, &text[..1]);
            composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 255, 255, 255);
            // Clear tmp again
            core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
        }

        match s.display {
            NativePillState::Recording => {
                // Mode badge at right side
                SelectObject(s.tmp_dc, s.font_mode.into());
                let badge = mode_label(&s.hotkey_mode);
                let text = to_wide(badge);
                let tx = (PILL_W - PAD - 28.0) * sc;
                let ty = ((PILL_H - 10.0) / 2.0) * sc;
                TextOutW(s.tmp_dc, tx as i32, ty as i32, &text[..text.len()-1]);
                composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 128, 131, 133);
                core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
            }

            NativePillState::Transcribing | NativePillState::Cleaning => {
                let label = if matches!(s.display, NativePillState::Transcribing) {
                    "Transcribing..."
                } else {
                    "Cleaning up..."
                };
                let (ar8, ag8, ab8) = accent_to_u8(s.display.accent());
                SelectObject(s.tmp_dc, s.font_label.into());
                let text = to_wide(label);
                let tx = LABEL_X_AFTER_SPIN * sc;
                let ty = ((PILL_H - 11.0) / 2.0) * sc;
                TextOutW(s.tmp_dc, tx as i32, ty as i32, &text[..text.len()-1]);
                composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 170, 172, 173);
                let _ = (ar8, ag8, ab8);
                core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
            }

            NativePillState::Done => {
                SelectObject(s.tmp_dc, s.font_label.into());
                let text = to_wide("Done");
                let tx = LABEL_X_AFTER_SPIN * sc;
                let ty = ((PILL_H - 11.0) / 2.0) * sc;
                TextOutW(s.tmp_dc, tx as i32, ty as i32, &text[..text.len()-1]);
                composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 74, 222, 128);
                core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
            }

            NativePillState::DoneClipboard => {
                SelectObject(s.tmp_dc, s.font_label_lg.into());
                let text = to_wide("In Clipboard");
                let tx = LABEL_X_AFTER_SPIN * sc;
                let ty = ((PILL_H - 12.0) / 2.0) * sc;
                TextOutW(s.tmp_dc, tx as i32, ty as i32, &text[..text.len()-1]);
                composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 255, 163, 68);
                core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
            }

            NativePillState::Error => {
                SelectObject(s.tmp_dc, s.font_label.into());
                let text = to_wide("Error");
                let tx = LABEL_X_AFTER_SPIN * sc;
                let ty = ((PILL_H - 11.0) / 2.0) * sc;
                TextOutW(s.tmp_dc, tx as i32, ty as i32, &text[..text.len()-1]);
                composite_text_mask(s.tmp_bits as *const u8, s.main_bits as *mut u8, pw, ph, 255, 115, 105);
                core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
            }

            NativePillState::Idle => {}
        }
    }

    // --- 4. UpdateLayeredWindow ---
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let pt_src = POINT { x: 0, y: 0 };
    let pt_dst = POINT { x: s.win_x, y: s.win_y };
    let sz = SIZE { cx: pw, cy: ph };

    let ulw = UpdateLayeredWindow(
        hwnd,
        None,
        Some(&pt_dst),
        Some(&sz),
        Some(s.main_dc),
        Some(&pt_src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );
    if ulw.is_err() {
        // A discarded present failure is how the standby-blank defect stayed
        // invisible (Story 10-3). Surface it; recovery is the recreate-on-record
        // path in pipeline.rs, not a retry here.
        log::warn!(
            "[native_pill] UpdateLayeredWindow failed: {ulw:?} (last error {:?})",
            GetLastError()
        );
    }

    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    if !s.was_visible {
        // Re-assert top-of-the-topmost-band on the hidden→visible transition.
        // The window is created WS_EX_TOPMOST, but the topmost *position* can be
        // lost across fullscreen apps / session transitions while the style bit
        // persists (measured: pill at z-index 133, below a maximized foreground
        // app). The old WebView2 bar re-asserted this on every recording start
        // (commit b7acdb3); the native rewrite dropped it. Gated to the edge
        // (not every frame) to avoid z-order churn at 15-30 Hz (10-3 review).
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    s.was_visible = true;
}

/// Build a rounded-rect path. radius is clamped to half the smaller side, so
/// passing a large radius yields a full capsule (SOLL bars use borderRadius:9999).
fn round_rect_path(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<tiny_skia::Path> {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = r * 0.5522847498_f32;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

fn render_waveform(
    pixmap: &mut Pixmap,
    waveform: &[f32; 20],
    waveform_pos: usize,
    sc: f32,
    badge_w_phys: f32,
) {
    // Samples are already boosted at ingest (WM_PILL_SET_RMS); no re-boost here.
    // The waveform fills the space between the stop button (WAVE_X) and the mode
    // badge reserved on the right — SOLL: badge flexShrink:0, waveform flex:1.
    let right_reserve = badge_w_phys + GAP * sc;
    let total_wave_w = ((PILL_W - PAD) * sc - WAVE_X * sc - right_reserve).max(0.0);
    let bar_gap: f32 = 3.0 * sc;
    let bar_w =
        ((total_wave_w - bar_gap * (WAVE_BARS as f32 - 1.0)) / WAVE_BARS as f32).max(1.0);
    let wave_center_y = (PILL_H / 2.0) * sc;

    for i in 0..WAVE_BARS {
        // Sample relative to waveform_pos so index 0 = oldest, last = newest,
        // reproducing FloatingBar's [...prev.slice(1), boosted] ordering.
        let level_idx = ((i as f32 / (WAVE_BARS - 1) as f32) * 19.0).round() as usize;
        let abs_idx = (waveform_pos + level_idx) % 20;
        let sample = waveform[abs_idx];
        let amplitude = sample.max(0.12);
        let bar_h = (amplitude * 19.0).max(3.0) * sc;

        let bx = (WAVE_X * sc) + i as f32 * (bar_w + bar_gap);
        let by = wave_center_y - bar_h / 2.0;

        // borderRadius:9999 → full capsule (radius = half the smaller dimension).
        let radius = bar_w.min(bar_h) / 2.0;
        if let Some(path) = round_rect_path(bx, by, bar_w, bar_h, radius) {
            let mut paint = Paint::default();
            paint.shader = Shader::SolidColor(
                Color::from_rgba(42.0/255.0, 195.0/255.0, 168.0/255.0, 0.85).unwrap(),
            );
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}

fn render_spinner(pixmap: &mut Pixmap, sc: f32, angle_deg: f32, color: Color) {
    let cx = (SPIN_X + SPIN_SIZE / 2.0) * sc;
    let cy = (SPIN_Y + SPIN_SIZE / 2.0) * sc;
    // SVG: r=10 in 24×24 viewBox, scaled to 13px: r = 10 * (13/24) ≈ 5.4
    let r = 5.4_f32 * sc;

    // Track circle (light, 18% opacity)
    {
        let mut paint = Paint::default();
        paint.shader = Shader::SolidColor(Color::from_rgba(1.0, 1.0, 1.0, 0.18).unwrap());
        paint.anti_alias = true;
        let mut stroke = Stroke::default();
        stroke.width = 2.5 * sc;
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // Active arc: quarter circle, rotating
    {
        let start = angle_deg.to_radians();
        let end = start + std::f32::consts::FRAC_PI_2; // 90°
        let steps = 24usize;
        let mut pb = PathBuilder::new();
        let x0 = cx + r * start.cos();
        let y0 = cy + r * start.sin();
        pb.move_to(x0, y0);
        for i in 1..=steps {
            let t = start + (end - start) * i as f32 / steps as f32;
            pb.line_to(cx + r * t.cos(), cy + r * t.sin());
        }
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.shader = Shader::SolidColor(color);
            paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 2.5 * sc;
            stroke.line_cap = LineCap::Round;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn render_check(pixmap: &mut Pixmap, cx: f32, cy: f32, size: f32, color: Color) {
    // SVG polyline: "20 6 9 17 4 12" in 24×24 viewBox → scale to `size`
    let s = size / 24.0;
    let p = |x: f32, y: f32| -> (f32, f32) { (cx + x * s, cy + y * s) };
    let (x0, y0) = p(20.0, 6.0);
    let (x1, y1) = p(9.0, 17.0);
    let (x2, y2) = p(4.0, 12.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    pb.line_to(x2, y2);
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.shader = Shader::SolidColor(color);
        paint.anti_alias = true;
        let mut stroke = Stroke::default();
        stroke.width = 3.0 * (size / 11.0); // 3px in 11px space
        stroke.line_cap = LineCap::Round;
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn render_clipboard_icon(pixmap: &mut Pixmap, cx: f32, cy: f32, size: f32) {
    // Simple clipboard: amber outer rect with white inner (simplified representation)
    let amber = Color::from_rgba(1.0, 163.0/255.0, 68.0/255.0, 1.0).unwrap();
    if let Some(rect) = tiny_skia::Rect::from_xywh(cx, cy, size, size) {
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        paint.shader = Shader::SolidColor(amber);
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    // White inner (60%)
    let inner = size * 0.6;
    let off = (size - inner) / 2.0;
    if let Some(rect) = tiny_skia::Rect::from_xywh(cx + off, cy + off, inner, inner) {
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        paint.shader = Shader::SolidColor(Color::from_rgba(0.2, 0.2, 0.2, 1.0).unwrap());
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
}

fn accent_to_u8(accent: (f32, f32, f32)) -> (u8, u8, u8) {
    let f = |x: f32| (x * 255.0) as u8;
    (f(accent.0), f(accent.1), f(accent.2))
}

fn mode_label(mode: &str) -> &'static str {
    match mode {
        "hold" => "Hold",
        "toggle" => "Toggle",
        "autostop" => "Auto Stop",
        "auto" => "Auto",
        _ => "Hold",
    }
}

// ---------------------------------------------------------------------------
// Timer / state logic
// ---------------------------------------------------------------------------

unsafe fn handle_timer(hwnd: HWND, s: &mut PillWindowState) {
    // Advance spinner
    if matches!(s.display, NativePillState::Transcribing | NativePillState::Cleaning) {
        s.spinner_deg = (s.spinner_deg + 12.0) % 360.0; // ~30fps * 12°/frame ≈ 360°/sec
        render_frame(hwnd, s);
        return;
    }

    // Done timeout
    if let Some(started) = s.done_at {
        let elapsed = started.elapsed().as_millis();
        let limit = if matches!(s.display, NativePillState::DoneClipboard) {
            DONE_CLIPBOARD_MS
        } else {
            DONE_NORMAL_MS
        };
        if elapsed >= limit {
            s.display = NativePillState::Idle;
            s.done_at = None;
            stop_timer(hwnd, s);
            render_frame(hwnd, s);
        }
        return;
    }

    // Error timeout
    if let Some(started) = s.error_at {
        if started.elapsed().as_millis() >= ERROR_IDLE_MS {
            s.display = NativePillState::Idle;
            s.error_at = None;
            stop_timer(hwnd, s);
            render_frame(hwnd, s);
        }
    }
}

unsafe fn start_timer(hwnd: HWND, s: &mut PillWindowState) {
    if !s.timer_active {
        SetTimer(Some(hwnd), TIMER_ANIMATE, TIMER_MS, None);
        s.timer_active = true;
    }
}

unsafe fn stop_timer(hwnd: HWND, s: &mut PillWindowState) {
    if s.timer_active {
        let _ = KillTimer(Some(hwnd), TIMER_ANIMATE);
        s.timer_active = false;
    }
}

// ---------------------------------------------------------------------------
// Window procedure
// ---------------------------------------------------------------------------

unsafe extern "system" fn pill_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Retrieve state pointer from USERDATA
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PillWindowState;

    match msg {
        WM_CREATE => {
            // lparam = *CREATESTRUCTW, whose lpCreateParams is *mut PillWindowState
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            let ptr = cs.lpCreateParams as *mut PillWindowState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
            LRESULT(0)
        }

        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),

        WM_TIMER => {
            if wparam.0 == TIMER_ANIMATE && !state_ptr.is_null() {
                handle_timer(hwnd, &mut *state_ptr);
            }
            LRESULT(0)
        }

        WM_PILL_SET_STATE => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            let code = wparam.0 as u8;
            let clipboard_only = lparam.0 != 0;
            let new_state = NativePillState::from_code(code, clipboard_only);

            s.display = new_state;
            s.spinner_deg = 0.0;
            s.done_at = None;
            s.error_at = None;
            s.waveform = [0.0f32; 20];
            s.waveform_pos = 0; // reset ring-buffer head on state change

            match new_state {
                NativePillState::Done | NativePillState::DoneClipboard => {
                    s.done_at = Some(Instant::now());
                    start_timer(hwnd, s);
                }
                NativePillState::Error => {
                    s.error_at = Some(Instant::now());
                    start_timer(hwnd, s);
                }
                NativePillState::Transcribing | NativePillState::Cleaning => {
                    start_timer(hwnd, s);
                }
                NativePillState::Idle => {
                    stop_timer(hwnd, s);
                }
                NativePillState::Recording => {
                    stop_timer(hwnd, s);
                }
            }

            render_frame(hwnd, s);
            LRESULT(0)
        }

        WM_PILL_SET_RMS => {
            if state_ptr.is_null() || !matches!((*state_ptr).display, NativePillState::Recording) {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            let level = f32::from_bits(wparam.0 as u32);
            // RMS boost: matches FloatingBar.tsx line 388
            const NOISE_FLOOR: f32 = 0.006;
            let boosted = if level <= NOISE_FLOOR {
                0.0f32
            } else {
                (level * 10.0).min(1.0).powf(0.4)
            };
            s.waveform[s.waveform_pos] = boosted;
            s.waveform_pos = (s.waveform_pos + 1) % 20;
            render_frame(hwnd, s);
            LRESULT(0)
        }

        WM_PILL_SET_MODE => {
            if state_ptr.is_null() { return LRESULT(0); }
            // Caller allocated Box<String> via into_raw; we own it now
            let mode = Box::from_raw(wparam.0 as *mut String);
            (*state_ptr).hotkey_mode = *mode;
            if matches!((*state_ptr).display, NativePillState::Recording) {
                render_frame(hwnd, &mut *state_ptr);
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if state_ptr.is_null() { return LRESULT(0); }
            let s = &mut *state_ptr;

            // Check stop button hit region (only in Recording state)
            if matches!(s.display, NativePillState::Recording) {
                let mx = (lparam.0 & 0xFFFF) as i16 as i32;
                let my = (lparam.0 >> 16 & 0xFFFF) as i16 as i32;
                let stop_left = (STOP_X * s.scale as f32) as i32;
                let stop_top = (STOP_Y * s.scale as f32) as i32;
                let stop_right = ((STOP_X + STOP_OUTER) * s.scale as f32) as i32;
                let stop_bottom = ((STOP_Y + STOP_OUTER) * s.scale as f32) as i32;
                if mx >= stop_left && mx <= stop_right && my >= stop_top && my <= stop_bottom {
                    // Cancel recording via AppState
                    cancel_recording_now(&s.app_handle);
                    return LRESULT(0);
                }
            }

            // Begin drag
            let mut cur = POINT::default();
            let _ = GetCursorPos(&mut cur);
            let mut wr = RECT::default();
            let _ = GetWindowRect(hwnd, &mut wr);
            s.drag = Some(Drag {
                start_cur_x: cur.x,
                start_cur_y: cur.y,
                start_win_x: wr.left,
                start_win_y: wr.top,
            });
            SetCapture(hwnd);
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            if state_ptr.is_null() { return LRESULT(0); }
            let s = &mut *state_ptr;
            if let Some(ref drag) = s.drag {
                let mut cur = POINT::default();
                let _ = GetCursorPos(&mut cur);
                let dx = cur.x - drag.start_cur_x;
                let dy = cur.y - drag.start_cur_y;
                let new_x = drag.start_win_x + dx;
                let new_y = drag.start_win_y + dy;
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    new_x,
                    new_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
                s.win_x = new_x;
                s.win_y = new_y;
                // Throttle bar-moved emit during drag: at most once per ~16 ms
                // (~60 Hz), matching the rAF-equivalent cadence in FloatingBar.
                let now = Instant::now();
                let should_emit = s
                    .last_bar_moved_emit
                    .map(|t| now.duration_since(t).as_millis() >= 16)
                    .unwrap_or(true);
                if should_emit {
                    s.last_bar_moved_emit = Some(now);
                    let logical_x = new_x as f64 / s.scale;
                    let logical_y = new_y as f64 / s.scale;
                    let _ = s.app_handle.emit(
                        "klarvo://bar-moved",
                        serde_json::json!({ "x": logical_x, "y": logical_y }),
                    );
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONUP => {
            if state_ptr.is_null() { return LRESULT(0); }
            let s = &mut *state_ptr;
            if s.drag.take().is_some() {
                ReleaseCapture();
                // Save final position
                let logical_x = s.win_x as f64 / s.scale;
                let logical_y = s.win_y as f64 / s.scale;
                // Save to config (non-async, works from any thread)
                let state = s.app_handle.state::<crate::AppState>();
                if let Err(e) = state.save_config_locked("bar position", |cfg| {
                    cfg.bar_x = Some(logical_x);
                    cfg.bar_y = Some(logical_y);
                }) {
                    log::warn!("[native_pill] Failed to save bar position: {e}");
                }
                // Final settled bar-moved event (for preview window alignment)
                let _ = s.app_handle.emit(
                    "klarvo://bar-moved",
                    serde_json::json!({ "x": logical_x, "y": logical_y }),
                );
            }
            LRESULT(0)
        }

        WM_PAINT => {
            // Layered windows don't need WM_PAINT — just validate
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        WM_PILL_SHUTDOWN => {
            // Triggered by NativePill::drop. DestroyWindow sends the real
            // WM_DESTROY + WM_NCDESTROY sequence through the wndproc so
            // the window and its GDI state are cleaned up in the right order.
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }

        WM_DESTROY => {
            if !state_ptr.is_null() {
                // Zero USERDATA before freeing so any messages still in the
                // queue find null and bail, avoiding use-after-free.
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let s = Box::from_raw(state_ptr);
                if s.timer_active {
                    let _ = KillTimer(Some(hwnd), TIMER_ANIMATE);
                }
                // Fonts are never selected into a DC — safe to delete directly.
                DeleteObject(s.font_k.into());
                DeleteObject(s.font_label.into());
                DeleteObject(s.font_mode.into());
                DeleteObject(s.font_label_lg.into());
                // Delete the DCs first; once a DC is destroyed the DIB section
                // it owned is no longer "selected" and can be safely freed.
                DeleteDC(s.main_dc);
                DeleteObject(s.main_bmp.into());
                DeleteDC(s.tmp_dc);
                DeleteObject(s.tmp_bmp.into());
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Cancel recording synchronously from any thread.
fn cancel_recording_now(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    if state.recorder.is_recording() {
        let _ = state.recorder.stop_recording();
        if let Ok(mut g) = state.recording_start.lock() {
            *g = None;
        }
        // Route through emit_pipeline_state so the native pill transitions
        // to Idle/hidden (stop-button cancel must not leave pill on "Recording").
        crate::emit_pipeline_state(app, crate::hotkey::PipelineEvent::idle());
    }
}

// ---------------------------------------------------------------------------
// Window class name (wide string)
// ---------------------------------------------------------------------------

fn class_name_wide() -> Vec<u16> {
    "KlarvoPillNative\0".encode_utf16().collect()
}

// ---------------------------------------------------------------------------
// Pill thread entry point
// ---------------------------------------------------------------------------

fn pill_thread(
    app_handle: AppHandle,
    saved_x: Option<f64>,
    saved_y: Option<f64>,
    tx: mpsc::Sender<Result<isize, String>>,
) {
    unsafe {
        // --- Determine DPI via the monitor at the expected window position ---
        // SPI_GETWORKAREA returns physical-pixel coordinates regardless of DPI awareness.
        // Under per-monitor-v2 (Tauri's embedded manifest), GetDeviceCaps(desktop_dc, LOGPIXELSX)
        // always returns 96 → scale=1.0, wrong on high-DPI monitors. Correct: GetDpiForMonitor.
        let mut work_area = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&raw mut work_area as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        // Candidate point for monitor selection:
        // - saved_x/saved_y are logical px; under the previously broken scale=1.0 they equal
        //   physical px, so they select the correct monitor in the common case.
        // - Default: center-bottom of work area (physical coords from SPI_GETWORKAREA).
        let candidate_pt = match (saved_x, saved_y) {
            (Some(lx), Some(ly)) => POINT { x: lx as i32, y: ly as i32 },
            _ => POINT {
                x: work_area.left + (work_area.right - work_area.left) / 2,
                y: work_area.bottom.saturating_sub(5),
            },
        };
        let hmon = MonitorFromPoint(candidate_pt, MONITOR_DEFAULTTONEAREST);
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        let scale = if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
            let screen_dc = GetDC(None);
            let legacy = GetDeviceCaps(Some(screen_dc), LOGPIXELSX) as u32;
            ReleaseDC(None, screen_dc);
            log::info!(
                "[native_pill] DPI: GetDpiForMonitor={dpi_x} GetDeviceCaps(legacy)={legacy} \
                 scale_real={:.3} scale_was={:.3}",
                dpi_x as f64 / 96.0,
                legacy as f64 / 96.0
            );
            dpi_x as f64 / 96.0
        } else {
            log::warn!("[native_pill] GetDpiForMonitor failed — falling back to GetDeviceCaps");
            let screen_dc = GetDC(None);
            let d = GetDeviceCaps(Some(screen_dc), LOGPIXELSX);
            ReleaseDC(None, screen_dc);
            d as f64 / 96.0
        };
        // phys_w, phys_h, compute_initial_pos follow unchanged — they already use `scale`
        let phys_w = (PILL_W as f64 * scale) as i32;
        let phys_h = (PILL_H as f64 * scale) as i32;

        // --- Compute initial physical position ---
        let (win_x, win_y) = compute_initial_pos(saved_x, saved_y, scale, phys_w, phys_h);

        // --- Create GDI resources ---
        let mut main_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let (main_dc, main_bmp) = match create_dib(phys_w, phys_h, &mut main_bits) {
            Ok(v) => v,
            Err(e) => { let _ = tx.send(Err(e)); return; }
        };

        let mut tmp_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let (tmp_dc, tmp_bmp) = match create_dib(phys_w, phys_h, &mut tmp_bits) {
            Ok(v) => v,
            Err(e) => {
                DeleteObject(main_bmp.into()); DeleteDC(main_dc);
                let _ = tx.send(Err(e)); return;
            }
        };

        // Load the bundled Geist font (the app's UI typeface) into this process's
        // GDI font table so the native pill matches the WebView2 SOLL 1:1 — the
        // .woff2 the web UI uses can't be loaded by GDI, so equivalent .ttf files
        // (derived from the same source) are embedded and registered from memory.
        // GDI family mapping: Regular(400) + Bold(700) register under "Geist";
        // SemiBold registers as its own family "Geist SemiBold".
        load_embedded_font(include_bytes!("../fonts/Geist-Regular.ttf"));
        load_embedded_font(include_bytes!("../fonts/Geist-Bold.ttf"));
        load_embedded_font(include_bytes!("../fonts/Geist-SemiBold.ttf"));

        // Fonts (scale font height with DPI)
        let geist: Vec<u16> = "Geist\0".encode_utf16().collect();
        let geist_ptr = PCWSTR(geist.as_ptr());
        let geist_sb: Vec<u16> = "Geist SemiBold\0".encode_utf16().collect();
        let geist_sb_ptr = PCWSTR(geist_sb.as_ptr());
        let fh = |logical: i32| -> i32 { (logical as f64 * scale) as i32 };
        let font_k = create_font(geist_ptr, fh(14), FW_BOLD.0 as i32);
        let font_label = create_font(geist_ptr, fh(11), FW_NORMAL.0 as i32);
        let font_mode = create_font(geist_ptr, fh(10), FW_NORMAL.0 as i32);
        // "Geist SemiBold" is its own GDI family (weight 600 lives there, not under "Geist").
        let font_label_lg = create_font(geist_sb_ptr, fh(12), FW_NORMAL.0 as i32);

        // --- Register window class ---
        let hinstance = match GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => h,
            Err(e) => {
                DeleteObject(main_bmp.into()); DeleteDC(main_dc);
                DeleteObject(tmp_bmp.into()); DeleteDC(tmp_dc);
                let _ = tx.send(Err(format!("GetModuleHandleW: {e}")));
                return;
            }
        };
        let class_name = class_name_wide();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(pill_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: HICON::default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: HICON::default(),
        };
        // Ignore re-registration error (class may already be registered on restart)
        let _ = RegisterClassExW(&wc);

        // --- Build PillWindowState (will be moved into Box and stored in USERDATA) ---
        let state = Box::new(PillWindowState {
            display: NativePillState::Idle,
            waveform: [0.0f32; 20],
            waveform_pos: 0,
            spinner_deg: 0.0,
            done_at: None,
            error_at: None,
            drag: None,
            last_bar_moved_emit: None,
            main_dc,
            main_bmp,
            main_bits,
            tmp_dc,
            tmp_bmp,
            tmp_bits,
            font_k,
            font_label,
            font_mode,
            font_label_lg,
            phys_w,
            phys_h,
            win_x,
            win_y,
            scale,
            app_handle,
            hotkey_mode: "hold".to_string(),
            timer_active: false,
            was_visible: false,
        });
        let state_ptr = Box::into_raw(state);

        // --- Create the layered window ---
        let ex_style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let hwnd = match CreateWindowExW(
            ex_style,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            win_x,
            win_y,
            phys_w,
            phys_h,
            None,
            None,
            Some(hinstance.into()),
            Some(state_ptr as *const core::ffi::c_void),
        ) {
            Ok(h) => h,
            Err(e) => {
                // CreateWindowExW dispatches WM_DESTROY synchronously on failure
                // (after WM_CREATE ran), so state_ptr was already freed by the
                // WM_DESTROY handler. Do NOT call Box::from_raw here.
                let _ = tx.send(Err(format!("CreateWindowExW: {e}")));
                return;
            }
        };

        // Send HWND back to the creating thread
        let _ = tx.send(Ok(hwnd.0 as isize));

        // --- Message loop ---
        let mut msg = MSG::default();
        loop {
            match GetMessageW(&mut msg, None, 0, 0) {
                BOOL(1) => {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                _ => break, // WM_QUIT or error
            }
        }

        // Class cleanup (best-effort; may fail if other windows still use it)
        let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), Some(hinstance.into()));
    }
}

/// Compute the initial physical-pixel window position.
/// Priority: saved config → SPI_GETWORKAREA center-bottom → fallback.
unsafe fn compute_initial_pos(
    saved_x: Option<f64>,
    saved_y: Option<f64>,
    scale: f64,
    phys_w: i32,
    phys_h: i32,
) -> (i32, i32) {
    // 1. Saved position
    if let (Some(lx), Some(ly)) = (saved_x, saved_y) {
        return ((lx * scale) as i32, (ly * scale) as i32);
    }

    // 2. Work area center-bottom (mirrors create_bar_window logic)
    let mut work_area = RECT::default();
    let ok = SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&raw mut work_area as *mut _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    );
    if ok.is_ok() {
        let work_w = work_area.right - work_area.left;
        let x = work_area.left + (work_w - phys_w) / 2;
        let y = work_area.bottom - phys_h - (8.0 * scale) as i32;
        return (x, y);
    }

    // 3. Hard-coded fallback
    (400, 10)
}
