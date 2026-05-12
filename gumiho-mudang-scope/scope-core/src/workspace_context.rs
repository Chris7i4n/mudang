//! Typed workspace-context traits (R4).
//!
//! R4 splits the historical `WorkspaceContext` into two traits so that
//! the Rust compiler refuses any language-plugin code that tries to
//! read framework versions or other fields that would tempt a C2
//! violation. This is the **mechanical safeguard** for C2 referenced
//! in `ARCHITECTURAL-REFACTOR.md` § R4 and `CHARTER.md` § 5.
//!
//! - `LanguageWorkspaceContext` (`pub`) — visible to `LanguagePlugin`
//!   (R2, sprint 0003). Exposes workspace-internal-vs-external,
//!   module layout, and per-file package membership. **Does not**
//!   expose `edition` / `target` / `python_requires` / `go_directive`
//!   / `tsconfig_target` / `framework_versions`. Reading those would
//!   let a plugin branch on language or framework version, weakening
//!   C2.
//! - `FrameworkWorkspaceContext` (`pub` since sprint 0005, Phase C
//!   first-impl commit) — extends `LanguageWorkspaceContext` with
//!   framework-version and lockfile access. Phase B (sprints 0002,
//!   0003, 0004) shipped this as `pub(crate)` so the compiler refused
//!   language-plugin code that tried to bound on framework-version
//!   accessors; sprint 0005 widens to `pub` in the same commit that
//!   lands the first `FrameworkPlugin` impl. See
//!   `ARCHITECTURAL-REFACTOR.md` § R4 → "Visibility of
//!   FrameworkWorkspaceContext".
//!
//! No filesystem handle is reachable from either trait. Config readers
//! in `crate::workspace::*` populate the typed structs in this module;
//! plugins consume the structs through accessor methods only.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Stable identifier for a file in the workspace.
///
/// `FileId` is opaque to plugins; the indexer assigns it when the file
/// enters the index. Plugins use it to look up package membership and
/// related metadata via `LanguageWorkspaceContext` without ever
/// touching the filesystem.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileId(pub u32);

/// A package / module-group inside the workspace.
///
/// Concretely: a `Cargo.toml` package, a `package.json` package, a
/// Python package (directory with `__init__.py` or pyproject scope),
/// a Go module, a Ruby gem. Per-language readers populate this struct
/// from the relevant manifest; plugins read it via
/// `LanguageWorkspaceContext::package_for_file`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// Stable identifier — typically the package name from the manifest.
    pub name: String,
    /// Root directory of the package within the workspace.
    pub root: PathBuf,
    /// Direct dependencies declared by this package's manifest.
    pub dependencies: Vec<Dependency>,
}

/// A single dependency declaration in a package manifest.
///
/// The version field is the declared range / constraint string from
/// the manifest (e.g., `^7.0`, `~=1.4`, `>= 2.5`). Resolution to a
/// concrete version (lockfile lookup) is `FrameworkWorkspaceContext`'s
/// job, not `LanguageWorkspaceContext`'s — language plugins must not
/// branch on resolved versions (C2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    /// Dependency name as it appears in the manifest.
    pub name: String,
    /// Declared version range / constraint, verbatim from the manifest.
    pub version_req: String,
    /// Whether this is a dev-only / test-only dependency.
    pub dev_only: bool,
}

/// Module layout for a package.
///
/// Records where each module / namespace declared by the package lives
/// on disk, so plugins can decide whether a cross-file import is
/// workspace-internal (resolvable via the graph) or external
/// (treated as a bare-name reference).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleLayout {
    /// Maps fully qualified module path (`crate::foo::bar`,
    /// `my_pkg.submodule`, `github.com/org/repo/pkg`) to the file or
    /// directory that defines it.
    pub modules: BTreeMap<String, PathBuf>,
}

/// Resolved framework versions for the workspace.
///
/// Populated by `FrameworkWorkspaceContext` readers (lockfile + manifest
/// pairs) per framework name. Not accessible to language plugins —
/// `LanguageWorkspaceContext` has no accessor that returns this struct.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameworkVersions {
    /// Maps framework name (e.g., `"rails"`, `"react"`) to the resolved
    /// `DetectedVersion` (see R5 / sprint 0005).
    pub versions: BTreeMap<String, String>,
}

/// Parsed contents of a lockfile (Cargo.lock, package-lock.json, etc.).
///
/// Lockfiles are framework-scope data: language plugins must not
/// branch on them (C2). Exposed only through `FrameworkWorkspaceContext`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lockfile {
    /// Concrete versions resolved by the lockfile, keyed by package name.
    pub resolved_versions: BTreeMap<String, String>,
    /// Path on disk for diagnostics; never exposed to plugins as a
    /// filesystem handle.
    pub source_path: PathBuf,
}

/// Visible to `LanguagePlugin` (R2, sprint 0003).
///
/// **Does not** expose `edition`, `target`, `python_requires`,
/// `go_directive`, `tsconfig_target`, `framework_versions`. Adding such
/// a method is a charter-amendment-grade change per
/// `ARCHITECTURAL-REFACTOR.md` § R4. CI gate
/// `scripts/audit_context_shape.sh` (`just ci-context-shape`) refuses
/// any method whose name suggests these fields.
pub trait LanguageWorkspaceContext: Send + Sync {
    /// Package that owns the given file, if any. Returns `None` for
    /// files outside any declared package (loose scripts, fixtures).
    fn package_for_file(&self, file: FileId) -> Option<&Package>;

    /// Direct dependencies declared by the package's manifest.
    fn dependencies(&self, package: &Package) -> &[Dependency];

    /// Whether `import` (as written at the call site in `from`) resolves
    /// to a workspace-internal symbol. The plugin uses this to decide
    /// between emitting a `references` edge to a graph symbol vs. a
    /// `bare_name` edge that the resolver leaves dangling.
    fn is_workspace_internal(&self, import: &str, from: FileId) -> bool;

    /// Module layout for the given package.
    fn module_layout(&self, package: &Package) -> &ModuleLayout;
}

/// Visible to `FrameworkPlugin` (R5, sprint 0005).
///
/// Extends `LanguageWorkspaceContext` with framework-version and
/// lockfile access. Frameworks branch on framework version because
/// framework patterns diverge between releases — the deliberate
/// asymmetry with the language layer (see C2 in
/// `LANGUAGE-PLAYBOOK.md` Step 4).
///
/// **Visibility:** `pub` since sprint 0005, Phase C first-impl commit
/// (this commit). Phase B (sprints 0002, 0003, 0004) shipped this as
/// `pub(crate)` so the Rust compiler refused language-plugin code that
/// tried to bound on framework-version accessors. Sprint 0005 widens
/// to `pub` in the same commit as the first `FrameworkPlugin` impl —
/// mechanical one-keyword flip, unconditional, recorded in the sprint
/// plan ambiguity register (#3) as mandatory not conditional. See
/// `ARCHITECTURAL-REFACTOR.md` § R4 → "Visibility of
/// FrameworkWorkspaceContext".
pub trait FrameworkWorkspaceContext: LanguageWorkspaceContext {
    /// Framework versions resolved from manifest + lockfile pairs.
    fn framework_versions(&self) -> &FrameworkVersions;

    /// Lockfile for the package's primary ecosystem, if present.
    fn lockfile(&self) -> Option<&Lockfile>;
}

/// No-op `LanguageWorkspaceContext` for callers that have not yet wired
/// a real one.
///
/// Used by `parser.rs` and test fixtures during Phase A (sprint 0002).
/// R2/R3 (sprint 0003) replace these call sites with a real context
/// populated from `crate::workspace::*` readers. **Not a stub** in the
/// stubs-outstanding sense — this is a legitimate no-op implementation
/// for environments that have no workspace (one-off snippet parsing,
/// unit tests, dry runs). The type is retained post-refactor.
#[derive(Default)]
pub struct NoopWorkspaceContext {
    empty_layout: ModuleLayout,
}

impl LanguageWorkspaceContext for NoopWorkspaceContext {
    fn package_for_file(&self, _file: FileId) -> Option<&Package> {
        None
    }

    fn dependencies(&self, _package: &Package) -> &[Dependency] {
        &[]
    }

    fn is_workspace_internal(&self, _import: &str, _from: FileId) -> bool {
        false
    }

    fn module_layout(&self, _package: &Package) -> &ModuleLayout {
        &self.empty_layout
    }
}
