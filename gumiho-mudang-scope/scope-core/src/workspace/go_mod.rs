//! `go.mod` reader.
//!
//! Hand-parses the line-oriented `go.mod` format. Recognises:
//! - `module <path>` — package name.
//! - `require <path> <version>` — single-line require.
//! - `require ( ... )` — multi-line require block.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see
//! `crate::workspace_context`), not this reader. The trait has no
//! `go_directive` accessor — CI gate `audit_context_shape.sh` (R4)
//! refuses any method whose name suggests one, so language plugins
//! cannot reach the `go <version>` directive through any plugin-side
//! path. This reader lives indexer-side and may expose a
//! `go <version>` extraction function for indexer-side consumers
//! (R8 audit emit per `BACKLOG.md` § Priority 1 sub-item (d));
//! plugins never reach it because they receive the trait, not this
//! module.
//!
//! Format reference: <https://go.dev/ref/mod#go-mod-file>.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::workspace_context::{Dependency, Package};

/// Read and parse a `go.mod` at `path`.
pub fn read_go_mod(path: &Path) -> Result<Package> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read go.mod at {}", path.display()))?;
    let root = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    parse_go_mod(&content).map(|mut pkg| {
        pkg.root = root;
        pkg
    })
}

/// Parse an in-memory `go.mod` string.
pub fn parse_go_mod(content: &str) -> Result<Package> {
    let mut name = String::from("<unnamed>");
    let mut dependencies: Vec<Dependency> = Vec::new();
    let mut in_require_block = false;

    for raw_line in content.lines() {
        let line = strip_line_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if in_require_block {
            if line == ")" {
                in_require_block = false;
                continue;
            }
            if let Some(dep) = parse_require_entry(line) {
                dependencies.push(dep);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("module ") {
            name = rest.trim().trim_matches('"').to_string();
            continue;
        }
        if line == "require (" {
            in_require_block = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("require ") {
            if let Some(dep) = parse_require_entry(rest.trim()) {
                dependencies.push(dep);
            }
        }
    }

    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Package {
        name,
        root: Default::default(),
        dependencies,
    })
}

/// Indexer-side carveout (R4): extract the `go <version>` directive
/// from a `go.mod` string. Returns `None` when absent (rare in
/// practice — every modern `go.mod` carries one). Walks line-by-line
/// like `parse_go_mod` to share the comment-stripping behaviour;
/// stops at the first matching line.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_go_directive(content: &str) -> Option<String> {
    for raw_line in content.lines() {
        let line = strip_line_comment(raw_line).trim();
        if let Some(rest) = line.strip_prefix("go ") {
            let version = rest.trim();
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn parse_require_entry(line: &str) -> Option<Dependency> {
    // Go's `// indirect` marker means the dependency is not directly
    // imported by this module — NOT that it is test/dev-only. Don't
    // map it onto `dev_only`; any consumer filtering `dev_only` would
    // drop real runtime requirements. go.mod has no dev-deps concept
    // (test-only deps live in test files, not the manifest).
    let mut parts = line.split_whitespace();
    let name = parts.next()?.to_string();
    let version = parts.next().unwrap_or("").to_string();
    Some(Dependency {
        name,
        version_req: version,
        dev_only: false,
    })
}

/// Strip end-of-line `//` comments. Comment-only lines collapse to an
/// empty string and are skipped by the empty-check upstream. Without
/// this, `// foo` inside a `require (...)` block becomes a bogus
/// dependency named `//`, and `module example.com/app // comment`
/// stores the comment as part of the package name.
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => line[..idx].trim_end(),
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module_and_requires() {
        let content = r#"
module example.com/my/app

go 1.22

require github.com/gin-gonic/gin v1.9.1

require (
    github.com/stretchr/testify v1.8.4
    golang.org/x/sync v0.5.0 // indirect
)
"#;
        let pkg = parse_go_mod(content).unwrap();
        assert_eq!(pkg.name, "example.com/my/app");
        assert_eq!(pkg.dependencies.len(), 3);
        let gin = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "github.com/gin-gonic/gin")
            .unwrap();
        assert_eq!(gin.version_req, "v1.9.1");
        assert!(!gin.dev_only);
        // Indirect deps are runtime requirements, not dev/test deps.
        let xsync = pkg
            .dependencies
            .iter()
            .find(|d| d.name == "golang.org/x/sync")
            .unwrap();
        assert!(!xsync.dev_only);
    }

    #[test]
    fn strips_comment_only_lines_inside_require_block() {
        let content = r#"
module example.com/app

require (
    // top-of-block comment
    github.com/foo/bar v1.0.0
    // inline comment between entries
)
"#;
        let pkg = parse_go_mod(content).unwrap();
        assert_eq!(pkg.dependencies.len(), 1);
        assert_eq!(pkg.dependencies[0].name, "github.com/foo/bar");
    }

    #[test]
    fn strips_trailing_comment_on_module_line() {
        let content = "module example.com/app // a comment\n";
        let pkg = parse_go_mod(content).unwrap();
        assert_eq!(pkg.name, "example.com/app");
    }

    #[test]
    fn strips_trailing_comment_on_single_line_require() {
        let content = "require github.com/foo/bar v1.2.3 // some note\n";
        let pkg = parse_go_mod(content).unwrap();
        assert_eq!(pkg.dependencies.len(), 1);
        assert_eq!(pkg.dependencies[0].version_req, "v1.2.3");
    }

    #[test]
    fn extracts_go_directive_major_minor() {
        let content = "module x\n\ngo 1.22\n";
        assert_eq!(extract_go_directive(content).as_deref(), Some("1.22"));
    }

    #[test]
    fn extracts_go_directive_major_minor_patch() {
        let content = "module x\ngo 1.21.5\n";
        assert_eq!(extract_go_directive(content).as_deref(), Some("1.21.5"));
    }

    #[test]
    fn extracts_go_directive_strips_trailing_comment() {
        let content = "module x\ngo 1.22 // pinned\n";
        assert_eq!(extract_go_directive(content).as_deref(), Some("1.22"));
    }

    #[test]
    fn extracts_go_directive_returns_none_when_absent() {
        let content = "module x\n\nrequire github.com/foo/bar v1.0.0\n";
        assert_eq!(extract_go_directive(content), None);
    }
}
