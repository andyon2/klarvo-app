//! `xtask gen-tokens` — CSS Custom Properties aus design-tokens.toml generieren.
//!
//! Input:  {repo-root}/design-tokens.toml
//! Output: shells/windows/src/styles/tokens.css

use std::path::PathBuf;
use std::process::ExitCode;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Tokens {
    color: ColorTokens,
    timing: TimingTokens,
    radius: RadiusTokens,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ColorTokens {
    surface: SurfaceColors,
    role: RoleColors,
    overlay: OverlayColors,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceColors {
    bg: String,
    surface: String,
    elevated: String,
    text: String,
    muted: String,
    dim: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleColors {
    action: String,
    activity: String,
    success: String,
    info: String,
    warm: String,
    danger: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayColors {
    bg: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TimingTokens {
    fast: String,
    medium: String,
    slow: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RadiusTokens {
    sm: String,
    md: String,
    lg: String,
    pill: String,
}

pub fn run() -> ExitCode {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = match manifest_dir.parent() {
        Some(p) => p,
        None => {
            eprintln!(
                "gen-tokens: failed to resolve repo root from {}",
                manifest_dir.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let toml_path = repo_root.join("design-tokens.toml");
    let css_path = repo_root.join("shells/windows/src/styles/tokens.css");

    let toml_src = match std::fs::read_to_string(&toml_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gen-tokens: failed to read {}: {e}", toml_path.display());
            return ExitCode::FAILURE;
        }
    };

    let tokens: Tokens = match toml::from_str(&toml_src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("gen-tokens: failed to parse {}: {e}", toml_path.display());
            return ExitCode::FAILURE;
        }
    };

    let s = &tokens.color.surface;
    let r = &tokens.color.role;
    let o = &tokens.color.overlay;
    let t = &tokens.timing;
    let rd = &tokens.radius;

    let mut css = String::new();
    css.push_str("/* Auto-generated from design-tokens.toml — DO NOT EDIT MANUALLY */\n");
    css.push_str("/* Regenerate: cargo xtask gen-tokens */\n");
    css.push_str("\n:root {\n");

    css.push_str("  /* Surface */\n");
    css.push_str(&format!("  --klarvo-color-surface-bg: {};\n", s.bg));
    css.push_str(&format!("  --klarvo-color-surface-surface: {};\n", s.surface));
    css.push_str(&format!("  --klarvo-color-surface-elevated: {};\n", s.elevated));
    css.push_str(&format!("  --klarvo-color-surface-text: {};\n", s.text));
    css.push_str(&format!("  --klarvo-color-surface-muted: {};\n", s.muted));
    css.push_str(&format!("  --klarvo-color-surface-dim: {};\n", s.dim));

    css.push_str("\n  /* Roles */\n");
    css.push_str(&format!("  --klarvo-color-action: {};\n", r.action));
    css.push_str(&format!("  --klarvo-color-activity: {};\n", r.activity));
    css.push_str(&format!("  --klarvo-color-success: {};\n", r.success));
    css.push_str(&format!("  --klarvo-color-info: {};\n", r.info));
    css.push_str(&format!("  --klarvo-color-warm: {};\n", r.warm));
    css.push_str(&format!("  --klarvo-color-danger: {};\n", r.danger));

    css.push_str("\n  /* Overlay */\n");
    css.push_str(&format!("  --klarvo-color-overlay-bg: {};\n", o.bg));

    css.push_str("\n  /* Timing */\n");
    css.push_str(&format!("  --klarvo-timing-fast: {};\n", t.fast));
    css.push_str(&format!("  --klarvo-timing-medium: {};\n", t.medium));
    css.push_str(&format!("  --klarvo-timing-slow: {};\n", t.slow));

    css.push_str("\n  /* Radius */\n");
    css.push_str(&format!("  --klarvo-radius-sm: {};\n", rd.sm));
    css.push_str(&format!("  --klarvo-radius-md: {};\n", rd.md));
    css.push_str(&format!("  --klarvo-radius-lg: {};\n", rd.lg));
    css.push_str(&format!("  --klarvo-radius-pill: {};\n", rd.pill));

    css.push_str("}\n");

    let styles_dir = match css_path.parent() {
        Some(p) => p,
        None => {
            eprintln!(
                "gen-tokens: failed to resolve parent dir for {}",
                css_path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(styles_dir) {
        eprintln!("gen-tokens: failed to create {}: {e}", styles_dir.display());
        return ExitCode::FAILURE;
    }

    if let Err(e) = std::fs::write(&css_path, &css) {
        eprintln!("gen-tokens: failed to write {}: {e}", css_path.display());
        return ExitCode::FAILURE;
    }

    println!("gen-tokens: wrote {}", css_path.display());
    ExitCode::SUCCESS
}
