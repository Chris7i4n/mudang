use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: DiagnosticRange,
    pub severity: Option<u8>,
    pub code: Option<DiagnosticCode>,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFile {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspCapabilities {
    pub go_to_definition: bool,
    pub find_references: bool,
    pub hover: bool,
    pub document_symbol: bool,
    pub workspace_symbol: bool,
    pub go_to_implementation: bool,
    pub prepare_call_hierarchy: bool,
    pub incoming_calls: bool,
    pub outgoing_calls: bool,
}

impl LspCapabilities {
    pub const FULL: Self = Self {
        go_to_definition: true,
        find_references: true,
        hover: true,
        document_symbol: true,
        workspace_symbol: true,
        go_to_implementation: true,
        prepare_call_hierarchy: true,
        incoming_calls: true,
        outgoing_calls: true,
    };

    pub const NO_CALL_HIERARCHY: Self = Self {
        go_to_definition: true,
        find_references: true,
        hover: true,
        document_symbol: true,
        workspace_symbol: true,
        go_to_implementation: true,
        prepare_call_hierarchy: false,
        incoming_calls: false,
        outgoing_calls: false,
    };

    pub const BASIC: Self = Self {
        go_to_definition: true,
        find_references: true,
        hover: true,
        document_symbol: true,
        workspace_symbol: false,
        go_to_implementation: false,
        prepare_call_hierarchy: false,
        incoming_calls: false,
        outgoing_calls: false,
    };

    pub fn supports(&self, op: LspOperation) -> bool {
        match op {
            LspOperation::GoToDefinition => self.go_to_definition,
            LspOperation::FindReferences => self.find_references,
            LspOperation::Hover => self.hover,
            LspOperation::DocumentSymbol => self.document_symbol,
            LspOperation::WorkspaceSymbol => self.workspace_symbol,
            LspOperation::GoToImplementation => self.go_to_implementation,
            LspOperation::PrepareCallHierarchy => self.prepare_call_hierarchy,
            LspOperation::IncomingCalls => self.incoming_calls,
            LspOperation::OutgoingCalls => self.outgoing_calls,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LspOperation {
    GoToDefinition,
    FindReferences,
    Hover,
    DocumentSymbol,
    WorkspaceSymbol,
    GoToImplementation,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
}

impl LspOperation {
    pub const ALL: &[Self] = &[
        Self::GoToDefinition,
        Self::FindReferences,
        Self::Hover,
        Self::DocumentSymbol,
        Self::WorkspaceSymbol,
        Self::GoToImplementation,
        Self::PrepareCallHierarchy,
        Self::IncomingCalls,
        Self::OutgoingCalls,
    ];
}

pub fn format_lsp_uri(uri: &str) -> String {
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    // Handle Windows drive-letter paths: /C:/foo → C:/foo
    let path = if path.len() >= 3
        && path.as_bytes()[0] == b'/'
        && path.as_bytes()[1].is_ascii_alphabetic()
        && path.as_bytes()[2] == b':'
    {
        &path[1..]
    } else {
        path
    };
    // Percent-decode
    let decoded = percent_decode(path);
    decoded.replace('\\', "/")
}

fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_lsp_uri_basic() {
        assert_eq!(
            format_lsp_uri("file:///home/user/file.rs"),
            "/home/user/file.rs"
        );
    }

    #[test]
    fn test_format_lsp_uri_windows() {
        assert_eq!(
            format_lsp_uri("file:///C:/Users/test.rs"),
            "C:/Users/test.rs"
        );
    }

    #[test]
    fn test_format_lsp_uri_percent_encoded() {
        assert_eq!(
            format_lsp_uri("file:///home/user/my%20file.rs"),
            "/home/user/my file.rs"
        );
    }

    #[test]
    fn test_capabilities_full_supports_all() {
        for op in LspOperation::ALL {
            assert!(LspCapabilities::FULL.supports(*op));
        }
    }

    #[test]
    fn test_capabilities_basic_no_workspace_symbol() {
        assert!(!LspCapabilities::BASIC.supports(LspOperation::WorkspaceSymbol));
        assert!(!LspCapabilities::BASIC.supports(LspOperation::GoToImplementation));
        assert!(LspCapabilities::BASIC.supports(LspOperation::Hover));
    }
}
