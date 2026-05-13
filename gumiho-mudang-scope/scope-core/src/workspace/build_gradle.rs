//! `build.gradle` / `build.gradle.kts` reader — Gradle projects.
//!
//! Gradle build scripts are Groovy or Kotlin source, not a static
//! manifest. This reader does *not* execute them — it scans for the
//! canonical Java-version directives textually.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see `crate::workspace_context`),
//! not this reader. No `java_version` accessor exists on the trait —
//! CI gate `audit_context_shape.sh` (R4) refuses any method whose name
//! suggests one, so language plugins cannot reach the Java version
//! through any plugin-side path. This reader lives indexer-side and
//! may expose Java-version extraction for indexer-side consumers (R8
//! audit emit per `BACKLOG.md` § Priority 1 sub-item (d)); plugins
//! never reach this function because they receive the trait, not this
//! module.
//!
//! ## Honest limits
//!
//! The scanner recognises:
//!
//! - `sourceCompatibility = '17'` / `sourceCompatibility "17"` / `sourceCompatibility = JavaVersion.VERSION_17`
//! - `targetCompatibility = '17'` (same shapes)
//! - `languageVersion = JavaLanguageVersion.of(17)` (Gradle Java toolchain)
//!
//! It does not handle dynamic expressions (`sourceCompatibility = libs.versions.java.get()`),
//! external property references, or convention plugins that set the
//! value indirectly. Projects relying on those will return `None` and
//! surface as `null` in the audit emit.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read a Gradle build script at `path` and extract the Java version.
///
/// Handles both Groovy (`build.gradle`) and Kotlin DSL
/// (`build.gradle.kts`) — both share the directive surface this reader
/// recognises.
pub fn read_java_version(path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read Gradle build at {}", path.display()))?;
    Ok(extract_java_version(&content))
}

/// Indexer-side carveout (R4): extract the Java version from a Gradle
/// build-script string. Returns the raw version (e.g. `"17"`, `"1.8"`)
/// stripping the `JavaVersion.VERSION_` / `JavaLanguageVersion.of(...)`
/// wrappers when present. Resolution priority:
///
/// 1. `targetCompatibility`
/// 2. `sourceCompatibility`
/// 3. `languageVersion = JavaLanguageVersion.of(N)` (toolchain block)
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_java_version(content: &str) -> Option<String> {
    for keyword in ["targetCompatibility", "sourceCompatibility"] {
        if let Some(v) = find_compatibility_value(content, keyword) {
            return Some(v);
        }
    }
    if let Some(v) = find_language_version(content) {
        return Some(v);
    }
    None
}

fn find_compatibility_value(content: &str, keyword: &str) -> Option<String> {
    for line in content.lines() {
        let stripped = strip_line_comment(line).trim();
        let Some(rest) = stripped.strip_prefix(keyword) else {
            continue;
        };
        // Word boundary — `sourceCompatibilityFoo` must not match.
        if let Some(c) = rest.chars().next() {
            if c.is_alphanumeric() || c == '_' {
                continue;
            }
        }
        let after = rest.trim_start().strip_prefix('=').unwrap_or(rest);
        let after = after.trim();
        if let Some(v) = parse_value_token(after) {
            return Some(v);
        }
    }
    None
}

fn find_language_version(content: &str) -> Option<String> {
    let needle = "JavaLanguageVersion.of(";
    let idx = content.find(needle)?;
    let after = &content[idx + needle.len()..];
    let end = after.find(')')?;
    let inner = after[..end].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

/// Parse the value after `sourceCompatibility =`. Handles:
/// - `'17'` or `"17"` — quoted literal.
/// - `JavaVersion.VERSION_17` → `17`.
/// - `JavaVersion.VERSION_1_8` → `1.8`.
fn parse_value_token(after: &str) -> Option<String> {
    let trimmed = after.trim();
    if let Some(rest) = trimmed.strip_prefix('\'') {
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("JavaVersion.VERSION_") {
        let token: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if token.is_empty() {
            return None;
        }
        return Some(token.replace('_', "."));
    }
    None
}

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quoted_target_compatibility() {
        let content = "sourceCompatibility = '11'\ntargetCompatibility = '17'\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn extracts_double_quoted() {
        let content = r#"targetCompatibility = "17""#;
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn extracts_java_version_enum_modern() {
        let content = "sourceCompatibility = JavaVersion.VERSION_17\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn extracts_java_version_enum_legacy() {
        let content = "sourceCompatibility = JavaVersion.VERSION_1_8\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("1.8"));
    }

    #[test]
    fn extracts_groovy_method_call_form() {
        // Groovy permits `sourceCompatibility "17"` without `=`.
        let content = "sourceCompatibility '17'\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn extracts_toolchain_language_version() {
        let content = r#"
            java {
                toolchain {
                    languageVersion = JavaLanguageVersion.of(21)
                }
            }
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("21"));
    }

    #[test]
    fn target_wins_over_source() {
        let content = "sourceCompatibility = '11'\ntargetCompatibility = '17'\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn ignores_commented_directive() {
        let content = "// sourceCompatibility = '11'\ntargetCompatibility = '17'\n";
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn returns_none_when_absent() {
        let content = "plugins { id 'java' }\n";
        assert_eq!(extract_java_version(content), None);
    }
}
