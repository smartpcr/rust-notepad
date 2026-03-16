use eframe::egui;

use super::RustNotepadApp;

/// Styled toolbar button — compact, with icon and optional label.
fn tool_btn(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(icon).size(15.0),
    )
    .min_size(egui::vec2(28.0, 24.0));
    ui.add(btn).on_hover_text(tooltip).clicked()
}

/// Toolbar button with icon + text label.
fn tool_btn_label(ui: &mut egui::Ui, icon: &str, label: &str, tooltip: &str) -> bool {
    let text = format!("{} {}", icon, label);
    let btn = egui::Button::new(
        egui::RichText::new(text).size(12.0),
    )
    .min_size(egui::vec2(0.0, 24.0));
    ui.add(btn).on_hover_text(tooltip).clicked()
}

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;

        if tool_btn(ui, "\u{2795}", "New Tab (Ctrl+N)") {
            app.editor.new_tab();
        }
        if tool_btn(ui, "\u{1F4C2}", "Open File (Ctrl+O)") {
            app.open_file();
        }
        if tool_btn(ui, "\u{1F4BE}", "Save (Ctrl+S)") {
            app.save_active();
        }
        if tool_btn(ui, "\u{1F4CB}", "Save All") {
            app.editor.save_all();
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if tool_btn(ui, "\u{1F50D}", "Find (Ctrl+F)") {
            app.find_state.show_panel = !app.find_state.show_panel;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Zoom controls — compact
        if tool_btn(ui, "\u{2796}", "Zoom Out") {
            app.view.zoom_out();
        }
        ui.label(
            egui::RichText::new(format!("{:.0}pt", app.view.font_size))
                .size(11.0)
                .color(app.app_theme.text_dim()),
        );
        if tool_btn(ui, "\u{2795}", "Zoom In") {
            app.view.zoom_in();
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if tool_btn_label(ui, "\u{2705}", "Validate", "Run Validation Plugins") {
            app.run_plugins();
        }
    });
}
