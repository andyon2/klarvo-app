//! `xtask lint-events` — Validation-Patch G1.
//!
//! Scans every Rust file in the workspace for struct/enum items deriving
//! `tauri_specta::Event` (matched by the `Event` ident inside any `derive`
//! attribute) and enforces the project's event-naming policy:
//!
//!   - An explicit `#[tauri_specta(event_name = "…")]` attribute must be present.
//!   - Its value must contain `.` (dot-notation, per ADR-0002 + Amendment 1).
//!
//! Rationale: in `tauri-specta = 2.0.0-rc.24`, the absence of
//! `#[tauri_specta(event_name)]` falls back to `struct_name.to_kebab_case()`.
//! Kebab-case silently breaks the dot-notation namespace policy
//! (e.g. `"recording.started"` vs. `"recording-started"`) — this lint turns
//! that silent default into a loud CI failure.
//!
//! Note: the `Event`-derive match is ident-based, so any custom trait also
//! called `Event` would be flagged. In Phase-0 only `tauri_specta::Event`
//! exists in the workspace, so the heuristic is exact. If a second trait with
//! the same ident ever appears, this scanner must be tightened (e.g. by
//! resolving `use` statements).

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use syn::{Attribute, Expr, Item, Lit, Meta, punctuated::Punctuated};
use walkdir::WalkDir;

pub fn run() -> ExitCode {
    let workspace_root = match locate_workspace_root() {
        Some(p) => p,
        None => {
            eprintln!("xtask lint-events: could not locate workspace root");
            return ExitCode::from(2);
        }
    };

    let mut events_scanned = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_entry(|e| !is_excluded(e.path(), &workspace_root))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file = match syn::parse_file(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for item in &file.items {
            scan_item(item, path, &mut events_scanned, &mut violations);
        }
    }

    if violations.is_empty() {
        println!("xtask lint-events: OK ({events_scanned} event(s) scanned)");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "xtask lint-events: FAIL ({} violation(s), {events_scanned} event(s) scanned)",
            violations.len()
        );
        for v in &violations {
            eprintln!("  - {v}");
        }
        ExitCode::from(1)
    }
}

fn locate_workspace_root() -> Option<PathBuf> {
    // xtask/Cargo.toml → workspace root is the parent of CARGO_MANIFEST_DIR.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf)
}

fn is_excluded(path: &Path, workspace_root: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // hard-skip cargo/git/framework dirs anywhere in tree
    if matches!(
        name,
        "target"
            | ".git"
            | "node_modules"
            | "dist"
            | "_bmad"
            | "_bmad-output"
            | "gen"
            | "output"
    ) {
        return true;
    }
    // skip the legacy v1 src-tauri/ at workspace root (not part of v2 workspace)
    if path == workspace_root.join("src-tauri") || path == workspace_root.join("src") {
        return true;
    }
    false
}

fn scan_item(item: &Item, path: &Path, events_scanned: &mut usize, violations: &mut Vec<String>) {
    let (ident_str, line, attrs) = match item {
        Item::Struct(s) => (s.ident.to_string(), s.ident.span().start().line, &s.attrs),
        Item::Enum(e) => (e.ident.to_string(), e.ident.span().start().line, &e.attrs),
        _ => return,
    };

    if !derives_event(attrs) {
        return;
    }
    *events_scanned += 1;

    match extract_event_name(attrs) {
        Some(name) if name.contains('.') => { /* OK */ }
        Some(name) => violations.push(format!(
            "{}:{line} — event `{ident_str}` has name {name:?} without `.` (dot-notation required; ADR-0002 Amendment 1)",
            path.display()
        )),
        None => violations.push(format!(
            "{}:{line} — event `{ident_str}` is missing `#[tauri_specta(event_name = \"…\")]` (ADR-0002 Amendment 1; kebab-case default breaks dot-notation policy)",
            path.display()
        )),
    }
}

fn derives_event(attrs: &[Attribute]) -> bool {
    for a in attrs {
        if !a.path().is_ident("derive") {
            continue;
        }
        let Meta::List(list) = &a.meta else { continue };
        let Ok(paths) =
            list.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };
        for p in paths {
            // matches `Event` and `tauri_specta::Event` (segment-based, whitespace-agnostic)
            if p.segments
                .last()
                .map(|seg| seg.ident == "Event")
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn extract_event_name(attrs: &[Attribute]) -> Option<String> {
    for a in attrs {
        if !a.path().is_ident("tauri_specta") {
            continue;
        }
        let Meta::List(list) = &a.meta else { continue };
        let Ok(items) = list.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };
        for m in items {
            let Meta::NameValue(nv) = m else { continue };
            if !nv.path.is_ident("event_name") {
                continue;
            }
            let Expr::Lit(el) = &nv.value else { continue };
            let Lit::Str(s) = &el.lit else { continue };
            return Some(s.value());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Item> {
        syn::parse_file(src).unwrap().items
    }

    fn scan(src: &str) -> (usize, Vec<String>) {
        let items = parse(src);
        let mut events = 0;
        let mut violations = vec![];
        let path = Path::new("test.rs");
        for item in &items {
            scan_item(item, path, &mut events, &mut violations);
        }
        (events, violations)
    }

    #[test]
    fn ok_with_dot_notation() {
        let (events, v) = scan(
            r#"
            #[derive(tauri_specta::Event)]
            #[tauri_specta(event_name = "recording.started")]
            struct RecordingStarted;
            "#,
        );
        assert_eq!(events, 1);
        assert!(v.is_empty(), "unexpected violations: {v:?}");
    }

    #[test]
    fn ok_with_short_path_event() {
        let (events, v) = scan(
            r#"
            #[derive(Clone, Debug, Event)]
            #[tauri_specta(event_name = "app.ready")]
            struct AppReady;
            "#,
        );
        assert_eq!(events, 1);
        assert!(v.is_empty(), "unexpected violations: {v:?}");
    }

    #[test]
    fn fails_without_attribute() {
        let (events, v) = scan(
            r#"
            #[derive(Event)]
            struct NoAttrEvent;
            "#,
        );
        assert_eq!(events, 1);
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("missing"), "{}", v[0]);
    }

    #[test]
    fn fails_without_dot() {
        let (events, v) = scan(
            r#"
            #[derive(Event)]
            #[tauri_specta(event_name = "kebab-case-name")]
            struct KebabCase;
            "#,
        );
        assert_eq!(events, 1);
        assert_eq!(v.len(), 1);
        assert!(v[0].contains("without"), "{}", v[0]);
    }

    #[test]
    fn ignores_non_event_structs() {
        let (events, v) = scan(
            r#"
            #[derive(Clone, Debug)]
            struct Plain;

            #[derive(serde::Serialize)]
            struct WithSerde;
            "#,
        );
        assert_eq!(events, 0);
        assert!(v.is_empty());
    }

    #[test]
    fn handles_event_on_enum() {
        let (events, v) = scan(
            r#"
            #[derive(tauri_specta::Event)]
            #[tauri_specta(event_name = "recording.phase")]
            enum RecordingPhase { Start, Stop }
            "#,
        );
        assert_eq!(events, 1);
        assert!(v.is_empty(), "unexpected violations: {v:?}");
    }
}
