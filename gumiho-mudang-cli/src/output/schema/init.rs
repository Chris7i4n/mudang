//! `scope init` view — bespoke top-level JSON shape (not wrapped in
//! the `JsonOutput<T>` envelope).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InitResult<'a> {
    pub command: &'static str,
    pub project_name: &'a str,
    pub languages: Vec<&'a str>,
    pub scope_dir: &'static str,
}
