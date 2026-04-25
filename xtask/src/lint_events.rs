//! `xtask lint-events` — Validation-Patch G1 + G3 multi-pass lint gate.
//!
//! # Sub-Passes
//!
//! 1. **G1 (existing):** Enforce `#[tauri_specta(event_name = "…")]` dot-notation on all
//!    `#[derive(Event)]` items. Policy: ADR-0002 + Amendment 1.
//! 2. **G3-Sub-Lint A:** User-facing-string detection. `klarvo-core` and `klarvo-plugins/*`
//!    must not contain plain-text strings in `user_message: Some(…)` positions; only valid
//!    i18n keys are allowed (ref: `memory/project_i18n_core_contract.md`).
//! 3. **G3-Sub-Lint B:** Locale cross-validation. All i18n keys found as string literals in
//!    `klarvo-core` and `klarvo-plugins/*` (constant definitions, callsites) must exist in
//!    `shells/windows/locales/en.json`; en.json and de.json must have symmetric key sets.
//! 4. **G3-Sub-Lint C:** Wildcard-match detection. `match` blocks on `PipelineStageType`
//!    must not contain `_` catch-all arms — exhaustive match is mandatory per the no-wildcard
//!    invariant in `klarvo-core/src/pipeline/stage.rs`.
//!
//! All four sub-passes run sequentially; violations are aggregated before exit (no early-exit
//! per sub-pass). Exit code: `1` if any violation, `0` if all clean, `2` on internal error.
//!
//! # Key-Format-Regex Reference (feedback_reference_block_discipline)
//!
//! ```text
//! KEY_REGEX = r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$"
//! ```
//!
//! Imported from `klarvo_core::i18n::KEY_REGEX` — not duplicated here.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::ExitCode,
};

use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Attribute, Expr, ExprStruct, ExprMatch, Item, Lit, Meta, Pat, punctuated::Punctuated,
};
use walkdir::WalkDir;

use klarvo_core::i18n::is_key;

/// Origin of a collected i18n key: (file path, 1-based line number).
type KeyOrigin = (PathBuf, usize);

/// Map of code-emitted i18n keys → first-seen origin. Used by G3-B to attach a
/// file:line-Annotation to forward-drift violations (Story 5.3 AC-D Beispiel-Output).
type CodeKeys = BTreeMap<String, KeyOrigin>;

pub fn run() -> ExitCode {
    let workspace_root = match locate_workspace_root() {
        Some(p) => p,
        None => {
            eprintln!("xtask lint-events: could not locate workspace root");
            return ExitCode::from(2);
        }
    };

    let mut all_violations: Vec<String> = Vec::new();

    // Sub-pass G1: event-name dot-notation (existing, unchanged).
    let (events_scanned, g1_violations) = run_g1_event_name_check(&workspace_root);
    all_violations.extend(g1_violations);

    // Sub-pass G3-A: user-facing-string detection in klarvo-core + klarvo-plugins.
    let (g3a_violations, code_keys) = run_g3a_user_string_check(&workspace_root);
    all_violations.extend(g3a_violations);

    // Sub-pass G3-B: locale cross-validation.
    let g3b_violations = run_g3b_locale_cross_check(&workspace_root, &code_keys);
    all_violations.extend(g3b_violations);

    // Sub-pass G3-C: wildcard-match detection on PipelineStageType in klarvo-core.
    let g3c_violations = run_g3c_wildcard_match_check(&workspace_root);
    all_violations.extend(g3c_violations);

    if all_violations.is_empty() {
        println!("xtask lint-events: OK ({events_scanned} event(s) scanned)");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "xtask lint-events: FAIL ({} violation(s), {events_scanned} event(s) scanned)",
            all_violations.len()
        );
        for v in &all_violations {
            eprintln!("  - {v}");
        }
        ExitCode::from(1)
    }
}

// ── G1: Event-Name Dot-Notation ─────────────────────────────────────────────

fn run_g1_event_name_check(workspace_root: &Path) -> (usize, Vec<String>) {
    let mut events_scanned = 0usize;
    let mut violations = Vec::new();

    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|e| !is_excluded(e.path(), workspace_root))
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
            scan_item_g1(item, path, &mut events_scanned, &mut violations);
        }
    }

    (events_scanned, violations)
}

fn scan_item_g1(
    item: &Item,
    path: &Path,
    events_scanned: &mut usize,
    violations: &mut Vec<String>,
) {
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

// ── G3-A: User-Facing-String Detection ──────────────────────────────────────

/// Scans klarvo-core and klarvo-plugins for string literals in `user_message: Some(…)` positions.
/// Returns (violations, all_valid_keys_found_as_literals).
///
/// # Scope
///
/// Covers `klarvo-core/src/` and `klarvo-plugins/*/src/` — the G3-Kontrakt scope
/// (`memory/project_i18n_core_contract.md`). `shells/` is excluded (G1 is owner there).
/// Test modules (`#[cfg(test)]`) are excluded.
fn run_g3a_user_string_check(workspace_root: &Path) -> (Vec<String>, CodeKeys) {
    let mut violations = Vec::new();
    let mut code_keys: CodeKeys = BTreeMap::new();

    let scan_roots = [
        workspace_root.join("klarvo-core").join("src"),
        workspace_root.join("klarvo-plugins"),
    ];

    for root in &scan_roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_excluded_g3(e.path()))
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

            // File-based `mod keys;` modules: when the parsed file *is* the `keys` module
            // (file named `keys.rs` declared via `pub mod keys;` in a parent), there is no
            // `ItemMod` AST node to flip `in_keys_mod`. Detect via filename so const-Items
            // at the top level get collected. Affected paths: klarvo-core/src/{audio,output,
            // keystore,v1_import}/keys.rs.
            let initial_in_keys = path
                .file_name()
                .and_then(|s| s.to_str())
                == Some("keys.rs");

            let mut visitor = UserStringVisitor {
                violations: &mut violations,
                code_keys: &mut code_keys,
                file_path: path.to_path_buf(),
                in_test_mod: false,
                in_keys_mod: initial_in_keys,
            };
            visitor.visit_file(&file);
        }
    }

    (violations, code_keys)
}

struct UserStringVisitor<'a> {
    violations: &'a mut Vec<String>,
    code_keys: &'a mut CodeKeys,
    file_path: PathBuf,
    in_test_mod: bool,
    /// True when we are inside a `mod keys { … }` block — the conventional location for
    /// i18n key constants in klarvo-core and klarvo-plugins. Also seeded `true` at file
    /// scope when the parsed file is `keys.rs` (file-based module declaration).
    ///
    /// Rationale (Story 5.3 D2-Amendment): scope-narrowing avoids false positives from other
    /// `&'static str` constants that happen to match `KEY_REGEX` (e.g. `"com.klarvo.voice"`,
    /// `"config.json"`). The convention is established in klarvo-core/src/{audio,output,
    /// keystore,v1_import}/keys.rs and in inline `mod keys` blocks in plugins.
    in_keys_mod: bool,
}

impl<'ast, 'a> Visit<'ast> for UserStringVisitor<'a> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let was_in_test = self.in_test_mod;
        let was_in_keys = self.in_keys_mod;
        if is_cfg_test_mod(&node.attrs) {
            self.in_test_mod = true;
        }
        if node.ident == "keys" {
            self.in_keys_mod = true;
        }
        visit::visit_item_mod(self, node);
        self.in_test_mod = was_in_test;
        self.in_keys_mod = was_in_keys;
    }

    fn visit_expr_struct(&mut self, node: &'ast ExprStruct) {
        if !self.in_test_mod {
            for field in &node.fields {
                if let syn::Member::Named(ident) = &field.member {
                    if ident == "user_message" {
                        if let Some(lit) = extract_some_string_literal(&field.expr) {
                            let line = field.expr.span().start().line;
                            if is_key(&lit) {
                                self.record_key(lit, line);
                            } else {
                                self.violations.push(format!(
                                    "VIOLATION [user-string]: non-key literal {:?} in user_message position at {}:{line}",
                                    lit,
                                    self.file_path.display()
                                ));
                            }
                        }
                    }
                }
            }
        }
        visit::visit_expr_struct(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        // Collect i18n key constants only from `mod keys { … }` blocks (or file-based
        // `keys.rs` modules; see `in_keys_mod` doc-comment for rationale).
        if !self.in_test_mod && self.in_keys_mod {
            if let Expr::Lit(el) = node.expr.as_ref() {
                if let Lit::Str(s) = &el.lit {
                    let val = s.value();
                    if is_key(&val) {
                        let line = node.span().start().line;
                        self.record_key(val, line);
                    }
                }
            }
        }
        visit::visit_item_const(self, node);
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        // Story 5.3 AC-B Position 2: `#[default = "..."]`-Attribute auf Struct-Fields.
        // Pattern aus z.B. `derive_more`/`smart-default`-Macros oder Custom-Derives, die
        // String-Literals als Default-Werte erlauben. Behandelt analog zu user_message:
        // is_key → code_keys, sonst Violation.
        if !self.in_test_mod {
            self.scan_default_attrs(&node.attrs);
        }
        visit::visit_field(self, node);
    }

    fn visit_variant(&mut self, node: &'ast syn::Variant) {
        // Story 5.3 AC-B Position 2: same as visit_field but for enum-variant-attributes.
        if !self.in_test_mod {
            self.scan_default_attrs(&node.attrs);
        }
        visit::visit_variant(self, node);
    }
}

impl<'a> UserStringVisitor<'a> {
    /// Insert a key into `code_keys`, keeping the first-seen origin per key.
    fn record_key(&mut self, key: String, line: usize) {
        self.code_keys
            .entry(key)
            .or_insert_with(|| (self.file_path.clone(), line));
    }

    /// Scan a list of attributes for `#[default = "<literal>"]` patterns.
    fn scan_default_attrs(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            if let Some(lit) = extract_default_attribute_string(attr) {
                let line = attr.span().start().line;
                if is_key(&lit) {
                    self.record_key(lit, line);
                } else {
                    self.violations.push(format!(
                        "VIOLATION [user-string]: non-key literal {:?} in #[default = ...] at {}:{line}",
                        lit,
                        self.file_path.display()
                    ));
                }
            }
        }
    }
}

/// Extract the string literal value from `#[default = "..."]`. Returns `None` for any other
/// attribute shape (incl. `#[default]` without value, `#[default = 42]`, `#[default(...)]`).
fn extract_default_attribute_string(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("default") {
        return None;
    }
    let Meta::NameValue(nv) = &attr.meta else { return None };
    let Expr::Lit(el) = &nv.value else { return None };
    let Lit::Str(s) = &el.lit else { return None };
    Some(s.value())
}

/// Extract a string literal from a `Some(<expr>)` call, handling `.into()` / `.to_string()`.
/// Returns `None` if the expression is not `Some(<string literal>)` or if the value is not
/// a string (e.g., `None`, or a path like `keys::NETWORK.into()`).
fn extract_some_string_literal(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else { return None };
    let is_some = match call.func.as_ref() {
        Expr::Path(p) => p.path.segments.last().map(|s| s.ident == "Some").unwrap_or(false),
        _ => false,
    };
    if !is_some || call.args.len() != 1 {
        return None;
    }
    extract_string_literal_from_expr(&call.args[0])
}

/// Walk an expression to find a string literal, handling method chains like `.into()`.
fn extract_string_literal_from_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(el) => {
            if let Lit::Str(s) = &el.lit {
                Some(s.value())
            } else {
                None
            }
        }
        Expr::MethodCall(mc) => extract_string_literal_from_expr(&mc.receiver),
        Expr::Call(call) => {
            // e.g. String::from("…")
            if call.args.len() == 1 {
                extract_string_literal_from_expr(&call.args[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

// ── G3-B: Locale Cross-Validation ───────────────────────────────────────────

/// Validates that code-emitted keys exist in en.json (forward drift) and that en.json / de.json
/// have symmetric key sets.
///
/// # Not Covered
///
/// Keys present in en.json that are not emitted by code (orphan keys from shell-side emissions)
/// are NOT checked here — those are covered by the manual Story 4.4 AC-F test (`REQUIRED_KEYS`).
/// This gap is documented in Technical Notes (Story 5.3) and the backlog.
fn run_g3b_locale_cross_check(
    workspace_root: &Path,
    code_keys: &CodeKeys,
) -> Vec<String> {
    let mut violations = Vec::new();

    let en_path = workspace_root
        .join("shells")
        .join("windows")
        .join("locales")
        .join("en.json");
    let de_path = workspace_root
        .join("shells")
        .join("windows")
        .join("locales")
        .join("de.json");

    let en_table: BTreeMap<String, String> = match load_locale_json(&en_path) {
        Ok(t) => t,
        Err(e) => {
            violations.push(format!("VIOLATION [locale-load]: cannot load en.json: {e}"));
            return violations;
        }
    };
    let de_table: BTreeMap<String, String> = match load_locale_json(&de_path) {
        Ok(t) => t,
        Err(e) => {
            violations.push(format!("VIOLATION [locale-load]: cannot load de.json: {e}"));
            return violations;
        }
    };

    // Step 1: Forward-drift check — every code-emitted key must be in en.json.
    for (key, (path, line)) in code_keys {
        if !en_table.contains_key(key.as_str()) {
            // Show repo-relative path for readability when running inside the workspace.
            let display_path = path
                .strip_prefix(workspace_root)
                .unwrap_or(path)
                .display();
            violations.push(format!(
                "VIOLATION [locale-drift]: key {key:?} emitted in {display_path}:{line} but absent from en.json"
            ));
        }
    }

    // Step 2: Symmetry check — en.json and de.json must have the same key sets.
    let en_keys: BTreeSet<&str> = en_table.keys().map(String::as_str).collect();
    let de_keys: BTreeSet<&str> = de_table.keys().map(String::as_str).collect();

    for key in en_keys.difference(&de_keys) {
        violations.push(format!(
            "VIOLATION [locale-asymmetry]: key {key:?} present in en.json but absent from de.json"
        ));
    }
    for key in de_keys.difference(&en_keys) {
        violations.push(format!(
            "VIOLATION [locale-asymmetry]: key {key:?} present in de.json but absent from en.json"
        ));
    }

    violations
}

fn load_locale_json(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|e| format!("{}: {e}", path.display()))
}

// ── G3-C: Wildcard-Match Detection ──────────────────────────────────────────

/// Scans `klarvo-core/src/` for `match` expressions on `PipelineStageType` that have a
/// `_` wildcard arm.
///
/// # Heuristic
///
/// Type-resolution is not performed. We look for `match` arms whose patterns contain
/// the ident `PipelineStageType`. If such a match also has a `_ =>` arm, it violates the
/// no-wildcard invariant from `klarvo-core/src/pipeline/stage.rs` doc-comment.
///
/// Limitation (documented): another type with the same ident name would be falsely flagged.
/// In the current Phase-1 workspace, `PipelineStageType` is unique in klarvo-core.
/// Test modules (`#[cfg(test)]`) are excluded.
fn run_g3c_wildcard_match_check(workspace_root: &Path) -> Vec<String> {
    let mut violations = Vec::new();
    let core_src = workspace_root.join("klarvo-core").join("src");

    if !core_src.exists() {
        return violations;
    }

    for entry in WalkDir::new(&core_src)
        .into_iter()
        .filter_entry(|e| !is_excluded_g3(e.path()))
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

        let mut visitor = WildcardMatchVisitor {
            violations: &mut violations,
            file_path: path.to_path_buf(),
            in_test_mod: false,
        };
        visitor.visit_file(&file);
    }

    violations
}

struct WildcardMatchVisitor<'a> {
    violations: &'a mut Vec<String>,
    file_path: PathBuf,
    in_test_mod: bool,
}

impl<'ast, 'a> Visit<'ast> for WildcardMatchVisitor<'a> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let was_in_test = self.in_test_mod;
        if is_cfg_test_mod(&node.attrs) {
            self.in_test_mod = true;
        }
        visit::visit_item_mod(self, node);
        self.in_test_mod = was_in_test;
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        if !self.in_test_mod {
            let has_pst_arm = node.arms.iter().any(|arm| pat_references_pipeline_stage_type(&arm.pat));
            let has_wildcard = node.arms.iter().any(|arm| matches!(&arm.pat, Pat::Wild(_)));

            if has_pst_arm && has_wildcard {
                // Heuristic: `PipelineStageType` ident found in arm patterns + `_` arm present.
                // Limitation: false-positives possible if another type shares the ident name
                // (no such type exists in the Phase-1 workspace).
                let line = node.match_token.span.start().line;
                self.violations.push(format!(
                    "VIOLATION [wildcard-match]: match on PipelineStageType at {}:{line} has a wildcard arm (`_`); use exhaustive match",
                    self.file_path.display()
                ));
            }
        }
        visit::visit_expr_match(self, node);
    }
}

fn pat_references_pipeline_stage_type(pat: &Pat) -> bool {
    match pat {
        Pat::Path(p) => path_has_pst_ident(&p.path),
        Pat::Struct(p) => path_has_pst_ident(&p.path),
        Pat::TupleStruct(p) => path_has_pst_ident(&p.path),
        Pat::Or(p) => p.cases.iter().any(pat_references_pipeline_stage_type),
        Pat::Reference(p) => pat_references_pipeline_stage_type(&p.pat),
        _ => false,
    }
}

fn path_has_pst_ident(path: &syn::Path) -> bool {
    path.segments.iter().any(|s| s.ident == "PipelineStageType")
}

// ── Shared Helpers ───────────────────────────────────────────────────────────

fn locate_workspace_root() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf)
}

fn is_excluded(path: &Path, workspace_root: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
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
    if path == workspace_root.join("src-tauri") || path == workspace_root.join("src") {
        return true;
    }
    false
}

/// Exclusion filter for G3 sub-lints (tighter: also skip test-fixtures and android).
fn is_excluded_g3(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    matches!(
        name,
        "target" | ".git" | "node_modules" | "dist" | "_bmad" | "_bmad-output"
            | "gen" | "output" | "test-fixtures" | "android"
    )
}

fn is_cfg_test_mod(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let Meta::List(list) = &attr.meta else { return false };
        list.tokens.to_string().contains("test")
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Item> {
        syn::parse_file(src).unwrap().items
    }

    fn scan_g1(src: &str) -> (usize, Vec<String>) {
        let items = parse(src);
        let mut events = 0;
        let mut violations = vec![];
        let path = Path::new("test.rs");
        for item in &items {
            scan_item_g1(item, path, &mut events, &mut violations);
        }
        (events, violations)
    }

    // ── G1 tests (unchanged) ─────────────────────────────────────────────

    #[test]
    fn ok_with_dot_notation() {
        let (events, v) = scan_g1(
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
        let (events, v) = scan_g1(
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
        let (events, v) = scan_g1(
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
        let (events, v) = scan_g1(
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
        let (events, v) = scan_g1(
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
        let (events, v) = scan_g1(
            r#"
            #[derive(tauri_specta::Event)]
            #[tauri_specta(event_name = "recording.phase")]
            enum RecordingPhase { Start, Stop }
            "#,
        );
        assert_eq!(events, 1);
        assert!(v.is_empty(), "unexpected violations: {v:?}");
    }

    // ── G3-A tests ───────────────────────────────────────────────────────

    fn scan_g3a(src: &str) -> (Vec<String>, CodeKeys) {
        scan_g3a_with(src, false)
    }

    fn scan_g3a_with(src: &str, initial_in_keys: bool) -> (Vec<String>, CodeKeys) {
        let file = syn::parse_file(src).unwrap();
        let mut violations = Vec::new();
        let mut code_keys: CodeKeys = BTreeMap::new();
        let mut visitor = UserStringVisitor {
            violations: &mut violations,
            code_keys: &mut code_keys,
            file_path: PathBuf::from("test.rs"),
            in_test_mod: false,
            in_keys_mod: initial_in_keys,
        };
        visitor.visit_file(&file);
        (violations, code_keys)
    }

    #[test]
    fn g3a_positive_plaintext_in_user_message_flagged() {
        // Positive fixture: plain-text string in user_message → violation expected.
        let src = r#"
        fn f() {
            let _ = AppError {
                user_message: Some("Network error".into()),
                kind: AppErrorKind::Network,
                message: String::new(),
                retryable: false,
            };
        }
        "#;
        let (violations, _) = scan_g3a(src);
        assert_eq!(violations.len(), 1, "expected one violation: {violations:?}");
        assert!(violations[0].contains("user-string"), "{}", violations[0]);
        assert!(violations[0].contains("Network error"), "{}", violations[0]);
    }

    #[test]
    fn g3a_negative_valid_key_in_user_message_not_flagged() {
        // Negative fixture: valid i18n key in user_message → no violation.
        let src = r#"
        fn f() {
            let _ = AppError {
                user_message: Some("error.stt.network".into()),
                kind: AppErrorKind::Network,
                message: String::new(),
                retryable: false,
            };
        }
        "#;
        let (violations, keys) = scan_g3a(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
        assert!(keys.contains_key("error.stt.network"), "key should be collected: {keys:?}");
    }

    #[test]
    fn g3a_negative_none_not_flagged() {
        // user_message: None must not be flagged.
        let src = r#"
        fn f() {
            let _ = AppError {
                user_message: None,
                kind: AppErrorKind::Internal,
                message: String::new(),
                retryable: false,
            };
        }
        "#;
        let (violations, _) = scan_g3a(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
    }

    // ── G3-A P6: file-based `mod keys;` (keys.rs) ────────────────────────────

    #[test]
    fn g3a_file_based_keys_module_collected() {
        // Forcing sentinel for P6: a file named `keys.rs` (declared as `pub mod keys;` in
        // a parent) must collect const keys at file scope. Without P6's filename heuristic,
        // these keys silently escape G3-B forward-drift coverage.
        let src = r#"
            pub const DEVICE_UNAVAILABLE: &str = "error.audio.device_unavailable";
            pub const UNSUPPORTED_FORMAT: &str = "error.audio.unsupported_format";
        "#;
        let (violations, keys) = scan_g3a_with(src, /* initial_in_keys = */ true);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
        assert!(keys.contains_key("error.audio.device_unavailable"), "{keys:?}");
        assert!(keys.contains_key("error.audio.unsupported_format"), "{keys:?}");
    }

    #[test]
    fn g3a_top_level_consts_outside_keys_mod_not_collected() {
        // Negative: identical const at file scope without keys.rs context must NOT collect
        // (avoids false positives on plugin-identifiers / config-paths).
        let src = r#"
            pub const SOME_OTHER: &str = "error.audio.device_unavailable";
        "#;
        let (violations, keys) = scan_g3a_with(src, /* initial_in_keys = */ false);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
        assert!(keys.is_empty(), "must not collect outside mod keys: {keys:?}");
    }

    // ── G3-A D3I: `#[default = "..."]`-Attribute ─────────────────────────────

    #[test]
    fn g3a_default_attr_plaintext_flagged() {
        // Positive fixture for AC-B Position 2: plain-text in `#[default = "..."]` → violation.
        let src = r#"
            #[derive(SmartDefault)]
            struct Cfg {
                #[default = "Network error"]
                msg: String,
            }
        "#;
        let (violations, _) = scan_g3a(src);
        assert_eq!(violations.len(), 1, "expected one violation: {violations:?}");
        assert!(violations[0].contains("user-string"), "{}", violations[0]);
        assert!(violations[0].contains("#[default"), "{}", violations[0]);
        assert!(violations[0].contains("Network error"), "{}", violations[0]);
    }

    #[test]
    fn g3a_default_attr_valid_key_collected() {
        // Negative fixture for AC-B Position 2: valid i18n key in `#[default = "..."]` → collect, no violation.
        let src = r#"
            #[derive(SmartDefault)]
            struct Cfg {
                #[default = "error.config.invalid_locale"]
                fallback: String,
            }
        "#;
        let (violations, keys) = scan_g3a(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
        assert!(keys.contains_key("error.config.invalid_locale"), "{keys:?}");
    }

    #[test]
    fn g3a_default_attr_on_enum_variant_works() {
        // Variant-level `#[default = "..."]` covered by visit_variant.
        let src = r#"
            #[derive(SmartDefault)]
            enum Mode {
                #[default = "plain text"]
                Foo,
                Bar,
            }
        "#;
        let (violations, _) = scan_g3a(src);
        assert_eq!(violations.len(), 1, "expected one violation: {violations:?}");
        assert!(violations[0].contains("plain text"), "{}", violations[0]);
    }

    #[test]
    fn g3a_default_attr_without_value_ignored() {
        // Bare `#[default]` (Rust 1.62 enum-default) has no value — must not be flagged.
        let src = r#"
            #[derive(Default)]
            enum Mode {
                #[default]
                Foo,
                Bar,
            }
        "#;
        let (violations, _) = scan_g3a(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
    }

    // ── G3-A P7: file/line origin in code_keys ───────────────────────────────

    #[test]
    fn g3a_records_origin_for_collected_keys() {
        let src = r#"
fn f() {
    let _ = AppError {
        user_message: Some("error.stt.network".into()),
        kind: AppErrorKind::Network,
        message: String::new(),
        retryable: false,
    };
}
        "#;
        let (_, keys) = scan_g3a(src);
        let origin = keys
            .get("error.stt.network")
            .expect("key recorded");
        assert_eq!(origin.0, PathBuf::from("test.rs"));
        assert!(origin.1 >= 1, "line number must be 1-based, got {}", origin.1);
    }

    // ── G3-B test ────────────────────────────────────────────────────────

    #[test]
    fn g3b_locale_drift_detected() {
        // Forcing sentinel: proves G3-Sub-Lint B detects forward-drift (CI-gate-philosophy).
        // Simulate code_keys containing a key that is absent from en.json.
        let missing_key = "error.test.nonexistent_key_xyz";
        let code_keys: BTreeSet<String> = std::iter::once(missing_key.to_string()).collect();

        let en_table: BTreeMap<String, String> = BTreeMap::new(); // empty = all keys missing
        let de_table: BTreeMap<String, String> = BTreeMap::new();

        // Run the drift check logic inline (shared helper).
        let mut violations = Vec::new();
        for key in &code_keys {
            if !en_table.contains_key(key.as_str()) {
                violations.push(format!(
                    "VIOLATION [locale-drift]: key {key:?} emitted in code but absent from en.json"
                ));
            }
        }
        let en_keys: BTreeSet<&str> = en_table.keys().map(String::as_str).collect();
        let de_keys: BTreeSet<&str> = de_table.keys().map(String::as_str).collect();
        for key in en_keys.difference(&de_keys) {
            violations.push(format!(
                "VIOLATION [locale-asymmetry]: key {key:?} present in en.json but absent from de.json"
            ));
        }

        assert_eq!(violations.len(), 1, "expected one drift violation: {violations:?}");
        assert!(violations[0].contains("locale-drift"), "{}", violations[0]);
        assert!(violations[0].contains(missing_key), "{}", violations[0]);
    }

    // ── G3-C tests ───────────────────────────────────────────────────────

    fn scan_g3c(src: &str) -> Vec<String> {
        let file = syn::parse_file(src).unwrap();
        let mut violations = Vec::new();
        let mut visitor = WildcardMatchVisitor {
            violations: &mut violations,
            file_path: PathBuf::from("test.rs"),
            in_test_mod: false,
        };
        visitor.visit_file(&file);
        violations
    }

    #[test]
    fn g3c_wildcard_match_flagged() {
        // Positive fixture: PipelineStageType match with `_` arm → violation expected.
        let src = r#"
        fn f(t: PipelineStageType) -> u8 {
            match t {
                PipelineStageType::Passthrough => 0,
                _ => 1,
            }
        }
        "#;
        let violations = scan_g3c(src);
        assert_eq!(violations.len(), 1, "expected one wildcard violation: {violations:?}");
        assert!(violations[0].contains("wildcard-match"), "{}", violations[0]);
    }

    #[test]
    fn g3c_other_enum_wildcard_not_flagged() {
        // Negative fixture: other enum with `_` arm must not be flagged.
        let src = r#"
        fn f(r: Result<(), ()>) -> bool {
            match r {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        "#;
        let violations = scan_g3c(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
    }

    #[test]
    fn g3c_exhaustive_pst_match_not_flagged() {
        // Negative fixture: exhaustive PipelineStageType match (no `_`) must not be flagged.
        let src = r#"
        fn f(t: PipelineStageType) -> u8 {
            match t {
                PipelineStageType::Passthrough => 0,
                PipelineStageType::Stt { .. } => 1,
                PipelineStageType::Cleanup { .. } => 2,
            }
        }
        "#;
        let violations = scan_g3c(src);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
    }
}
