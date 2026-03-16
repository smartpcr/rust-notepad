use eframe::egui;

use super::RustNotepadApp;

pub fn render_go_to_line(app: &mut RustNotepadApp, ctx: &egui::Context) {
    if !app.go_to_line.open {
        return;
    }

    egui::Window::new("Go to Line")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Line number:");
                let response = ui.text_edit_singleline(&mut app.go_to_line.input);
                response.request_focus();
            });
            ui.horizontal(|ui| {
                if ui.button("Go").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // TODO: scroll editor to line once custom CodeEditor is implemented
                    if let Ok(_line) = app.go_to_line.input.trim().parse::<usize>() {
                        // Will navigate to line in future
                    }
                    app.go_to_line.open = false;
                }
                if ui.button("Cancel").clicked() {
                    app.go_to_line.open = false;
                }
            });
        });
}
