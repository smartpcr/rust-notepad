use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    let accent = app.app_theme.accent();
    let dim = app.app_theme.text_dim();
    let active_bg = app.app_theme.tab_active_bg();
    let active_text = app.app_theme.tab_active_text();

    // Collect tab display data first to avoid borrow conflicts.
    let tab_info: Vec<(String, bool, Option<String>)> = app
        .editor
        .docs
        .iter()
        .map(|doc| {
            let dirty = doc.is_dirty();
            let label = doc.title.clone();
            let path_str = doc.path.as_ref().map(|p| p.display().to_string());
            (label, dirty, path_str)
        })
        .collect();

    #[derive(Clone)]
    enum TabAction {
        Select(usize),
        Close(usize),
        CloseOthers(usize),
        CloseAll,
        CopyPath(String),
    }

    let mut actions: Vec<TabAction> = Vec::new();
    let tab_wrap = app.view.tab_wrap;

    // Render a single tab widget. Returns the combined response.
    // Uses a flat layout (no inner ui.horizontal) so horizontal_wrapped can wrap between tabs.
    let render_one_tab = |ui: &mut egui::Ui,
                          i: usize,
                          label: &str,
                          dirty: bool,
                          path_str: &Option<String>,
                          actions: &mut Vec<TabAction>| {
        let is_active = i == app.editor.current_tab;

        let tab_text = if dirty {
            format!(" {} \u{25CF} ", label)
        } else {
            format!(" {} ", label)
        };

        let text_color = if is_active { active_text } else { dim };
        let bg = if is_active {
            active_bg
        } else {
            egui::Color32::TRANSPARENT
        };
        let stroke = if is_active {
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 80),
            )
        } else {
            egui::Stroke::NONE
        };

        // Combine tab text + close into one selectable area
        let combined_text = format!("{} \u{00D7}", tab_text.trim());
        let btn = egui::Button::new(
            egui::RichText::new(&combined_text)
                .size(12.0)
                .color(text_color),
        )
        .fill(bg)
        .stroke(stroke)
        .rounding(egui::CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        });
        let response = ui.add(btn);

        if response.clicked() {
            // Check if click was on the right side (close button area)
            if let Some(pos) = response.interact_pointer_pos() {
                let close_zone_x = response.rect.right() - 20.0;
                if pos.x >= close_zone_x {
                    actions.push(TabAction::Close(i));
                } else {
                    actions.push(TabAction::Select(i));
                }
            }
        }

        // Right-click context menu
        response.context_menu(|ui: &mut egui::Ui| {
            if ui.button("Close").clicked() {
                actions.push(TabAction::Close(i));
                ui.close_menu();
            }
            if ui.button("Close Others").clicked() {
                actions.push(TabAction::CloseOthers(i));
                ui.close_menu();
            }
            if ui.button("Close All").clicked() {
                actions.push(TabAction::CloseAll);
                ui.close_menu();
            }
            ui.separator();
            if let Some(path) = path_str {
                if ui.button(format!("Copy Path: {}", path)).clicked() {
                    actions.push(TabAction::CopyPath(path.clone()));
                    ui.close_menu();
                }
            }
        });
    };

    // Wrap or scroll based on setting
    if tab_wrap {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
            for (i, (label, dirty, path_str)) in tab_info.iter().enumerate() {
                render_one_tab(ui, i, label, *dirty, path_str, &mut actions);
            }
        });
    } else {
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    for (i, (label, dirty, path_str)) in tab_info.iter().enumerate() {
                        render_one_tab(ui, i, label, *dirty, path_str, &mut actions);
                    }
                });
            });
    }

    // Bottom accent line
    let rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(
            0.5,
            egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 40),
        ),
    );

    // Execute deferred actions.
    for action in actions {
        match action {
            TabAction::Select(i) => app.editor.current_tab = i,
            TabAction::Close(i) => app.request_close_tab(i),
            TabAction::CloseOthers(i) => {
                if i < app.editor.docs.len() {
                    app.request_close_others(i);
                }
            }
            TabAction::CloseAll => app.request_close_all(),
            TabAction::CopyPath(path) => {
                ui.ctx().copy_text(path);
            }
        }
    }
}
