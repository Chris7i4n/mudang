//! `package.json` reader.
//!
//! Extracts `name`, `dependencies`, `devDependencies`. The
//! `peerDependencies` and `optionalDependencies` sections are out of
//! scope; if a sprint demonstrates a concrete need, they extend
//! `Dependency` (open question for that sprint) rather than being
//! silently merged into the existing slots.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::workspace_context::{Dependency, Package};

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: BTreeMap<String, String>,
}

/// Read and parse a `package.json` at `path`.
pub fn read_package_json(path: &Path) -> Result<Package> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read package.json at {}", path.display()))?;
    let root = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    parse_package_json(&content).map(|mut pkg| {
        pkg.root = root;
        pkg
    })
}

/// Parse an in-memory `package.json` string.
pub fn parse_package_json(content: &str) -> Result<Package> {
    let manifest: Manifest = serde_json::from_str(content).context("parse package.json")?;
    let name = manifest.name.unwrap_or_else(|| "<unnamed>".to_string());

    let mut dependencies = Vec::new();
    for (name, req) in manifest.dependencies {
        dependencies.push(Dependency {
            name,
            version_req: req,
            dev_only: false,
        });
    }
    for (name, req) in manifest.dev_dependencies {
        dependencies.push(Dependency {
            name,
            version_req: req,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_manifest() {
        let content = r#"{
            "name": "my-app",
            "dependencies": { "react": "^18.0.0", "axios": "1.6.0" },
            "devDependencies": { "jest": "^29.0.0" }
        }"#;
        let pkg = parse_package_json(content).unwrap();
        assert_eq!(pkg.name, "my-app");
        assert_eq!(pkg.dependencies.len(), 3);
        let react = pkg.dependencies.iter().find(|d| d.name == "react").unwrap();
        assert_eq!(react.version_req, "^18.0.0");
        assert!(!react.dev_only);
        let jest = pkg.dependencies.iter().find(|d| d.name == "jest").unwrap();
        assert!(jest.dev_only);
    }

    #[test]
    fn handles_missing_name() {
        let pkg = parse_package_json("{}").unwrap();
        assert_eq!(pkg.name, "<unnamed>");
    }
}
