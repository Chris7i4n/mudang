//! Workspace config readers (R4).
//!
//! Each reader parses one manifest format and returns the typed structs
//! declared in `crate::workspace_context`. Plugins never see the raw
//! file content — they consume the typed structs through
//! `LanguageWorkspaceContext` / `FrameworkWorkspaceContext` (R4).
//!
//! Readers are minimum-functional: they extract only the fields that
//! the typed structs expose. Fields beyond that (license, author,
//! build scripts, scripts blocks) are intentionally ignored — scope is
//! a graph indexer, not a manifest mirror.
//!
//! Readers expose two forms:
//! - `read_<format>(path)` — reads from disk, returns the typed struct.
//! - `parse_<format>(content)` — parses an in-memory string. Used by
//!   unit tests and by callers that already hold the manifest content.
//!
//! Filesystem access lives **here**, in config readers, not in plugin
//! code. CHARTER §5 forbids plugins from reading non-source files;
//! readers in this module are the only typed access path.

pub mod cargo_toml;
pub mod gemfile_lock;
pub mod go_mod;
pub mod package_json;
pub mod pyproject_toml;
pub mod setup_py;
pub mod tsconfig_json;

pub use cargo_toml::{parse_cargo_toml, read_cargo_toml};
pub use gemfile_lock::{parse_gemfile_lock, read_gemfile_lock};
pub use go_mod::{parse_go_mod, read_go_mod};
pub use package_json::{parse_package_json, read_package_json};
pub use pyproject_toml::{parse_pyproject_toml, read_pyproject_toml};
pub use tsconfig_json::{parse_tsconfig_json, read_tsconfig_json};
