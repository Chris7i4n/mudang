//! `Gemfile.lock` reader.
//!
//! Bundler's lockfile format is line-oriented, not JSON/TOML. We parse
//! the `GEM > specs:` section and the `DEPENDENCIES` section to recover
//! resolved gem versions; the result feeds `FrameworkWorkspaceContext::
//! lockfile()` (R5 consumer).
//!
//! Format reference: <https://bundler.io/man/gemfile.5.html>.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::workspace_context::Lockfile;

/// Read and parse a `Gemfile.lock` at `path`.
pub fn read_gemfile_lock(path: &Path) -> Result<Lockfile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read Gemfile.lock at {}", path.display()))?;
    let mut lockfile = parse_gemfile_lock(&content)?;
    lockfile.source_path = path.to_path_buf();
    Ok(lockfile)
}

/// Parse an in-memory `Gemfile.lock` string.
pub fn parse_gemfile_lock(content: &str) -> Result<Lockfile> {
    let mut resolved_versions: BTreeMap<String, String> = BTreeMap::new();
    let mut in_specs = false;

    for line in content.lines() {
        let trimmed = line.trim_end();

        if trimmed == "GEM" {
            in_specs = false;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("  specs:") {
            let _ = rest;
            in_specs = true;
            continue;
        }
        if !line.starts_with(' ') {
            in_specs = false;
            continue;
        }
        if !in_specs {
            continue;
        }

        // Spec entry shape: "    rails (7.0.4)".
        // Nested dep shape: "      activemodel (= 7.0.4)" — six leading
        // spaces, we skip those (nested transitive deps).
        if line.starts_with("      ") {
            continue;
        }
        let Some(spec) = line.strip_prefix("    ") else {
            continue;
        };
        let Some((name, rest)) = spec.split_once(' ') else {
            continue;
        };
        let version = rest.trim_start_matches('(').trim_end_matches(')');
        resolved_versions.insert(name.to_string(), version.to_string());
    }

    Ok(Lockfile {
        resolved_versions,
        source_path: PathBuf::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_lockfile() {
        let content = r#"GEM
  remote: https://rubygems.org/
  specs:
    rails (7.0.4)
      actionmailbox (= 7.0.4)
      actionpack (= 7.0.4)
    actionpack (7.0.4)
      actionview (= 7.0.4)
    sidekiq (6.5.7)
      redis (>= 4.5.0)

PLATFORMS
  ruby

DEPENDENCIES
  rails (~> 7.0)
  sidekiq

BUNDLED WITH
   2.4.1
"#;
        let lockfile = parse_gemfile_lock(content).unwrap();
        assert_eq!(
            lockfile.resolved_versions.get("rails"),
            Some(&"7.0.4".to_string())
        );
        assert_eq!(
            lockfile.resolved_versions.get("actionpack"),
            Some(&"7.0.4".to_string())
        );
        assert_eq!(
            lockfile.resolved_versions.get("sidekiq"),
            Some(&"6.5.7".to_string())
        );
        assert_eq!(lockfile.resolved_versions.len(), 3);
    }
}
