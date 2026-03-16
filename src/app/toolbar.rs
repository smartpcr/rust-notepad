use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(6.0, 4.0);

        if ui.button("New").on_hover_text("New Tab (Ctrl+N)").clicked() {
            app.editor.new_tab();
        }
        if ui
            .button("Open")
            .on_hover_text("Open File (Ctrl+O)")
            .clicked()
        {
            app.open_file();
        }
        if ui.button("Save").on_hover_text("Save (Ctrl+S)").clicked() {
            app.save_active();
        }
        if ui.button("Save All").on_hover_text("Save All").clicked() {
            app.editor.save_all();
        }
        if ui
            .button("Close")
            .on_hover_text("Close Tab (Ctrl+W)")
            .clicked()
        {
            let idx = app.editor.current_tab;
            app.editor.close_tab(idx);
        }

        ui.separator();

        if ui
            .button("Find")
            .on_hover_text("Find (Ctrl+F)")
            .clicked()
        {
            app.find_state.show_panel = !app.find_state.show_panel;
        }

        ui.separator();

        if ui
            .button("Validate")
            .on_hover_text("Run Plugins")
            .clicked()
        {
            app.run_plugins();
        }
    });
}
