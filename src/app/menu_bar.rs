use eframe::egui;
use rust_notepad::shortcuts::{menu_item, Shortcuts};

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::menu::bar(ui, |ui| {
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
            ui.close_menu();
        }
        if menu_item(ui, "Open...", &Shortcuts::open()) {
            app.open_file();
            ui.close_menu();
        }
        ui.separator();
        if menu_item(ui, "Save", &Shortcuts::save()) {
            app.save_active();
            ui.close_menu();
        }
        if menu_item(ui, "Save As...", &Shortcuts::save_as()) {
            app.save_active_as();
            ui.close_menu();
        }
        if ui.button("Save All").clicked() {
            app.editor.save_all();
            ui.close_menu();
        }
        ui.separator();
        if menu_item(ui, "Close", &Shortcuts::close_tab()) {
            let idx = app.editor.current_tab;
            app.request_close_tab(idx);
            ui.close_menu();
        }
        if ui.button("Close All").clicked() {
            app.request_close_all();
            ui.close_menu();
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
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Convert to UPPERCASE").clicked() {
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.to_uppercase();
            ui.close_menu();
        }
        if ui.button("Convert to lowercase").clicked() {
            let doc = app.editor.active_doc_mut();
            doc.content = doc.content.to_lowercase();
            ui.close_menu();
        }
    });
}

fn render_search_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Search", |ui| {
        if menu_item(ui, "Find...", &Shortcuts::find()) {
            app.find_state.show_panel = true;
            ui.close_menu();
        }
        if menu_item(ui, "Replace...", &Shortcuts::replace()) {
            app.find_state.show_panel = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Find Next          F3").clicked() {
            app.find_state.find_next();
            ui.close_menu();
        }
        if ui.button("Find Previous  Shift+F3").clicked() {
            app.find_state.find_prev();
            ui.close_menu();
        }
        ui.separator();
        if menu_item(ui, "Go to Line...", &Shortcuts::go_to_line()) {
            app.go_to_line.open = true;
            app.go_to_line.input.clear();
            ui.close_menu();
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
            ui.close_menu();
        }
        if ui.button("Unfold All").clicked() {
            app.editor.active_doc_mut().fold_state.unfold_all();
            ui.close_menu();
        }
    });
}

fn render_settings_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.menu_button("Settings", |ui| {
        let current_label = format!("Theme: {}", app.app_theme.label());
        if ui.button(current_label).clicked() {
            app.app_theme = app.app_theme.toggle();
            app.app_theme.apply(ctx);
            ui.close_menu();
        }
    });
}

fn render_tools_menu(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.menu_button("Tools", |ui| {
        if ui.button("Run Validation Plugins").clicked() {
            app.run_plugins();
            ui.close_menu();
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
                ui.close_menu();
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
