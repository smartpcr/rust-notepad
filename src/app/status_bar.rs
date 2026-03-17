use eframe::egui;

use super::RustNotepadApp;

/// Common syntax choices for the language selector.
const SYNTAX_OPTIONS: &[(&str, &str)] = &[
    ("txt", "Plain Text"),
    ("rs", "Rust"),
    ("js", "JavaScript"),
    ("py", "Python"),
    ("java", "Java"),
    ("cs", "C#"),
    ("cpp", "C++"),
    ("c", "C"),
    ("go", "Go"),
    ("html", "HTML"),
    ("xml", "XML"),
    ("json", "JSON"),
    ("yaml", "YAML"),
    ("css", "CSS"),
    ("sql", "SQL"),
    ("sh", "Shell"),
    ("md", "Markdown"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("lua", "Lua"),
    ("scala", "Scala"),
    ("hs", "Haskell"),
    ("pl", "Perl"),
    ("r", "R"),
    ("d", "D"),
    ("bat", "Batch"),
    ("pas", "Pascal"),
    ("erl", "Erlang"),
    ("ml", "OCaml"),
    ("lisp", "Lisp"),
    ("clj", "Clojure"),
    ("groovy", "Groovy"),
    ("tex", "LaTeX"),
    ("tcl", "Tcl"),
    ("m", "Obj-C"),
    ("diff", "Diff"),
];

/// Small status label with dimmed color.
fn status_item(ui: &mut egui::Ui, text: &str, dim: egui::Color32) {
    ui.label(egui::RichText::new(text).size(11.0).color(dim));
}

/// Small status label with custom color.
fn status_colored(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text).size(11.0).color(color));
}

fn status_sep(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);
}

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let doc = app.editor.active_doc();
    let externally_changed = doc.externally_changed;
    let diagnostics = doc.diagnostics.clone();
    let syntax = doc.syntax.clone();
    let lines = doc.line_count();
    let chars = doc.char_count();
    let dirty = doc.is_dirty();
    let eol_label = doc.eol_style.label().to_owned();
    let encoding_label = doc.encoding.label().to_owned();
    let font_size = app.view.font_size;
    let cursor = app.view.cursor;
    let tab_size = app.view.tab_size;
    let accent = app.app_theme.accent();
    let dim = app.app_theme.text_dim();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // External change warning
        if externally_changed {
            status_colored(
                ui,
                "\u{26A0} File changed externally",
                egui::Color32::from_rgb(255, 200, 60),
            );
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Reload").size(11.0))
                        .min_size(egui::vec2(0.0, 16.0)),
                )
                .clicked()
            {
                let _ = app.editor.active_doc_mut().reload_from_disk();
            }
            status_sep(ui);
        }

        // Diagnostics
        if !diagnostics.is_empty() {
            status_item(ui, &diagnostics, dim);
            status_sep(ui);
        }

        // Cursor position
        status_colored(
            ui,
            &format!("Ln {}, Col {}", cursor.line, cursor.col),
            accent,
        );
        status_sep(ui);

        // Selection info
        if cursor.selection_len > 0 {
            status_item(ui, &format!("Sel {}", cursor.selection_len), dim);
            status_sep(ui);
        }

        // Document info
        status_item(ui, &format!("{} lines", lines), dim);
        status_sep(ui);
        status_item(ui, &format!("{} chars", chars), dim);
        status_sep(ui);

        // Syntax selector — clickable dropdown
        let display_name = SYNTAX_OPTIONS
            .iter()
            .find(|(ext, _)| *ext == syntax)
            .map(|(_, name)| *name)
            .unwrap_or(&syntax);

        egui::ComboBox::from_id_source("syntax_selector")
            .selected_text(
                egui::RichText::new(display_name)
                    .size(11.0)
                    .color(accent),
            )
            .width(120.0)
            .show_ui(ui, |ui: &mut egui::Ui| {
                for &(ext, name) in SYNTAX_OPTIONS {
                    let label = format!("{} ({})", name, ext);
                    if ui
                        .selectable_label(syntax == ext, egui::RichText::new(label).size(11.0))
                        .clicked()
                    {
                        app.editor.active_doc_mut().syntax = ext.to_string();
                        // Reset fold state since fold strategy depends on syntax
                        app.editor.active_doc_mut().fold_state = Default::default();
                    }
                }
            });
        status_sep(ui);

        // Encoding
        status_item(ui, &encoding_label, dim);
        status_sep(ui);

        // EOL
        status_item(ui, &eol_label, dim);
        status_sep(ui);

        // Tab size
        status_item(ui, &format!("Tab: {}", tab_size), dim);
        status_sep(ui);

        // Modified state
        if dirty {
            status_colored(
                ui,
                "\u{25CF} Modified",
                egui::Color32::from_rgb(255, 170, 60),
            );
        } else {
            status_item(ui, "\u{2713} Saved", dim);
        }

        // Right-aligned section
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            status_item(ui, &format!("{}%", app.view.ui_zoom_pct), dim);
            status_sep(ui);
            status_item(ui, &format!("{:.0}pt", font_size), dim);
            status_sep(ui);
            status_item(ui, app.app_theme.label(), dim);
        });
    });
}
