use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let accent = app.app_theme.accent();
    let dim = app.app_theme.text_dim();

    ui.add_space(4.0);
    ui.label(egui::RichText::new("Find & Replace").size(14.0).strong());
    ui.add_space(4.0);

    ui.label(egui::RichText::new("Find:").size(11.0).color(dim));
    let find_response = ui.add(
        egui::TextEdit::singleline(&mut app.find_state.query)
            .desired_width(f32::INFINITY)
            .font(egui::FontId::monospace(12.0)),
    );
    if find_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.refresh_matches();
        if !app.find_state.matches.is_empty() {
            app.find_state.select_match(0);
        }
    }

    ui.add_space(2.0);
    ui.label(egui::RichText::new("Replace:").size(11.0).color(dim));
    ui.add(
        egui::TextEdit::singleline(&mut app.find_state.replacement)
            .desired_width(f32::INFINITY)
            .font(egui::FontId::monospace(12.0)),
    );

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.find_state.case_sensitive, "Aa");
        ui.checkbox(&mut app.find_state.whole_word, "W");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.find_state.use_regex, "Regex");
        ui.checkbox(&mut app.find_state.extended_mode, "\\n \\t");
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Find All").clicked() {
            app.refresh_matches();
        }
        if ui.button("Replace").clicked() {
            app.replace_current();
        }
        if ui.button("Replace All").clicked() {
            app.replace_all();
        }
    });

    ui.horizontal(|ui| {
        if ui.button("\u{25C0} Prev").clicked() {
            app.find_state.find_prev();
        }
        if ui.button("Next \u{25B6}").clicked() {
            app.find_state.find_next();
        }
    });

    ui.add_space(4.0);
    let count = app.find_state.matches.len();
    let selected = app.find_state.selected_match;
    if count > 0 {
        ui.label(
            egui::RichText::new(format!("{} matches (#{}/{})", count, selected + 1, count))
                .size(11.0)
                .color(accent),
        );
    } else {
        ui.label(egui::RichText::new("No matches").size(11.0).color(dim));
    }

    // Build result display strings with line number and context
    let content = &app.editor.active_doc().content;
    let query_len = app.find_state.query.len();
    let result_labels: Vec<String> = app
        .find_state
        .matches
        .iter()
        .enumerate()
        .map(|(idx, &pos)| {
            let text_before = &content[..pos.min(content.len())];
            let line = text_before.chars().filter(|&c| c == '\n').count() + 1;
            let col = match text_before.rfind('\n') {
                Some(p) => pos - p,
                None => pos + 1,
            };
            let ctx_start = pos.saturating_sub(10);
            let ctx_end = (pos + query_len + 20).min(content.len());
            let snippet = &content[ctx_start..ctx_end];
            let snippet_clean: String = snippet
                .chars()
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect();
            format!("#{} Ln {}, Col {}: {}", idx + 1, line, col, snippet_clean)
        })
        .collect();

    let mut clicked_idx: Option<usize> = None;
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for (idx, label) in result_labels.iter().enumerate() {
                let is_selected = idx == app.find_state.selected_match;
                let text = egui::RichText::new(label)
                    .size(11.0)
                    .color(if is_selected { accent } else { dim })
                    .family(egui::FontFamily::Monospace);
                if ui.selectable_label(is_selected, text).clicked() {
                    clicked_idx = Some(idx);
                }
            }
        });
    if let Some(idx) = clicked_idx {
        app.find_state.select_match(idx);
    }

    ui.add_space(4.0);
    ui.separator();

    // Find in Files
    if ui.button("Find in Files...").clicked() {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let query = app.find_state.query.clone();
            if !query.is_empty() {
                let results = rust_notepad::extensibility::project_search(&folder, &query);
                if let Ok(hits) = results {
                    app.find_state.file_results = hits
                        .iter()
                        .map(|h| (h.file.display().to_string(), h.line, h.text.clone()))
                        .collect();
                    app.find_state.show_find_in_files = true;
                }
            }
        }
    }

    if app.find_state.show_find_in_files && !app.find_state.file_results.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("Files: {} hits", app.find_state.file_results.len()))
                .size(11.0)
                .color(accent),
        );
        let mut open_path: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_source("find_in_files_results")
            .max_height(200.0)
            .show(ui, |ui| {
                for (path, line, content) in &app.find_state.file_results {
                    let short_path = if path.len() > 30 {
                        format!("...{}", &path[path.len() - 27..])
                    } else {
                        path.clone()
                    };
                    let label = format!("{}:{} {}", short_path, line, content.trim());
                    let text = egui::RichText::new(&label)
                        .size(10.0)
                        .color(dim)
                        .family(egui::FontFamily::Monospace);
                    if ui.selectable_label(false, text).clicked() {
                        open_path = Some(path.clone());
                    }
                }
            });
        if let Some(path) = open_path {
            let pb = std::path::PathBuf::from(&path);
            if pb.exists() {
                let _ = app.editor.open_document(pb);
            }
        }
    }

    ui.add_space(8.0);
    if ui.button("Close").clicked() {
        app.find_state.show_panel = false;
        app.find_state.show_find_in_files = false;
    }
}
