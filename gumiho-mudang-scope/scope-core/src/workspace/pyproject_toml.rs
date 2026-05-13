//! `pyproject.toml` reader.
//!
//! Handles both PEP 621 (`[project]`) and Poetry (`[tool.poetry]`)
//! layouts. Dependencies come from `project.dependencies`,
//! `project.optional-dependencies`, `tool.poetry.dependencies`, and
//! `tool.poetry.dev-dependencies`. Resolved versions live in
//! `poetry.lock` — out of scope for this reader.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see
//! `crate::workspace_context`), not this reader. `python_requires`
//! is absent from the trait — CI gate `audit_context_shape.sh` (R4)
//! refuses any method whose name suggests it, so language plugins
//! cannot reach the Python-version range through any plugin-side
//! path. This reader lives indexer-side and may expose
//! `requires-python` extraction for indexer-side consumers (R8
//! audit emit per `BACKLOG.md` § Priority 1 sub-item (d)); plugins
//! never reach that function because they receive the trait, not
//! this module.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::workspace_context::{Dependency, Package};

#[derive(Deserialize, Default)]
struct Manifest {
    #[serde(default)]
    project: Option<Pep621Project>,
    #[serde(default)]
    tool: Option<ToolSection>,
}

#[derive(Deserialize, Default)]
struct Pep621Project {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default, rename = "optional-dependencies")]
    optional_dependencies: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize, Default)]
struct ToolSection {
    #[serde(default)]
    poetry: Option<PoetrySection>,
}

#[derive(Deserialize, Default)]
struct PoetrySection {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize, Default)]
struct VersionManifest {
    #[serde(default)]
    project: Option<VersionProject>,
    #[serde(default)]
    tool: Option<VersionTool>,
}

#[derive(Deserialize, Default)]
struct VersionProject {
    #[serde(default, rename = "requires-python")]
    requires_python: Option<String>,
}

#[derive(Deserialize, Default)]
struct VersionTool {
    #[serde(default)]
    poetry: Option<VersionPoetry>,
}

#[derive(Deserialize, Default)]
struct VersionPoetry {
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
}

/// Read and parse a `pyproject.toml` at `path`.
pub fn read_pyproject_toml(path: &Path) -> Result<Package> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read pyproject.toml at {}", path.display()))?;
    let root = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    parse_pyproject_toml(&content).map(|mut pkg| {
        pkg.root = root;
        pkg
    })
}

/// Parse an in-memory `pyproject.toml` string.
pub fn parse_pyproject_toml(content: &str) -> Result<Package> {
    let manifest: Manifest = toml::from_str(content).context("parse pyproject.toml")?;

    let mut name = manifest.project.as_ref().and_then(|p| p.name.clone());
    let mut dependencies = Vec::new();

    if let Some(project) = manifest.project {
        for spec in project.dependencies {
            dependencies.push(parse_pep508_dep(&spec, false));
        }
        for (_extra, deps) in project.optional_dependencies {
            for spec in deps {
                dependencies.push(parse_pep508_dep(&spec, true));
            }
        }
    }

    if let Some(tool) = manifest.tool {
        if let Some(poetry) = tool.poetry {
            if name.is_none() {
                name = poetry.name;
            }
            for (dep_name, value) in poetry.dependencies {
                if dep_name == "python" {
                    continue;
                }
                dependencies.push(Dependency {
                    name: dep_name,
                    version_req: poetry_version_string(&value),
                    dev_only: false,
                });
            }
            for (dep_name, value) in poetry.dev_dependencies {
                dependencies.push(Dependency {
                    name: dep_name,
                    version_req: poetry_version_string(&value),
                    dev_only: true,
                });
            }
        }
    }

    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Package {
        name: name.unwrap_or_else(|| "<unnamed>".to_string()),
        root: Default::default(),
        dependencies,
    })
}

/// PEP 508 dependency string: `requests>=2.31`, `flask[async] >= 2.0`,
/// `numpy ; python_version >= '3.10'`. The reader keeps name + version
/// constraint; extras and environment markers are dropped at this
/// layer.
fn parse_pep508_dep(spec: &str, dev_only: bool) -> Dependency {
    let spec = spec.split(';').next().unwrap_or(spec).trim();
    let (name_part, version_part) = spec
        .find(|c: char| "<>=~!".contains(c))
        .map(|idx| spec.split_at(idx))
        .unwrap_or((spec, ""));
    let name = name_part
        .split('[')
        .next()
        .unwrap_or(name_part)
        .trim()
        .to_string();
    Dependency {
        name,
        version_req: version_part.trim().to_string(),
        dev_only,
    }
}

/// Indexer-side carveout (R4): extract the Python version constraint
/// from a `pyproject.toml` string. Tries (in order):
///
/// 1. `[project].requires-python` (PEP 621) — e.g. `">=3.10"`.
/// 2. `[tool.poetry.dependencies].python` (Poetry) — e.g. `"^3.10"`.
///
/// Returns `None` when neither is present. The string is the raw
/// version requirement; the caller is responsible for normalising
/// (the R8 audit emit stores the raw spec verbatim for
/// `lang_version`).
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_requires_python(content: &str) -> Option<String> {
    let manifest: VersionManifest = toml::from_str(content).ok()?;
    if let Some(project) = manifest.project {
        if let Some(req) = project.requires_python {
            return Some(req);
        }
    }
    if let Some(tool) = manifest.tool {
        if let Some(poetry) = tool.poetry {
            if let Some(python_value) = poetry.dependencies.get("python") {
                let spec = poetry_version_string(python_value);
                if !spec.is_empty() {
                    return Some(spec);
                }
            }
        }
    }
    None
}

fn poetry_version_string(value: &toml::Value) -> String {
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
    fn parses_pep621() {
        let content = r#"
            [project]
            name = "my-app"
            dependencies = ["requests>=2.31", "flask[async] >= 2.0"]

            [project.optional-dependencies]
            test = ["pytest>=7", "hypothesis"]
        "#;
        let pkg = parse_pyproject_toml(content).unwrap();
        assert_eq!(pkg.name, "my-app");
        let names: Vec<_> = pkg.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"requests"));
        assert!(names.contains(&"flask"));
        assert!(names.contains(&"pytest"));
        let pytest = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "pytest")
            .unwrap();
        assert!(pytest.dev_only);
    }

    #[test]
    fn parses_poetry() {
        let content = r#"
            [tool.poetry]
            name = "my-app"

            [tool.poetry.dependencies]
            python = "^3.10"
            requests = "^2.31"

            [tool.poetry.dev-dependencies]
            pytest = { version = "^7.0", extras = ["all"] }
        "#;
        let pkg = parse_pyproject_toml(content).unwrap();
        assert_eq!(pkg.name, "my-app");
        let names: Vec<_> = pkg.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert!(!names.contains(&"python"));
        assert!(names.contains(&"requests"));
        let pytest = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "pytest")
            .unwrap();
        assert_eq!(pytest.version_req, "^7.0");
        assert!(pytest.dev_only);
    }

    #[test]
    fn extracts_requires_python_pep621() {
        let content = r#"
            [project]
            name = "x"
            requires-python = ">=3.10"
        "#;
        assert_eq!(extract_requires_python(content).as_deref(), Some(">=3.10"));
    }

    #[test]
    fn extracts_requires_python_poetry() {
        let content = r#"
            [tool.poetry]
            name = "x"

            [tool.poetry.dependencies]
            python = "^3.10"
        "#;
        assert_eq!(extract_requires_python(content).as_deref(), Some("^3.10"));
    }

    #[test]
    fn extracts_requires_python_poetry_table_form() {
        let content = r#"
            [tool.poetry.dependencies]
            python = { version = "^3.11" }
        "#;
        assert_eq!(extract_requires_python(content).as_deref(), Some("^3.11"));
    }

    #[test]
    fn extracts_requires_python_prefers_pep621_over_poetry() {
        let content = r#"
            [project]
            requires-python = ">=3.12"

            [tool.poetry.dependencies]
            python = "^3.10"
        "#;
        assert_eq!(extract_requires_python(content).as_deref(), Some(">=3.12"));
    }

    #[test]
    fn extracts_requires_python_returns_none_when_absent() {
        let content = r#"
            [project]
            name = "x"
        "#;
        assert_eq!(extract_requires_python(content), None);
    }
}
