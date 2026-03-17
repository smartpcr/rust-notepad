use eframe::egui;
use rust_notepad::core::EolStyle;
use rust_notepad::shortcuts::{menu_item, Shortcuts};

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::MenuBar::new().ui(ui, |ui| {
        render_file_menu(app, ui);
        render_edit_menu(app, ui);
        render_search_menu(app, ui);
        render_view_menu(app, ui);
        render_settings_menu(app, ui, ctx);
        render_tools_menu(app, ui);
        render_window_menu(app, ui);
        render_help_menu(ui);
    });
}

fn render_file_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        if menu_item(ui, "New", &Shortcuts::new_tab()) {
            app.editor.new_tab();
            ui.close();
        }
        if menu_item(ui, "Open...", &Shortcuts::open()) {
            app.open_file();
            ui.close();
        }
        ui.separator();
        if menu_item(ui, "Save", &Shortcuts::save()) {
            app.save_active();
            ui.close();
        }
        if menu_item(ui, "Save As...", &Shortcuts::save_as()) {
            app.save_active_as();
            ui.close();
        }
        if ui.button("Save All").clicked() {
            app.editor.save_all();
            ui.close();
        }
        ui.separator();
        // Recent Files
        let recent = app.recent_files.clone();
        if !recent.is_empty() {
            ui.menu_button("Recent Files", |ui| {
                for path_str in &recent {
                    let display = if path_str.len() > 50 {
                        format!("...{}", &path_str[path_str.len() - 47..])
                    } else {
                        path_str.clone()
                    };
                    if ui.button(&display).clicked() {
                        let path = std::path::PathBuf::from(path_str);
                        if path.exists() {
                            if let Err(e) = app.editor.open_document(path) {
                                app.editor.active_doc_mut().diagnostics =
                                    format!("Open failed: {:?}", e);
                            }
                        }
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Clear Recent Files").clicked() {
                    app.recent_files.clear();
                    ui.close();
                }
            });
            ui.separator();
        }
        if menu_item(ui, "Close", &Shortcuts::close_tab()) {
            let idx = app.editor.current_tab;
            app.request_close_tab(idx);
            ui.close();
        }
        if ui.button("Close All").clicked() {
            app.request_close_all();
            ui.close();
        }
    });
}

fn render_edit_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Edit", |ui| {
        if ui.button("Trim Trailing Whitespace").clicked() {
            let doc = app.editor.active_doc_mut();
            doc.content = doc
                .content
                .lines()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
            ui.close();
        }
        ui.separator();
        if ui.button("Convert to UPPERCASE").clicked() {
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.to_uppercase();
            ui.close();
        }
        if ui.button("Convert to lowercase").clicked() {
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.to_lowercase();
            ui.close();
        }
        ui.separator();

        // EOL conversion
        ui.menu_button("EOL Conversion", |ui| {
            let current = app.editor.active_doc().eol_style;
            for eol in &[EolStyle::LF, EolStyle::CRLF, EolStyle::CR] {
                let label = format!(
                    "{} {}",
                    if current == *eol { "\u{2713}" } else { "  " },
                    eol.label()
                );
                if ui.button(label).clicked() {
                    app.editor.active_doc_mut().eol_style = *eol;
                    ui.close();
                }
            }
        });

        ui.separator();

        // Tab-to-spaces / spaces-to-tabs
        let tab_size = app.view.tab_size as usize;
        if ui.button("Convert Tabs to Spaces").clicked() {
            let spaces: String = " ".repeat(tab_size);
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.replace('\t', &spaces);
            ui.close();
        }
        if ui.button("Convert Spaces to Tabs").clicked() {
            let spaces: String = " ".repeat(tab_size);
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.replace(&spaces, "\t");
            ui.close();
        }
    });
}

fn render_search_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Search", |ui| {
        if menu_item(ui, "Find...", &Shortcuts::find()) {
            app.find_state.show_panel = true;
            ui.close();
        }
        if menu_item(ui, "Replace...", &Shortcuts::replace()) {
            app.find_state.show_panel = true;
            ui.close();
        }
        ui.separator();
        if ui.button("Find Next          F3").clicked() {
            app.find_state.find_next();
            ui.close();
        }
        if ui.button("Find Previous  Shift+F3").clicked() {
            app.find_state.find_prev();
            ui.close();
        }
        ui.separator();
        if menu_item(ui, "Go to Line...", &Shortcuts::go_to_line()) {
            app.go_to_line.open = true;
            app.go_to_line.input.clear();
            ui.close();
        }
    });
}

fn render_view_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("View", |ui| {
        ui.checkbox(&mut app.view.show_toolbar, "Toolbar");
        ui.checkbox(&mut app.view.show_status_bar, "Status Bar");
        ui.checkbox(&mut app.view.show_line_numbers, "Line Numbers");
        ui.checkbox(&mut app.view.word_wrap, "Word Wrap          Alt+Z");
        ui.checkbox(&mut app.view.show_whitespace, "Show Whitespace");
        ui.checkbox(&mut app.view.tab_wrap, "Wrap Tabs (multi-line)");
        ui.checkbox(&mut app.view.auto_indent, "Auto Indent");
        ui.separator();

        // Tab size
        ui.menu_button(format!("Tab Size: {}", app.view.tab_size), |ui| {
            for size in &[2u8, 4, 8] {
                let label = format!(
                    "{} {}",
                    if app.view.tab_size == *size {
                        "\u{2713}"
                    } else {
                        "  "
                    },
                    size
                );
                if ui.button(label).clicked() {
                    app.view.tab_size = *size;
                    ui.close();
                }
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Editor Font:");
            if ui.button("-").clicked() {
                app.view.zoom_out();
            }
            ui.label(format!("{:.0}pt", app.view.font_size));
            if ui.button("+").clicked() {
                app.view.zoom_in();
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("UI Zoom:");
            if ui.button("-").clicked() {
                app.view.ui_zoom_out();
            }
            ui.label(format!("{}%", app.view.ui_zoom_pct));
            if ui.button("+").clicked() {
                app.view.ui_zoom_in();
            }
            if ui.button("Reset").clicked() {
                app.view.ui_zoom_reset();
            }
        });
        ui.separator();
        if ui.button("Fold All").clicked() {
            app.editor.active_doc_mut().fold_state.fold_all();
            ui.close();
        }
        if ui.button("Unfold All").clicked() {
            app.editor.active_doc_mut().fold_state.unfold_all();
            ui.close();
        }
        // Fold levels
        ui.menu_button("Fold Level", |ui| {
            for level in 1..=8 {
                if ui.button(format!("Level {}", level)).clicked() {
                    app.editor.active_doc_mut().fold_state.fold_level(level);
                    ui.close();
                }
            }
        });
    });
}

fn render_settings_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.menu_button("Settings", |ui| {
        let current_label = format!("Theme: {}", app.app_theme.label());
        if ui.button(current_label).clicked() {
            app.app_theme = app.app_theme.toggle();
            app.app_theme.apply(ctx);
            ui.close();
        }
    });
}

fn render_tools_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Tools", |ui| {
        if ui.button("Run Validation Plugins").clicked() {
            app.run_plugins();
            ui.close();
        }
    });
}

fn render_window_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Window", |ui| {
        ui.label("Open documents:");
        ui.separator();
        let tab_count = app.editor.docs.len();
        let current = app.editor.current_tab;
        for i in 0..tab_count {
            let title = &app.editor.docs[i].title;
            let dirty = app.editor.docs[i].is_dirty();
            let label = if dirty {
                format!("{} *", title)
            } else {
                title.clone()
            };
            if ui.selectable_label(i == current, label).clicked() {
                app.editor.current_tab = i;
                ui.close();
            }
        }
    });
}

fn render_help_menu(ui: &mut egui::Ui) {
    ui.menu_button("Help", |ui| {
        ui.label("TextEdit v0.1.0");
        ui.label("A Notepad++ clone in Rust");
        ui.separator();
        ui.label("Keyboard shortcuts:");
        ui.label("  Ctrl+N       New tab");
        ui.label("  Ctrl+O       Open file");
        ui.label("  Ctrl+S       Save");
        ui.label("  Ctrl+Shift+S Save As");
        ui.label("  Ctrl+W       Close tab");
        ui.label("  Ctrl+F       Find");
        ui.label("  Ctrl+H       Replace");
        ui.label("  Ctrl+G       Go to Line");
        ui.label("  F3           Find Next");
        ui.label("  Shift+F3     Find Previous");
        ui.label("  Ctrl+Tab     Next tab");
        ui.label("  Ctrl++/-     Zoom in/out");
        ui.label("  Alt+Z        Toggle word wrap");
    });
}
