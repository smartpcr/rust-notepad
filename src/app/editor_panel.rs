use eframe::egui::{self, text::LayoutJob, FontId, TextEdit};
use egui_extras::syntax_highlighting;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let syntax = app.editor.active_doc().syntax.clone();
    let theme = app.app_theme.code_theme();
    let font_size = app.view.font_size;
    let word_wrap = app.view.word_wrap;

    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
        let mut job: LayoutJob =
            syntax_highlighting::highlight(ui.ctx(), &theme, string, &syntax);
        job.wrap.max_width = if word_wrap { wrap_width } else { f32::INFINITY };
        ui.fonts(|f| f.layout_job(job))
    };

    // Editor fills all available space — each tab's content is full-size.
    let available = ui.available_size();
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_sized(
                available,
                TextEdit::multiline(&mut app.editor.active_doc_mut().content)
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .code_editor(),
            );
        });
}
