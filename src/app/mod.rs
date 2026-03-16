mod menu_bar;
mod toolbar;
mod tab_bar;
mod find_panel;
mod editor_panel;
mod status_bar;
mod dialogs;

use std::time::{Duration, SystemTime};

use eframe::egui;
use rust_notepad::{
    core::SearchQuery,
    editor_state::EditorState,
    plugins::{self, EditorPlugin},
    settings::{FindState, GoToLineState, ViewSettings},
    shortcuts::Shortcuts,
    theme::AppTheme,
};

// ---------------------------------------------------------------------------
// Main application struct
// ---------------------------------------------------------------------------

pub struct RustNotepadApp {
    pub(crate) editor: EditorState,
    pub(crate) find_state: FindState,
    pub(crate) go_to_line: GoToLineState,
    pub(crate) app_theme: AppTheme,
    pub(crate) plugins: Vec<Box<dyn EditorPlugin>>,
    pub(crate) last_scan: SystemTime,
    pub(crate) view: ViewSettings,
}

impl RustNotepadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply dark visuals by default
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        Self {
            editor: EditorState::default(),
            find_state: FindState::default(),
            go_to_line: GoToLineState::default(),
            app_theme: AppTheme::Dark,
            plugins: plugins::default_plugins(),
            last_scan: SystemTime::now(),
            view: ViewSettings::default(),
        }
    }

    // -- File I/O (uses rfd dialogs, so must stay in UI layer) ---------------

    pub(crate) fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Err(e) = self.editor.open_document(path) {
                self.editor.active_doc_mut().diagnostics = format!("Open failed: {:?}", e);
            }
        }
    }

    pub(crate) fn save_active(&mut self) {
        match self.editor.save_active() {
            Ok(true) => self.save_active_as(), // needs path
            Ok(false) => {}
            Err(e) => {
                self.editor.active_doc_mut().diagnostics = format!("Save failed: {:?}", e);
            }
        }
    }

    pub(crate) fn save_active_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            if let Err(e) = self.editor.save_active_as(path) {
                self.editor.active_doc_mut().diagnostics = format!("Save failed: {:?}", e);
            }
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
        if self
            .last_scan
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs_f32()
            < 1.0
        {
            return;
        }
        self.last_scan = SystemTime::now();
        self.editor.scan_external_changes();
    }

    // -- Find / Replace (delegates to editor_state::find_matches) ------------

    pub(crate) fn refresh_matches(&mut self) {
        let haystack = &self.editor.active_doc().content;
        let query = SearchQuery {
            query: self.find_state.query.clone(),
            case_sensitive: self.find_state.case_sensitive,
            whole_word: self.find_state.whole_word,
        };
        self.find_state.matches = rust_notepad::editor_state::find_matches(haystack, &query);
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
        let query_len = self.find_state.query.len();
        let replacement = self.find_state.replacement.clone();
        self.editor
            .active_doc_mut()
            .content
            .replace_range(at..at + query_len, &replacement);
        self.refresh_matches();
    }

    pub(crate) fn replace_all(&mut self) {
        if self.find_state.query.is_empty() {
            return;
        }
        let query = SearchQuery {
            query: self.find_state.query.clone(),
            case_sensitive: self.find_state.case_sensitive,
            whole_word: self.find_state.whole_word,
        };
        let replacement = &self.find_state.replacement;
        let result =
            rust_notepad::editor_state::replace_all(&self.editor.active_doc().content, &query, replacement);
        self.editor.active_doc_mut().content = result.new_content;
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
            self.editor.close_tab(idx);
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
            app_theme: AppTheme::Dark,
            plugins: vec![],
            last_scan: SystemTime::now(),
            view: ViewSettings::default(),
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
