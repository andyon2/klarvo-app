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
//!
//! ## Message mode (Story 7-10, AC8)
//!
//! The same card is also Klarvo's **message surface**. Andi's GATE-4 round 1 on
//! build `d73082d` showed why: the 200×36 pill cut `Model 'deepseek-typo' not
//! found — in clipboard` mid-sentence, so the user never learned where the text
//! had gone. A status light cannot carry a three-part sentence.
//!
//! When [`NativePreview::set_message`] delivers an
//! [`OverlayMessage`](crate::overlay_message::OverlayMessage) the card switches
//! into message mode: header · cause (model ID on an amber chip) · where the
//! text is · what to check, over an amber or danger border. This happens
//! **independently of `live_preview_enabled`** — the window exists for the
//! message even when the user never turned live preview on — and if a live
//! preview *is* running, the message replaces its text in the same card.
//! The card holds 4 s, fades over 1 s, and a new recording or a click dismisses
//! it at once — nothing else does, not even the state event that follows the
//! message microseconds later (AC8 review, P1).
//!
//! The card sits 8 px above the pill and grows upward. When the pill is dragged
//! so high that the card no longer fits above it, the whole window flips 8 px
//! **below** the pill and the card hugs it from there — live preview included
//! (AC8 review, D1; see [`compute_preview_geometry`]).

#![cfg(target_os = "windows")]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::mem::size_of;
use std::sync::mpsc;
use std::time::Instant;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::overlay_message::{MessageTone, OverlayMessage};

// ---------------------------------------------------------------------------
// Custom messages (WM_APP range: 0x8100-0x81FF — non-overlapping with pill)
// ---------------------------------------------------------------------------
const WM_PREVIEW_SET_STATE: u32 = 0x8101; // WPARAM=state_code (u8)
const WM_PREVIEW_APPEND_CHUNK: u32 = 0x8102; // WPARAM=ptr to Box<String>
const WM_PREVIEW_SET_PILL_POS: u32 = 0x8103; // WPARAM=x_bits (f64::to_bits), LPARAM=y_bits
const WM_PREVIEW_SET_MESSAGE: u32 = 0x8104; // WPARAM=ptr to Box<Option<OverlayMessage>>
const WM_PREVIEW_SHUTDOWN: u32 = 0x8110;

// Layout constants (logical px)
const OUTER_INSET: f32 = 2.0; // keep border inside DIB at fractional DPI
const INNER_PAD_TB: f32 = 8.0; // top/bottom inner padding
const INNER_PAD_LR: f32 = 12.0; // left/right inner padding (matches SOLL `padding: 8px 12px`)
const FADE_FRACTION: f32 = 0.18; // top-fade fraction of card height on overflow
const PILL_WIDTH_LOGICAL: f64 = 200.0; // must match PILL_W in native_pill.rs
const PILL_HEIGHT_LOGICAL: f64 = 36.0; // must match PILL_H in native_pill.rs
const GAP_LOGICAL: f64 = 8.0; // gap between the preview and the pill (either side)

// Geometry presets (logical px)
const BASE_FONT_PX: f64 = 11.0;
const BASE_MAX_HEIGHT: f64 = 600.0;

// ---------------------------------------------------------------------------
// Message mode (Story 7-10 AC8) — timing, layout, canon colours
// ---------------------------------------------------------------------------

const TIMER_MESSAGE: usize = 1;
const TIMER_MS: u32 = 33; // ~30 fps, matching the pill's animation timer
/// Fully opaque for this long (canon mock: same 4 s beat as the pill).
const MSG_HOLD_MS: u128 = 4000;
/// …then fades out over this long. Total lifetime = HOLD + FADE.
const MSG_FADE_MS: u128 = 1000;

// Card metrics, logical px, read off the approved render
// (`docs/design/overhaul/mockup-7-10-message-card.html`, `.card`).
const MSG_PAD_LR: f32 = 14.0;
const MSG_HEAD_PAD_TB: f32 = 9.0;
const MSG_BODY_PAD_TOP: f32 = 12.0;
const MSG_BODY_PAD_BOTTOM: f32 = 13.0;
const MSG_LINE_GAP: f32 = 6.0;
const MSG_DOT_SIZE: f32 = 7.0;
const MSG_DOT_GAP: f32 = 8.0;
const MSG_HEADER_PX: f32 = 10.0;
const MSG_CAUSE_PX: f32 = 13.0;
const MSG_NEXT_PX: f32 = 12.0;
const MSG_HINT_PX: f32 = 11.0;
/// `line-height: 1.45` on the cause line in the render; applied to every line so
/// the card's rhythm does not depend on `previewLineSpacing` (which is a
/// live-preview setting, not a message setting).
const MSG_LINE_MULT: f32 = 1.45;
const MSG_RADIUS: f32 = 16.0; // canon --k-r-lg
const MSG_BORDER_W: f32 = 1.0;
const MSG_CHIP_PAD_X: f32 = 5.0;
const MSG_CHIP_RADIUS: f32 = 4.0;
/// Chip height as a fraction of the cause line's box — the render's
/// `1.25 / MSG_LINE_MULT`. Taken off `cause_h` rather than off the font size so
/// the rect tracks the accessibility text scale the glyphs inside it already
/// honour (AC8 review, P7).
const MSG_CHIP_H_FRACTION: f32 = 1.25 / MSG_LINE_MULT;
/// Mono face for the header and the model-ID chip.
///
/// **Nothing mono is bundled** — the app embeds exactly three faces, all Geist
/// sans (`native_pill`'s `load_embedded_font` calls). Consolas is a stock
/// Windows face and therefore the one mono we can name and actually get; the
/// canon's `ui-monospace` cascade resolves to it on Windows too.
const MSG_MONO_FACE: &str = "Consolas";
/// Sans face for cause / next / hint.
///
/// Geist is the app's typeface and the one the pill beside the card draws with
/// (canon, ADR-0019). It is not installed on the machine — it is registered
/// **process-wide** from memory by `AddFontMemResourceEx`, so this window must
/// register it itself before creating its fonts: the preview window is created
/// at `lib::run` setup, which can run before the pill thread has started.
/// `preview_thread` does exactly that (see `load_embedded_geist`).
const MSG_SANS_FACE: &str = "Geist";

// Canon tokens, verbatim from `docs/design/overhaul/source/assets/klarvo.css`.
const C_AMBER: (u8, u8, u8) = (233, 162, 76); // --k-amber
const C_AMBER_HI: (u8, u8, u8) = (244, 186, 114); // --k-amber-hi
const C_DANGER: (u8, u8, u8) = (238, 111, 99); // --k-danger
const C_TEXT: (u8, u8, u8) = (236, 238, 239); // --k-text
const C_MUTED: (u8, u8, u8) = (164, 169, 172); // --k-muted
const C_DIM: (u8, u8, u8) = (111, 116, 121); // --k-dim
const C_HAIRLINE: (u8, u8, u8) = (40, 44, 47); // --k-border
/// The message card's own background — the approved render's
/// `rgba(14,16,18,.82)`, **not** the user's `previewBackground` (AC8 review, P2).
///
/// Every text run on this card is a fixed canon dark-theme colour, so the
/// shipped "Light" preview theme (`previewAppearance.ts::PREVIEW_THEMES`) would
/// render the failure message near-white on near-white. The border is pinned for
/// the same reason; so are the radius and the border width. A message is not the
/// user's preview text — it must read identically on every machine.
const MSG_BG: (u8, u8, u8) = (14, 16, 18);
const MSG_BG_ALPHA: f32 = 0.82;
/// `--k-amber-line` / `--k-danger` at 0.32 — the card's border alpha.
const MSG_LINE_ALPHA: f32 = 0.32;
/// `--k-amber-bg` at 0.12 — the model-ID chip's fill.
const MSG_CHIP_ALPHA: f32 = 0.12;

/// Height of a **four-line** message card (header · cause · next · hint) plus the
/// window's outer inset, in logical px — a **lower bound** for today's cards, and
/// the threshold that decides whether the window flips below the pill (AC8
/// re-review, Andi's directive DN1).
///
/// Summed from the constants `render_message_card` lays out with, so it tracks
/// those constants — not the drawn card: `msg_line_h`'s rounding, the
/// accessibility text scale and **line wrapping** (a long cause or the clipboard
/// line wraps in the compact form, ≈144 px) are left out. This is a placement
/// threshold, not a layout, and it must stay a constant the geometry pass can
/// compare against before any font exists. ≈ 126.7 logical px today. A pill
/// with slightly more room above it than this constant keeps a wrapped card
/// above and clamps it — residual, see the story's final re-review.
const MIN_CARD_H_LOGICAL: f64 = 2.0 * OUTER_INSET as f64
    + 2.0 * MSG_HEAD_PAD_TB as f64
    + (MSG_HEADER_PX * MSG_LINE_MULT) as f64
    + MSG_BORDER_W as f64
    + MSG_BODY_PAD_TOP as f64
    + MSG_BODY_PAD_BOTTOM as f64
    + (MSG_CAUSE_PX * MSG_LINE_MULT) as f64
    + MSG_LINE_GAP as f64
    + (MSG_NEXT_PX * MSG_LINE_MULT) as f64
    + MSG_LINE_GAP as f64
    + (MSG_HINT_PX * MSG_LINE_MULT) as f64;

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
    pub line_height_mult: f32, // 1.35 | 1.625 | 1.925 (from previewLineSpacing small/medium/large)
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
        // until the user touches the control. "large" keeps the full symmetric +0.30 em
        // offset. This GDI line-stepping multiplies the *font size* (like CSS
        // `lineHeight`), while Android's `setLineSpacing(0f, mult)` multiplies the font's
        // *natural line height* (~1.2× text size) — so Desktop's up-step uses +0.30 and
        // Android uses +0.25 (see `ListeningPanelView.kt`'s `LINE_SPACING_MULT`) to move
        // the same ±0.30 em on both platforms.
        // "small" is raised to 1.35 (a -0.275 em down-step, asymmetric against the +0.30
        // up-step; R-D1, 2026-08-10): the default `previewFontFamily` resolves to Segoe UI
        // (`:155`), whose natural line cell is ~1.330 em, and `DrawTextW` (`:773-778`) draws
        // with `DT_SINGLELINE | DT_VCENTER` WITHOUT `DT_NOCLIP` into a rect exactly
        // `line_h` high — a multiplier below ~1.330 risks clipping diacritics/descenders.
        // 1.35 keeps headroom above that cell. To be confirmed at GATE-4 on a real
        // Windows build.
        let line_height_mult = match cfg.preview_line_spacing.as_str() {
            "small" => 1.35f32,
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
    work_bottom: i32,
    // True when the window sits BELOW the pill because the card did not fit
    // above it (AC8 review, D1). The card then hugs the pill from below
    // (top-aligned) instead of from above (bottom-aligned).
    below_pill: bool,
    // Render state
    text_buffer: String,
    armed: bool,       // true when Recording received and live_preview_enabled
    was_visible: bool, // tracks hidden→visible edge for topmost re-assert
    // Message mode (Story 7-10 AC8). `message` takes precedence over the live
    // preview text and is independent of `config.live_preview_enabled`;
    // `message_at` starts the 4 s hold + 1 s fade the timer drives.
    message: Option<OverlayMessage>,
    message_at: Option<Instant>,
    msg_timer_active: bool,
    // GDI resources
    main_dc: HDC,
    main_bmp: HBITMAP,
    main_bits: *mut core::ffi::c_void,
    tmp_dc: HDC,
    tmp_bmp: HBITMAP,
    tmp_bits: *mut core::ffi::c_void,
    font: HFONT,
    // Message-card fonts. Fixed sizes from the canon render — deliberately NOT
    // the user's live-preview appearance settings: a failure message must read
    // the same on every machine.
    font_msg_header: HFONT,
    font_msg_cause: HFONT,
    font_msg_chip: HFONT,
    font_msg_next: HFONT,
    font_msg_hint: HFONT,
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

    /// Show (or clear) the message card — Story 7-10, AC8.
    ///
    /// `Some(msg)` puts the card into message mode and restarts its 4 s hold;
    /// `None` dismisses it. Callers post this **before** the matching
    /// `set_state` (PostMessage is FIFO per window), so the state that arrives
    /// with a message never hides the card it just opened, and a `Recording`
    /// state arrives after its own `None` and finds the card already cleared.
    ///
    /// Independent of `live_preview_enabled`: the window is created for every
    /// session, the live text is what that setting gates.
    pub fn set_message(&self, msg: Option<OverlayMessage>) {
        let ptr = Box::into_raw(Box::new(msg));
        unsafe {
            if PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_PREVIEW_SET_MESSAGE,
                WPARAM(ptr as usize),
                LPARAM(0),
            )
            .is_err()
            {
                // Window gone — free the box to avoid a leak.
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
/// caller positions each line at a `font_px × scale × text_scale × line_height_mult` step
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
/// Returns `(win_x_phys, win_y_phys, phys_w, phys_h, below_pill)`.
///
/// ## Above or below the pill (AC8 review, directive D1)
///
/// The card normally sits `GAP_LOGICAL` above the pill and grows upward. With
/// the pill dragged to the top of the work area there is nothing left above it:
/// the height collapsed to 1 px and **every** pipeline message was silently lost
/// — before AC8 the message lived on the pill, which always exists, so the
/// defect arrived with the card.
///
/// Andi's call (2026-09-14): when there is less room above than the card needs,
/// the card appears 8 px **below** the pill instead — and the live preview does
/// the same in that position, because it is the same window and the same card.
/// `below_pill` tells the renderer to hug the pill from below (top-aligned)
/// instead of from above (bottom-aligned).
///
/// **What "the card needs" is** ([`MIN_CARD_H_LOGICAL`], directive DN1): one card
/// height — the four-line card, ≈127 logical px — *not* `h_wanted`, the window's
/// 600 px upper bound. The first implementation compared against `h_wanted` and
/// thereby flipped the window for roughly the upper half of the screen, moving
/// the **live preview** (which has always grown upward) below the pill at pill
/// positions with three times the room the card asks for. The window may still be
/// clamped to whatever height is actually available; only the *side* is decided
/// here, and it is the top-edge case the directive names.
///
/// One arithmetic guard: if flipping would yield *less* usable height than
/// staying above, we stay above. Moving the card into an even smaller box would
/// defeat the directive it implements.
unsafe fn compute_preview_geometry(
    pill_x_logical: f64,
    pill_y_logical: f64,
    config: &PreviewConfig,
    scale: f64,
    work_left: i32,
    work_right: i32,
    work_top: i32,
    work_bottom: i32,
) -> (i32, i32, i32, i32, bool) {
    let k = config.font_px as f64 / BASE_FONT_PX;
    let w_logical = (config.w_base as f64 * k).round() as i32;
    let h_wanted = (BASE_MAX_HEIGHT * k).round();
    // Vertical room: between work-area top + 12 and pill - gap - 12 above,
    // between pill bottom + gap and work-area bottom - 12 below.
    let avail_above = (pill_y_logical - GAP_LOGICAL - work_top as f64 / scale - 12.0).max(0.0);
    let avail_below = (work_bottom as f64 / scale
        - (pill_y_logical + PILL_HEIGHT_LOGICAL)
        - GAP_LOGICAL
        - 12.0)
        .max(0.0);
    let below_pill = avail_above < MIN_CARD_H_LOGICAL && avail_below > avail_above;
    let h_max_logical = h_wanted.min(if below_pill { avail_below } else { avail_above }) as i32;

    let pill_center_x = pill_x_logical + PILL_WIDTH_LOGICAL / 2.0;
    let preview_left_raw = pill_center_x - w_logical as f64 / 2.0;
    let work_left_logical = work_left as f64 / scale;
    let work_right_logical = work_right as f64 / scale;
    let preview_left = preview_left_raw
        .max(work_left_logical + 12.0)
        .min(work_right_logical - w_logical as f64 - 12.0);
    let preview_top = if below_pill {
        pill_y_logical + PILL_HEIGHT_LOGICAL + GAP_LOGICAL
    } else {
        pill_y_logical - GAP_LOGICAL - h_max_logical as f64
    };

    let phys_w = (w_logical as f64 * scale) as i32;
    let phys_h = (h_max_logical as f64 * scale) as i32;
    let win_x = (preview_left * scale) as i32;
    let win_y = (preview_top * scale) as i32;
    (win_x, win_y, phys_w.max(1), phys_h.max(1), below_pill)
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
// Message-card helpers (Story 7-10 AC8)
// ---------------------------------------------------------------------------

/// Width of `text` in physical px with the font currently selected into `dc`.
unsafe fn text_width(dc: HDC, text: &str) -> i32 {
    let wide: Vec<u16> = text.encode_utf16().collect();
    if wide.is_empty() {
        return 0;
    }
    let mut sz = SIZE::default();
    if GetTextExtentPoint32W(dc, &wide, &mut sz).as_bool() {
        sz.cx
    } else {
        0
    }
}

/// Draw one already-wrapped line into its own `h`-tall box, vertically centred
/// (the CSS line-height behaviour the live preview also uses).
unsafe fn draw_msg_line(dc: HDC, line: &[u16], left: i32, top: i32, right: i32, h: i32) {
    if line.is_empty() {
        return;
    }
    let mut rect = RECT { left, top, right, bottom: top + h };
    let mut buf = line.to_vec();
    DrawTextW(dc, &mut buf, &mut rect, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
}

/// Fill a rounded rect into the pixmap with straight (non-premultiplied) RGB.
fn fill_round_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    rgb: (u8, u8, u8),
    alpha: f32,
) {
    let Some(path) = round_rect_path(x, y, w, h, radius) else {
        return;
    };
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.shader = Shader::SolidColor(
        Color::from_rgba(
            rgb.0 as f32 / 255.0,
            rgb.1 as f32 / 255.0,
            rgb.2 as f32 / 255.0,
            alpha,
        )
        .unwrap_or(Color::BLACK),
    );
    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

/// The message card's line-step for a given logical font size, in physical px.
fn msg_line_h(px: f32, sc: f32, text_scale: f64) -> i32 {
    (px * sc * text_scale as f32 * MSG_LINE_MULT).round().max(1.0) as i32
}

/// Toggle the window's click-through bit.
///
/// The preview is created `WS_EX_TRANSPARENT` so it never eats a click meant for
/// the app underneath. AC8 asks for "a click on the card dismisses it at once",
/// which requires the opposite for the ~5 s a card is up — so the bit is cleared
/// while a message is shown and restored the moment it is dismissed.
unsafe fn set_click_through(hwnd: HWND, on: bool) {
    let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let bit = WS_EX_TRANSPARENT.0 as isize;
    let next = if on { cur | bit } else { cur & !bit };
    if next != cur {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
    }
}

/// Clear the message, stop its timer and hand clicks back to the app below.
unsafe fn dismiss_message(hwnd: HWND, s: &mut PreviewWindowState) {
    s.message = None;
    s.message_at = None;
    if s.msg_timer_active {
        let _ = KillTimer(Some(hwnd), TIMER_MESSAGE);
        s.msg_timer_active = false;
    }
    set_click_through(hwnd, true);
}

/// Render the message card. Returns the layered-window alpha (0-255); `0` means
/// the fade has finished and the caller should dismiss.
///
/// Lays out header · cause · next · hint hugging the pill, exactly like the
/// live-preview card, so the message appears where the preview text would have
/// been (AC8: "If live preview is active, the message replaces the preview text
/// in the same card") — above it normally, below it when nothing fit above
/// (`below_pill`, D1).
unsafe fn render_message_card(s: &mut PreviewWindowState) -> u8 {
    let Some(msg) = s.message.clone() else { return 0 };
    let pw = s.phys_w;
    let ph = s.phys_h;
    let sc = s.scale as f32;
    let byte_count = (pw * ph) as usize * 4;

    // --- Fade schedule ---
    let elapsed = s.message_at.map(|t| t.elapsed().as_millis()).unwrap_or(0);
    let alpha: u8 = if elapsed < MSG_HOLD_MS {
        255
    } else if elapsed < MSG_HOLD_MS + MSG_FADE_MS {
        let k = (elapsed - MSG_HOLD_MS) as f32 / MSG_FADE_MS as f32;
        (255.0 * (1.0 - k)).round().clamp(0.0, 255.0) as u8
    } else {
        return 0;
    };

    // The tone colours the border and the status dot. The header *text* stays
    // `--k-dim` on both tones — that is what the approved render shows
    // (`.card .head .t { color: var(--k-dim) }`); the colour signal is the dot
    // and the line, not the word.
    let tone_rgb = match msg.tone {
        MessageTone::Warning => C_AMBER,
        MessageTone::Error => C_DANGER,
    };

    // --- 1. Measure. The tmp DIB doubles as the measuring context, as in the
    // live-preview path: zero it, set the text colour once, then select a font
    // per run. ---
    core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
    SetTextColor(s.tmp_dc, COLORREF(0x00FFFFFF));
    SetBkMode(s.tmp_dc, TRANSPARENT);

    let inset = OUTER_INSET * sc;
    let inner_left = (inset + MSG_PAD_LR * sc) as i32;
    let inner_right = (pw as f32 - inset - MSG_PAD_LR * sc) as i32;
    let text_area_w = (inner_right - inner_left).max(1);

    let header_h = msg_line_h(MSG_HEADER_PX, sc, s.text_scale);
    let cause_h = msg_line_h(MSG_CAUSE_PX, sc, s.text_scale);
    let next_h = msg_line_h(MSG_NEXT_PX, sc, s.text_scale);
    let hint_h = msg_line_h(MSG_HINT_PX, sc, s.text_scale);

    // Cause: keep the chip layout only while the whole line fits on one line.
    // A wrapped chip would need per-run wrapping the card does not warrant, so
    // the fallback is the plain reassembled sentence (`CauseLine::text`).
    SelectObject(s.tmp_dc, s.font_msg_cause.into());
    let cause_text = msg.cause.text();
    let mut chip_layout: Option<(String, String, String)> = None;
    if let Some(chip) = msg.cause.chip.as_deref() {
        let before_w = text_width(s.tmp_dc, &msg.cause.before);
        let after_w = text_width(s.tmp_dc, &msg.cause.after);
        SelectObject(s.tmp_dc, s.font_msg_chip.into());
        // Same padding the chip rect is drawn with below, text scale included —
        // otherwise the fit test and the rect disagree (AC8 review, P7).
        let chip_w =
            text_width(s.tmp_dc, chip) + 2 * (MSG_CHIP_PAD_X * sc * s.text_scale as f32) as i32;
        SelectObject(s.tmp_dc, s.font_msg_cause.into());
        if before_w + chip_w + after_w <= text_area_w {
            chip_layout = Some((
                msg.cause.before.clone(),
                chip.to_string(),
                msg.cause.after.clone(),
            ));
        }
    }
    // Always wrapped, so the plain fallback is ready even if the chip layout is
    // chosen; the chip form is a single line by construction (it was only chosen
    // because the whole line fits).
    let cause_lines: Vec<Vec<u16>> = wrap_text_lines(s.tmp_dc, &cause_text, text_area_w);
    let cause_line_count = if chip_layout.is_some() { 1 } else { cause_lines.len() as i32 };

    SelectObject(s.tmp_dc, s.font_msg_next.into());
    let next_lines: Vec<Vec<u16>> = msg
        .next
        .as_deref()
        .map(|t| wrap_text_lines(s.tmp_dc, t, text_area_w))
        .unwrap_or_default();

    SelectObject(s.tmp_dc, s.font_msg_hint.into());
    let hint_lines: Vec<Vec<u16>> = msg
        .hint
        .as_deref()
        .map(|t| wrap_text_lines(s.tmp_dc, t, text_area_w))
        .unwrap_or_default();

    // --- 2. Card geometry: content height, bottom-aligned (hugs the pill) ---
    let head_block = (2.0 * MSG_HEAD_PAD_TB * sc) as i32 + header_h.max((MSG_DOT_SIZE * sc) as i32);
    let divider_h = MSG_BORDER_W * sc;
    let mut body_h = (MSG_BODY_PAD_TOP * sc + MSG_BODY_PAD_BOTTOM * sc) as i32;
    body_h += cause_line_count * cause_h;
    if !next_lines.is_empty() {
        body_h += (MSG_LINE_GAP * sc) as i32 + next_lines.len() as i32 * next_h;
    }
    if !hint_lines.is_empty() {
        body_h += (MSG_LINE_GAP * sc) as i32 + hint_lines.len() as i32 * hint_h;
    }

    let card_x = inset;
    let card_w = pw as f32 - 2.0 * inset;
    let max_card_h = ph as f32 - 2.0 * inset;
    let card_h = (head_block as f32 + divider_h + body_h as f32).min(max_card_h);
    // Hug the pill: bottom-aligned above it, top-aligned below it (D1).
    let card_y = if s.below_pill { inset } else { (ph as f32 - inset) - card_h };

    // --- 3. Shapes: card, border, header divider, status dot, model chip ---
    let Some(mut pixmap) = Pixmap::new(pw as u32, ph as u32) else {
        // AC8 review, P10: returning the fade alpha here made the caller
        // `present()` the DIB it still held — the previous frame, or whatever
        // the live preview last drew — as if it were the message. `0` is the
        // caller's dismiss signal, which is the honest outcome: we could not
        // draw the card, so no card is shown.
        log::warn!("[native_preview] Pixmap::new({pw},{ph}) failed — dismissing the message card");
        return 0;
    };
    fill_round_rect(
        &mut pixmap,
        card_x,
        card_y,
        card_w,
        card_h,
        MSG_RADIUS * sc,
        MSG_BG,
        MSG_BG_ALPHA,
    );
    {
        // The border is the card's tone. Width is fixed at 1 px rather than the
        // user's `previewBorderWidth`: AC8 pins an amber (or danger) line, and a
        // configured 0 would erase it. Background, radius and border width are
        // all pinned to the approved render for the same reason — see MSG_BG.
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = Shader::SolidColor(
            Color::from_rgba(
                tone_rgb.0 as f32 / 255.0,
                tone_rgb.1 as f32 / 255.0,
                tone_rgb.2 as f32 / 255.0,
                MSG_LINE_ALPHA,
            )
            .unwrap_or(Color::WHITE),
        );
        let mut stroke = Stroke::default();
        stroke.width = MSG_BORDER_W * sc;
        if let Some(path) = round_rect_path(card_x, card_y, card_w, card_h, MSG_RADIUS * sc) {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
    let head_bottom = card_y + head_block as f32;
    fill_round_rect(
        &mut pixmap,
        card_x,
        head_bottom,
        card_w,
        divider_h,
        0.0,
        C_HAIRLINE,
        1.0,
    );
    // Status dot, vertically centred in the header block.
    let dot = MSG_DOT_SIZE * sc;
    fill_round_rect(
        &mut pixmap,
        inner_left as f32,
        card_y + (head_block as f32 - dot) / 2.0,
        dot,
        dot,
        dot / 2.0,
        tone_rgb,
        1.0,
    );

    // Body line tops, computed once so shapes and text agree.
    let body_top = head_bottom + divider_h + MSG_BODY_PAD_TOP * sc;
    let cause_top = body_top as i32;
    let next_top = cause_top + cause_line_count * cause_h + (MSG_LINE_GAP * sc) as i32;
    let hint_top = if next_lines.is_empty() {
        next_top
    } else {
        next_top + next_lines.len() as i32 * next_h + (MSG_LINE_GAP * sc) as i32
    };

    // The chip sits behind the model ID; measure again with the same fonts so
    // the rect and the glyphs cannot drift apart.
    let mut chip_geom: Option<(i32, i32, i32)> = None; // (chip_text_x, rect_x, rect_w)
    if let Some((before, chip, _after)) = chip_layout.as_ref() {
        SelectObject(s.tmp_dc, s.font_msg_cause.into());
        let before_w = text_width(s.tmp_dc, before);
        SelectObject(s.tmp_dc, s.font_msg_chip.into());
        let chip_text_w = text_width(s.tmp_dc, chip);
        // AC8 review, P7: the chip's box is derived from `cause_h` — the line
        // step that already carries DPI **and** the accessibility text scale —
        // not from `MSG_CAUSE_PX * sc`, which ignored the text scale while the
        // glyphs inside honoured it. At 225% the rect stayed put and the ID grew
        // out of it. `MSG_CHIP_H_FRACTION` reproduces the render's chip:line
        // ratio (1.25 / 1.45 of the line box).
        let pad = (MSG_CHIP_PAD_X * sc * s.text_scale as f32) as i32;
        let rect_x = inner_left + before_w;
        let rect_w = chip_text_w + 2 * pad;
        let chip_h = (cause_h as f32 * MSG_CHIP_H_FRACTION).max(1.0);
        fill_round_rect(
            &mut pixmap,
            rect_x as f32,
            cause_top as f32 + (cause_h as f32 - chip_h) / 2.0,
            rect_w as f32,
            chip_h,
            MSG_CHIP_RADIUS * sc,
            C_AMBER,
            MSG_CHIP_ALPHA,
        );
        chip_geom = Some((rect_x + pad, rect_x, rect_w));
    }

    copy_rgba_to_bgra(&pixmap, s.main_bits as *mut u8, byte_count);

    // --- 4. Text runs. One colour per run, so the tmp DIB is cleared between
    // them (same pattern as native_pill's label compositing). ---
    let paint_run = |font: HFONT, lines: &[Vec<u16>], top: i32, h: i32, rgb: (u8, u8, u8)| {
        SelectObject(s.tmp_dc, font.into());
        for (i, line) in lines.iter().enumerate() {
            draw_msg_line(s.tmp_dc, line, inner_left, top + i as i32 * h, inner_right, h);
        }
        composite_text_mask(
            s.tmp_bits as *const u8,
            s.main_bits as *mut u8,
            pw,
            ph,
            rgb.0,
            rgb.1,
            rgb.2,
            255,
        );
        core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
    };

    // Header — mono, uppercase, dim, offset past the status dot.
    {
        SelectObject(s.tmp_dc, s.font_msg_header.into());
        let header_left = inner_left + ((MSG_DOT_SIZE + MSG_DOT_GAP) * sc) as i32;
        let wide: Vec<u16> = msg.header.encode_utf16().collect();
        draw_msg_line(
            s.tmp_dc,
            &wide,
            header_left,
            card_y as i32 + ((head_block - header_h) / 2).max(0),
            inner_right,
            header_h,
        );
        composite_text_mask(
            s.tmp_bits as *const u8,
            s.main_bits as *mut u8,
            pw,
            ph,
            C_DIM.0,
            C_DIM.1,
            C_DIM.2,
            255,
        );
        core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
    }

    // Cause — either three runs around the chip, or plain wrapped lines.
    match (chip_layout.as_ref(), chip_geom) {
        (Some((before, chip, after)), Some((chip_text_x, rect_x, rect_w))) => {
            SelectObject(s.tmp_dc, s.font_msg_cause.into());
            let before_wide: Vec<u16> = before.encode_utf16().collect();
            draw_msg_line(s.tmp_dc, &before_wide, inner_left, cause_top, rect_x, cause_h);
            let after_wide: Vec<u16> = after.encode_utf16().collect();
            draw_msg_line(
                s.tmp_dc,
                &after_wide,
                rect_x + rect_w,
                cause_top,
                inner_right,
                cause_h,
            );
            composite_text_mask(
                s.tmp_bits as *const u8,
                s.main_bits as *mut u8,
                pw,
                ph,
                C_TEXT.0,
                C_TEXT.1,
                C_TEXT.2,
                255,
            );
            core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);

            SelectObject(s.tmp_dc, s.font_msg_chip.into());
            let chip_wide: Vec<u16> = chip.encode_utf16().collect();
            draw_msg_line(s.tmp_dc, &chip_wide, chip_text_x, cause_top, inner_right, cause_h);
            composite_text_mask(
                s.tmp_bits as *const u8,
                s.main_bits as *mut u8,
                pw,
                ph,
                C_AMBER_HI.0,
                C_AMBER_HI.1,
                C_AMBER_HI.2,
                255,
            );
            core::ptr::write_bytes(s.tmp_bits as *mut u8, 0u8, byte_count);
        }
        _ => paint_run(s.font_msg_cause, cause_lines.as_slice(), cause_top, cause_h, C_TEXT),
    }

    if !next_lines.is_empty() {
        paint_run(s.font_msg_next, next_lines.as_slice(), next_top, next_h, C_MUTED);
    }
    if !hint_lines.is_empty() {
        paint_run(s.font_msg_hint, hint_lines.as_slice(), hint_top, hint_h, C_DIM);
    }

    alpha
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

unsafe fn render_frame(hwnd: HWND, s: &mut PreviewWindowState) {
    let pw = s.phys_w;
    let ph = s.phys_h;
    let sc = s.scale as f32;

    // Story 7-10 AC8: a message outranks the live preview text and does not ask
    // whether live preview is enabled at all.
    if s.message.is_some() {
        let alpha = render_message_card(s);
        if alpha == 0 {
            dismiss_message(hwnd, s);
            ShowWindow(hwnd, SW_HIDE);
            s.was_visible = false;
            return;
        }
        present(hwnd, s, alpha);
        return;
    }

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
    // Hug the pill: bottom-aligned above it (grow-up), top-aligned when the
    // window sits below it because nothing fit above (D1) — the newest line
    // stays at the card's growing edge either way.
    let card_y = if s.below_pill { inset } else { (ph as f32 - inset) - card_h };
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
    present(hwnd, s, 255);
}

/// Present the composed BGRA DIB and show the window.
///
/// `alpha` is the layered window's `SourceConstantAlpha` — 255 for the live
/// preview, and the fade ramp for a message card (Story 7-10 AC8). Factored out
/// of `render_frame` so both paths share one present + topmost-re-assert.
unsafe fn present(hwnd: HWND, s: &mut PreviewWindowState, alpha: u8) {
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: alpha,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let pt_src = POINT { x: 0, y: 0 };
    let pt_dst = POINT { x: s.win_x, y: s.win_y };
    let sz = SIZE { cx: s.phys_w, cy: s.phys_h };

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
                // A new recording dismisses any message card at once (AC8) —
                // and this arm is now the ONLY thing that does it on that route.
                // P1's hold guard makes the `set_message(None)` posted just
                // before this a no-op while the card is inside its 4 s hold, so
                // the premise the old comment rested on ("belt to that braces")
                // is gone. `dismiss_message` only clears state; without the hide
                // below, a card opened seconds earlier stays painted for the
                // whole record → STT → cleanup cycle whenever live preview is
                // off — AC8's headline case (AC8 re-review, item 1).
                let had_card = s.message.is_some();
                dismiss_message(hwnd, s);
                // Arm only if live preview is enabled in the config snapshot
                if s.config.live_preview_enabled {
                    s.armed = true;
                    s.text_buffer.clear();
                } else {
                    // live preview disabled: never show the live text
                    s.armed = false;
                }
                if had_card {
                    // Mirrors the terminal branch below: the card is gone and
                    // there is no live text yet, so the window comes off the
                    // screen. `render_frame` would reach the same `SW_HIDE` via
                    // its `!armed || text_buffer.is_empty()` guard — this states
                    // it without a full measure-and-draw pass.
                    ShowWindow(hwnd, SW_HIDE);
                    s.was_visible = false;
                }
            } else {
                // Done / Idle / Error / Warning — the live preview is over.
                s.armed = false;
                s.text_buffer.clear();
                // Story 7-10 AC8: do NOT hide when this state brought a message
                // with it. `set_message` is posted first (FIFO), so the card is
                // already up and this state must leave it alone.
                if s.message.is_some() {
                    render_frame(hwnd, s);
                } else {
                    ShowWindow(hwnd, SW_HIDE);
                    s.was_visible = false;
                }
            }
            LRESULT(0)
        }

        WM_PREVIEW_SET_MESSAGE => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            // Caller allocated Box<Option<OverlayMessage>> via into_raw.
            let msg = *Box::from_raw(wparam.0 as *mut Option<OverlayMessage>);
            match msg {
                Some(m) => {
                    s.message = Some(m);
                    s.message_at = Some(Instant::now());
                    if !s.msg_timer_active {
                        // AC8 review, P9: the timer is the only thing that fades
                        // and finally dismisses the card. If it cannot be armed
                        // the card stays up forever — and a card that is not
                        // click-through and never goes away swallows every click
                        // meant for the app underneath. So on failure we keep
                        // the click-through bit and let the card be a passive
                        // (if stuck) overlay rather than a trap.
                        if SetTimer(Some(hwnd), TIMER_MESSAGE, TIMER_MS, None) == 0 {
                            log::warn!(
                                "[native_preview] SetTimer(TIMER_MESSAGE) failed (last error {:?}) \
                                 — the card will not fade and stays click-through",
                                GetLastError()
                            );
                        } else {
                            s.msg_timer_active = true;
                        }
                    }
                    if s.msg_timer_active {
                        // Clicks dismiss the card, so it has to stop being
                        // click-through for as long as it is up.
                        set_click_through(hwnd, false);
                    }
                    render_frame(hwnd, s);
                }
                None => {
                    // AC8 review, P1: a message-less event must NOT take a live
                    // card down. `lib::emit_pipeline_state` posts `set_message`
                    // for EVERY event, and the state that follows a warning
                    // (Transcribing after the STT-ladder warning, Idle after a
                    // boot-time config warning) arrives microseconds later — so
                    // this arm used to erase the card before it could be read.
                    // Only three things dismiss a card: a new recording
                    // (`WM_PREVIEW_SET_STATE`), a click on it (`WM_LBUTTONDOWN`)
                    // and its own timer.
                    let within_hold = s
                        .message_at
                        .map(|t| t.elapsed().as_millis() < MSG_HOLD_MS)
                        .unwrap_or(false);
                    if s.message.is_some() && within_hold {
                        return LRESULT(0);
                    }
                    let had = s.message.is_some();
                    dismiss_message(hwnd, s);
                    if had {
                        // Falls back to the live preview, or hides.
                        render_frame(hwnd, s);
                    }
                }
            }
            LRESULT(0)
        }

        WM_TIMER => {
            if wparam.0 == TIMER_MESSAGE && !state_ptr.is_null() {
                let s = &mut *state_ptr;
                if s.message.is_some() {
                    // render_frame runs the fade and dismisses at alpha 0.
                    render_frame(hwnd, s);
                } else {
                    dismiss_message(hwnd, s);
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if state_ptr.is_null() {
                return LRESULT(0);
            }
            let s = &mut *state_ptr;
            // AC8: a click on the card dismisses it at once. The window is
            // click-through again the moment this returns, so the next click
            // reaches the app underneath as before.
            if s.message.is_some() {
                dismiss_message(hwnd, s);
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
            // Recompute position (and which side of the pill we are on — D1)
            let (wx, wy, pw, ph, below) = compute_preview_geometry(
                new_pill_x,
                new_pill_y,
                &s.config,
                s.scale,
                s.work_left,
                s.work_right,
                s.work_top,
                s.work_bottom,
            );
            s.win_x = wx;
            s.win_y = wy;
            s.below_pill = below;
            // Only re-render (and thus reposition via UpdateLayeredWindow.pptDst)
            // if the preview is currently visible — a message card counts.
            if s.message.is_some() || (s.armed && !s.text_buffer.is_empty()) {
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
                DeleteObject(s.font_msg_header.into());
                DeleteObject(s.font_msg_cause.into());
                DeleteObject(s.font_msg_chip.into());
                DeleteObject(s.font_msg_next.into());
                DeleteObject(s.font_msg_hint.into());
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
        let work_bottom = work_area.bottom;

        // --- Default pill position (center-bottom of work area if not saved) ---
        let pill_x_logical = pill_x.unwrap_or_else(|| {
            let w = (work_area.right - work_area.left) as f64 / scale;
            (work_left as f64 / scale) + w / 2.0 - PILL_WIDTH_LOGICAL / 2.0
        });
        let pill_y_logical = pill_y.unwrap_or_else(|| {
            (work_area.bottom as f64 / scale) - 8.0 - 36.0 // 36 = pill height
        });

        // --- Compute initial geometry ---
        let (win_x, win_y, phys_w, phys_h, below_pill) = compute_preview_geometry(
            pill_x_logical,
            pill_y_logical,
            &config,
            scale,
            work_left,
            work_right,
            work_top,
            work_bottom,
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

        // --- Message-card fonts (Story 7-10 AC8) ---
        // Fixed sizes from the canon render, independent of the user's
        // live-preview appearance settings: a failure message must read the
        // same on every machine. They still honour DPI + the accessibility
        // text scale, like every other text in this window.
        //
        // Geist is bundled, not installed, so it has to be in this process's GDI
        // font table BEFORE the fonts below are selected. Since AC8 this window
        // can be created at `lib::run` setup, i.e. before the pill thread that
        // used to be the sole registrar — hence the call here (AC8 review, P3).
        crate::native_pill::load_embedded_geist();
        let mono_face = to_wide(MSG_MONO_FACE);
        let sans_face = to_wide(MSG_SANS_FACE);
        let msg_font_h = |px: f32| (px as f64 * scale * text_scale) as i32;
        let font_msg_header = create_font(PCWSTR(mono_face.as_ptr()), msg_font_h(MSG_HEADER_PX));
        let font_msg_cause = create_font(PCWSTR(sans_face.as_ptr()), msg_font_h(MSG_CAUSE_PX));
        let font_msg_chip = create_font(PCWSTR(mono_face.as_ptr()), msg_font_h(MSG_CAUSE_PX - 1.0));
        let font_msg_next = create_font(PCWSTR(sans_face.as_ptr()), msg_font_h(MSG_NEXT_PX));
        let font_msg_hint = create_font(PCWSTR(sans_face.as_ptr()), msg_font_h(MSG_HINT_PX));

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
            work_bottom,
            below_pill,
            text_buffer: String::new(),
            armed: false,
            was_visible: false,
            message: None,
            message_at: None,
            msg_timer_active: false,
            main_dc,
            main_bmp,
            main_bits,
            tmp_dc,
            tmp_bmp,
            tmp_bits,
            font,
            font_msg_header,
            font_msg_cause,
            font_msg_chip,
            font_msg_next,
            font_msg_hint,
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
