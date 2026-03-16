use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.heading("Find & Replace");
    ui.separator();

    ui.label("Find:");
    ui.text_edit_singleline(&mut app.find_state.query);

    ui.label("Replace:");
    ui.text_edit_singleline(&mut app.find_state.replacement);

    ui.horizontal(|ui| {
        ui.checkbox(&mut app.find_state.case_sensitive, "Case sensitive");
        ui.checkbox(&mut app.find_state.whole_word, "Whole word");
    });

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
        if ui.button("<< Prev").clicked() {
            app.find_state.find_prev();
        }
        if ui.button("Next >>").clicked() {
            app.find_state.find_next();
        }
    });

    ui.separator();
    let count = app.find_state.matches.len();
    let selected = app.find_state.selected_match;
    ui.label(format!(
        "Matches: {} {}",
        count,
        if count == 0 {
            String::new()
        } else {
            format!("(#{}/{})", selected + 1, count)
        }
    ));

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            for (idx, pos) in app.find_state.matches.iter().enumerate() {
                if ui
                    .selectable_label(
                        idx == app.find_state.selected_match,
                        format!("#{} @ byte {}", idx + 1, pos),
                    )
                    .clicked()
                {
                    app.find_state.selected_match = idx;
                }
            }
        });

    ui.separator();
    if ui.button("Close").clicked() {
        app.find_state.show_panel = false;
    }
}
