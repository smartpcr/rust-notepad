use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let doc = app.editor.active_doc();
    let externally_changed = doc.externally_changed;
    let diagnostics = doc.diagnostics.clone();
    let syntax = doc.syntax.clone();
    let lines = doc.line_count();
    let chars = doc.char_count();
    let dirty = doc.is_dirty();
    let theme_label = app.app_theme.label();
    let font_size = app.view.font_size;

    ui.horizontal(|ui| {
        // External change warning
        if externally_changed {
            ui.colored_label(egui::Color32::YELLOW, "File changed externally!");
            if ui.button("Reload").clicked() {
                let _ = app.editor.active_doc_mut().reload_from_disk();
            }
            ui.separator();
        }

        // Diagnostics
        if !diagnostics.is_empty() {
            ui.label(&diagnostics);
            ui.separator();
        }

        // Document info
        ui.label(format!("Lines: {}", lines));
        ui.separator();
        ui.label(format!("Chars: {}", chars));
        ui.separator();
        ui.label(format!("Syntax: {}", syntax));
        ui.separator();
        ui.label(format!("Theme: {}", theme_label));
        ui.separator();
        if dirty {
            ui.label("Modified");
        } else {
            ui.label("Saved");
        }
        ui.separator();
        ui.label(format!("Font: {:.0}pt", font_size));
    });
}
