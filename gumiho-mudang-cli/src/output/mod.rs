/// Output formatting for Scope CLI commands.
///
/// All human-readable output flows through `formatter`.
/// All `--json` output flows through `json`.
/// All typed output structs (R10) live in `schema`.
pub mod formatter;
pub mod json;
pub mod schema;
