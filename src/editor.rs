use std::{collections::HashMap, fs, path::Path, path::PathBuf, time::SystemTime};

use crate::core::{default_syntax_map, AppError, AppResult, Document, SearchQuery, TabId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceResult {
    pub replaced: usize,
    pub new_content: String,
}

#[derive(Debug)]
pub struct EditorState {
    pub docs: Vec<Document>,
    pub current_tab: usize,
    pub untitled_count: usize,
    pub syntax_map: HashMap<&'static str, &'static str>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            docs: vec![Document::new_untitled(TabId(1), 1)],
            current_tab: 0,
            untitled_count: 1,
            syntax_map: default_syntax_map(),
        }
    }
}

impl EditorState {
    pub fn new_tab(&mut self) {
        self.untitled_count += 1;
        self.docs.push(Document::new_untitled(
            TabId(self.untitled_count as u64),
            self.untitled_count,
        ));
        self.current_tab = self.docs.len() - 1;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.docs.len() == 1 {
            self.untitled_count += 1;
            self.docs[0] =
                Document::new_untitled(TabId(self.untitled_count as u64), self.untitled_count);
            self.current_tab = 0;
            return;
        }

        self.docs.remove(idx);
        if self.current_tab >= self.docs.len() {
            self.current_tab = self.docs.len() - 1;
        }
    }

    pub fn close_all(&mut self) {
        self.docs.clear();
        self.new_tab();
    }

    pub fn close_others(&mut self) {
        let active = self.docs.remove(self.current_tab);
        self.docs = vec![active];
        self.current_tab = 0;
    }

    pub fn active_doc(&self) -> &Document {
        &self.docs[self.current_tab]
    }

    pub fn active_doc_mut(&mut self) -> &mut Document {
        &mut self.docs[self.current_tab]
    }

    pub fn syntax_for_path(&self, path: &Path) -> String {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        self.syntax_map
            .get(ext)
            .copied()
            .unwrap_or("txt")
            .to_owned()
    }
}

pub fn load_document(
    path: PathBuf,
    syntax_map: &HashMap<&'static str, &'static str>,
) -> AppResult<Document> {
    let content = fs::read_to_string(&path).map_err(|err| map_io_err(err.kind(), &path))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
    Ok(Document {
        id: TabId(0),
        title: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_owned(),
        path: Some(path.clone()),
        content: content.clone(),
        saved_content: content,
        syntax: syntax_map.get(ext).copied().unwrap_or("txt").to_owned(),
        last_modified: fs::metadata(path).ok().and_then(|m| m.modified().ok()),
    })
}

pub fn write_document(doc: &mut Document, path: PathBuf) -> AppResult<()> {
    fs::write(&path, &doc.content).map_err(|err| map_io_err(err.kind(), &path))?;
    let modified = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    doc.mark_saved(Some(path), modified);
    Ok(())
}

pub fn detect_external_change(doc: &Document, current_mtime: Option<SystemTime>) -> bool {
    if doc.is_dirty() {
        return false;
    }
    match (doc.last_modified, current_mtime) {
        (Some(prev), Some(current)) => current > prev,
        _ => false,
    }
}

pub fn find_matches(haystack: &str, query: &SearchQuery) -> Vec<usize> {
    if query.query.is_empty() {
        return Vec::new();
    }

    let search_text = if query.case_sensitive {
        haystack.to_owned()
    } else {
        haystack.to_lowercase()
    };
    let needle = if query.case_sensitive {
        query.query.clone()
    } else {
        query.query.to_lowercase()
    };

    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(pos) = search_text[offset..].find(&needle) {
        let absolute = offset + pos;
        if !query.whole_word || is_whole_word_boundary(&search_text, absolute, needle.len()) {
            out.push(absolute);
        }
        offset = absolute + needle.len().max(1);
        if offset >= search_text.len() {
            break;
        }
    }
    out
}

pub fn replace_all(content: &str, query: &SearchQuery, replacement: &str) -> ReplaceResult {
    if query.query.is_empty() {
        return ReplaceResult {
            replaced: 0,
            new_content: content.to_owned(),
        };
    }

    if !query.whole_word && query.case_sensitive {
        let replaced = content.matches(&query.query).count();
        return ReplaceResult {
            replaced,
            new_content: content.replace(&query.query, replacement),
        };
    }

    let matches = find_matches(content, query);
    if matches.is_empty() {
        return ReplaceResult {
            replaced: 0,
            new_content: content.to_owned(),
        };
    }

    let mut rebuilt = String::with_capacity(content.len());
    let mut previous = 0;
    for start in &matches {
        rebuilt.push_str(&content[previous..*start]);
        rebuilt.push_str(replacement);
        previous = *start + query.query.len();
    }
    rebuilt.push_str(&content[previous..]);

    ReplaceResult {
        replaced: matches.len(),
        new_content: rebuilt,
    }
}

fn map_io_err(kind: std::io::ErrorKind, path: &Path) -> AppError {
    match kind {
        std::io::ErrorKind::NotFound => AppError::MissingFile(path.to_path_buf()),
        std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied(path.to_path_buf()),
        _ => AppError::Validation(format!("I/O failure for {}", path.display())),
    }
}

fn is_whole_word_boundary(text: &str, start: usize, len: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        text[..start].chars().last()
    };
    let end_idx = start + len;
    let after = if end_idx >= text.len() {
        None
    } else {
        text[end_idx..].chars().next()
    };

    before.map(|c| !c.is_alphanumeric()).unwrap_or(true)
        && after.map(|c| !c.is_alphanumeric()).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{Duration, UNIX_EPOCH},
    };

    use tempfile::tempdir;

    use super::{
        detect_external_change, find_matches, load_document, replace_all, write_document,
        EditorState,
    };
    use crate::core::{AppError, Document, SearchQuery, TabId};

    #[test]
    fn tab_management_keeps_single_placeholder_when_closing_all() {
        let mut state = EditorState::default();
        state.new_tab();
        state.close_all();
        assert_eq!(state.docs.len(), 1);
        assert_eq!(state.current_tab, 0);
    }

    #[test]
    fn close_others_retains_active_document() {
        let mut state = EditorState::default();
        state.new_tab();
        state.active_doc_mut().title = "keep".to_owned();
        state.new_tab();
        state.current_tab = 1;
        state.close_others();
        assert_eq!(state.docs.len(), 1);
        assert_eq!(state.active_doc().title, "keep");
    }

    #[test]
    fn open_and_save_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("sample.rs");
        fs::write(&path, "fn main() {}\n").expect("seed file");

        let syntax_map = crate::core::default_syntax_map();
        let mut doc = load_document(path.clone(), &syntax_map).expect("load document");
        assert_eq!(doc.syntax, "rust");

        doc.set_content("fn main() { println!(\"hi\"); }\n");
        write_document(&mut doc, path.clone()).expect("save document");
        assert_eq!(
            fs::read_to_string(path).expect("read saved"),
            doc.saved_content
        );
    }

    #[test]
    fn missing_file_maps_to_domain_error() {
        let syntax_map = crate::core::default_syntax_map();
        let err = load_document("missing.txt".into(), &syntax_map).expect_err("must fail");
        assert!(matches!(err, AppError::MissingFile(_)));
    }

    #[test]
    fn supports_case_sensitive_and_whole_word_matching() {
        let query = SearchQuery {
            query: "cat".into(),
            case_sensitive: false,
            whole_word: true,
        };
        let matches = find_matches("cat scatter CAT", &query);
        assert_eq!(matches, vec![0, 12]);
    }

    #[test]
    fn replace_all_respects_whole_word_option() {
        let query = SearchQuery {
            query: "foo".into(),
            case_sensitive: true,
            whole_word: true,
        };
        let result = replace_all("foo food foo", &query, "bar");
        assert_eq!(result.replaced, 2);
        assert_eq!(result.new_content, "bar food bar");
    }

    #[test]
    fn external_change_detected_only_for_clean_docs() {
        let doc = Document {
            id: TabId(1),
            title: "a".into(),
            path: None,
            content: "abc".into(),
            saved_content: "abc".into(),
            syntax: "txt".into(),
            last_modified: Some(UNIX_EPOCH + Duration::from_secs(5)),
        };
        assert!(detect_external_change(
            &doc,
            Some(UNIX_EPOCH + Duration::from_secs(9))
        ));

        let mut dirty = doc.clone();
        dirty.content.push('x');
        assert!(!detect_external_change(
            &dirty,
            Some(UNIX_EPOCH + Duration::from_secs(9))
        ));
    }
}
