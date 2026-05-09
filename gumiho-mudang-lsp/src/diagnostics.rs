use std::collections::{HashMap, HashSet};

use lru::LruCache;
use std::num::NonZeroUsize;

use crate::types::{Diagnostic, DiagnosticFile};

const MAX_DIAGNOSTICS_PER_FILE: usize = 10;
const MAX_TOTAL_DIAGNOSTICS: usize = 30;
const MAX_DELIVERED_FILES: usize = 500;

pub struct DiagnosticRegistry {
    pending: Vec<DiagnosticFile>,
    delivered: LruCache<String, HashSet<String>>,
}

impl DiagnosticRegistry {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            delivered: LruCache::new(NonZeroUsize::new(MAX_DELIVERED_FILES).unwrap()),
        }
    }

    pub fn register(&mut self, uri: String, diagnostics: Vec<Diagnostic>) {
        if diagnostics.is_empty() {
            self.clear_delivered_for_file(&uri);
            return;
        }
        self.pending.push(DiagnosticFile { uri, diagnostics });
    }

    pub fn drain(&mut self) -> Vec<DiagnosticFile> {
        if self.pending.is_empty() {
            return Vec::new();
        }

        let all_files = std::mem::take(&mut self.pending);

        // Deduplicate within-batch and cross-turn
        let mut file_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut deduped: HashMap<String, Vec<Diagnostic>> = HashMap::new();

        for file in &all_files {
            let seen = file_map.entry(file.uri.clone()).or_default();
            let deduped_diags = deduped.entry(file.uri.clone()).or_default();
            let previously_delivered = self.delivered.get(&file.uri);

            for diag in &file.diagnostics {
                let key = diagnostic_key(diag);
                if seen.contains(&key) {
                    continue;
                }
                if let Some(prev) = previously_delivered {
                    if prev.contains(&key) {
                        continue;
                    }
                }
                seen.insert(key);
                deduped_diags.push(diag.clone());
            }
        }

        // Sort by severity and apply volume limits
        let mut result: Vec<DiagnosticFile> = Vec::new();
        let mut total = 0;

        for (uri, mut diagnostics) in deduped {
            if diagnostics.is_empty() {
                continue;
            }

            diagnostics.sort_by_key(|d| d.severity.unwrap_or(4));

            if diagnostics.len() > MAX_DIAGNOSTICS_PER_FILE {
                diagnostics.truncate(MAX_DIAGNOSTICS_PER_FILE);
            }

            let remaining = MAX_TOTAL_DIAGNOSTICS.saturating_sub(total);
            if diagnostics.len() > remaining {
                diagnostics.truncate(remaining);
            }

            total += diagnostics.len();

            if !diagnostics.is_empty() {
                result.push(DiagnosticFile { uri, diagnostics });
            }

            if total >= MAX_TOTAL_DIAGNOSTICS {
                break;
            }
        }

        // Track delivered for cross-turn dedup
        for file in &result {
            let delivered_set = self
                .delivered
                .get_or_insert_mut(file.uri.clone(), HashSet::new);
            for diag in &file.diagnostics {
                delivered_set.insert(diagnostic_key(diag));
            }
        }

        result
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn reset(&mut self) {
        self.pending.clear();
        self.delivered.clear();
    }

    pub fn clear_delivered_for_file(&mut self, uri: &str) {
        self.delivered.pop(uri);
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for DiagnosticRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn diagnostic_key(diag: &Diagnostic) -> String {
    serde_json::json!({
        "message": diag.message,
        "severity": diag.severity,
        "range": diag.range,
        "source": diag.source,
        "code": diag.code,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiagnosticPosition, DiagnosticRange};

    fn make_diag(msg: &str, severity: u8, line: u32) -> Diagnostic {
        Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition { line, character: 0 },
                end: DiagnosticPosition {
                    line,
                    character: 10,
                },
            },
            severity: Some(severity),
            code: None,
            source: None,
            message: msg.to_string(),
        }
    }

    #[test]
    fn test_register_and_drain_basic() {
        let mut reg = DiagnosticRegistry::new();
        reg.register("file:///a.rs".into(), vec![make_diag("error", 1, 1)]);
        let result = reg.drain();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].diagnostics.len(), 1);
    }

    #[test]
    fn test_drain_empty_after_drain() {
        let mut reg = DiagnosticRegistry::new();
        reg.register("file:///a.rs".into(), vec![make_diag("error", 1, 1)]);
        let _ = reg.drain();
        let result = reg.drain();
        assert!(result.is_empty());
    }

    #[test]
    fn test_within_batch_dedup() {
        let mut reg = DiagnosticRegistry::new();
        let diag = make_diag("same error", 1, 1);
        reg.register("file:///a.rs".into(), vec![diag.clone(), diag]);
        let result = reg.drain();
        assert_eq!(result[0].diagnostics.len(), 1);
    }

    #[test]
    fn test_cross_turn_dedup() {
        let mut reg = DiagnosticRegistry::new();
        let diag = make_diag("persistent error", 1, 1);
        reg.register("file:///a.rs".into(), vec![diag.clone()]);
        let _ = reg.drain();

        // Same diagnostic again
        reg.register("file:///a.rs".into(), vec![diag]);
        let result = reg.drain();
        assert!(result.is_empty());
    }

    #[test]
    fn test_severity_sort() {
        let mut reg = DiagnosticRegistry::new();
        reg.register(
            "file:///a.rs".into(),
            vec![
                make_diag("hint", 4, 3),
                make_diag("error", 1, 1),
                make_diag("warning", 2, 2),
            ],
        );
        let result = reg.drain();
        let severities: Vec<u8> = result[0]
            .diagnostics
            .iter()
            .map(|d| d.severity.unwrap())
            .collect();
        assert_eq!(severities, vec![1, 2, 4]);
    }

    #[test]
    fn test_per_file_volume_limit() {
        let mut reg = DiagnosticRegistry::new();
        let diags: Vec<Diagnostic> = (0..20)
            .map(|i| make_diag(&format!("error {i}"), 1, i))
            .collect();
        reg.register("file:///a.rs".into(), diags);
        let result = reg.drain();
        assert_eq!(result[0].diagnostics.len(), MAX_DIAGNOSTICS_PER_FILE);
    }

    #[test]
    fn test_total_volume_limit() {
        let mut reg = DiagnosticRegistry::new();
        for i in 0..5 {
            let diags: Vec<Diagnostic> = (0..10)
                .map(|j| make_diag(&format!("error {i}-{j}"), 1, j))
                .collect();
            reg.register(format!("file:///file{i}.rs"), diags);
        }
        let result = reg.drain();
        let total: usize = result.iter().map(|f| f.diagnostics.len()).sum();
        assert!(total <= MAX_TOTAL_DIAGNOSTICS);
    }

    #[test]
    fn test_clear_delivered_allows_redeliver() {
        let mut reg = DiagnosticRegistry::new();
        let diag = make_diag("error", 1, 1);
        reg.register("file:///a.rs".into(), vec![diag.clone()]);
        let _ = reg.drain();

        reg.clear_delivered_for_file("file:///a.rs");
        reg.register("file:///a.rs".into(), vec![diag]);
        let result = reg.drain();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_empty_diagnostics_clears_delivered() {
        let mut reg = DiagnosticRegistry::new();
        let diag = make_diag("error", 1, 1);
        reg.register("file:///a.rs".into(), vec![diag.clone()]);
        let _ = reg.drain();

        // Empty vec = server cleared diagnostics
        reg.register("file:///a.rs".into(), vec![]);
        reg.register("file:///a.rs".into(), vec![diag]);
        let result = reg.drain();
        assert_eq!(result.len(), 1);
    }
}
