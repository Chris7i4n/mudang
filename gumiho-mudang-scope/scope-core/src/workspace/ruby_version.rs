//! Ruby version sources.
//!
//! Ruby projects declare their interpreter version through two
//! conventions, often together:
//!
//! - `.ruby-version` — a single-line file at repo root, the de-facto
//!   convention from `rbenv` / `chruby` / `asdf`. Body is a bare
//!   version string (`3.2.2`) sometimes prefixed `ruby-3.2.2`.
//! - `Gemfile` `ruby '<version>'` directive — bundler's in-Gemfile
//!   declaration. Accepts plain version strings, version ranges, or
//!   `:engine => 'jruby'` style options.
//!
//! **C2 boundary**: the C2 enforcement surface is the plugin-facing
//! trait `LanguageWorkspaceContext` (see `crate::workspace_context`),
//! not this reader. No `ruby_version` accessor exists on the trait —
//! CI gate `audit_context_shape.sh` (R4) refuses any method whose name
//! suggests one, so language plugins cannot reach the Ruby version
//! through any plugin-side path. This reader lives indexer-side and
//! may expose Ruby-version extraction for indexer-side consumers (R8
//! audit emit per `BACKLOG.md` § Priority 1 sub-item (d)); plugins
//! never reach these functions because they receive the trait, not
//! this module.
//!
//! ## Honest limits
//!
//! `extract_gemfile_ruby_directive` recognises only the literal-string
//! form (`ruby '3.2.2'`, `ruby "3.2.2"`, `ruby '~> 3.2'`). It does not
//! evaluate the Gemfile as Ruby code; expressions like
//! `ruby File.read('.ruby-version').chomp` return `None`. Projects
//! using that pattern should fall back to `.ruby-version`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read a `.ruby-version` file at `path`. The body is a single bare
/// version string, optionally prefixed `ruby-` (e.g. `ruby-3.2.2`),
/// which this reader strips. Returns `None` for empty / whitespace-
/// only files.
pub fn read_ruby_version_file(path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read .ruby-version at {}", path.display()))?;
    Ok(parse_ruby_version_file(&content))
}

/// Read a `Gemfile` at `path` and extract the `ruby '<version>'`
/// directive.
pub fn read_gemfile_ruby_directive(path: &Path) -> Result<Option<String>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("read Gemfile at {}", path.display()))?;
    Ok(extract_gemfile_ruby_directive(&content))
}

/// Parse a `.ruby-version` body. Strips the optional `ruby-` prefix.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn parse_ruby_version_file(content: &str) -> Option<String> {
    let line = content.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let body = line.strip_prefix("ruby-").unwrap_or(line);
    Some(body.to_string())
}

/// Indexer-side carveout (R4): extract the `ruby '<version>'` directive
/// from a `Gemfile` source string. First match wins. Only the literal
/// string form is recognised — see module doc-comment for limits.
///
/// **Not part of `LanguageWorkspaceContext`** — see module doc-comment.
pub fn extract_gemfile_ruby_directive(content: &str) -> Option<String> {
    for raw_line in content.lines() {
        let line = strip_line_comment(raw_line).trim();
        let Some(rest) = line.strip_prefix("ruby") else {
            continue;
        };
        let first = rest.chars().next();
        if !matches!(first, Some(c) if c.is_whitespace() || c == '\'' || c == '"' || c == '(') {
            continue;
        }
        let after = rest.trim_start().trim_start_matches('(').trim_start();
        if let Some(value) = parse_quoted_literal(after) {
            return Some(value);
        }
    }
    None
}

fn parse_quoted_literal(slice: &str) -> Option<String> {
    let mut chars = slice.chars();
    let quote = chars.next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let body = &slice[quote.len_utf8()..];
    let end = body.find(quote)?;
    Some(body[..end].to_string())
}

fn strip_line_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_version() {
        assert_eq!(parse_ruby_version_file("3.2.2\n").as_deref(), Some("3.2.2"));
    }

    #[test]
    fn parses_ruby_prefixed_version() {
        assert_eq!(
            parse_ruby_version_file("ruby-3.2.2\n").as_deref(),
            Some("3.2.2")
        );
    }

    #[test]
    fn parses_with_trailing_whitespace() {
        assert_eq!(
            parse_ruby_version_file("  3.1.4  \n").as_deref(),
            Some("3.1.4")
        );
    }

    #[test]
    fn parse_returns_none_on_empty_file() {
        assert_eq!(parse_ruby_version_file(""), None);
        assert_eq!(parse_ruby_version_file("\n\n"), None);
    }

    #[test]
    fn extracts_gemfile_directive_single_quoted() {
        let content = "source 'https://rubygems.org'\nruby '3.2.2'\n";
        assert_eq!(
            extract_gemfile_ruby_directive(content).as_deref(),
            Some("3.2.2")
        );
    }

    #[test]
    fn extracts_gemfile_directive_double_quoted() {
        let content = "ruby \"3.1.4\"\n";
        assert_eq!(
            extract_gemfile_ruby_directive(content).as_deref(),
            Some("3.1.4")
        );
    }

    #[test]
    fn extracts_gemfile_directive_with_constraint() {
        let content = "ruby '~> 3.2'\n";
        assert_eq!(
            extract_gemfile_ruby_directive(content).as_deref(),
            Some("~> 3.2")
        );
    }

    #[test]
    fn ignores_commented_directive() {
        let content = "# ruby '2.7.0'\nruby '3.2.2'\n";
        assert_eq!(
            extract_gemfile_ruby_directive(content).as_deref(),
            Some("3.2.2")
        );
    }

    #[test]
    fn ignores_identifier_prefixed_with_ruby() {
        let content = "ruby_version = '3.2.2'\nruby '3.1.0'\n";
        assert_eq!(
            extract_gemfile_ruby_directive(content).as_deref(),
            Some("3.1.0")
        );
    }

    #[test]
    fn returns_none_for_dynamic_expression() {
        let content = "ruby File.read('.ruby-version').chomp\n";
        assert_eq!(extract_gemfile_ruby_directive(content), None);
    }

    #[test]
    fn returns_none_when_absent() {
        let content = "source 'https://rubygems.org'\ngem 'rails'\n";
        assert_eq!(extract_gemfile_ruby_directive(content), None);
    }
}
