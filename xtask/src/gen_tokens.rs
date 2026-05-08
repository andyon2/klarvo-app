//! `xtask gen-tokens` — CSS Custom Properties aus design-tokens.toml generieren.
//!
//! Input:  {repo-root}/design-tokens.toml
//! Output: shells/windows/src/styles/tokens.css

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(serde::Deserialize)]
struct Tokens {
    color: ColorTokens,
    timing: BTreeMap<String, String>,
    radius: BTreeMap<String, String>,
}

#[derive(serde::Deserialize)]
struct ColorTokens {
    surface: BTreeMap<String, String>,
    role: BTreeMap<String, String>,
    overlay: BTreeMap<String, String>,
}

pub fn run() -> ExitCode {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().expect("xtask has parent dir");

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

    let mut css = String::new();
    css.push_str("/* Auto-generated from design-tokens.toml — DO NOT EDIT MANUALLY */\n");
    css.push_str("/* Regenerate: cargo xtask gen-tokens */\n");
    css.push_str("\n:root {\n");
    css.push_str("  /* Surface */\n");

    for (key, val) in &tokens.color.surface {
        css.push_str(&format!("  --klarvo-color-surface-{key}: {val};\n"));
    }

    css.push_str("\n  /* Roles */\n");
    for (key, val) in &tokens.color.role {
        css.push_str(&format!("  --klarvo-color-{key}: {val};\n"));
    }

    css.push_str("\n  /* Overlay */\n");
    for (key, val) in &tokens.color.overlay {
        css.push_str(&format!("  --klarvo-color-overlay-{key}: {val};\n"));
    }

    css.push_str("\n  /* Timing */\n");
    for (key, val) in &tokens.timing {
        css.push_str(&format!("  --klarvo-timing-{key}: {val};\n"));
    }

    css.push_str("\n  /* Radius */\n");
    for (key, val) in &tokens.radius {
        css.push_str(&format!("  --klarvo-radius-{key}: {val};\n"));
    }

    css.push_str("}\n");

    let styles_dir = css_path.parent().expect("tokens.css has parent dir");
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
