//! `lang_version` detector dispatcher — wires per-language workspace
//! readers (this module's siblings) into a single entry point that the
//! R8 audit emit calls when populating the `lang_version` slot of a
//! `SampleRecord` (per `docs/AUDIT-LABEL-SCHEMA.md`).
//!
//! **C2 boundary**: this module is indexer-side. It composes the
//! version-extraction free functions in sibling readers (the R4
//! indexer-side carveout); none of the version-extraction surface is
//! reachable through `LanguageWorkspaceContext`, so language plugins
//! cannot reach this dispatcher. See `workspace_context.rs` trait
//! doc-comment.
//!
//! ## Resolution algorithm
//!
//! Given a source file path and a project root:
//!
//! 1. Determine the [`LanguageId`] from the file extension via
//!    `dispatch::dispatch_extension`.
//! 2. Walk from the file's parent directory upwards toward
//!    `project_root` (inclusive). At each directory, try the
//!    language's manifest candidates in priority order; return the
//!    first match.
//! 3. If no manifest along the walk yields a value, return `None` —
//!    the audit emit writes `null` and downstream consumers see
//!    "unknown" rather than a fabricated guess.
//!
//! Manifests are scanned in the order documented per-arm below. The
//! ordering reflects the canonical version-of-record for each
//! language; later candidates exist purely as fallbacks for projects
//! that haven't adopted the canonical form.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::languages::dispatch;
use crate::languages::LanguageId;

use super::{
    build_gradle, cargo_toml, csproj, go_mod, pom_xml, pyproject_toml, ruby_version, setup_py,
    tsconfig_json,
};

/// Detect the `lang_version` for the source file at `file_path`,
/// rooted under `project_root`. Both paths should be absolute; the
/// caller is responsible for canonicalisation.
///
/// Returns `None` when:
/// - the file extension does not map to any supported language,
/// - the file is not under `project_root`,
/// - no manifest on the walk yields a version.
pub fn detect_lang_version(project_root: &Path, file_path: &Path) -> Option<String> {
    debug_assert!(
        project_root.is_absolute() && file_path.is_absolute(),
        "detect_lang_version requires absolute paths (project_root={:?}, file_path={:?}); \
         the audit-emit caller passes `project_root.join(...)` against a canonicalised root",
        project_root,
        file_path
    );
    let lang = lang_for_path(file_path)?;
    let start = file_path.parent()?;
    if !start.starts_with(project_root) {
        return None;
    }
    let mut dir = start;
    loop {
        if let Some(v) = detect_in_dir(lang, dir) {
            return Some(v);
        }
        if dir == project_root {
            return None;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

fn lang_for_path(file_path: &Path) -> Option<LanguageId> {
    let ext = file_path.extension().and_then(OsStr::to_str)?;
    dispatch::dispatch_extension(ext)
}

fn detect_in_dir(lang: LanguageId, dir: &Path) -> Option<String> {
    match lang {
        LanguageId::Rust => read_first(dir, "Cargo.toml", |c| {
            cargo_toml::extract_rust_version(c).or_else(|| cargo_toml::extract_edition(c))
        }),
        LanguageId::Go => read_first(dir, "go.mod", go_mod::extract_go_directive),
        LanguageId::Python => read_first(
            dir,
            "pyproject.toml",
            pyproject_toml::extract_requires_python,
        )
        .or_else(|| read_first(dir, "setup.py", setup_py::extract_python_requires)),
        LanguageId::TypeScript => {
            read_first(dir, "tsconfig.json", tsconfig_json::extract_tsconfig_target)
        }
        LanguageId::Java => read_first(dir, "pom.xml", pom_xml::extract_java_version)
            .or_else(|| read_first(dir, "build.gradle", build_gradle::extract_java_version))
            .or_else(|| read_first(dir, "build.gradle.kts", build_gradle::extract_java_version)),
        LanguageId::CSharp => first_csproj_target(dir),
        LanguageId::Ruby => read_first(dir, ".ruby-version", ruby_version::parse_ruby_version_file)
            .or_else(|| read_first(dir, "Gemfile", ruby_version::extract_gemfile_ruby_directive)),
    }
}

fn read_first<F>(dir: &Path, file: &str, extract: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    let path = dir.join(file);
    let content = fs::read_to_string(&path).ok()?;
    extract(&content)
}

/// C# projects don't have a fixed manifest filename — every project
/// folder owns a `*.csproj`. Scan the directory for any `.csproj`
/// (alphabetical order for determinism) and return the first match.
fn first_csproj_target(dir: &Path) -> Option<String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(OsStr::to_str) == Some("csproj"))
        .collect();
    entries.sort();
    for path in entries {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Some(v) = csproj::extract_target_framework(&content) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn detects_rust_via_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"x\"\nedition = \"2021\"\nrust-version = \"1.74\"\n",
        );
        let src = root.join("src/lib.rs");
        write(&src, "fn main() {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("1.74"));
    }

    #[test]
    fn rust_falls_back_to_edition_when_msrv_absent() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"x\"\nedition = \"2021\"\n",
        );
        let src = root.join("src/lib.rs");
        write(&src, "fn main() {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("2021"));
    }

    #[test]
    fn detects_go_via_go_mod() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("go.mod"), "module x\n\ngo 1.22\n");
        let src = root.join("cmd/main.go");
        write(&src, "package main\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("1.22"));
    }

    #[test]
    fn detects_python_via_pyproject_pep621() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("pyproject.toml"),
            "[project]\nname=\"x\"\nrequires-python = \">=3.10\"\n",
        );
        let src = root.join("pkg/mod.py");
        write(&src, "x = 1\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some(">=3.10"));
    }

    #[test]
    fn python_falls_back_to_setup_py() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("setup.py"),
            "from setuptools import setup\nsetup(name='x', python_requires='>=3.8')\n",
        );
        let src = root.join("mod.py");
        write(&src, "x = 1\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some(">=3.8"));
    }

    #[test]
    fn detects_typescript_via_tsconfig() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("tsconfig.json"),
            r#"{"compilerOptions":{"target":"es2022"}}"#,
        );
        let src = root.join("src/index.ts");
        write(&src, "export {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("es2022"));
    }

    #[test]
    fn detects_java_via_pom_xml() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("pom.xml"),
            "<project><properties><maven.compiler.release>21</maven.compiler.release></properties></project>",
        );
        let src = root.join("src/main/java/App.java");
        write(&src, "class App {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("21"));
    }

    #[test]
    fn java_falls_back_to_gradle() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("build.gradle"),
            "sourceCompatibility = '17'\ntargetCompatibility = '17'\n",
        );
        let src = root.join("src/main/java/App.java");
        write(&src, "class App {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("17"));
    }

    #[test]
    fn detects_csharp_via_csproj() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("App.csproj"),
            "<Project><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>",
        );
        let src = root.join("Program.cs");
        write(&src, "class Program {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("net8.0"));
    }

    #[test]
    fn detects_ruby_via_ruby_version_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join(".ruby-version"), "3.2.2\n");
        let src = root.join("lib/app.rb");
        write(&src, "puts 1\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("3.2.2"));
    }

    #[test]
    fn ruby_falls_back_to_gemfile_directive() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(&root.join("Gemfile"), "source 'rubygems'\nruby '3.1.4'\n");
        let src = root.join("app.rb");
        write(&src, "1\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("3.1.4"));
    }

    #[test]
    fn walks_up_to_find_manifest() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            &root.join("Cargo.toml"),
            "[package]\nname=\"x\"\nedition=\"2021\"\n",
        );
        let src = root.join("crates/sub/src/lib.rs");
        write(&src, "fn x() {}\n");
        assert_eq!(detect_lang_version(root, &src).as_deref(), Some("2021"));
    }

    #[test]
    fn stops_at_project_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("project");
        fs::create_dir_all(&root).unwrap();
        // Manifest sits *above* the project root — must not be picked up.
        write(
            &tmp.path().join("Cargo.toml"),
            "[package]\nname=\"outer\"\nedition=\"2018\"\n",
        );
        let src = root.join("src/lib.rs");
        write(&src, "fn x() {}\n");
        assert_eq!(detect_lang_version(&root, &src), None);
    }

    #[test]
    fn returns_none_for_unknown_extension() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let src = root.join("README.md");
        write(&src, "# x\n");
        assert_eq!(detect_lang_version(root, &src), None);
    }

    #[test]
    fn returns_none_when_no_manifest_present() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let src = root.join("src/lib.rs");
        write(&src, "fn x() {}\n");
        assert_eq!(detect_lang_version(root, &src), None);
    }
}
