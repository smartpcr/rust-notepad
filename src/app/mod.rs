mod dialogs;
mod editor_panel;
mod find_panel;
mod menu_bar;
mod status_bar;
mod tab_bar;
mod toolbar;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use eframe::egui;
use rust_notepad::{
    core::SearchQuery,
    editor_state::EditorState,
    plugins::{self, EditorPlugin},
    settings::{FindState, GoToLineState, PersistedState, ViewSettings},
    shortcuts::Shortcuts,
    theme::AppTheme,
};

// ---------------------------------------------------------------------------
// Main application struct
// ---------------------------------------------------------------------------

/// Tracks pending close operations that need user confirmation.
#[derive(Default)]
pub(crate) struct CloseConfirm {
    /// Tab indices pending close (dirty ones need confirmation).
    pub pending_tabs: Vec<usize>,
    /// True when the dialog is showing.
    pub open: bool,
}

pub struct RustNotepadApp {
    pub(crate) editor: EditorState,
    pub(crate) find_state: FindState,
    pub(crate) go_to_line: GoToLineState,
    pub(crate) close_confirm: CloseConfirm,
    pub(crate) app_theme: AppTheme,
    pub(crate) plugins: Vec<Box<dyn EditorPlugin>>,
    pub(crate) last_scan: SystemTime,
    pub(crate) view: ViewSettings,
    /// Recently opened file paths (most recent first).
    pub(crate) recent_files: Vec<String>,
    /// File watcher event receiver (from notify).
    pub(crate) watcher_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    /// File watcher instance (kept alive).
    #[allow(dead_code)]
    pub(crate) watcher: Option<notify::RecommendedWatcher>,
}

impl RustNotepadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_files(cc, Vec::new())
    }

    pub fn new_with_files(cc: &eframe::CreationContext<'_>, initial_files: Vec<PathBuf>) -> Self {
        // Load persisted session
        let persisted = PersistedState::load();

        let theme = if persisted.theme == "Light" {
            AppTheme::Light
        } else {
            AppTheme::Dark
        };
        theme.apply(&cc.egui_ctx);

        let view = ViewSettings {
            show_toolbar: persisted.show_toolbar,
            show_status_bar: persisted.show_status_bar,
            show_line_numbers: persisted.show_line_numbers,
            word_wrap: persisted.word_wrap,
            show_whitespace: persisted.show_whitespace,
            font_size: persisted.font_size,
            ui_zoom_pct: persisted.ui_zoom_pct,
            tab_wrap: persisted.tab_wrap,
            tab_size: persisted.tab_size,
            auto_indent: persisted.auto_indent,
            ..Default::default()
        };

        let mut editor = EditorState::default();

        // Restore open tabs from persisted paths
        let mut restored_any = false;
        for path_str in &persisted.open_tabs {
            if path_str.is_empty() {
                continue;
            }
            let path = std::path::PathBuf::from(path_str);
            if path.exists() {
                if let Ok(()) = editor.open_document(path) {
                    restored_any = true;
                }
            }
        }

        // Open files from CLI args
        for path in &initial_files {
            if path.exists() {
                if let Ok(()) = editor.open_document(path.clone()) {
                    restored_any = true;
                }
            }
        }

        if restored_any {
            // Remove the default "Untitled 1" tab that EditorState creates
            if editor.docs.len() > 1 && editor.docs[0].path.is_none() && !editor.docs[0].is_dirty()
            {
                editor.docs.remove(0);
            }
            // Restore active tab index (only if no CLI files were opened)
            if initial_files.is_empty() {
                editor.current_tab = persisted
                    .active_tab
                    .min(editor.docs.len().saturating_sub(1));
            }
        }

        // Set up file watcher
        let (watcher, watcher_rx) = Self::create_watcher();

        Self {
            editor,
            find_state: FindState::default(),
            go_to_line: GoToLineState::default(),
            close_confirm: CloseConfirm::default(),
            app_theme: theme,
            plugins: plugins::default_plugins(),
            last_scan: SystemTime::now(),
            view,
            recent_files: persisted.recent_files,
            watcher_rx,
            watcher,
        }
    }

    fn create_watcher() -> (
        Option<notify::RecommendedWatcher>,
        Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    ) {
        use notify::Watcher;
        let (tx, rx) = mpsc::channel();
        match notify::RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        ) {
            Ok(w) => (Some(w), Some(rx)),
            Err(_) => (None, None), // Fall back to polling
        }
    }

    /// Save current session state to disk.
    fn save_session(&self) {
        let open_tabs: Vec<String> = self
            .editor
            .docs
            .iter()
            .map(|doc| {
                doc.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            })
            .collect();

        let state = PersistedState {
            theme: self.app_theme.label().to_string(),
            font_size: self.view.font_size,
            ui_zoom_pct: self.view.ui_zoom_pct,
            show_toolbar: self.view.show_toolbar,
            show_status_bar: self.view.show_status_bar,
            show_line_numbers: self.view.show_line_numbers,
            word_wrap: self.view.word_wrap,
            show_whitespace: self.view.show_whitespace,
            tab_wrap: self.view.tab_wrap,
            tab_size: self.view.tab_size,
            auto_indent: self.view.auto_indent,
            open_tabs,
            active_tab: self.editor.current_tab,
            recent_files: self.recent_files.clone(),
        };
        state.save();
    }

    /// Add a file path to the recent files list.
    pub(crate) fn add_to_recent(&mut self, path: &std::path::Path) {
        let path_str = path.display().to_string();
        self.recent_files.retain(|p| *p != path_str);
        self.recent_files.insert(0, path_str);
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
    }

    // -- Close with confirmation -------------------------------------------

    /// Request closing a single tab. If dirty, shows confirmation dialog.
    pub(crate) fn request_close_tab(&mut self, idx: usize) {
        if idx < self.editor.docs.len() && self.editor.docs[idx].is_dirty() {
            self.close_confirm.pending_tabs = vec![idx];
            self.close_confirm.open = true;
        } else {
            self.editor.close_tab(idx);
        }
    }

    /// Request closing all tabs. If any are dirty, shows confirmation dialog.
    pub(crate) fn request_close_all(&mut self) {
        let dirty_indices: Vec<usize> = self
            .editor
            .docs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.is_dirty())
            .map(|(i, _)| i)
            .collect();
        if dirty_indices.is_empty() {
            self.editor.close_all();
        } else {
            self.close_confirm.pending_tabs = dirty_indices;
            self.close_confirm.open = true;
        }
    }

    /// Request closing all tabs except the given index.
    pub(crate) fn request_close_others(&mut self, keep_idx: usize) {
        let dirty_indices: Vec<usize> = self
            .editor
            .docs
            .iter()
            .enumerate()
            .filter(|(i, d)| *i != keep_idx && d.is_dirty())
            .map(|(i, _)| i)
            .collect();
        if dirty_indices.is_empty() {
            self.editor.current_tab = keep_idx;
            self.editor.close_others();
        } else {
            // Include all non-kept tabs (dirty ones need confirmation)
            let all_others: Vec<usize> = (0..self.editor.docs.len())
                .filter(|&i| i != keep_idx)
                .collect();
            self.close_confirm.pending_tabs = all_others;
            self.close_confirm.open = true;
        }
    }

    /// Execute "Save All" for pending tabs, then close them.
    pub(crate) fn confirm_save_and_close(&mut self) {
        // Save all dirty pending tabs
        for &idx in &self.close_confirm.pending_tabs {
            if idx < self.editor.docs.len() && self.editor.docs[idx].is_dirty() {
                if let Some(path) = self.editor.docs[idx].path.clone() {
                    let _ = rust_notepad::editor_state::write_document(
                        &mut self.editor.docs[idx],
                        path,
                    );
                }
                // If no path, skip (can't auto-save untitled)
            }
        }
        self.execute_pending_close();
    }

    /// Discard changes and close pending tabs.
    pub(crate) fn confirm_discard_and_close(&mut self) {
        self.execute_pending_close();
    }

    fn execute_pending_close(&mut self) {
        // Close tabs in reverse order to keep indices valid
        let mut tabs = self.close_confirm.pending_tabs.clone();
        tabs.sort_unstable();
        tabs.reverse();
        for idx in tabs {
            if idx < self.editor.docs.len() {
                self.editor.close_tab(idx);
            }
        }
        self.close_confirm.pending_tabs.clear();
        self.close_confirm.open = false;
    }

    // -- File I/O (uses rfd dialogs, so must stay in UI layer) ---------------

    pub(crate) fn open_file(&mut self) {
        let paths = rfd::FileDialog::new().pick_files();
        if let Some(paths) = paths {
            for path in paths {
                self.add_to_recent(&path);
                self.watch_file(&path);
                if let Err(e) = self.editor.open_document(path) {
                    self.editor.active_doc_mut().diagnostics = format!("Open failed: {:?}", e);
                }
            }
        }
    }

    pub(crate) fn save_active(&mut self) {
        match self.editor.save_active() {
            Ok(true) => self.save_active_as(), // needs path
            Ok(false) => {
                if let Some(path) = self.editor.active_doc().path.clone() {
                    self.add_to_recent(&path);
                }
            }
            Err(e) => {
                self.editor.active_doc_mut().diagnostics = format!("Save failed: {:?}", e);
            }
        }
    }

    pub(crate) fn save_active_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            self.add_to_recent(&path);
            self.watch_file(&path);
            if let Err(e) = self.editor.save_active_as(path) {
                self.editor.active_doc_mut().diagnostics = format!("Save failed: {:?}", e);
            }
        }
    }

    fn watch_file(&mut self, path: &std::path::Path) {
        use notify::Watcher;
        if let Some(watcher) = &mut self.watcher {
            let _ = watcher.watch(path, notify::RecursiveMode::NonRecursive);
        }
    }

    // -- Plugin orchestration ------------------------------------------------

    pub(crate) fn run_plugins(&mut self) {
        let ext = self
            .editor
            .active_doc()
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        let content = self.editor.active_doc().content.clone();
        let messages = plugins::run_plugins(&self.plugins, &ext, &content);
        if !messages.is_empty() {
            self.editor.active_doc_mut().diagnostics = messages.join(" | ");
        }
    }

    // -- External change detection (throttled) -------------------------------

    fn scan_external_changes(&mut self) {
        // Check notify watcher events first
        if let Some(rx) = &self.watcher_rx {
            let mut changed_paths: Vec<PathBuf> = Vec::new();
            while let Ok(Ok(event)) = rx.try_recv() {
                if matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    changed_paths.extend(event.paths);
                }
            }
            // Mark documents as externally changed
            for doc in &mut self.editor.docs {
                if let Some(doc_path) = &doc.path {
                    if changed_paths.iter().any(|p| p == doc_path) && !doc.is_dirty() {
                        doc.externally_changed = true;
                    }
                }
            }
        }

        // Fall back to polling (throttled) if watcher missed anything
        if self
            .last_scan
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs_f32()
            < 2.0
        {
            return;
        }
        self.last_scan = SystemTime::now();
        self.editor.scan_external_changes();
    }

    // -- Find / Replace (delegates to editor_state::find_matches) ------------

    pub(crate) fn refresh_matches(&mut self) {
        let haystack = &self.editor.active_doc().content;

        // Expand extended search escapes if enabled
        let query_str = if self.find_state.extended_mode {
            rust_notepad::editor_state::expand_extended(&self.find_state.query)
        } else {
            self.find_state.query.clone()
        };

        if self.find_state.use_regex {
            // Regex search
            if let Some((positions, lengths)) = rust_notepad::editor_state::find_matches_regex(
                haystack,
                &query_str,
                self.find_state.case_sensitive,
            ) {
                self.find_state.matches = positions;
                self.find_state.match_lengths = lengths;
            } else {
                self.find_state.matches.clear();
                self.find_state.match_lengths.clear();
            }
        } else {
            // Standard search
            let query = SearchQuery {
                query: query_str,
                case_sensitive: self.find_state.case_sensitive,
                whole_word: self.find_state.whole_word,
            };
            self.find_state.matches = rust_notepad::editor_state::find_matches(haystack, &query);
            self.find_state.match_lengths = self
                .find_state
                .matches
                .iter()
                .map(|_| self.find_state.query.len())
                .collect();
        }
    }

    pub(crate) fn replace_current(&mut self) {
        self.refresh_matches();
        if self.find_state.matches.is_empty() {
            return;
        }
        let idx = self
            .find_state
            .selected_match
            .min(self.find_state.matches.len() - 1);
        let at = self.find_state.matches[idx];
        let match_len = self
            .find_state
            .match_lengths
            .get(idx)
            .copied()
            .unwrap_or(self.find_state.query.len());
        let replacement = self.find_state.replacement.clone();
        self.editor
            .active_doc_mut()
            .content
            .replace_range(at..at + match_len, &replacement);
        self.refresh_matches();
    }

    pub(crate) fn replace_all(&mut self) {
        if self.find_state.query.is_empty() {
            return;
        }
        let query_str = if self.find_state.extended_mode {
            rust_notepad::editor_state::expand_extended(&self.find_state.query)
        } else {
            self.find_state.query.clone()
        };
        let replacement = if self.find_state.extended_mode {
            rust_notepad::editor_state::expand_extended(&self.find_state.replacement)
        } else {
            self.find_state.replacement.clone()
        };

        if self.find_state.use_regex {
            // For regex replace, use the regex crate directly
            let pattern = if self.find_state.case_sensitive {
                query_str.clone()
            } else {
                format!("(?i){}", query_str)
            };
            if let Ok(re) = regex::Regex::new(&pattern) {
                let new_content = re
                    .replace_all(&self.editor.active_doc().content, replacement.as_str())
                    .to_string();
                self.editor.active_doc_mut().content = new_content;
            }
        } else {
            let query = SearchQuery {
                query: query_str,
                case_sensitive: self.find_state.case_sensitive,
                whole_word: self.find_state.whole_word,
            };
            let result = rust_notepad::editor_state::replace_all(
                &self.editor.active_doc().content,
                &query,
                &replacement,
            );
            self.editor.active_doc_mut().content = result.new_content;
        }
        self.refresh_matches();
    }

    // -- Keyboard shortcuts --------------------------------------------------

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // File — check more specific (Ctrl+Shift) before less specific (Ctrl)
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::save_as())) {
            self.save_active_as();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::new_tab())) {
            self.editor.new_tab();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::open())) {
            self.open_file();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::save())) {
            self.save_active();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::close_tab())) {
            let idx = self.editor.current_tab;
            self.request_close_tab(idx);
        }

        // Search
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::find())) {
            self.find_state.show_panel = true;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::replace())) {
            self.find_state.show_panel = true;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::go_to_line())) {
            self.go_to_line.open = true;
            self.go_to_line.input.clear();
        }

        // View
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::toggle_word_wrap())) {
            self.view.word_wrap = !self.view.word_wrap;
        }

        // Tab navigation
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::prev_tab())) {
            self.editor.prev_tab();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::next_tab())) {
            self.editor.next_tab();
        }

        // Zoom
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::zoom_in())) {
            self.view.zoom_in();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::zoom_out())) {
            self.view.zoom_out();
        }

        // Find next/prev
        if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::find_prev())) {
            self.find_state.find_prev();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Shortcuts::find_next())) {
            self.find_state.find_next();
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App — Notepad++ style layout
// ---------------------------------------------------------------------------

impl eframe::App for RustNotepadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0. Apply theme + UI zoom every frame (eframe may reset after init)
        self.app_theme.apply(ctx);
        ctx.set_pixels_per_point(self.view.pixels_per_point());

        // 0.5 Handle drag-and-drop files
        let dropped_files: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped_files {
            self.add_to_recent(&path);
            self.watch_file(&path);
            if let Err(e) = self.editor.open_document(path) {
                self.editor.active_doc_mut().diagnostics = format!("Open failed: {:?}", e);
            }
        }

        // 1. Background tasks
        self.scan_external_changes();
        self.refresh_matches();

        // 2. Keyboard shortcuts (before UI so they take priority)
        self.handle_shortcuts(ctx);

        // 3. Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu_bar::render(self, ui, ctx);
        });

        // 4. Toolbar (togglable)
        if self.view.show_toolbar {
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                toolbar::render(self, ui);
            });
        }

        // 5. Tab bar
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            tab_bar::render(self, ui);
        });

        // 6. Status bar (togglable)
        if self.view.show_status_bar {
            egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
                status_bar::render(self, ui);
            });
        }

        // 7. Find & Replace side panel
        if self.find_state.show_panel {
            egui::SidePanel::right("find_panel")
                .default_width(300.0)
                .show(ctx, |ui| {
                    find_panel::render(self, ui);
                });
        }

        // 8. Central editor (fills all remaining space)
        egui::CentralPanel::default().show(ctx, |ui| {
            editor_panel::render(self, ui);
        });

        // 9. Dialogs (floating windows)
        dialogs::render_go_to_line(self, ctx);
        dialogs::render_close_confirm(self, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_session();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> RustNotepadApp {
        RustNotepadApp {
            editor: EditorState::default(),
            find_state: FindState::default(),
            go_to_line: GoToLineState::default(),
            close_confirm: CloseConfirm::default(),
            app_theme: AppTheme::Dark,
            plugins: vec![],
            last_scan: SystemTime::now(),
            view: ViewSettings::default(),
            recent_files: Vec::new(),
            watcher_rx: None,
            watcher: None,
        }
    }

    #[test]
    fn new_tab_increments_count() {
        let mut app = create_test_app();
        assert_eq!(app.editor.docs.len(), 1);
        app.editor.new_tab();
        assert_eq!(app.editor.docs.len(), 2);
        assert_eq!(app.editor.current_tab, 1);
    }

    #[test]
    fn close_tab_keeps_one() {
        let mut app = create_test_app();
        app.editor.close_tab(0);
        assert_eq!(app.editor.docs.len(), 1);
    }

    #[test]
    fn save_all_skips_untitled() {
        let mut app = create_test_app();
        app.editor.save_all();
        assert_eq!(app.editor.docs.len(), 1);
    }

    #[test]
    fn next_prev_tab() {
        let mut app = create_test_app();
        app.editor.new_tab();
        app.editor.new_tab();
        assert_eq!(app.editor.current_tab, 2);
        app.editor.next_tab();
        assert_eq!(app.editor.current_tab, 0); // wraps
        app.editor.prev_tab();
        assert_eq!(app.editor.current_tab, 2); // wraps back
    }

    #[test]
    fn refresh_matches_finds_occurrences() {
        let mut app = create_test_app();
        app.editor.active_doc_mut().content = "foo bar foo".to_string();
        app.find_state.query = "foo".to_string();
        app.refresh_matches();
        assert_eq!(app.find_state.matches, vec![0, 8]);
    }

    #[test]
    fn replace_all_updates_content() {
        let mut app = create_test_app();
        app.editor.active_doc_mut().content = "hello hello".to_string();
        app.find_state.query = "hello".to_string();
        app.find_state.replacement = "world".to_string();
        app.replace_all();
        assert_eq!(app.editor.active_doc().content, "world world");
    }

    #[test]
    fn run_plugins_on_non_matching_ext() {
        let mut app = create_test_app();
        app.plugins = plugins::default_plugins();
        app.editor.active_doc_mut().content = "fn main() {}".to_string();
        // No path set, ext is empty → no plugins match
        app.run_plugins();
        assert!(app.editor.active_doc().diagnostics.is_empty());
    }
}
