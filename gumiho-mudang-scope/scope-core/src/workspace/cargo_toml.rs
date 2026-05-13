//! `Cargo.toml` reader.
//!
//! Extracts the package name, root directory (parent of the manifest),
//! and `[dependencies]` + `[dev-dependencies]` entries. Workspace
//! members and virtual manifests are out of scope at this layer —
//! a workspace `Cargo.toml` is parsed by its members' own manifests.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see
//! `crate::workspace_context`), not this reader. `edition` and
//! `rust-version` are absent from the trait — CI gate
//! `audit_context_shape.sh` (R4) refuses any method whose name
//! suggests them, so language plugins cannot reach those fields
//! through any plugin-side path. This reader lives indexer-side and
//! may expose `edition` / `rust-version` extraction functions for
//! indexer-side consumers (R8 audit emit per `BACKLOG.md` §
//! Priority 1 sub-item (d)); plugins never reach those functions
//! because they receive the trait, not this module.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::workspace_context::{Dependency, Package};

#[derive(Deserialize)]
struct Manifest {
    package: Option<PackageSection>,
    #[serde(default)]
    dependencies: toml::Table,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: toml::Table,
}

#[derive(Deserialize)]
struct PackageSection {
    name: String,
}

#[derive(Deserialize)]
struct VersionSection {
    edition: Option<String>,
    #[serde(rename = "rust-version")]
    rust_version: Option<String>,
}

#[derive(Deserialize)]
struct VersionManifest {
    package: Option<VersionSection>,
}

/// Read and parse a `Cargo.toml` at `path`.
pub fn read_cargo_toml(path: &Path) -> Result<Package> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read Cargo.toml at {}", path.display()))?;
    let root = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    parse_cargo_toml(&content).map(|mut pkg| {
        pkg.root = root;
        pkg
    })
}

/// Parse an in-memory `Cargo.toml` string.
pub fn parse_cargo_toml(content: &str) -> Result<Package> {
    let manifest: Manifest = toml::from_str(content).context("parse Cargo.toml")?;
    let name = manifest
        .package
        .map(|p| p.name)
        .unwrap_or_else(|| "<workspace>".to_string());

    let mut dependencies = Vec::new();
    for (dep_name, value) in manifest.dependencies {
        dependencies.push(Dependency {
            name: dep_name,
            version_req: extract_version(&value),
            dev_only: false,
        });
    }
    for (dep_name, value) in manifest.dev_dependencies {
        dependencies.push(Dependency {
            name: dep_name,
            version_req: extract_version(&value),
            dev_only: true,
        });
    }
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Package {
        name,
        root: Default::default(),
        dependencies,
    })
}

/// Indexer-side carveout (R4): extract the `[package].edition` value
/// from a `Cargo.toml` string. Returns `None` when the manifest is a
/// virtual workspace (no `[package]` section) or omits `edition`
/// (Rust defaults to `"2015"` in that case; we surface `None` to
/// distinguish "absent" from "declared" — the R8 emit caller decides
/// the default).
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_edition(content: &str) -> Option<String> {
    let manifest: VersionManifest = toml::from_str(content).ok()?;
    manifest.package.and_then(|p| p.edition)
}

/// Indexer-side carveout (R4): extract `[package].rust-version` from a
/// `Cargo.toml` string. Returns `None` when absent. The value is the
/// minimum supported Rust toolchain (MSRV); orthogonal to `edition`.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_rust_version(content: &str) -> Option<String> {
    let manifest: VersionManifest = toml::from_str(content).ok()?;
    manifest.package.and_then(|p| p.rust_version)
}

fn extract_version(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_manifest() {
        let content = r#"
            [package]
            name = "scope-core"

            [dependencies]
            anyhow = "1.0"
            serde = { version = "1", features = ["derive"] }

            [dev-dependencies]
            tempfile = "3.10"
        "#;
        let pkg = parse_cargo_toml(content).unwrap();
        assert_eq!(pkg.name, "scope-core");
        assert_eq!(pkg.dependencies.len(), 3);
        let anyhow = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "anyhow")
            .unwrap();
        assert_eq!(anyhow.version_req, "1.0");
        assert!(!anyhow.dev_only);
        let tempfile = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "tempfile")
            .unwrap();
        assert!(tempfile.dev_only);
    }

    #[test]
    fn handles_virtual_workspace() {
        let content = r#"
            [workspace]
            members = ["a", "b"]
        "#;
        let pkg = parse_cargo_toml(content).unwrap();
        assert_eq!(pkg.name, "<workspace>");
        assert!(pkg.dependencies.is_empty());
    }

    #[test]
    fn extracts_edition_when_present() {
        let content = r#"
            [package]
            name = "x"
            edition = "2021"
        "#;
        assert_eq!(extract_edition(content).as_deref(), Some("2021"));
    }

    #[test]
    fn extracts_edition_returns_none_when_omitted() {
        let content = r#"
            [package]
            name = "x"
        "#;
        assert_eq!(extract_edition(content), None);
    }

    #[test]
    fn extracts_edition_returns_none_for_virtual_workspace() {
        let content = r#"
            [workspace]
            members = ["a"]
        "#;
        assert_eq!(extract_edition(content), None);
    }

    #[test]
    fn extracts_rust_version_when_present() {
        let content = r#"
            [package]
            name = "x"
            rust-version = "1.75"
        "#;
        assert_eq!(extract_rust_version(content).as_deref(), Some("1.75"));
    }

    #[test]
    fn extracts_rust_version_returns_none_when_omitted() {
        let content = r#"
            [package]
            name = "x"
            edition = "2021"
        "#;
        assert_eq!(extract_rust_version(content), None);
    }

    #[test]
    fn extraction_returns_none_on_malformed_toml() {
        let content = "this is not valid TOML [[[";
        assert_eq!(extract_edition(content), None);
        assert_eq!(extract_rust_version(content), None);
    }
}
