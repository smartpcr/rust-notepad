use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use eframe::egui::{self, text::LayoutJob, FontId, TextEdit};
use egui_extras::syntax_highlighting::{self, CodeTheme};

#[derive(Default)]
struct FindState {
    query: String,
    replacement: String,
    case_sensitive: bool,
    whole_word: bool,
    matches: Vec<usize>,
    selected_match: usize,
    show_panel: bool,
}

trait EditorPlugin {
    fn name(&self) -> &'static str;
    fn supports_extension(&self, ext: &str) -> bool;
    fn validate(&self, text: &str) -> anyhow::Result<()>;
}

struct JsonPlugin;

impl EditorPlugin for JsonPlugin {
    fn name(&self) -> &'static str {
        "JSON Validator"
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("json")
    }

    fn validate(&self, text: &str) -> anyhow::Result<()> {
        serde_json::from_str::<serde_json::Value>(text)?;
        Ok(())
    }
}

struct XmlPlugin;

impl EditorPlugin for XmlPlugin {
    fn name(&self) -> &'static str {
        "XML Validator"
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("xml")
    }

    fn validate(&self, text: &str) -> anyhow::Result<()> {
        quick_xml::de::from_str::<serde_json::Value>(text)?;
        Ok(())
    }
}

#[derive(Default)]
struct Document {
    title: String,
    path: Option<PathBuf>,
    content: String,
    saved_content: String,
    syntax: String,
    last_modified: Option<SystemTime>,
    externally_changed: bool,
    diagnostics: String,
}

impl Document {
    fn new_untitled(index: usize) -> Self {
        Self {
            title: format!("Untitled {}", index),
            syntax: "txt".to_string(),
            ..Default::default()
        }
    }

    fn is_dirty(&self) -> bool {
        self.content != self.saved_content
    }

    fn detect_external_changes(&mut self) {
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

    fn reload_from_disk(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            let content = fs::read_to_string(path)?;
            self.content = content.clone();
            self.saved_content = content;
            self.last_modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
            self.externally_changed = false;
        }
        Ok(())
    }
}

pub struct RustNotepadApp {
    docs: Vec<Document>,
    current_tab: usize,
    untitled_count: usize,
    find_state: FindState,
    theme: CodeTheme,
    syntax_map: HashMap<&'static str, &'static str>,
    plugins: Vec<Box<dyn EditorPlugin>>,
    last_scan: SystemTime,
}

impl RustNotepadApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            docs: vec![Document::new_untitled(1)],
            current_tab: 0,
            untitled_count: 1,
            find_state: FindState::default(),
            theme: CodeTheme::dark(),
            syntax_map: HashMap::new(),
            plugins: vec![Box::new(JsonPlugin), Box::new(XmlPlugin)],
            last_scan: SystemTime::now(),
        };

        app.seed_syntax_map();
        app
    }

    fn seed_syntax_map(&mut self) {
        self.syntax_map.extend([
            ("rs", "rust"),
            ("js", "javascript"),
            ("ts", "typescript"),
            ("py", "python"),
            ("java", "java"),
            ("cpp", "cpp"),
            ("c", "c"),
            ("go", "go"),
            ("json", "json"),
            ("xml", "xml"),
            ("toml", "toml"),
            ("md", "markdown"),
            ("html", "html"),
            ("css", "css"),
            ("sql", "sql"),
        ]);
    }

    fn active_doc_mut(&mut self) -> &mut Document {
        &mut self.docs[self.current_tab]
    }

    fn active_doc(&self) -> &Document {
        &self.docs[self.current_tab]
    }

    fn new_tab(&mut self) {
        self.untitled_count += 1;
        self.docs.push(Document::new_untitled(self.untitled_count));
        self.current_tab = self.docs.len() - 1;
    }

    fn close_tab(&mut self, idx: usize) {
        if self.docs.len() == 1 {
            self.docs[0] = Document::new_untitled(self.untitled_count + 1);
            self.current_tab = 0;
            return;
        }

        self.docs.remove(idx);
        if self.current_tab >= self.docs.len() {
            self.current_tab = self.docs.len() - 1;
        }
    }

    fn close_all(&mut self) {
        self.docs.clear();
        self.untitled_count += 1;
        self.docs.push(Document::new_untitled(self.untitled_count));
        self.current_tab = 0;
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match self.load_document(path) {
                Ok(doc) => {
                    self.docs.push(doc);
                    self.current_tab = self.docs.len() - 1;
                }
                Err(err) => {
                    self.active_doc_mut().diagnostics = format!("Open failed: {err}");
                }
            }
        }
    }

    fn load_document(&self, path: PathBuf) -> anyhow::Result<Document> {
        let content = fs::read_to_string(&path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        Ok(Document {
            title: file_name(&path),
            path: Some(path.clone()),
            content: content.clone(),
            saved_content: content,
            syntax: self
                .syntax_map
                .get(ext)
                .copied()
                .unwrap_or("txt")
                .to_string(),
            last_modified: fs::metadata(path).ok().and_then(|m| m.modified().ok()),
            externally_changed: false,
            diagnostics: String::new(),
        })
    }

    fn save_active(&mut self) {
        let needs_path = self.active_doc().path.is_none();
        if needs_path {
            self.save_active_as();
            return;
        }

        let path = self.active_doc().path.clone().unwrap();
        self.write_document(path);
    }

    fn save_active_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            self.write_document(path);
        }
    }

    fn write_document(&mut self, path: PathBuf) {
        let doc = self.active_doc_mut();
        match fs::write(&path, &doc.content) {
            Ok(_) => {
                doc.path = Some(path.clone());
                doc.title = file_name(&path);
                doc.saved_content = doc.content.clone();
                doc.last_modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
                doc.diagnostics = "Saved.".to_string();
            }
            Err(err) => doc.diagnostics = format!("Save failed: {err}"),
        }
    }

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
        for doc in &mut self.docs {
            doc.detect_external_changes();
        }
    }

    fn run_plugins(&mut self) {
        let (ext, content) = {
            let doc = self.active_doc();
            let ext = doc
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            (ext, doc.content.clone())
        };

        let mut messages = Vec::new();
        for plugin in &self.plugins {
            if plugin.supports_extension(&ext) {
                match plugin.validate(&content) {
                    Ok(_) => messages.push(format!("{}: OK", plugin.name())),
                    Err(err) => messages.push(format!("{}: {}", plugin.name(), err)),
                }
            }
        }

        if !messages.is_empty() {
            self.active_doc_mut().diagnostics = messages.join(" | ");
        }
    }

    fn refresh_matches(&mut self) {
        let haystack = self.active_doc().content.clone();
        let query = self.find_state.query.clone();
        let case_sensitive = self.find_state.case_sensitive;
        let whole_word = self.find_state.whole_word;

        self.find_state.matches.clear();

        if query.is_empty() {
            return;
        }

        let needle = if case_sensitive {
            query.clone()
        } else {
            query.to_lowercase()
        };

        let search_area = if case_sensitive {
            haystack
        } else {
            haystack.to_lowercase()
        };

        let mut idx = 0;
        while let Some(found) = search_area[idx..].find(&needle) {
            let pos = idx + found;
            if !whole_word || is_whole_word_boundary(&search_area, pos, needle.len()) {
                self.find_state.matches.push(pos);
            }
            idx = pos + needle.len();
            if idx >= search_area.len() {
                break;
            }
        }
    }

    fn replace_current(&mut self) {
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
        self.active_doc_mut()
            .content
            .replace_range(at..at + query_len, &replacement);
        self.refresh_matches();
    }

    fn replace_all(&mut self) {
        if self.find_state.query.is_empty() {
            return;
        }

        let query = self.find_state.query.clone();
        let replacement = self.find_state.replacement.clone();
        let doc = self.active_doc_mut();
        doc.content = doc.content.replace(&query, &replacement);
        self.refresh_matches();
    }
}

impl eframe::App for RustNotepadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.scan_external_changes();
        self.refresh_matches();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New Tab").clicked() {
                    self.new_tab();
                }
                if ui.button("Open").clicked() {
                    self.open_file();
                }
                if ui.button("Save").clicked() {
                    self.save_active();
                }
                if ui.button("Save As").clicked() {
                    self.save_active_as();
                }
                if ui.button("Close Tab").clicked() {
                    self.close_tab(self.current_tab);
                }
                if ui.button("Close All").clicked() {
                    self.close_all();
                }
                if ui.button("Find/Replace").clicked() {
                    self.find_state.show_panel = !self.find_state.show_panel;
                }
                if ui.button("Run Extension Plugins").clicked() {
                    self.run_plugins();
                }
            });
        });

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, doc) in self.docs.iter().enumerate() {
                    let mut label = doc.title.clone();
                    if doc.is_dirty() {
                        label.push('*');
                    }
                    if ui.selectable_label(i == self.current_tab, label).clicked() {
                        self.current_tab = i;
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let (externally_changed, diagnostics, syntax) = {
                let doc = self.active_doc();
                (
                    doc.externally_changed,
                    doc.diagnostics.clone(),
                    doc.syntax.clone(),
                )
            };
            ui.horizontal(|ui| {
                if externally_changed {
                    ui.colored_label(egui::Color32::YELLOW, "File changed externally.");
                    if ui.button("Reload").clicked() {
                        let _ = self.active_doc_mut().reload_from_disk();
                    }
                }
                if !diagnostics.is_empty() {
                    ui.label(diagnostics);
                }
                ui.separator();
                ui.label(format!("Syntax: {}", syntax));
            });
        });

        if self.find_state.show_panel {
            egui::SidePanel::right("find_panel")
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Find & Replace");
                    ui.text_edit_singleline(&mut self.find_state.query);
                    ui.label("Replace with");
                    ui.text_edit_singleline(&mut self.find_state.replacement);
                    ui.checkbox(&mut self.find_state.case_sensitive, "Case sensitive");
                    ui.checkbox(&mut self.find_state.whole_word, "Whole word");

                    ui.horizontal(|ui| {
                        if ui.button("Find All").clicked() {
                            self.refresh_matches();
                        }
                        if ui.button("Replace").clicked() {
                            self.replace_current();
                        }
                        if ui.button("Replace All").clicked() {
                            self.replace_all();
                        }
                    });

                    ui.separator();
                    ui.label(format!("Matches: {}", self.find_state.matches.len()));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (idx, pos) in self.find_state.matches.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    idx == self.find_state.selected_match,
                                    format!("Occurrence {} @ byte {}", idx + 1, pos),
                                )
                                .clicked()
                            {
                                self.find_state.selected_match = idx;
                            }
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let syntax = self.active_doc().syntax.clone();
            let theme = self.theme.clone();
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut job: LayoutJob =
                    syntax_highlighting::highlight(ui.ctx(), &theme, string, &syntax);
                job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(job))
            };

            ui.add(
                TextEdit::multiline(&mut self.active_doc_mut().content)
                    .font(FontId::monospace(14.0))
                    .desired_rows(35)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .code_editor(),
            );
        });
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled")
        .to_string()
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
    use super::is_whole_word_boundary;

    #[test]
    fn checks_word_boundaries() {
        let s = "abc def";
        assert!(is_whole_word_boundary(s, 0, 3));
        assert!(is_whole_word_boundary(s, 4, 3));
        assert!(!is_whole_word_boundary("abc1", 0, 3));
    }
}
