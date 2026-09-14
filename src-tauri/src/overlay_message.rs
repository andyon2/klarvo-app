//! Structured pipeline messages for the native overlay card (Story 7-10, AC8).
//!
//! ## Why this module exists
//!
//! Until GATE-4 round 1 the pipeline handed the overlay a single flat string and
//! the 200×36 pill rendered it through `native_pill::fit_text`. Andi's Windows
//! smoke on build `d73082d` showed the consequence: `Model 'deepseek-typo' not
//! found — in clipboard` was cut mid-sentence and the clipboard hint — the one
//! part that tells the user where the text actually is — never appeared. The
//! message was on the wrong surface: a status light carrying a three-part
//! sentence.
//!
//! Option A (Andi, 2026-09-14, canon + `docs/design/overhaul/mockup-7-10-message-card.html`)
//! splits the two jobs:
//!
//! - the **pill** stays a status light with **static** labels ([`PILL_LABEL_*`](PILL_LABEL_DONE));
//! - the **preview card** carries the message, laid out over several lines.
//!
//! A card needs structure, not a sentence: a header, a cause (with the model ID
//! on its own chip), where the text went, and what to check. That structure is
//! built **once**, here, from the failure the pipeline already knows about — it
//! is never re-derived by parsing the flat string back apart.
//!
//! ## Platform
//!
//! Deliberately **not** `cfg(target_os = "windows")`. The renderer
//! (`native_preview.rs`) is Windows-gated and has no test module, so the model
//! is the only part of AC8 that a Linux `cargo test --lib` can reach. What that
//! proves and what it does not is spelled out in the story's coverage statement:
//! these tests decide the **message model** — structure and wording — never that
//! a single pixel was drawn.

/// Colour family of a card. Decides the border and the header dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTone {
    /// Amber (`--k-amber` / `--k-amber-line`): the run produced output.
    Warning,
    /// Danger (`--k-danger`): the run failed.
    Error,
}

/// The cause line, split so the renderer can put the model ID on an amber chip
/// in mono (canon `.card .cause code`).
///
/// `chip` is `None` for every message that has no identifier to highlight, and
/// then `before` carries the whole line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CauseLine {
    pub before: String,
    pub chip: Option<String>,
    pub after: String,
}

impl CauseLine {
    /// A cause with nothing to highlight.
    pub fn plain(text: impl Into<String>) -> Self {
        CauseLine { before: text.into(), chip: None, after: String::new() }
    }

    /// The whole line as one string — what the renderer measures against the
    /// card width, and what a test asserts when the chip split is not the point.
    pub fn text(&self) -> String {
        match &self.chip {
            Some(chip) => format!("{}{}{}", self.before, chip, self.after),
            None => format!("{}{}", self.before, self.after),
        }
    }
}

/// One pipeline message, laid out for the native preview card.
///
/// Line order is the render order (canon mock, top to bottom): header · cause ·
/// next · hint. `next` and `hint` are optional — a plain STT-ladder warning has
/// neither, a model-not-found cleanup failure has both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayMessage {
    pub tone: MessageTone,
    /// Rendered mono + uppercase. Already uppercase here so the renderer does
    /// not need a locale-aware transform in GDI.
    pub header: String,
    pub cause: CauseLine,
    /// Where the text went and what to press. Omitted when nothing landed.
    pub next: Option<String>,
    /// What to check. Only set when there is a single place to fix it.
    pub hint: Option<String>,
}

impl OverlayMessage {
    /// A non-fatal pipeline warning with no follow-up action: the STT fallback
    /// ladder (`process_audio`'s "⚠ Groq am Limit → lokale Transkription") and
    /// the boot-time config warnings `lib::run` emits through the same funnel.
    pub fn warning(text: impl Into<String>) -> Self {
        OverlayMessage {
            tone: MessageTone::Warning,
            header: "WARNING".to_string(),
            cause: CauseLine::plain(text),
            next: None,
            hint: None,
        }
    }

    /// Several warnings on one card.
    ///
    /// `lib::run` surfaces the boot-time config warnings through the same
    /// funnel, one event each. Before AC8 they were four-second pill labels that
    /// nobody could read anyway; on the card each event replaced the previous
    /// one instantly, so only the last survived (AC8 review, P8). One card with
    /// one line per warning is the fix — `native_preview::wrap_text_lines`
    /// breaks on `\n`, so the lines lay out without any renderer change.
    ///
    /// Returns `None` for an empty list: there is no message to show.
    pub fn warnings(texts: Vec<String>) -> Option<Self> {
        if texts.is_empty() {
            return None;
        }
        Some(Self::warning(texts.join("\n")))
    }

    /// A terminal failure — danger border, no output produced.
    ///
    /// The text is run through [`user_facing_error_text`] first (AC8 review,
    /// D3): before AC8 a machine token was a truncated fragment on the pill, and
    /// the card now gives it four readable lines, so the ones we know get user
    /// wording. `PipelineEvent::error` keeps the raw token in its own `error`
    /// field — only the card is translated.
    pub fn error(text: impl Into<String>) -> Self {
        OverlayMessage {
            tone: MessageTone::Error,
            header: "ERROR".to_string(),
            cause: CauseLine::plain(user_facing_error_text(&text.into())),
            next: None,
            hint: None,
        }
    }
}

/// Machine tokens that can reach **this card**, and what it says instead.
///
/// The table is tiny because the reachable set is tiny, and the first version of
/// this comment got its provenance wrong (AC8 re-review, item 3). What the tree
/// actually shows:
///
/// - `pipeline::start_command_mode`'s licence gate emits
///   `PipelineEvent::error("feature_requires_license:CommandMode")` — the one
///   token that becomes an [`OverlayMessage`]. That is the whole table.
/// - `lib.rs`'s `require_license!` also formats `feature_requires_license:{:?}`,
///   but only inside Tauri commands and only for `WhisperMode` · `AppProfiles` ·
///   `Snippets` · `Sync` · `UnlimitedHistory` · `FillerAnalysis` · `VoiceNotes` ·
///   `AlternativeProviders`. Those are `Err(String)` returns to the frontend;
///   none of them is ever emitted as a pipeline event, so none reaches a card.
/// - `commands/whisper.rs` returns `feature_requires_license:OfflineMode` the
///   same way — a command error, never an overlay message. It had a row here and
///   the row was dead; it is gone.
///
/// Anything not listed is prose already, or a command error that never gets
/// this far.
const ERROR_TOKEN_TEXT: &[(&str, &str)] =
    &[("feature_requires_license:CommandMode", "Command mode needs a license")];

/// Translate a known machine token into user wording; leave everything else
/// **verbatim** (AC8 review, D3).
///
/// Verbatim is the deliberate fallback: a token we have not mapped is still the
/// exact string in `Klarvo.log`, which is what a support answer needs. Guessing
/// at an unknown token's meaning would be worse than showing it.
fn user_facing_error_text(raw: &str) -> String {
    ERROR_TOKEN_TEXT
        .iter()
        .find(|(token, _)| *token == raw)
        .map(|(_, text)| (*text).to_string())
        .unwrap_or_else(|| raw.to_string())
}

// ---------------------------------------------------------------------------
// Pill labels — static, never truncated (AC8)
// ---------------------------------------------------------------------------
//
// Their only production consumer is `native_pill.rs`, which is
// `cfg(target_os = "windows")` — so on a Linux build they are genuinely dead,
// hence the `allow`. They live here rather than as literals in the renderer so
// the one thing a Linux host *can* check about the status light — that every
// label is a short fixed string — is checked by
// `spec_pill_labels_are_static_literals` below.

/// Paste succeeded.
#[allow(dead_code)]
pub const PILL_LABEL_DONE: &str = "Done";
/// Clipboard-only because the paste target was gone (focus verification failed).
/// Unchanged by AC8 — it is what distinguishes that route from the degrade one.
#[allow(dead_code)]
pub const PILL_LABEL_CLIPBOARD: &str = "In Clipboard";
/// Clipboard-only because cleanup failed (Story 7-10 AC1). The *cause* is on the
/// card; the pill only says which light is on.
#[allow(dead_code)]
pub const PILL_LABEL_DEGRADED: &str = "Cleanup failed";
/// A `Warning` state. The text is on the card.
#[allow(dead_code)]
pub const PILL_LABEL_WARNING: &str = "Warning";
/// An `Error` state. The text is on the card.
#[allow(dead_code)]
pub const PILL_LABEL_ERROR: &str = "Error";

// ---------------------------------------------------------------------------
// Cleanup degrade
// ---------------------------------------------------------------------------

/// Why a cleanup run degraded to the raw transcript, in the shape both surfaces
/// need: [`status_line`](DegradeCause::status_line) for the one-line consumers
/// (the event's `warning` field → the main window status line, D1) and
/// [`card`](DegradeCause::card) for the native preview card.
///
/// Carrying the *cause* instead of a finished sentence is what AC8 asks for:
/// "the pipeline carries the message in a shape the card can lay out (cause /
/// clipboard-only / model-not-found are distinguishable)". The alternative —
/// shipping the flat string and parsing it apart at the renderer — is the
/// re-derivation the story's Dev Notes rule out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradeCause {
    /// The provider rejected the configured model ID (Story 7-9, D2). The one
    /// degrade cause the user can fix in a single place.
    ModelNotFound { model: String },
    /// Everything else. `reason` is `lib::friendly_error("", err)`'s output —
    /// it therefore starts with `": "` and may be empty in principle.
    Generic { reason: String },
    /// Cleanup failed *and* the clipboard write failed (review round 1, F6).
    /// Nothing landed in a window or on the clipboard, so no surface may promise
    /// either.
    ///
    /// `in_history` is the outcome of the run's History write, not an
    /// expectation of it: AC5 / Epic-12 "never silent loss" makes that write the
    /// last place the raw transcript can be, but
    /// `pipeline::stop_and_process_pipeline` performs it best-effort — it skips
    /// silently on a poisoned mutex and logs an `add_entry` error away. D2 has
    /// the card point at History; it may only do so when the write actually
    /// happened (AC8 re-review, item 2).
    ClipboardWriteFailed { in_history: bool },
}

impl DegradeCause {
    /// The one-line form. **Byte-identical to the pre-AC8 wording** — it is what
    /// `PipelineEvent::warning` carries, and `src/App.tsx`'s status line renders
    /// it verbatim (D1). AC8 moved the *pill* off this string; it did not change
    /// the string.
    pub fn status_line(&self) -> String {
        match self {
            DegradeCause::ModelNotFound { model } => {
                format!("Model '{model}' not found — in clipboard")
            }
            DegradeCause::Generic { reason } => format!(
                "Cleanup failed — raw text in clipboard (Ctrl+V){}",
                if reason.is_empty() { String::new() } else { format!(" {reason}") }
            ),
            // Names no surface, so the History outcome does not change it —
            // byte-identical on both, which keeps D1's status line unchanged.
            DegradeCause::ClipboardWriteFailed { .. } => {
                "Cleanup failed — clipboard write failed".to_string()
            }
        }
    }

    /// The card form (canon mock, "SOLL A").
    ///
    /// The two *degrade* variants keep the amber tone — the text landed, the
    /// cleanup did not. `ClipboardWriteFailed` is the double failure, and AC8
    /// review directive D2 (Andi, 2026-09-14) gives it the danger line, its own
    /// `TEXT LOST` header and — *when the write succeeded* — the one surface the
    /// text is still on: History. It drops the `next` line for the same reason
    /// review round 1 (F6) removed the flat string's clipboard promise — Ctrl+V
    /// would paste whatever was in the clipboard *before* this run.
    ///
    /// The **pill** stays on the cleanup family for all three (`Cleanup
    /// failed`): the label is chosen by the run's ending in
    /// `lib::emit_pipeline_state`, not by the card's tone.
    pub fn card(&self) -> OverlayMessage {
        match self {
            DegradeCause::ModelNotFound { model } => OverlayMessage {
                tone: MessageTone::Warning,
                header: "CLEANUP FAILED".to_string(),
                // The quotes stay with the surrounding runs, not in the chip:
                // the chip renders the bare ID in mono, and `text()` still
                // reassembles the AC8 / 7-9-D2 wording `Model '<id>' not found`
                // for the wrapped fallback (AC8 review, P11).
                cause: CauseLine {
                    before: "Model '".to_string(),
                    chip: Some(model.clone()),
                    after: "' not found".to_string(),
                },
                next: Some(CLIPBOARD_NEXT.to_string()),
                hint: Some("Check Advanced → Model IDs".to_string()),
            },
            DegradeCause::Generic { reason } => OverlayMessage {
                tone: MessageTone::Warning,
                header: "CLEANUP FAILED".to_string(),
                cause: CauseLine::plain(generic_cause_text(reason)),
                next: Some(CLIPBOARD_NEXT.to_string()),
                hint: None,
            },
            DegradeCause::ClipboardWriteFailed { in_history } => OverlayMessage {
                tone: MessageTone::Error,
                header: "TEXT LOST".to_string(),
                cause: CauseLine::plain(if *in_history {
                    "Clipboard write failed — raw text is in History"
                } else {
                    "Clipboard write failed — the raw text could not be saved"
                }),
                next: None,
                hint: None,
            },
        }
    }
}

/// The card's "where is my text" line. Identical on every cleanup-degrade card
/// (canon mock: "Zweite Zeile ist auf allen Karten gleich").
const CLIPBOARD_NEXT: &str = "Raw text is in the clipboard · Ctrl+V to paste";

/// Turn `friendly_error("", err)`'s output into a cause line.
///
/// That helper formats as `"{context}: {err}{hint}"`, so with an empty context
/// every reason starts with `": "`. The flat status line keeps it (it is glued
/// behind a sentence there); the card's cause line is a line of its own and
/// drops it.
fn generic_cause_text(reason: &str) -> String {
    let trimmed = reason.trim_start_matches(':').trim_start();
    if trimmed.is_empty() {
        "The cleanup provider did not respond".to_string()
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// AC8: a model-not-found failure lays out as four lines, with the model ID
    /// on its own chip instead of buried in a sentence the pill had to cut.
    ///
    /// PINS: the header, that the ID is a chip (not part of `before`/`after`),
    /// the clipboard line, and that 7-9's D2 pointer survives as the hint.
    /// DOES NOT PIN: any rendering. `native_preview.rs` is Windows-gated with no
    /// test module — nothing here proves a pixel, a font or a wrap.
    #[test]
    fn spec_model_not_found_card_has_chip_clipboard_line_and_hint() {
        let card = DegradeCause::ModelNotFound { model: "deepseek-typo".to_string() }.card();
        assert_eq!(card.tone, MessageTone::Warning);
        assert_eq!(card.header, "CLEANUP FAILED");
        assert_eq!(card.cause.chip.as_deref(), Some("deepseek-typo"));
        // The quotes are part of AC8's wording and of 7-9 D2's. They live in
        // `before`/`after`, so the chip still carries the bare ID the renderer
        // measures, and the reassembled line keeps its delimiters.
        assert_eq!(card.cause.text(), "Model 'deepseek-typo' not found");
        assert_eq!(
            card.next.as_deref(),
            Some("Raw text is in the clipboard · Ctrl+V to paste")
        );
        assert_eq!(card.hint.as_deref(), Some("Check Advanced → Model IDs"));
    }

    /// The generic failure has no single place to fix, so it carries no hint —
    /// and its cause line is the provider's short reason, not the sentence the
    /// pill used to show.
    #[test]
    fn spec_generic_degrade_card_has_no_hint_and_no_chip() {
        let card = DegradeCause::Generic {
            reason: ": connection refused No internet connection.".to_string(),
        }
        .card();
        assert_eq!(card.header, "CLEANUP FAILED");
        assert_eq!(card.cause.chip, None);
        assert_eq!(card.cause.text(), "connection refused No internet connection.");
        assert!(card.next.is_some(), "the text IS in the clipboard on this path");
        assert_eq!(card.hint, None);
    }

    /// Review round 1, F6, carried onto the card: when the clipboard write
    /// failed the text reached neither the window nor the clipboard, so the card
    /// must not tell the user to press Ctrl+V.
    ///
    /// AC8 review directive D2 (Andi, 2026-09-14) settles the tone question the
    /// review raised: this is the one total-loss case, so it wears the danger
    /// line and its own header, and it points at the surface where the text
    /// still is — History.
    #[test]
    fn spec_clipboard_write_failure_card_never_promises_the_clipboard() {
        let card = DegradeCause::ClipboardWriteFailed { in_history: true }.card();
        assert_eq!(card.tone, MessageTone::Error, "D2: danger line, not amber");
        assert_eq!(card.header, "TEXT LOST");
        assert_eq!(
            card.cause.text(),
            "Clipboard write failed — raw text is in History"
        );
        assert_eq!(card.next, None);
        assert_eq!(card.hint, None);
        assert!(
            !card.cause.text().contains("Ctrl+V"),
            "no key hint for text that never landed: {:?}",
            card.cause.text()
        );
    }

    /// AC8 re-review, item 2: D2's card pointed at History unconditionally, but
    /// the History write is best-effort — `pipeline::stop_and_process_pipeline`
    /// skips it silently on a poisoned mutex and swallows an `add_entry` error
    /// into a `log::warn!`. The card is the surface that just told the user the
    /// text is somewhere; on the one route where the text is genuinely nowhere
    /// it may not name a surface at all.
    ///
    /// Same false-promise shape review round 1 (F6) removed from the clipboard
    /// half of this very variant — the fix is the same: consult the outcome
    /// instead of asserting it.
    #[test]
    fn spec_clipboard_write_failure_only_names_history_when_the_write_succeeded() {
        let lost = DegradeCause::ClipboardWriteFailed { in_history: false }.card();
        assert_eq!(lost.tone, MessageTone::Error);
        assert_eq!(lost.header, "TEXT LOST");
        assert_eq!(
            lost.cause.text(),
            "Clipboard write failed — the raw text could not be saved"
        );
        assert!(
            !lost.cause.text().contains("History"),
            "History is not a surface the text is on here: {:?}",
            lost.cause.text()
        );
        assert_eq!(lost.next, None);
        assert_eq!(lost.hint, None);

        // The saved half still names it — the promise is kept where it is true.
        assert!(DegradeCause::ClipboardWriteFailed { in_history: true }
            .card()
            .cause
            .text()
            .contains("History"));

        // The one-line form claims no surface either way, so it stays
        // byte-identical on both (D1's status line, unchanged since AC8).
        assert_eq!(
            DegradeCause::ClipboardWriteFailed { in_history: true }.status_line(),
            DegradeCause::ClipboardWriteFailed { in_history: false }.status_line(),
        );
    }

    /// D2 keeps the **pill** on the cleanup family: the status light still says
    /// `Cleanup failed` for this route, because the pill label is chosen by the
    /// run's ending (`lib::emit_pipeline_state`'s `TerminalKind::Degraded`), not
    /// by the card's tone. This pins that the labels did not gain a variant.
    #[test]
    fn spec_clipboard_write_failure_keeps_the_degraded_pill_label() {
        assert_eq!(PILL_LABEL_DEGRADED, "Cleanup failed");
        assert_eq!(
            DegradeCause::ClipboardWriteFailed { in_history: true }.card().tone,
            MessageTone::Error,
            "the card diverges from the pill on purpose (D2)"
        );
    }

    /// AC8 review directive D3: a machine token gets a prominent, untruncated
    /// card now, so the known ones are translated into user wording. Anything
    /// not in the table stays verbatim — a half-understood token is worse than
    /// the raw one.
    ///
    /// The table holds exactly one row (AC8 re-review, item 3): `CommandMode` is
    /// the only `feature_requires_license:*` token any producer turns into a
    /// pipeline event. `Sync` below is a real token in the tree — a
    /// `require_license!` one — and it is here precisely because it never
    /// reaches a card: an unmapped token must survive verbatim, whether or not
    /// it exists elsewhere.
    #[test]
    fn spec_known_error_tokens_are_translated_unknown_ones_are_verbatim() {
        assert_eq!(
            OverlayMessage::error("feature_requires_license:CommandMode").cause.text(),
            "Command mode needs a license"
        );
        // Not in the table → shown exactly as the pipeline produced it.
        assert_eq!(
            OverlayMessage::error("STT failed: timeout").cause.text(),
            "STT failed: timeout"
        );
        assert_eq!(
            OverlayMessage::error("feature_requires_license:Sync").cause.text(),
            "feature_requires_license:Sync"
        );
        // A command-only token: `commands/whisper.rs` returns this to the
        // frontend and nothing emits it as an event, so the card has no wording
        // to give it and must not invent one.
        assert_eq!(
            OverlayMessage::error("feature_requires_license:OfflineMode").cause.text(),
            "feature_requires_license:OfflineMode"
        );
        // The tone and header are untouched by the translation.
        let m = OverlayMessage::error("feature_requires_license:CommandMode");
        assert_eq!(m.tone, MessageTone::Error);
        assert_eq!(m.header, "ERROR");
    }

    /// AC8 review finding P8: several boot-time config warnings each replaced
    /// the previous card instantly, so only the last was ever readable. They
    /// become one card with one line per warning.
    #[test]
    fn spec_boot_warnings_merge_into_one_card() {
        assert_eq!(OverlayMessage::warnings(Vec::new()), None);

        let one = OverlayMessage::warnings(vec!["config.json was corrupt".to_string()])
            .expect("one warning is still a card");
        assert_eq!(one.cause.text(), "config.json was corrupt");

        let many = OverlayMessage::warnings(vec![
            "config.json was corrupt".to_string(),
            "dictionary.json was corrupt".to_string(),
        ])
        .expect("two warnings are one card");
        assert_eq!(many.tone, MessageTone::Warning);
        assert_eq!(many.header, "WARNING");
        assert_eq!(
            many.cause.text(),
            "config.json was corrupt\ndictionary.json was corrupt",
            "one card, one line each — `wrap_text_lines` breaks on '\\n'"
        );
        assert_eq!(many.cause.chip, None);
    }

    /// AC8 moved the message off the pill; it did **not** change the one-line
    /// wording the main window renders (D1). These three strings are the ones
    /// `pipeline::degrade_warn_msg*` produced before this task, so the D1 proxy
    /// smoke's assertions stay valid.
    #[test]
    fn spec_status_line_wording_is_unchanged_by_the_card() {
        assert_eq!(
            DegradeCause::ModelNotFound { model: "deepseek-typo".to_string() }.status_line(),
            "Model 'deepseek-typo' not found — in clipboard"
        );
        assert_eq!(
            DegradeCause::Generic { reason: String::new() }.status_line(),
            "Cleanup failed — raw text in clipboard (Ctrl+V)"
        );
        assert_eq!(
            DegradeCause::Generic { reason: ": boom".to_string() }.status_line(),
            "Cleanup failed — raw text in clipboard (Ctrl+V) : boom"
        );
        assert_eq!(
            DegradeCause::ClipboardWriteFailed { in_history: true }.status_line(),
            "Cleanup failed — clipboard write failed"
        );
    }

    /// Every other pipeline message rides the same card (AC8): the STT fallback
    /// ladder and the boot-time config warnings as amber `WARNING`, a terminal
    /// failure as a danger-bordered `ERROR`.
    #[test]
    fn spec_ladder_and_error_messages_use_the_same_card() {
        let ladder = OverlayMessage::warning("⚠ Groq am Limit → lokale Transkription");
        assert_eq!(ladder.tone, MessageTone::Warning);
        assert_eq!(ladder.header, "WARNING");
        assert_eq!(ladder.cause.text(), "⚠ Groq am Limit → lokale Transkription");
        assert_eq!(ladder.next, None);
        assert_eq!(ladder.hint, None);

        let failure = OverlayMessage::error("✗ Transkription fehlgeschlagen — Audio gesichert");
        assert_eq!(failure.tone, MessageTone::Error);
        assert_eq!(failure.header, "ERROR");
        assert_eq!(
            failure.cause.text(),
            "✗ Transkription fehlgeschlagen — Audio gesichert"
        );
    }

    /// The pill is a status light: every label is a fixed literal, short enough
    /// that the 200×36 surface never has to cut it. This pins the **constants**,
    /// not the rendering — `native_pill.rs` is not compiled on this host.
    ///
    /// 14 chars is the longest label ("Cleanup failed"); the pill's label box is
    /// ~131 logical px, which fits it at the existing label font. The real proof
    /// is Andi's GATE-4, not this number.
    #[test]
    fn spec_pill_labels_are_static_literals() {
        for label in [
            PILL_LABEL_DONE,
            PILL_LABEL_CLIPBOARD,
            PILL_LABEL_DEGRADED,
            PILL_LABEL_WARNING,
            PILL_LABEL_ERROR,
        ] {
            assert!(!label.is_empty());
            assert!(
                label.chars().count() <= 14,
                "pill labels must fit the status light without truncation: {label:?}"
            );
            assert!(
                !label.contains('…'),
                "a truncated label means the message is back on the pill: {label:?}"
            );
        }
        // The two clipboard-only routes stay distinguishable (canon mock,
        // "Offene Entscheidungen" row 1: Andi chose "Cleanup failed").
        assert_ne!(PILL_LABEL_CLIPBOARD, PILL_LABEL_DEGRADED);
    }

    /// `CauseLine::text()` is what the renderer measures when the chip does not
    /// fit on one line, so it must reassemble the line exactly.
    #[test]
    fn spec_cause_line_text_reassembles_the_chip() {
        let c = CauseLine {
            before: "Model ".to_string(),
            chip: Some("x".to_string()),
            after: " not found".to_string(),
        };
        assert_eq!(c.text(), "Model x not found");
        assert_eq!(CauseLine::plain("just this").text(), "just this");
    }
}
