use eframe::egui;

use super::RustNotepadApp;

pub fn render_close_confirm(app: &mut RustNotepadApp, ctx: &egui::Context) {
    if !app.close_confirm.open {
        return;
    }

    // Global keyboard handling for this dialog
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    if escape {
        app.close_confirm.pending_tabs.clear();
        app.close_confirm.open = false;
        return;
    }
    if enter {
        app.confirm_save_and_close();
        return;
    }

    let accent = app.app_theme.accent();

    egui::Window::new("Unsaved Changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(350.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            let dirty_names: Vec<String> = app
                .close_confirm
                .pending_tabs
                .iter()
                .filter_map(|&idx| {
                    if idx < app.editor.docs.len() && app.editor.docs[idx].is_dirty() {
                        Some(app.editor.docs[idx].title.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if dirty_names.is_empty() {
                app.confirm_discard_and_close();
                return;
            }

            let count = dirty_names.len();
            ui.label(
                egui::RichText::new(format!(
                    "{} file{} {} unsaved changes:",
                    count,
                    if count == 1 { "" } else { "s" },
                    if count == 1 { "has" } else { "have" },
                ))
                .size(13.0),
            );

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for name in &dirty_names {
                        ui.label(
                            egui::RichText::new(format!("  \u{25CF} {}", name))
                                .size(12.0)
                                .color(accent),
                        );
                    }
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("  Save All & Close  ").size(12.0))
                    .clicked()
                {
                    app.confirm_save_and_close();
                }
                if ui
                    .button(egui::RichText::new("  Discard All  ").size(12.0))
                    .clicked()
                {
                    app.confirm_discard_and_close();
                }
                if ui
                    .button(egui::RichText::new("  Cancel  ").size(12.0))
                    .clicked()
                {
                    app.close_confirm.pending_tabs.clear();
                    app.close_confirm.open = false;
                }
            });

            ui.add_space(4.0);
        });
}

pub fn render_go_to_line(app: &mut RustNotepadApp, ctx: &egui::Context) {
    if !app.go_to_line.open {
        return;
    }

    // Escape closes the dialog from anywhere
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.go_to_line.open = false;
        return;
    }

    egui::Window::new("Go to Line")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(250.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            let total = app.editor.active_doc().line_count();
            ui.label(
                egui::RichText::new(format!("Enter line number (1 \u{2013} {}):", total))
                    .size(12.0),
            );
            ui.add_space(4.0);

            let response = ui.add(
                egui::TextEdit::singleline(&mut app.go_to_line.input)
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::monospace(14.0))
                    .hint_text("Line number..."),
            );
            response.request_focus();

            // Enter in the text field = submit (lost_focus + Enter is the egui pattern)
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let go_clicked = ui
                    .button(egui::RichText::new("  Go  ").size(12.0))
                    .clicked();

                let cancel_clicked = ui
                    .button(egui::RichText::new("Cancel").size(12.0))
                    .clicked();

                if go_clicked || submitted {
                    if let Ok(target_line) = app.go_to_line.input.trim().parse::<usize>() {
                        let content = &app.editor.active_doc().content;
                        let line = target_line.max(1);
                        let mut offset = 0;
                        let mut current_line = 1;
                        for ch in content.chars() {
                            if current_line >= line {
                                break;
                            }
                            if ch == '\n' {
                                current_line += 1;
                            }
                            offset += ch.len_utf8();
                        }
                        offset = offset.min(content.len());
                        app.find_state.navigate_to = Some((offset, offset));
                        app.find_state.focus_editor = true;
                    }
                    app.go_to_line.open = false;
                }
                if cancel_clicked {
                    app.go_to_line.open = false;
                }
            });

            ui.add_space(4.0);
        });
}
