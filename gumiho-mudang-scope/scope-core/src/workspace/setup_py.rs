//! `setup.py` reader — legacy Python projects.
//!
//! Pre-PEP-621 projects declare their metadata as a `setup(...)` call
//! in `setup.py`. The full file is Python source code, not a static
//! manifest format; the version slot is exposed via a kwarg
//! `python_requires='>=3.10'`. This reader does *not* execute the
//! file (R4 / CHARTER §5 — no plugin / no filesystem-effect Python
//! eval) — it scans for the kwarg textually.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see `crate::workspace_context`),
//! not this reader. This reader lives indexer-side and may expose
//! `python_requires` extraction for indexer-side consumers (R8 audit
//! emit per `BACKLOG.md` § Priority 1 sub-item (d)); plugins never
//! reach this function because they receive the trait, not this
//! module.
//!
//! ## Honest limits
//!
//! The scanner handles the canonical literal form used by the vast
//! majority of real `setup.py` files:
//!
//! ```python
//! setup(
//!     name='myapp',
//!     python_requires='>=3.10',
//! )
//! ```
//!
//! It does not handle:
//! - Dynamic values: `python_requires=open('requires').read()`,
//!   `python_requires=PYTHON_REQUIRES_CONST`. Returns `None`.
//! - Triple-quoted strings (rare for this kwarg). Returns `None`.
//! - Multi-`setup(...)` files where the kwarg appears in a non-setup
//!   call. First match wins; this is a non-issue in practice.
//! - Commented-out lines containing the kwarg. A leading `#`-comment
//!   could yield a false match; documented and accepted, since
//!   setup.py rarely carries commented-out version directives.
//!
//! Projects with these shapes can migrate to `pyproject.toml`
//! `[project].requires-python` (the canonical PEP 621 source), which
//! the `pyproject_toml` reader handles unambiguously.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read `setup.py` at `path` and extract the `python_requires` kwarg.
///
/// Returns `Ok(None)` when the file exists but the kwarg is absent or
/// uses a non-literal form; returns `Err` only on I/O failure.
pub fn read_python_requires(path: &Path) -> Result<Option<String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read setup.py at {}", path.display()))?;
    Ok(extract_python_requires(&content))
}

/// Indexer-side carveout (R4): extract the `python_requires` kwarg
/// from a `setup.py` source string. See module doc-comment for the
/// supported shape and honest limits.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_python_requires(content: &str) -> Option<String> {
    let key = "python_requires";
    let mut search_from = 0;
    while let Some(rel) = content[search_from..].find(key) {
        let idx = search_from + rel;
        // Ensure the match is a word boundary — not a prefix of a
        // longer identifier (`python_requires_extras` etc.).
        let after_key = idx + key.len();
        if let Some(c) = content[after_key..].chars().next() {
            if c.is_alphanumeric() || c == '_' {
                search_from = after_key;
                continue;
            }
        }
        if let Some(value) = parse_quoted_after_eq(&content[after_key..]) {
            return Some(value);
        }
        search_from = after_key;
    }
    None
}

/// Given the slice after `python_requires`, parse the trailing
/// `= '...'` (or `= "..."`, with optional `r` prefix). Returns the
/// quoted body verbatim, or `None` if the next non-whitespace tokens
/// do not form `= <quote>...<quote>`.
fn parse_quoted_after_eq(slice: &str) -> Option<String> {
    let after_eq = slice.trim_start().strip_prefix('=')?.trim_start();
    let after_eq = after_eq.strip_prefix('r').unwrap_or(after_eq);
    let mut chars = after_eq.chars();
    let quote = chars.next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let body_start = quote.len_utf8();
    let body = &after_eq[body_start..];
    let end = body.find(quote)?;
    Some(body[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_double_quoted() {
        let content = r#"
from setuptools import setup

setup(
    name="myapp",
    python_requires=">=3.10",
)
"#;
        assert_eq!(extract_python_requires(content).as_deref(), Some(">=3.10"));
    }

    #[test]
    fn extracts_single_quoted() {
        let content = "setup(name='x', python_requires='>=3.8')\n";
        assert_eq!(extract_python_requires(content).as_deref(), Some(">=3.8"));
    }

    #[test]
    fn extracts_with_whitespace_around_eq() {
        let content = "setup(python_requires    =   '>=3.11')\n";
        assert_eq!(extract_python_requires(content).as_deref(), Some(">=3.11"));
    }

    #[test]
    fn extracts_with_raw_string_prefix() {
        let content = "setup(python_requires=r'>=3.9')\n";
        assert_eq!(extract_python_requires(content).as_deref(), Some(">=3.9"));
    }

    #[test]
    fn returns_none_when_absent() {
        let content = "setup(name='x', install_requires=['flask'])\n";
        assert_eq!(extract_python_requires(content), None);
    }

    #[test]
    fn returns_none_for_dynamic_value() {
        let content = "setup(python_requires=open('req').read())\n";
        assert_eq!(extract_python_requires(content), None);
    }

    #[test]
    fn returns_none_for_const_reference() {
        let content = "setup(python_requires=PYTHON_REQUIRES)\n";
        assert_eq!(extract_python_requires(content), None);
    }

    #[test]
    fn ignores_longer_identifier_prefix() {
        // A kwarg named `python_requires_extras` must not match.
        let content = "setup(python_requires_extras={'test': '>=3.10'})\n";
        assert_eq!(extract_python_requires(content), None);
    }

    #[test]
    fn first_literal_match_wins() {
        let content = r#"
setup(
    python_requires=">=3.10",
)
# A later comment containing python_requires=">=3.99" is ignored
# because the earlier literal already matched.
"#;
        assert_eq!(extract_python_requires(content).as_deref(), Some(">=3.10"));
    }
}
