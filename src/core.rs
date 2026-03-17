use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::folding::FoldState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

// ---------------------------------------------------------------------------
// EOL style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EolStyle {
    LF,
    CRLF,
    CR,
}

impl EolStyle {
    pub fn label(&self) -> &'static str {
        match self {
            EolStyle::LF => "LF",
            EolStyle::CRLF => "CRLF",
            EolStyle::CR => "CR",
        }
    }

    pub fn sequence(&self) -> &'static str {
        match self {
            EolStyle::LF => "\n",
            EolStyle::CRLF => "\r\n",
            EolStyle::CR => "\r",
        }
    }

    /// Detect predominant EOL style from raw content.
    pub fn detect(content: &str) -> Self {
        let crlf = content.matches("\r\n").count();
        let lf_total = content.matches('\n').count();
        let cr_total = content.matches('\r').count();
        let lf = lf_total.saturating_sub(crlf);
        let cr = cr_total.saturating_sub(crlf);

        if crlf >= lf && crlf >= cr && crlf > 0 {
            EolStyle::CRLF
        } else if cr > lf && cr > 0 {
            EolStyle::CR
        } else {
            EolStyle::LF
        }
    }

    /// Convert content to use this EOL style.
    pub fn apply(&self, content: &str) -> String {
        // First normalize to LF, then convert to target
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
        match self {
            EolStyle::LF => normalized,
            EolStyle::CRLF => normalized.replace('\n', "\r\n"),
            EolStyle::CR => normalized.replace('\n', "\r"),
        }
    }
}

impl Default for EolStyle {
    fn default() -> Self {
        if cfg!(target_os = "windows") {
            EolStyle::CRLF
        } else {
            EolStyle::LF
        }
    }
}

// ---------------------------------------------------------------------------
// Detected encoding
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DetectedEncoding {
    #[default]
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Windows1252,
}

impl DetectedEncoding {
    pub fn label(&self) -> &'static str {
        match self {
            DetectedEncoding::Utf8 => "UTF-8",
            DetectedEncoding::Utf8Bom => "UTF-8 BOM",
            DetectedEncoding::Utf16Le => "UTF-16 LE",
            DetectedEncoding::Utf16Be => "UTF-16 BE",
            DetectedEncoding::Windows1252 => "Windows-1252",
        }
    }
}

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
    /// The EOL style detected on load (used when saving).
    #[serde(default)]
    pub eol_style: EolStyle,
    /// The encoding detected on load.
    #[serde(default)]
    pub encoding: DetectedEncoding,
    /// Transient: true when the file was changed on disk.
    #[serde(skip, default)]
    pub externally_changed: bool,
    /// Transient: diagnostic messages from plugins/validation.
    #[serde(skip, default)]
    pub diagnostics: String,
    /// Transient: code folding state.
    #[serde(skip, default)]
    pub fold_state: FoldState,
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
            eol_style: EolStyle::default(),
            encoding: DetectedEncoding::default(),
            externally_changed: false,
            diagnostics: String::new(),
            fold_state: FoldState::default(),
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

    /// Check if the file was modified externally (only for clean documents).
    pub fn detect_external_changes(&mut self) {
        if let Some(path) = &self.path {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if let Some(prev) = self.last_modified {
                        if modified > prev && !self.is_dirty() {
                            self.externally_changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Reload content from disk and clear externally_changed flag.
    pub fn reload_from_disk(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            let content = fs::read_to_string(path)?;
            self.content = content.clone();
            self.saved_content = content;
            self.last_modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
            self.externally_changed = false;
        }
        Ok(())
    }

    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    pub fn char_count(&self) -> usize {
        self.content.len()
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

/// Detect syntax from a shebang line (e.g. `#!/usr/bin/env python3`).
pub fn detect_syntax_from_shebang(content: &str) -> Option<&'static str> {
    let first_line = content.lines().next()?;
    if !first_line.starts_with("#!") {
        return None;
    }
    let shebang = first_line.to_lowercase();
    if shebang.contains("python") {
        Some("py")
    } else if shebang.contains("ruby") {
        Some("rb")
    } else if shebang.contains("node") || shebang.contains("deno") {
        Some("js")
    } else if shebang.contains("bash") || shebang.contains("/sh") || shebang.contains("/zsh") {
        Some("sh")
    } else if shebang.contains("perl") {
        Some("pl")
    } else if shebang.contains("lua") {
        Some("lua")
    } else if shebang.contains("php") {
        Some("php")
    } else {
        None
    }
}

/// Returns a map from file extension to the syntax name used for highlighting.
///
/// Values are looked up by `egui_extras::syntax_highlighting` via syntect in two steps:
/// 1. `find_syntax_by_name(value)` — matches syntect's built-in name (case-insensitive)
/// 2. `find_syntax_by_extension(value)` — matches file extensions registered in syntect
///
/// Syntect's default pack includes ~50 languages. For extensions not covered,
/// we map to the closest available syntax (e.g. csproj → xml).
/// If syntect doesn't recognize the value at all, it falls back to plain text.
pub fn default_syntax_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // === Languages with full syntect support (use extension for lookup) ===
        ("rs", "rs"),
        ("js", "js"),
        ("py", "py"),
        ("json", "json"),
        ("xml", "xml"),
        ("txt", "txt"),
        ("java", "java"),
        ("cpp", "cpp"),
        ("cc", "cpp"),
        ("cxx", "cpp"),
        ("c", "c"),
        ("h", "h"),
        ("hpp", "hpp"),
        ("hxx", "hpp"),
        ("go", "go"),
        ("md", "md"),
        ("markdown", "md"),
        ("html", "html"),
        ("htm", "html"),
        ("css", "css"),
        ("sql", "sql"),
        ("sh", "sh"),
        ("bash", "sh"),
        ("zsh", "sh"),
        ("fish", "sh"),
        ("yaml", "yaml"),
        ("yml", "yaml"),
        ("rb", "rb"),
        ("cs", "cs"), // C#
        ("csx", "cs"),
        ("php", "php"),
        ("pl", "pl"), // Perl
        ("pm", "pl"),
        ("lua", "lua"),
        ("r", "r"),
        ("R", "r"),
        ("scala", "scala"),
        ("sbt", "scala"),
        ("hs", "hs"),   // Haskell
        ("clj", "clj"), // Clojure
        ("d", "d"),
        ("pas", "pas"), // Pascal
        ("p", "pas"),
        ("bat", "bat"),
        ("cmd", "bat"),
        ("erl", "erl"), // Erlang
        ("hrl", "erl"),
        ("lisp", "lisp"),
        ("cl", "lisp"),
        ("el", "lisp"),
        ("ml", "ml"), // OCaml
        ("mli", "ml"),
        ("m", "m"),   // Objective-C
        ("mm", "mm"), // Objective-C++
        ("groovy", "groovy"),
        ("gradle", "groovy"),
        ("diff", "diff"),
        ("patch", "diff"),
        ("tex", "tex"), // LaTeX
        ("ltx", "tex"),
        ("re", "re"),   // Regular expression
        ("dot", "dot"), // Graphviz
        ("gv", "dot"),
        ("tcl", "tcl"),
        ("haml", "haml"),
        ("erb", "rails"),
        ("properties", "properties"),
        ("mk", "make"), // Makefile
        ("make", "make"),
        // === XML-based formats (use xml syntax) ===
        ("csproj", "xml"),
        ("fsproj", "xml"),
        ("vbproj", "xml"),
        ("props", "xml"),
        ("targets", "xml"),
        ("nuspec", "xml"),
        ("config", "xml"),
        ("xaml", "xml"),
        ("svg", "xml"),
        ("plist", "xml"),
        ("xsl", "xml"),
        ("xslt", "xml"),
        ("xsd", "xml"),
        ("wsdl", "xml"),
        ("rss", "xml"),
        ("opml", "xml"),
        // === JSON-based formats ===
        ("jsonc", "json"),
        ("geojson", "json"),
        // === HTML-based formats ===
        ("vue", "html"),
        ("svelte", "html"),
        ("jsp", "jsp"),
        // === Languages NOT in syntect defaults (fall back to plain text) ===
        // These are mapped but syntect will silently render as plain text.
        // When we add custom syntax loading, these will work.
        ("ts", "js"), // TypeScript → highlight as JavaScript (close enough)
        ("tsx", "js"),
        ("jsx", "js"),
        ("toml", "yaml"), // TOML → highlight as YAML (similar key-value style)
        ("ini", "yaml"),
        ("cfg", "yaml"),
        ("conf", "yaml"),
        ("ps1", "sh"), // PowerShell → highlight as Shell (better than nothing)
        ("psm1", "sh"),
        ("psd1", "sh"),
        ("swift", "java"), // Swift → Java (similar C-style syntax)
        ("kt", "java"),    // Kotlin → Java
        ("dart", "java"),  // Dart → Java
        ("lock", "yaml"),  // Cargo.lock → YAML
        ("csv", "txt"),    // CSV → plain text
        ("tsv", "txt"),
        ("log", "txt"),
    ])
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{
        default_syntax_map, detect_syntax_from_shebang, Document, EolStyle, FakeClock,
        SessionState, TabId,
    };
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
        assert_eq!(syntax.get("rs"), Some(&"rs"));
        assert_eq!(syntax.get("txt"), Some(&"txt"));
        assert_eq!(syntax.get("cs"), Some(&"cs"));
        assert_eq!(syntax.get("ps1"), Some(&"sh")); // PowerShell → sh fallback
        assert_eq!(syntax.get("csproj"), Some(&"xml"));
        assert_eq!(syntax.get("csv"), Some(&"txt")); // CSV → plain text
        assert_eq!(syntax.get("ts"), Some(&"js")); // TypeScript → JS fallback
    }

    #[test]
    fn eol_detect_lf() {
        assert_eq!(EolStyle::detect("hello\nworld\n"), EolStyle::LF);
    }

    #[test]
    fn eol_detect_crlf() {
        assert_eq!(EolStyle::detect("hello\r\nworld\r\n"), EolStyle::CRLF);
    }

    #[test]
    fn eol_detect_cr() {
        assert_eq!(EolStyle::detect("hello\rworld\r"), EolStyle::CR);
    }

    #[test]
    fn eol_apply_converts() {
        let lf = "a\nb\nc";
        assert_eq!(EolStyle::CRLF.apply(lf), "a\r\nb\r\nc");
        let crlf = "a\r\nb\r\nc";
        assert_eq!(EolStyle::LF.apply(crlf), "a\nb\nc");
    }

    #[test]
    fn shebang_detection() {
        assert_eq!(
            detect_syntax_from_shebang("#!/usr/bin/env python3\n"),
            Some("py")
        );
        assert_eq!(detect_syntax_from_shebang("#!/bin/bash\n"), Some("sh"));
        assert_eq!(detect_syntax_from_shebang("#!/usr/bin/node\n"), Some("js"));
        assert_eq!(detect_syntax_from_shebang("#!/usr/bin/ruby\n"), Some("rb"));
        assert_eq!(detect_syntax_from_shebang("no shebang\n"), None);
    }
}
