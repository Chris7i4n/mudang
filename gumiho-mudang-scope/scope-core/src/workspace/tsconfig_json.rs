//! `tsconfig.json` reader.
//!
//! Extracts `compilerOptions.paths` and `compilerOptions.baseUrl` to
//! seed `ModuleLayout`.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see
//! `crate::workspace_context`), not this reader. `target`, `module`,
//! `lib`, and any other version-coupled field are absent from the
//! trait — CI gate `audit_context_shape.sh` (R4) refuses any method
//! whose name suggests them, so language plugins cannot reach those
//! fields through any plugin-side path. This reader lives
//! indexer-side and may expose `target` extraction (and similar
//! version-coupled fields) for indexer-side consumers (R8 audit
//! emit per `BACKLOG.md` § Priority 1 sub-item (d)); plugins never
//! reach those functions because they receive the trait, not this
//! module.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::workspace_context::ModuleLayout;

#[derive(Deserialize, Default)]
struct Manifest {
    #[serde(rename = "compilerOptions", default)]
    compiler_options: Option<CompilerOptions>,
}

#[derive(Deserialize, Default)]
struct CompilerOptions {
    #[serde(default, rename = "baseUrl")]
    base_url: Option<String>,
    #[serde(default)]
    paths: BTreeMap<String, Vec<String>>,
}

/// Read and parse a `tsconfig.json` at `path`.
pub fn read_tsconfig_json(path: &Path) -> Result<ModuleLayout> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read tsconfig.json at {}", path.display()))?;
    let base_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    parse_tsconfig_json(&content, &base_dir)
}

/// Parse an in-memory `tsconfig.json` string. `base_dir` anchors any
/// relative paths declared in `baseUrl` / `paths`.
pub fn parse_tsconfig_json(content: &str, base_dir: &Path) -> Result<ModuleLayout> {
    let stripped = strip_jsonc_comments(content);
    let manifest: Manifest = serde_json::from_str(&stripped).context("parse tsconfig.json")?;

    let mut modules = BTreeMap::new();
    if let Some(opts) = manifest.compiler_options {
        let base = opts
            .base_url
            .as_deref()
            .map(|b| base_dir.join(b))
            .unwrap_or_else(|| base_dir.to_path_buf());
        for (pattern, targets) in opts.paths {
            let module_name = pattern.trim_end_matches("/*").to_string();
            if let Some(first) = targets.first() {
                let target = first.trim_end_matches("/*");
                modules.insert(module_name, base.join(target));
            }
        }
    }

    Ok(ModuleLayout { modules })
}

/// Strip `//` and `/* ... */` comments — tsconfig.json is JSONC, not
/// strict JSON. String-literal aware: `/*` inside `"..."` is preserved
/// (otherwise patterns like `"@app/*"` would be eaten).
fn strip_jsonc_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    while let Some(&next) = chars.peek() {
                        if next == '\n' {
                            break;
                        }
                        chars.next();
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '*' {
                            if let Some(&'/') = chars.peek() {
                                chars.next();
                                break;
                            }
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_paths_with_base_url() {
        let content = r#"{
            // primary project tsconfig
            "compilerOptions": {
                "baseUrl": "./src",
                "paths": {
                    "@app/*": ["app/*"],
                    "@shared": ["shared/index.ts"]
                }
            }
        }"#;
        let layout = parse_tsconfig_json(content, Path::new("/repo")).unwrap();
        assert_eq!(
            layout.modules.get("@app"),
            Some(&PathBuf::from("/repo/./src/app"))
        );
        assert_eq!(
            layout.modules.get("@shared"),
            Some(&PathBuf::from("/repo/./src/shared/index.ts"))
        );
    }

    #[test]
    fn handles_empty_config() {
        let layout = parse_tsconfig_json("{}", Path::new("/repo")).unwrap();
        assert!(layout.modules.is_empty());
    }
}
