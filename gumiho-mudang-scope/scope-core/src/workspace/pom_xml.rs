//! `pom.xml` reader — Maven projects.
//!
//! Maven encodes the Java compiler version through several conventions.
//! Modern projects use the `<maven.compiler.release>` property (Java 9+
//! `--release`), older projects use `<maven.compiler.target>` /
//! `<maven.compiler.source>`, and a long-standing community convention
//! uses a plain `<java.version>` property consumed by Spring Boot's
//! parent POM. This reader looks at all of them and returns the first
//! match in canonical priority order.
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
//! This is a textual scan, not a full Maven model evaluation. It does
//! not resolve parent POM inheritance, profile activation, or property
//! interpolation (`${some.prop}`). The scan handles the canonical
//! literal forms used by the vast majority of real `pom.xml` files;
//! projects relying on parent-POM or profile-driven version selection
//! will return `None` here and surface as `null` in the audit emit.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read `pom.xml` at `path` and extract the Java compiler version.
///
/// See `extract_java_version` for resolution priority.
pub fn read_java_version(path: &Path) -> Result<Option<String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read pom.xml at {}", path.display()))?;
    Ok(extract_java_version(&content))
}

/// Indexer-side carveout (R4): extract the Java compiler version from a
/// `pom.xml` string. Tries (in order):
///
/// 1. `<maven.compiler.release>` — modern Java-9+ form.
/// 2. `<maven.compiler.target>` — pre-9 form.
/// 3. `<maven.compiler.source>` — fallback when only source is set.
/// 4. `<java.version>` — Spring Boot / community convention.
///
/// Returns `None` when none match. The returned string is the raw
/// element body verbatim (`"17"`, `"1.8"`, `"21"`); normalisation is
/// the caller's concern.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_java_version(content: &str) -> Option<String> {
    for tag in [
        "maven.compiler.release",
        "maven.compiler.target",
        "maven.compiler.source",
        "java.version",
    ] {
        if let Some(v) = find_element_body(content, tag) {
            return Some(v);
        }
    }
    None
}

/// Find the first `<tag>body</tag>` and return the trimmed body. Naive
/// — no namespace handling, no nested-same-name tags. Enough for the
/// property-element forms this reader targets.
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
    fn extracts_release_when_present() {
        let content = r#"
            <project>
              <properties>
                <maven.compiler.release>21</maven.compiler.release>
              </properties>
            </project>
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("21"));
    }

    #[test]
    fn extracts_target_when_release_absent() {
        let content = r#"
            <project>
              <properties>
                <maven.compiler.source>1.8</maven.compiler.source>
                <maven.compiler.target>1.8</maven.compiler.target>
              </properties>
            </project>
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("1.8"));
    }

    #[test]
    fn extracts_source_when_only_source_set() {
        let content = r#"
            <properties>
                <maven.compiler.source>11</maven.compiler.source>
            </properties>
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("11"));
    }

    #[test]
    fn extracts_java_version_community_property() {
        let content = r#"
            <properties>
                <java.version>17</java.version>
            </properties>
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("17"));
    }

    #[test]
    fn prefers_release_over_target() {
        let content = r#"
            <properties>
                <maven.compiler.release>21</maven.compiler.release>
                <maven.compiler.target>17</maven.compiler.target>
            </properties>
        "#;
        assert_eq!(extract_java_version(content).as_deref(), Some("21"));
    }

    #[test]
    fn returns_verbatim_placeholder_on_unresolved_property() {
        // A property placeholder is not a literal value; this reader
        // doesn't resolve parent POM inheritance, so the body is the
        // verbatim placeholder string. Document the behaviour: we
        // return the placeholder rather than None, because the caller
        // (audit emit) records the raw spec and downstream consumers
        // see it is unresolved.
        let content = r#"
            <properties>
                <maven.compiler.release>${java.target}</maven.compiler.release>
            </properties>
        "#;
        assert_eq!(
            extract_java_version(content).as_deref(),
            Some("${java.target}")
        );
    }

    #[test]
    fn returns_none_when_absent() {
        let content = "<project><modelVersion>4.0.0</modelVersion></project>";
        assert_eq!(extract_java_version(content), None);
    }

    #[test]
    fn returns_none_on_empty_body() {
        let content = "<properties><maven.compiler.release></maven.compiler.release></properties>";
        assert_eq!(extract_java_version(content), None);
    }
}
