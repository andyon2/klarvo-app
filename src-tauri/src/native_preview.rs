//! Native Win32 layered-window live-preview overlay.
//!
//! Replaces the WebView2 "preview" window with a `WS_EX_LAYERED | WS_EX_TOPMOST |
//! WS_EX_TRANSPARENT` window that stays fully composited even when occluded — the
//! same defect class that Story 10-1 solved for the pill. The preview is simpler
//! than the pill (no drag, no animations) but adds multi-line scrollable text.
//!
//! ## Threading
//! The window and its message loop run on a dedicated OS thread.
//! The public [`NativePreview`] handle communicates via `PostMessageW`.
//!
//! ## Rendering
//! 1. tiny-skia draws the dark rounded-rect card (background + border) into a
//!    premultiplied RGBA [`Pixmap`].
//! 2. The Pixmap is RGBA→BGRA swapped into the main DIB.
//! 3. GDI draws the text (white-on-black) into a tmp DIB, then the B-channel
//!    coverage is alpha-composited onto the BGRA DIB using the desired text color.
//! 4. When text overflows, a top-to-bottom alpha fade is applied in-place on the
//!    BGRA DIB (matches the CSS `mask-image: linear-gradient(to bottom, ...)` in
//!    `PreviewPanel.tsx`).
//! 5. `UpdateLayeredWindow(ULW_ALPHA)` presents the final BGRA DIB to DWM.

#![cfg(target_os = "windows")]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::mem::size_of;
use std::sync::mpsc;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

// ---------------------------------------------------------------------------
// Custom messages (WM_APP range: 0x8100-0x81FF — non-overlapping with pill)
// ---------------------------------------------------------------------------
const WM_PREVIEW_SET_STATE: u32 = 0x8101; // WPARAM=state_code (u8)
const WM_PREVIEW_APPEND_CHUNK: u32 = 0x8102; // WPARAM=ptr to Box<String>
const WM_PREVIEW_SET_PILL_POS: u32 = 0x8103; // WPARAM=x_bits (f64::to_bits), LPARAM=y_bits
const WM_PREVIEW_SHUTDOWN: u32 = 0x8110;

// Layout constants (logical px)
const OUTER_INSET: f32 = 2.0; // keep border inside DIB at fractional DPI
const INNER_PAD_TB: f32 = 8.0; // top/bottom inner padding
const INNER_PAD_LR: f32 = 12.0; // left/right inner padding (matches SOLL `padding: 8px 12px`)
const FADE_FRACTION: f32 = 0.18; // top-fade fraction of card height on overflow
const PILL_WIDTH_LOGICAL: f64 = 200.0; // must match PILL_W in native_pill.rs
const GAP_LOGICAL: f64 = 8.0; // gap between preview bottom edge and pill top

// Geometry presets (logical px)
const BASE_FONT_PX: f64 = 11.0;
const BASE_MAX_HEIGHT: f64 = 600.0;

// ---------------------------------------------------------------------------
// State codes
// ---------------------------------------------------------------------------
const STATE_RECORDING: u8 = 1;

// ---------------------------------------------------------------------------
// PreviewConfig — snapshot taken from AppConfig at recording-start
// ---------------------------------------------------------------------------

/// Snapshot of all AppConfig values needed by the preview renderer.
/// Built from `AppConfig` in `pipeline.rs` at recording-start and passed to
/// `NativePreview::create`. Immutable for the lifetime of the recording cycle.
#[derive(Default)]
pub struct PreviewConfig {
    // Appearance (straight-alpha RGB/RGBA — premultiplication happens in renderer)
    pub bg_r: u8,
    pub bg_g: u8,
    pub bg_b: u8,
    pub bg_a: u8,
    pub text_r: u8,
    pub text_g: u8,
    pub text_b: u8,
    pub text_a: u8,
    pub border_r: u8,
    pub border_g: u8,
    pub border_b: u8,
    pub border_a: u8,
    pub border_width: u8,
    pub border_radius: u8,
    pub font_px: u32,     // 11 | 13 | 15 (from previewFontSize small/medium/large)
    pub font_face: String, // first token of previewFontFamily CSS cascade (default "Inter")
    pub w_base: i32,      // 260 | 320 | 400 (from previewPanelForm compact/comfortable/wide)
    pub line_height_mult: f32, // 1.325 | 1.625 | 1.925 (from previewLineSpacing small/medium/large)
    pub live_preview_enabled: bool,
}

impl PreviewConfig {
    /// Build from `AppConfig`. Call with the config lock held.
    pub fn from_app_config(cfg: &crate::config::AppConfig) -> Self {
        let font_px = match cfg.preview_font_size.as_str() {
            "medium" => 13u32,
            "large" => 15,
            _ => 11, // "small" or default
        };
        // Story 11.6 DESIGN DECISION 2 (step size widened at the 2026-08-10 review gate,
        // finding D2): "medium" = today's hardcoded 1.625, so nothing changes visually
        // until the user touches the control. "small"/"large" are a symmetric ±0.30 em
        // offset. This GDI line-stepping multiplies the *font size* (like CSS
        // `lineHeight`), while Android's `setLineSpacing(0f, mult)` multiplies the font's
        // *natural line height* (~1.2× text size) — so Desktop uses ±0.30 and Android uses
        // ±0.25 (see `ListeningPanelView.kt`'s `LINE_SPACING_MULT`) to move the same ±0.30
        // em on both platforms. To be confirmed at GATE-4 on a real Windows build.
        let line_height_mult = match cfg.preview_line_spacing.as_str() {
            "small" => 1.325f32,
            "large" => 1.925,
            _ => 1.625, // "medium" or default
        };
        let w_base = match cfg.preview_panel_form.as_str() {
            "compact" => 260i32,
            "wide" => 400,
            _ => 320, // "comfortable" or default
        };
        let (bg_r, bg_g, bg_b, bg_a) =
            parse_css_rgba(&cfg.preview_bg_color, (25, 25, 25, 245)); // 0.96×255≈245
        let (text_r, text_g, text_b, text_a) =
            parse_css_rgba(&cfg.preview_text_color, (220, 220, 220, 224)); // 0.88×255≈224
        let (border_r, border_g, border_b, border_a) =
            parse_css_rgba(&cfg.preview_border_color, (42, 195, 168, 64)); // 0.25×255≈64
        // Parse the first family token from the CSS cascade string.
        // e.g. "'Inter', system-ui, -apple-system, sans-serif" → "Inter"
        let font_face = {
            let s = cfg.preview_font_family.trim();
            let token = if s.starts_with('\'') || s.starts_with('"') {
                let q = s.chars().next().unwrap();
                let inner = &s[1..];
                inner.find(q).map(|i| inner[..i].trim().to_string()).unwrap_or_default()
            } else {
                s.find(',').map(|i| s[..i].trim().to_string()).unwrap_or_else(|| s.to_string())
            };
            let token = if token.is_empty() { "Inter".to_string() } else { token };
            // Resolve only the CSS *generic* keywords to their Windows equivalents,
            // matching the browser's fallback. "Inter" is NOT installed on the
            // target machine, so the default stack 'Inter', system-ui, … resolves
            // to Segoe UI (system-ui) in the browser — mirror that here.
            //
            // Named fonts that ARE installed must pass through UNCHANGED: the
            // monospace preset's first token "Cascadia Code" is installed
            // (CascadiaCode.ttf), so the browser renders Cascadia Code — a prior
            // hard-remap of "Cascadia Code" => "Consolas" forced a font mismatch
            // against the Settings SOLL (measured, gate4-evidence/10-4). GDI's
            // CreateFontW matches the installed family by name.
            match token.as_str() {
                "Inter" | "system-ui" => "Segoe UI".to_string(),
                _ => token,
            }
        };
        PreviewConfig {
            bg_r,
            bg_g,
            bg_b,
            bg_a,
            text_r,
            text_g,
            text_b,
            text_a,
            border_r,
            border_g,
            border_b,
            border_a,
            border_width: cfg.preview_border_width,
            border_radius: cfg.preview_border_radius,
            font_px,
            font_face,
            w_base,
            line_height_mult,
            live_preview_enabled: cfg.live_preview_enabled,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-window state (stored in GWLP_USERDATA)
// ---------------------------------------------------------------------------

struct PreviewWindowState {
    config: PreviewConfig,
    // Geometry (physical pixels)
    phys_w: i32,
    phys_h: i32,
    win_x: i32,
    win_y: i32,
    scale: f64,
    // Windows "Text size" accessibility multiplier (1.0 = 100%). Applied to the font
    // (and line-height) ONLY — like Chromium's text scaling — so the native preview's
    // text matches the Settings webview card, which honors this factor. Box/padding/
    // border stay at `scale` (DPI) only.
    text_scale: f64,
    // Current pill position in logical pixels (for reposition on bar-moved)
    pill_x_logical: f64,
    pill_y_logical: f64,
    // Work area (physical, cached at startup)
    work_left: i32,
    work_right: i32,
    work_top: i32,
    // Render state
    text_buffer: String,
    armed: bool,       // true when Recording received and live_preview_enabled
    was_visible: bool, // tracks hidden→visible edge for topmost re-assert
    // GDI resources
    main_dc: HDC,
    main_bmp: HBITMAP,
    main_bits: *mut core::ffi::c_void,
    tmp_dc: HDC,
    tmp_bmp: HBITMAP,
    tmp_bits: *mut core::ffi::c_void,
    font: HFONT,
}

// SAFETY: PreviewWindowState is only ever touched from the preview thread (WndProc).
unsafe impl Send for PreviewWindowState {}

// ---------------------------------------------------------------------------
// Public handle
// ---------------------------------------------------------------------------

/// Handle to the native preview window. Cheap to clone (HWND + thread handle).
pub struct NativePreview {
    hwnd: isize, // stored as isize for Send
    _thread: std::thread::JoinHandle<()>,
}

// SAFETY: hwnd is used only via PostMessageW (thread-safe) and IsWindow (read-only).
unsafe impl Send for NativePreview {}

impl NativePreview {
    /// Spawn the preview window on a dedicated thread and return a handle.
    /// `pill_x` / `pill_y` are the saved pill position from config (logical px).
    /// `config` is the recording-cycle snapshot built from AppConfig.
    pub fn create(
        pill_x: Option<f64>,
        pill_y: Option<f64>,
        config: PreviewConfig,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Result<isize, String>>();
        let thread = std::thread::spawn(move || {
            preview_thread(pill_x, pill_y, config, tx);
        });
        let hwnd = rx
            .recv()
            .map_err(|_| "preview thread died before sending HWND".to_string())??;
        Ok(NativePreview { hwnd, _thread: thread })
    }

    /// Arm (Recording) or disarm (Idle/Done/Error) the preview.
    pub fn set_state(&self, state: &crate::hotkey::PipelineState) {
        let code: u8 = match state {
            crate::hotkey::PipelineState::Recording => STATE_RECORDING,
            _ => 0,
        };
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PREVIEW_SET_STATE,
                WPARAM(code as usize),
                LPARAM(0),
            );
        }
    }

    /// Append a new transcript chunk to the preview text buffer and re-render.
    /// Caller allocates `chunk` with `Box::new(text)`; ownership transfers to
    /// the preview thread which frees it after consuming.
    pub fn append_chunk(&self, chunk: Box<String>) {
        let ptr = Box::into_raw(chunk);
        unsafe {
            if PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PREVIEW_APPEND_CHUNK,
                WPARAM(ptr as usize),
                LPARAM(0),
            )
            .is_err()
            {
                // PostMessage failed (window gone) — free the box to avoid a leak.
                drop(Box::from_raw(ptr));
            }
        }
    }

    /// Reposition the preview above the (moved) pill.
    /// `x` / `y` are the new pill position in logical pixels.
    pub fn set_pill_pos(&self, x: f64, y: f64) {
        // Encode f64 as bit-level u64; WPARAM/LPARAM are usize (= u64 on 64-bit Windows).
        let xbits = x.to_bits() as usize;
        let ybits = y.to_bits() as isize;
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PREVIEW_SET_PILL_POS,
                WPARAM(xbits),
                LPARAM(ybits),
            );
        }
    }

    /// Check if the native preview window still exists.
    pub fn is_alive(&self) -> bool {
        unsafe { IsWindow(Some(HWND(self.hwnd as *mut _))).as_bool() }
    }
}

impl Drop for NativePreview {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PREVIEW_SHUTDOWN,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CSS color parsing
// ---------------------------------------------------------------------------

/// Parse `rgba(r,g,b,a)` or `rgb(r,g,b)` CSS color strings.
/// Alpha in the CSS string is 0.0–1.0; returned as 0–255.
/// Falls back to `default` on parse failure.
fn parse_css_rgba(s: &str, default: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|t| t.strip_suffix(')')) {
        let p: Vec<&str> = inner.split(',').collect();
        if p.len() == 4 {
            if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                p[0].trim().parse::<f64>(),
                p[1].trim().parse::<f64>(),
                p[2].trim().parse::<f64>(),
                p[3].trim().parse::<f64>(),
            ) {
                return (
                    r.round().clamp(0.0, 255.0) as u8,
                    g.round().clamp(0.0, 255.0) as u8,
                    b.round().clamp(0.0, 255.0) as u8,
                    (a * 255.0).round().clamp(0.0, 255.0) as u8,
                );
            }
        }
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|t| t.strip_suffix(')')) {
        let p: Vec<&str> = inner.split(',').collect();
        if p.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                p[0].trim().parse::<f64>(),
                p[1].trim().parse::<f64>(),
                p[2].trim().parse::<f64>(),
            ) {
                return (
                    r.round().clamp(0.0, 255.0) as u8,
                    g.round().clamp(0.0, 255.0) as u8,
                    b.round().clamp(0.0, 255.0) as u8,
                    255,
                );
            }
        }
    }
    default
}

// ---------------------------------------------------------------------------
// DIB helpers (same pattern as native_pill.rs)
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
    let bmp = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, bits_out, None, 0)
        .map_err(|e| format!("CreateDIBSection failed: {e}"))?;
    SelectObject(dc, bmp.into());
    Ok((dc, bmp))
}

unsafe fn create_font(name: PCWSTR, height_px: i32) -> HFONT {
    CreateFontW(
        -height_px,
        0,
        0,
        0,
        FW_NORMAL.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        ANTIALIASED_QUALITY,
        (FF_DONTCARE.0 | VARIABLE_PITCH.0) as u32,
        name,
    )
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Read the Windows "Text size" accessibility factor
/// (`HKCU\Software\Microsoft\Accessibility\TextScaleFactor`, a REG_DWORD percent
/// like 123 for 123%). Chromium/WebView2 honors this for ALL page text, so the
/// Settings "Live-Vorschau" card renders its font at `font_px × this`. GDI-drawn
/// text does NOT honor it — so the native preview must apply the same factor to its
/// font to match the Settings preview 1:1 (Andi decision 2026-06-29; the floating
/// preview is a faithful predictor of itself). Returns a multiplier (1.23 for 123%);
/// defaults to 1.0 when the value is unset or unreadable. Windows clamps the slider
/// to 100–225%.
unsafe fn read_text_scale_factor() -> f64 {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let subkey = to_wide("Software\\Microsoft\\Accessibility");
    let value = to_wide("TextScaleFactor");
    let mut data: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    let res = RegGetValueW(
        HKEY_CURRENT_USER,
        PCWSTR(subkey.as_ptr()),
        PCWSTR(value.as_ptr()),
        RRF_RT_REG_DWORD,
        None,
        Some(&mut data as *mut u32 as *mut core::ffi::c_void),
        Some(&mut size),
    );
    if res == ERROR_SUCCESS && data >= 100 {
        (data as f64 / 100.0).min(2.25)
    } else {
        1.0
    }
}

/// Word-wrap `text` into visual lines fitting within `max_w` physical px,
/// measured with the font currently selected into `dc`. Each returned line is a
/// UTF-16 slice WITHOUT a null terminator (ready for `DrawTextW` with an explicit
/// length). An explicit '\n' forces a new line. Mirrors the SOLL card's CSS
/// word-wrap (break at spaces; an over-long single word takes its own line and is
/// clipped by `inner_right`, like `overflow:hidden`). We wrap manually instead of
/// using `DrawTextW(DT_WORDBREAK)` because GDI has no line-height control — the
/// caller positions each line at a `font_px × line_height_mult × scale` step
/// (the configured `previewLineSpacing`, default matching the SOLL's `leading-relaxed`).
unsafe fn wrap_text_lines(dc: HDC, text: &str, max_w: i32) -> Vec<Vec<u16>> {
    let mut lines: Vec<Vec<u16>> = Vec::new();
    for paragraph in text.split('\n') {
        let mut cur = String::new();
        for word in paragraph.split(' ') {
            let candidate = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{cur} {word}")
            };
            let wide: Vec<u16> = candidate.encode_utf16().collect();
            let mut sz = SIZE::default();
            let fits = !wide.is_empty()
                && GetTextExtentPoint32W(dc, &wide, &mut sz).as_bool()
                && sz.cx <= max_w;
            if fits || cur.is_empty() {
                cur = candidate;
            } else {
                lines.push(cur.encode_utf16().collect());
                cur = word.to_string();
            }
        }
        lines.push(cur.encode_utf16().collect());
    }
    lines
}

fn class_name_wide() -> Vec<u16> {
    "KlarvoPreviewNative\0".encode_utf16().collect()
}

// ---------------------------------------------------------------------------
// Geometry computation
// ---------------------------------------------------------------------------

/// Compute preview window position and physical size from pill position + config.
/// Returns `(win_x_phys, win_y_phys, phys_w, phys_h)`.
unsafe fn compute_preview_geometry(
    pill_x_logical: f64,
    pill_y_logical: f64,
    config: &PreviewConfig,
    scale: f64,
    work_left: i32,
    work_right: i32,
    work_top: i32,
) -> (i32, i32, i32, i32) {
    let k = config.font_px as f64 / BASE_FONT_PX;
    let w_logical = (config.w_base as f64 * k).round() as i32;
    let h_max_logical_unclamped = (BASE_MAX_HEIGHT * k).round() as i64;
    // Vertical clamp: must fit between work-area top + 12 and pill - gap - 12
    let max_avail = (pill_y_logical - GAP_LOGICAL - work_top as f64 / scale - 12.0).max(0.0);
    let h_max_logical = (h_max_logical_unclamped as f64).min(max_avail) as i32;

    let pill_center_x = pill_x_logical + PILL_WIDTH_LOGICAL / 2.0;
    let preview_left_raw = pill_center_x - w_logical as f64 / 2.0;
    let work_left_logical = work_left as f64 / scale;
    let work_right_logical = work_right as f64 / scale;
    let preview_left = preview_left_raw
        .max(work_left_logical + 12.0)
        .min(work_right_logical - w_logical as f64 - 12.0);
    let preview_top = pill_y_logical - GAP_LOGICAL - h_max_logical as f64;

    let phys_w = (w_logical as f64 * scale) as i32;
    let phys_h = (h_max_logical as f64 * scale) as i32;
    let win_x = (preview_left * scale) as i32;
    let win_y = (preview_top * scale) as i32;
    (win_x, win_y, phys_w.max(1), phys_h.max(1))
}

// ---------------------------------------------------------------------------
// Pixel helpers (same as native_pill.rs)
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
        *main_bits.add(base) = b;
        *main_bits.add(base + 1) = g;
        *main_bits.add(base + 2) = r;
        *main_bits.add(base + 3) = a;
    }
}

/// Composite white-on-black GDI text (B-channel coverage) onto main BGRA DIB
/// using the specified straight-alpha text color (including text_a for overall opacity).
unsafe fn composite_text_mask(
    tmp_bits: *const u8,
    main_bits: *mut u8,
    w: i32,
    h: i32,
    text_r: u8,
    text_g: u8,
    text_b: u8,
    text_a: u8,
) {
    let total = (w * h) as usize;
    for i in 0..total {
        let base = i * 4;
        // Scale glyph coverage by the configured text alpha so e.g. the default
        // 0.88 opacity (text_a≈224) renders text at 88% rather than fully opaque.
        let coverage = (*tmp_bits.add(base) as u32 * text_a as u32) / 255;
        if coverage == 0 {
            continue;
        }
        let src_a = coverage as u8;
        let pm_b = (text_b as u32 * coverage / 255) as u8;
        let pm_g = (text_g as u32 * coverage / 255) as u8;
        let pm_r = (text_r as u32 * coverage / 255) as u8;
        let inv = 255u32 - src_a as u32;
        let dst = main_bits.add(base);
        *dst = (pm_b as u32 + *dst as u32 * inv / 255).min(255) as u8;
        *dst.add(1) = (pm_g as u32 + *dst.add(1) as u32 * inv / 255).min(255) as u8;
        *dst.add(2) = (pm_r as u32 + *dst.add(2) as u32 * inv / 255).min(255) as u8;
        *dst.add(3) = (src_a as u32 + *dst.add(3) as u32 * inv / 255).min(255) as u8;
    }
}

/// Apply a linear alpha fade from transparent (row `fade_start`) to opaque
/// (row `fade_end`) over the premultiplied BGRA DIB. Used for the top-fade
/// on overflowing text (mirrors `WebkitMaskImage: linear-gradient(to bottom, ...)`)
unsafe fn apply_top_fade(bits: *mut u8, w: i32, fade_start: i32, fade_end: i32) {
    if fade_end <= fade_start {
        return;
    }
    let range = (fade_end - fade_start) as f32;
    for y in fade_start..fade_end {
        let alpha_scale = (y - fade_start) as f32 / range; // 0.0→1.0
        for x in 0..w {
            let idx = (y * w + x) as usize * 4;
            let ptr = bits.add(idx);
            *ptr = (*ptr as f32 * alpha_scale) as u8;
            *ptr.add(1) = (*ptr.add(1) as f32 * alpha_scale) as u8;
            *ptr.add(2) = (*ptr.add(2) as f32 * alpha_scale) as u8;
            *ptr.add(3) = (*ptr.add(3) as f32 * alpha_scale) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Rounded-rect path helper
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

unsafe fn render_frame(hwnd: HWND, s: &mut PreviewWindowState) {
    let pw = s.phys_w;
    let ph = s.phys_h;
    let sc = s.scale as f32;

    // Hide when not armed or no text yet
    if !s.armed || s.text_buffer.is_empty() {
        ShowWindow(hwnd, SW_HIDE);
        s.was_visible = false;
        return;
    }

    let byte_count = (pw * ph) as usize * 4;

    // --- 1. Measure text height FIRST (DT_CALCRECT does not draw) ---
    // GDI context is set up here so we can size the card to the actual content before
    // rasterising the card background. The tmp DIB is zeroed; font + colors are selected.
    core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
    SetTextColor(s.tmp_dc, COLORREF(0x00FFFFFF));
    SetBkMode(s.tmp_dc, TRANSPARENT);
    SelectObject(s.tmp_dc, s.font.into());

    // Horizontal inner bounds (independent of card height — only padding L/R matters).
    let inner_left = (OUTER_INSET * sc + INNER_PAD_LR * sc) as i32;
    let inner_right = (pw as f32 - OUTER_INSET * sc - INNER_PAD_LR * sc) as i32;
    let text_area_w = (inner_right - inner_left).max(1);

    // Word-wrap into visual lines, then size by the configured line-height multiplier
    // (`previewLineSpacing`, default 1.625 matching SOLL `leading-relaxed`). GDI's own
    // DrawTextW line spacing is the font's natural ~1.2, which made the native preview
    // look denser/smaller than the Settings live-preview — so we lay lines out manually
    // at this step.
    let line_h = (s.config.font_px as f32 * sc * s.text_scale as f32 * s.config.line_height_mult)
        .round()
        .max(1.0) as i32;
    let lines = wrap_text_lines(s.tmp_dc, &s.text_buffer, text_area_w);
    let text_h = lines.len() as i32 * line_h;

    // --- 2. Card geometry: content-height, bottom-aligned (grow-up) ---
    // The window is fixed at max-height; the opaque dark card is only as tall as the text
    // plus inner padding, bottom-aligned so it hugs the pill. The transparent region above
    // the card lets the pill show through. Mirrors PreviewPanel.tsx justifyContent:flex-end.
    let inset = OUTER_INSET * sc;
    let card_x = inset;
    let card_w = pw as f32 - 2.0 * inset;
    let max_card_h = ph as f32 - 2.0 * inset;
    let content_h = text_h as f32 + 2.0 * INNER_PAD_TB * sc;
    let card_h = content_h.min(max_card_h);
    let card_y = (ph as f32 - inset) - card_h; // bottom-aligned: card bottom at (ph - inset)
    let overflows = content_h > max_card_h;
    let radius = s.config.border_radius as f32 * sc;

    // --- 3. tiny-skia: rounded-rect card background + border at content height ---
    let Some(mut pixmap) = Pixmap::new(pw as u32, ph as u32) else {
        log::warn!("[native_preview] Pixmap::new({pw},{ph}) failed — skipping frame");
        return;
    };

    // Background fill — pass straight RGB; Color::from_rgba premultiplies internally.
    // (Scaling RGB by alpha here would cause double-premultiplication: rgb·a².)
    let bg_a = s.config.bg_a as f32 / 255.0;
    {
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = Shader::SolidColor(
            Color::from_rgba(
                s.config.bg_r as f32 / 255.0,
                s.config.bg_g as f32 / 255.0,
                s.config.bg_b as f32 / 255.0,
                bg_a,
            )
            .unwrap_or(Color::BLACK),
        );
        if let Some(path) = round_rect_path(card_x, card_y, card_w, card_h, radius) {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    // Border stroke — straight RGB, same reasoning as bg fill.
    if s.config.border_width > 0 && s.config.border_a > 0 {
        let border_a = s.config.border_a as f32 / 255.0;
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = Shader::SolidColor(
            Color::from_rgba(
                s.config.border_r as f32 / 255.0,
                s.config.border_g as f32 / 255.0,
                s.config.border_b as f32 / 255.0,
                border_a,
            )
            .unwrap_or(Color::WHITE),
        );
        let mut stroke = Stroke::default();
        stroke.width = s.config.border_width as f32 * sc;
        if let Some(path) = round_rect_path(card_x, card_y, card_w, card_h, radius) {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // --- 4. Copy RGBA→BGRA into main DIB ---
    copy_rgba_to_bgra(&pixmap, s.main_bits as *mut u8, byte_count);

    // --- 5. GDI text compositing (tmp DIB already zeroed + context set in step 1) ---
    {
        // Inner vertical bounds derived from card position (not window height).
        // inner_bottom marks the baseline for the newest (bottom) text line.
        let inner_bottom = (card_y + card_h - INNER_PAD_TB * sc) as i32;

        // Newest text always anchored at inner_bottom (grow-up model).
        // When overflowing, oldest lines extend above inner_top and are hidden by the top-fade.
        let start_y = inner_bottom - text_h;

        // Draw each wrapped line in its own `line_h` box, vertically centred so the
        // extra leading splits above/below the glyphs (CSS line-height behaviour).
        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue; // blank line still occupies a line_h slot via the index
            }
            let line_top = start_y + i as i32 * line_h;
            let mut rect = RECT {
                left: inner_left,
                top: line_top,
                right: inner_right,
                bottom: line_top + line_h,
            };
            let mut buf = line.clone();
            DrawTextW(
                s.tmp_dc,
                &mut buf,
                &mut rect,
                DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
            );
        }

        composite_text_mask(
            s.tmp_bits as *const u8,
            s.main_bits as *mut u8,
            pw,
            ph,
            s.config.text_r,
            s.config.text_g,
            s.config.text_b,
            s.config.text_a,
        );

        // --- 6. Top-fade gradient: only when text actually overflows the max card height ---
        if overflows {
            let fade_start = card_y as i32; // card top (= inset when card fills max)
            let fade_h = ((card_h * FADE_FRACTION) * sc).max(1.0) as i32;
            let fade_end = (fade_start + fade_h).min(ph);
            apply_top_fade(s.main_bits as *mut u8, pw, fade_start, fade_end);
        }
    }

    // --- 5. UpdateLayeredWindow ---
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
        log::warn!(
            "[native_preview] UpdateLayeredWindow failed: {ulw:?} (last error {:?})",
            GetLastError()
        );
    }

    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    if !s.was_visible {
        // Re-assert topmost on the hidden→visible edge (mirrors 10-3 AC-3 pattern).
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

// ---------------------------------------------------------------------------
// Window procedure
// ---------------------------------------------------------------------------

unsafe extern "system" fn preview_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PreviewWindowState;

    match msg {
        WM_CREATE => {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            let ptr = cs.lpCreateParams as *mut PreviewWindowState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
            LRESULT(0)
        }

        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),

        WM_PREVIEW_SET_STATE => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            let code = wparam.0 as u8;
            if code == STATE_RECORDING {
                // Arm only if live preview is enabled in the config snapshot
                if s.config.live_preview_enabled {
                    s.armed = true;
                    s.text_buffer.clear();
                } else {
                    // live preview disabled: never show
                    s.armed = false;
                }
            } else {
                // Done / Idle / Error / Warning — disarm and hide
                s.armed = false;
                s.text_buffer.clear();
                ShowWindow(hwnd, SW_HIDE);
                s.was_visible = false;
            }
            LRESULT(0)
        }

        WM_PREVIEW_APPEND_CHUNK => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            // Recover Box<String> allocated by append_chunk()
            let chunk = Box::from_raw(wparam.0 as *mut String);
            if s.armed {
                if !s.text_buffer.is_empty() {
                    s.text_buffer.push(' ');
                }
                s.text_buffer.push_str(&chunk);
                render_frame(hwnd, s);
            }
            LRESULT(0)
        }

        WM_PREVIEW_SET_PILL_POS => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            let new_pill_x = f64::from_bits(wparam.0 as u64);
            let new_pill_y = f64::from_bits(lparam.0 as u64);
            s.pill_x_logical = new_pill_x;
            s.pill_y_logical = new_pill_y;
            // Recompute position
            let (wx, wy, pw, ph) = compute_preview_geometry(
                new_pill_x,
                new_pill_y,
                &s.config,
                s.scale,
                s.work_left,
                s.work_right,
                s.work_top,
            );
            s.win_x = wx;
            s.win_y = wy;
            // Only re-render (and thus reposition via UpdateLayeredWindow.pptDst)
            // if the preview is currently visible.
            if s.armed && !s.text_buffer.is_empty() {
                // Resize DIBs if physical size changed (font-scale can change on DPI change)
                if pw != s.phys_w || ph != s.phys_h {
                    rebuild_dibs(hwnd, s, pw, ph);
                }
                render_frame(hwnd, s);
            } else {
                // Preview is hidden: update geometry so the next show lands at the right spot.
                if pw != s.phys_w || ph != s.phys_h {
                    // Size changed while hidden — rebuild DIBs (updates phys_w/phys_h and
                    // repositions the HWND via SetWindowPos inside rebuild_dibs).
                    rebuild_dibs(hwnd, s, pw, ph);
                } else {
                    // Size unchanged — just move the HWND without a render.
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        wx,
                        wy,
                        pw,
                        ph,
                        SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOREDRAW,
                    );
                }
            }
            LRESULT(0)
        }

        WM_PREVIEW_SHUTDOWN => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }

        WM_DESTROY => {
            if !state_ptr.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let s = Box::from_raw(state_ptr);
                DeleteObject(s.font.into());
                DeleteDC(s.main_dc);
                DeleteObject(s.main_bmp.into());
                DeleteDC(s.tmp_dc);
                DeleteObject(s.tmp_bmp.into());
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Rebuild main + tmp DIBs when window physical size changes.
/// Called from `WM_PREVIEW_SET_PILL_POS` when geometry changes.
///
/// Safety: creates BOTH new DIBs before freeing the old ones so that
/// `s.*` is never left dangling on failure. On partial failure the
/// successful new DIB is freed and the old ones remain valid.
unsafe fn rebuild_dibs(hwnd: HWND, s: &mut PreviewWindowState, new_w: i32, new_h: i32) {
    // Create new DIBs first — do NOT free old ones yet.
    let mut new_main_bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut new_tmp_bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let main_res = create_dib(new_w, new_h, &mut new_main_bits);
    let tmp_res = create_dib(new_w, new_h, &mut new_tmp_bits);

    let (mdc, mbmp) = match main_res {
        Ok(v) => v,
        Err(e) => {
            // main failed; free any tmp that succeeded, leave s.* untouched.
            if let Ok((tdc, tbmp)) = tmp_res {
                DeleteDC(tdc);
                DeleteObject(tbmp.into());
            }
            log::warn!("[native_preview] rebuild_dibs(main) failed: {e} — old DIBs still active");
            return;
        }
    };
    let (tdc, tbmp) = match tmp_res {
        Ok(v) => v,
        Err(e) => {
            // tmp failed; free the new main DIB to avoid leak, leave s.* untouched.
            DeleteDC(mdc);
            DeleteObject(mbmp.into());
            log::warn!("[native_preview] rebuild_dibs(tmp) failed: {e} — old DIBs still active");
            return;
        }
    };

    // Both succeeded: free old GDI objects and install new ones.
    DeleteDC(s.main_dc);
    DeleteObject(s.main_bmp.into());
    DeleteDC(s.tmp_dc);
    DeleteObject(s.tmp_bmp.into());
    s.main_dc = mdc;
    s.main_bmp = mbmp;
    s.main_bits = new_main_bits;
    s.tmp_dc = tdc;
    s.tmp_bmp = tbmp;
    s.tmp_bits = new_tmp_bits;
    s.phys_w = new_w;
    s.phys_h = new_h;
    // Resize the window to match (uses updated s.win_x/win_y set by caller).
    let _ = SetWindowPos(
        hwnd,
        None,
        s.win_x,
        s.win_y,
        new_w,
        new_h,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

// ---------------------------------------------------------------------------
// Preview thread entry point
// ---------------------------------------------------------------------------

fn preview_thread(
    pill_x: Option<f64>,
    pill_y: Option<f64>,
    config: PreviewConfig,
    tx: mpsc::Sender<Result<isize, String>>,
) {
    unsafe {
        // --- DPI + scale ---
        let screen_dc = GetDC(None);
        let dpi = GetDeviceCaps(Some(screen_dc), LOGPIXELSX);
        ReleaseDC(None, screen_dc);
        let scale = dpi as f64 / 96.0;
        // Windows "Text size" accessibility factor — honored by the Settings webview
        // card but not by GDI text; apply it to the font so the two match (see helper).
        let text_scale = read_text_scale_factor();

        // --- Work area ---
        let mut work_area = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&raw mut work_area as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let work_left = work_area.left;
        let work_right = work_area.right;
        let work_top = work_area.top;

        // --- Default pill position (center-bottom of work area if not saved) ---
        let pill_x_logical = pill_x.unwrap_or_else(|| {
            let w = (work_area.right - work_area.left) as f64 / scale;
            (work_left as f64 / scale) + w / 2.0 - PILL_WIDTH_LOGICAL / 2.0
        });
        let pill_y_logical = pill_y.unwrap_or_else(|| {
            (work_area.bottom as f64 / scale) - 8.0 - 36.0 // 36 = pill height
        });

        // --- Compute initial geometry ---
        let (win_x, win_y, phys_w, phys_h) = compute_preview_geometry(
            pill_x_logical,
            pill_y_logical,
            &config,
            scale,
            work_left,
            work_right,
            work_top,
        );

        // --- Create GDI resources ---
        let mut main_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let (main_dc, main_bmp) = match create_dib(phys_w, phys_h, &mut main_bits) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(Err(e));
                return;
            }
        };
        let mut tmp_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let (tmp_dc, tmp_bmp) = match create_dib(phys_w, phys_h, &mut tmp_bits) {
            Ok(v) => v,
            Err(e) => {
                DeleteObject(main_bmp.into());
                DeleteDC(main_dc);
                let _ = tx.send(Err(e));
                return;
            }
        };

        // --- Font: first family token from configured previewFontFamily cascade ---
        // Default "Inter" (system-installed on Andi's machine; GDI substitutes Segoe UI if absent).
        let font_face_null = {
            let mut v: Vec<u16> = config.font_face.encode_utf16().collect();
            v.push(0);
            v
        };
        // Font height includes the accessibility text-scale (line-height matches it in
        // render_frame) so the native text tracks the Settings webview card 1:1.
        let font_h = (config.font_px as f64 * scale * text_scale) as i32;
        let font = create_font(PCWSTR(font_face_null.as_ptr()), font_h);

        // --- Register window class ---
        let hinstance = match GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => h,
            Err(e) => {
                DeleteObject(main_bmp.into());
                DeleteDC(main_dc);
                DeleteObject(tmp_bmp.into());
                DeleteDC(tmp_dc);
                let _ = tx.send(Err(format!("GetModuleHandleW: {e}")));
                return;
            }
        };
        let class_name = class_name_wide();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(preview_wnd_proc),
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

        // --- Build initial state ---
        let state = Box::new(PreviewWindowState {
            config,
            phys_w,
            phys_h,
            win_x,
            win_y,
            scale,
            text_scale,
            pill_x_logical,
            pill_y_logical,
            work_left,
            work_right,
            work_top,
            text_buffer: String::new(),
            armed: false,
            was_visible: false,
            main_dc,
            main_bmp,
            main_bits,
            tmp_dc,
            tmp_bmp,
            tmp_bits,
            font,
        });
        let state_ptr = Box::into_raw(state);

        // --- Create the layered window (hidden at start) ---
        // WS_EX_TRANSPARENT: click-through (AC-3); no drag handling needed.
        let ex_style = WS_EX_LAYERED
            | WS_EX_TOPMOST
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE
            | WS_EX_TRANSPARENT;
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

        let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), Some(hinstance.into()));
    }
}
