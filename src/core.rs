use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub source: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: TabId,
    pub title: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub saved_content: String,
    pub syntax: String,
    pub last_modified: Option<SystemTime>,
}

impl Document {
    pub fn new_untitled(id: TabId, index: usize) -> Self {
        Self {
            id,
            title: format!("Untitled {index}"),
            path: None,
            content: String::new(),
            saved_content: String::new(),
            syntax: "txt".to_owned(),
            last_modified: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.content != self.saved_content
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    pub fn mark_saved(&mut self, path: Option<PathBuf>, timestamp: Option<SystemTime>) {
        if let Some(new_path) = path {
            self.title = new_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("untitled")
                .to_owned();
            self.path = Some(new_path);
        }
        self.saved_content = self.content.clone();
        self.last_modified = timestamp;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub tabs: Vec<Document>,
    pub selected: Option<TabId>,
    pub recent_files: Vec<PathBuf>,
}

impl SessionState {
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            selected: None,
            recent_files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    MissingFile(PathBuf),
    PermissionDenied(PathBuf),
    InvalidEncoding(PathBuf),
    Validation(String),
}

pub type AppResult<T> = Result<T, AppError>;

pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}

#[derive(Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

pub struct FakeClock {
    now: std::sync::Mutex<SystemTime>,
}

impl FakeClock {
    pub fn new(now: SystemTime) -> Self {
        Self {
            now: std::sync::Mutex::new(now),
        }
    }

    pub fn set_now(&self, now: SystemTime) {
        if let Ok(mut locked) = self.now.lock() {
            *locked = now;
        }
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().expect("fake clock poisoned")
    }
}

pub fn default_syntax_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("rs", "rust"),
        ("js", "javascript"),
        ("ts", "typescript"),
        ("py", "python"),
        ("json", "json"),
        ("xml", "xml"),
        ("txt", "txt"),
    ])
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{default_syntax_map, Document, FakeClock, SessionState, TabId};
    use crate::core::Clock;

    #[test]
    fn untitled_document_defaults_are_stable() {
        let doc = Document::new_untitled(TabId(1), 3);
        assert_eq!(doc.title, "Untitled 3");
        assert_eq!(doc.syntax, "txt");
        assert!(!doc.is_dirty());
        assert!(doc.path.is_none());
    }

    #[test]
    fn mark_saved_updates_saved_content_and_title() {
        let mut doc = Document::new_untitled(TabId(7), 1);
        doc.set_content("hello");
        let ts = UNIX_EPOCH + Duration::from_secs(42);
        let path = std::path::PathBuf::from("notes.md");
        doc.mark_saved(Some(path), Some(ts));

        assert_eq!(doc.title, "notes.md");
        assert!(!doc.is_dirty());
        assert_eq!(doc.last_modified, Some(ts));
    }

    #[test]
    fn session_empty_starts_without_tabs() {
        let session = SessionState::empty();
        assert!(session.tabs.is_empty());
        assert!(session.selected.is_none());
    }

    #[test]
    fn fake_clock_can_be_advanced() {
        let first = UNIX_EPOCH + Duration::from_secs(1);
        let second = UNIX_EPOCH + Duration::from_secs(99);
        let clock = FakeClock::new(first);
        assert_eq!(clock.now(), first);
        clock.set_now(second);
        assert_eq!(clock.now(), second);
    }

    #[test]
    fn syntax_map_contains_expected_defaults() {
        let syntax = default_syntax_map();
        assert_eq!(syntax.get("rs"), Some(&"rust"));
        assert_eq!(syntax.get("txt"), Some(&"txt"));
    }
}
