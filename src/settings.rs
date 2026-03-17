/// State for the Find & Replace panel.
#[derive(Default)]
pub struct FindState {
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub extended_mode: bool,
    pub matches: Vec<usize>,
    /// Lengths of each match (needed for regex where match lengths can vary).
    pub match_lengths: Vec<usize>,
    pub selected_match: usize,
    pub show_panel: bool,
    /// When set, the editor should move cursor to this byte range and select it.
    pub navigate_to: Option<(usize, usize)>,
    /// When true, the editor TextEdit should request focus (makes cursor visible and blinking).
    pub focus_editor: bool,
    /// Show Find in Files results.
    pub show_find_in_files: bool,
    /// Find in Files results: (file_path, line_number, line_content).
    pub file_results: Vec<(String, usize, String)>,
}

impl FindState {
    pub fn find_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected_match = (self.selected_match + 1) % self.matches.len();
            self.request_navigation();
        }
    }

    pub fn find_prev(&mut self) {
        if !self.matches.is_empty() {
            if self.selected_match == 0 {
                self.selected_match = self.matches.len() - 1;
            } else {
                self.selected_match -= 1;
            }
            self.request_navigation();
        }
    }

    pub fn select_match(&mut self, idx: usize) {
        if idx < self.matches.len() {
            self.selected_match = idx;
            self.request_navigation();
        }
    }

    fn request_navigation(&mut self) {
        if let Some(&pos) = self.matches.get(self.selected_match) {
            let match_len = self
                .match_lengths
                .get(self.selected_match)
                .copied()
                .unwrap_or(self.query.len());
            let end = pos + match_len;
            self.navigate_to = Some((pos, end));
            self.focus_editor = true;
        }
    }
}

/// State for the Go to Line dialog.
#[derive(Default)]
pub struct GoToLineState {
    pub open: bool,
    pub input: String,
}

/// Cursor position for display in status bar.
#[derive(Clone, Copy, Default)]
pub struct CursorPosition {
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
    pub selection_len: usize,
}

/// Persisted application state saved to disk between sessions.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistedState {
    pub theme: String,
    pub font_size: f32,
    pub ui_zoom_pct: u32,
    pub show_toolbar: bool,
    pub show_status_bar: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub show_whitespace: bool,
    /// If true, tabs wrap to multiple lines; if false, tabs scroll horizontally.
    #[serde(default = "default_true")]
    pub tab_wrap: bool,
    /// Tab stop size (2, 4, or 8 spaces).
    #[serde(default = "default_tab_size")]
    pub tab_size: u8,
    /// Auto-indent on newline.
    #[serde(default = "default_true")]
    pub auto_indent: bool,
    /// Paths of open tabs, in order. Empty string for untitled tabs.
    pub open_tabs: Vec<String>,
    /// Index of the active tab.
    pub active_tab: usize,
    /// Recently opened file paths (most recent first, max 10).
    #[serde(default)]
    pub recent_files: Vec<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            theme: "Dark".to_string(),
            font_size: 14.0,
            ui_zoom_pct: 100,
            show_toolbar: true,
            show_status_bar: true,
            show_line_numbers: true,
            word_wrap: false,
            show_whitespace: false,
            tab_wrap: true,
            tab_size: 4,
            auto_indent: true,
            open_tabs: Vec::new(),
            active_tab: 0,
            recent_files: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_tab_size() -> u8 {
    4
}

impl PersistedState {
    /// Path to the session file.
    pub fn session_path() -> std::path::PathBuf {
        let mut p = dirs_next().unwrap_or_else(|| std::path::PathBuf::from("."));
        p.push("session.json");
        p
    }

    /// Load from disk, falling back to defaults.
    pub fn load() -> Self {
        let path = Self::session_path();
        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to disk.
    pub fn save(&self) {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}

/// Returns the CodeEdit config directory (~/.codeedit/).
fn dirs_next() -> Option<std::path::PathBuf> {
    // Use HOME or USERPROFILE on Windows
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(|home| {
            let mut p = std::path::PathBuf::from(home);
            p.push(".codeedit");
            p
        })
}

/// View-related settings (togglable panels, display options).
#[derive(Clone)]
pub struct ViewSettings {
    pub show_toolbar: bool,
    pub show_status_bar: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub show_whitespace: bool,
    pub font_size: f32,
    pub cursor: CursorPosition,
    /// Global UI zoom percentage (100 = normal, 150 = 150% etc.)
    pub ui_zoom_pct: u32,
    /// If true, tabs wrap to multiple lines; if false, tabs scroll horizontally.
    pub tab_wrap: bool,
    /// Tab stop size (2, 4, or 8).
    pub tab_size: u8,
    /// Auto-indent new lines to match previous line indentation.
    pub auto_indent: bool,
    /// Tracks previous content length for auto-indent detection.
    pub prev_content_len: usize,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            show_toolbar: true,
            show_status_bar: true,
            show_line_numbers: true,
            word_wrap: false,
            show_whitespace: false,
            font_size: 14.0,
            cursor: CursorPosition::default(),
            ui_zoom_pct: 100,
            tab_wrap: true,
            tab_size: 4,
            auto_indent: true,
            prev_content_len: 0,
        }
    }
}

impl ViewSettings {
    pub fn zoom_in(&mut self) {
        self.font_size = (self.font_size + 1.0).min(72.0);
    }

    pub fn zoom_out(&mut self) {
        self.font_size = (self.font_size - 1.0).max(6.0);
    }

    pub fn ui_zoom_in(&mut self) {
        self.ui_zoom_pct = (self.ui_zoom_pct + 10).min(300);
    }

    pub fn ui_zoom_out(&mut self) {
        self.ui_zoom_pct = self.ui_zoom_pct.saturating_sub(10).max(50);
    }

    pub fn ui_zoom_reset(&mut self) {
        self.ui_zoom_pct = 100;
    }

    /// Pixels per point for the given zoom level.
    pub fn pixels_per_point(&self) -> f32 {
        self.ui_zoom_pct as f32 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_next_wraps_around() {
        let mut state = FindState {
            matches: vec![0, 5, 10],
            selected_match: 2,
            ..Default::default()
        };
        state.find_next();
        assert_eq!(state.selected_match, 0);
    }

    #[test]
    fn find_prev_wraps_around() {
        let mut state = FindState {
            matches: vec![0, 5, 10],
            selected_match: 0,
            ..Default::default()
        };
        state.find_prev();
        assert_eq!(state.selected_match, 2);
    }

    #[test]
    fn find_next_noop_on_empty() {
        let mut state = FindState::default();
        state.find_next(); // should not panic
        assert_eq!(state.selected_match, 0);
    }

    #[test]
    fn zoom_clamps_at_bounds() {
        let mut view = ViewSettings { font_size: 72.0, ..Default::default() };
        view.zoom_in();
        assert_eq!(view.font_size, 72.0);

        view.font_size = 6.0;
        view.zoom_out();
        assert_eq!(view.font_size, 6.0);
    }

    #[test]
    fn default_view_settings() {
        let view = ViewSettings::default();
        assert!(view.show_toolbar);
        assert!(view.show_status_bar);
        assert_eq!(view.font_size, 14.0);
        assert!(!view.word_wrap);
    }
}
