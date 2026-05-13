//! `.csproj` reader — C# / .NET SDK-style projects.
//!
//! Modern SDK-style csproj files declare the runtime target via
//! `<TargetFramework>net8.0</TargetFramework>` (single) or
//! `<TargetFrameworks>net8.0;net6.0</TargetFrameworks>` (multi-target).
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see `crate::workspace_context`),
//! not this reader. No `target_framework` accessor exists on the trait
//! — CI gate `audit_context_shape.sh` (R4) refuses any method whose
//! name suggests one, so language plugins cannot reach the
//! `TargetFramework` through any plugin-side path. This reader lives
//! indexer-side and may expose `TargetFramework` extraction for
//! indexer-side consumers (R8 audit emit per `BACKLOG.md` § Priority 1
//! sub-item (d)); plugins never reach this function because they
//! receive the trait, not this module.
//!
//! ## Honest limits
//!
//! Legacy non-SDK csproj files (pre-`.NET Core`) used
//! `<TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>` — this
//! reader also recognises that as a last-resort fallback. Multi-target
//! projects return the **first** moniker in `<TargetFrameworks>` (the
//! primary build target); callers needing the full set should query
//! the raw element. Property placeholders (`$(NetVersion)`) are
//! returned verbatim, unresolved.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read `.csproj` at `path` and extract the target framework moniker.
pub fn read_target_framework(path: &Path) -> Result<Option<String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read .csproj at {}", path.display()))?;
    Ok(extract_target_framework(&content))
}

/// Indexer-side carveout (R4): extract the .NET target framework
/// moniker from a `.csproj` string. Tries (in order):
///
/// 1. `<TargetFramework>` — SDK-style single target.
/// 2. `<TargetFrameworks>` — SDK-style multi-target (first wins).
/// 3. `<TargetFrameworkVersion>` — legacy non-SDK form.
///
/// Returns the raw element body (e.g. `"net8.0"`, `"netstandard2.1"`,
/// `"v4.7.2"`) verbatim. Returns `None` when none are present.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_target_framework(content: &str) -> Option<String> {
    if let Some(v) = find_element_body(content, "TargetFramework") {
        return Some(v);
    }
    if let Some(v) = find_element_body(content, "TargetFrameworks") {
        let first = v.split(';').next()?.trim().to_string();
        if !first.is_empty() {
            return Some(first);
        }
    }
    if let Some(v) = find_element_body(content, "TargetFrameworkVersion") {
        return Some(v);
    }
    None
}

fn find_element_body(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = content.find(&open)? + open.len();
    let end_rel = content[start..].find(&close)?;
    let body = content[start..start + end_rel].trim();
    if body.is_empty() {
        None
    } else {
        Some(body.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_target_framework() {
        let content = r#"
            <Project Sdk="Microsoft.NET.Sdk">
              <PropertyGroup>
                <TargetFramework>net8.0</TargetFramework>
              </PropertyGroup>
            </Project>
        "#;
        assert_eq!(extract_target_framework(content).as_deref(), Some("net8.0"));
    }

    #[test]
    fn extracts_first_of_multi_target() {
        let content = r#"
            <Project Sdk="Microsoft.NET.Sdk">
              <PropertyGroup>
                <TargetFrameworks>net8.0;net6.0;netstandard2.1</TargetFrameworks>
              </PropertyGroup>
            </Project>
        "#;
        assert_eq!(extract_target_framework(content).as_deref(), Some("net8.0"));
    }

    #[test]
    fn extracts_legacy_target_framework_version() {
        let content = r#"
            <Project>
              <PropertyGroup>
                <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>
              </PropertyGroup>
            </Project>
        "#;
        assert_eq!(extract_target_framework(content).as_deref(), Some("v4.7.2"));
    }

    #[test]
    fn prefers_target_framework_over_legacy() {
        let content = r#"
            <PropertyGroup>
                <TargetFramework>net8.0</TargetFramework>
                <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>
            </PropertyGroup>
        "#;
        assert_eq!(extract_target_framework(content).as_deref(), Some("net8.0"));
    }

    #[test]
    fn returns_property_placeholder_verbatim() {
        let content =
            "<PropertyGroup><TargetFramework>$(NetVersion)</TargetFramework></PropertyGroup>";
        assert_eq!(
            extract_target_framework(content).as_deref(),
            Some("$(NetVersion)")
        );
    }

    #[test]
    fn returns_none_when_absent() {
        let content = r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#;
        assert_eq!(extract_target_framework(content), None);
    }

    #[test]
    fn returns_none_on_empty_body() {
        let content = "<TargetFramework></TargetFramework>";
        assert_eq!(extract_target_framework(content), None);
    }
}
