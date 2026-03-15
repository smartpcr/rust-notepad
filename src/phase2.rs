use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::phase0::{AppError, AppResult, Diagnostic, SessionState, Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub tab_title: String,
    pub line: usize,
    pub start: usize,
    pub matched: String,
}

pub fn find_in_open_tabs(docs: &[(String, String)], needle: &str) -> Vec<SearchHit> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }

    for (title, content) in docs {
        for (line_idx, line) in content.lines().enumerate() {
            let mut offset = 0;
            while let Some(found) = line[offset..].find(needle) {
                let start = offset + found;
                out.push(SearchHit {
                    tab_title: title.clone(),
                    line: line_idx + 1,
                    start,
                    matched: needle.to_owned(),
                });
                offset = start + needle.len();
                if offset >= line.len() {
                    break;
                }
            }
        }
    }
    out
}

#[derive(Debug, Default)]
pub struct RecentFiles {
    cap: usize,
    entries: VecDeque<PathBuf>,
}

impl RecentFiles {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            entries: VecDeque::new(),
        }
    }

    pub fn visit(&mut self, path: PathBuf) {
        self.entries.retain(|existing| existing != &path);
        self.entries.push_front(path);
        while self.entries.len() > self.cap {
            self.entries.pop_back();
        }
    }

    pub fn prune_missing<F: Fn(&PathBuf) -> bool>(&mut self, exists: F) {
        self.entries.retain(exists);
    }

    pub fn as_vec(&self) -> Vec<PathBuf> {
        self.entries.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Keybinding {
    pub command: String,
    pub chord: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub theme: String,
    pub keybindings: Vec<Keybinding>,
}

impl Settings {
    pub fn validate_keybindings(&self) -> AppResult<()> {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for kb in &self.keybindings {
            if let Some(prev_cmd) = seen.insert(kb.chord.as_str(), kb.command.as_str()) {
                return Err(AppError::Validation(format!(
                    "chord {} conflicts between {prev_cmd} and {}",
                    kb.chord, kb.command
                )));
            }
        }
        Ok(())
    }
}

pub fn serialize_session(session: &SessionState) -> AppResult<String> {
    serde_json::to_string_pretty(session)
        .map_err(|err| AppError::Validation(format!("session serialize failed: {err}")))
}

pub fn deserialize_session(raw: &str) -> AppResult<SessionState> {
    serde_json::from_str(raw)
        .map_err(|err| AppError::Validation(format!("session deserialize failed: {err}")))
}

pub fn format_json(raw: &str) -> AppResult<String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|err| AppError::Validation(err.to_string()))?;
    serde_json::to_string_pretty(&value).map_err(|err| AppError::Validation(err.to_string()))
}

pub fn validate_json(raw: &str) -> Option<Diagnostic> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(_) => None,
        Err(err) => Some(Diagnostic {
            source: "json".to_owned(),
            message: err.to_string(),
            severity: Severity::Error,
        }),
    }
}

pub fn validate_xml(raw: &str) -> Option<Diagnostic> {
    match quick_xml::de::from_str::<serde_json::Value>(raw) {
        Ok(_) => None,
        Err(err) => Some(Diagnostic {
            source: "xml".to_owned(),
            message: err.to_string(),
            severity: Severity::Error,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::phase0::{Document, SessionState, TabId};

    use super::{
        deserialize_session, find_in_open_tabs, format_json, serialize_session, validate_json,
        validate_xml, RecentFiles, Settings,
    };

    #[test]
    fn search_groups_hits_across_tabs() {
        let docs = vec![
            ("a.txt".to_string(), "hello\nneedle".to_string()),
            ("b.txt".to_string(), "needle needle".to_string()),
        ];
        let hits = find_in_open_tabs(&docs, "needle");
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].tab_title, "a.txt");
        assert_eq!(hits[0].line, 2);
    }

    #[test]
    fn recent_files_is_lru_with_deduplication() {
        let mut recent = RecentFiles::new(2);
        recent.visit(PathBuf::from("a"));
        recent.visit(PathBuf::from("b"));
        recent.visit(PathBuf::from("a"));
        assert_eq!(
            recent.as_vec(),
            vec![PathBuf::from("a"), PathBuf::from("b")]
        );
    }

    #[test]
    fn recent_files_prunes_missing_entries() {
        let mut recent = RecentFiles::new(3);
        recent.visit(PathBuf::from("a"));
        recent.visit(PathBuf::from("b"));
        recent.prune_missing(|p| p != &PathBuf::from("a"));
        assert_eq!(recent.as_vec(), vec![PathBuf::from("b")]);
    }

    #[test]
    fn session_roundtrip_serialization() {
        let session = SessionState {
            tabs: vec![Document::new_untitled(TabId(1), 1)],
            selected: Some(TabId(1)),
            recent_files: vec![PathBuf::from("notes.txt")],
        };

        let raw = serialize_session(&session).expect("serialize");
        let decoded = deserialize_session(&raw).expect("deserialize");
        assert_eq!(decoded, session);
    }

    #[test]
    fn settings_detects_keybinding_conflicts() {
        let settings = Settings {
            theme: "dark".into(),
            keybindings: vec![
                super::Keybinding {
                    command: "save".into(),
                    chord: "Ctrl+S".into(),
                },
                super::Keybinding {
                    command: "search".into(),
                    chord: "Ctrl+S".into(),
                },
            ],
        };

        assert!(settings.validate_keybindings().is_err());
    }

    #[test]
    fn json_formatting_and_validation_works() {
        let raw = "{\"a\":1,\"b\":2}";
        let pretty = format_json(raw).expect("valid json");
        assert!(pretty.contains('\n'));
        assert!(validate_json(raw).is_none());
        assert!(validate_json("{").is_some());
    }

    #[test]
    fn xml_validation_reports_invalid_documents() {
        assert!(validate_xml("<root><a>1</a></root>").is_none());
        assert!(validate_xml("<root>").is_some());
    }
}
