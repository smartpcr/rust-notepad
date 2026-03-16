use eframe::egui;

use super::RustNotepadApp;

pub fn render(app: &mut RustNotepadApp, ui: &mut egui::Ui) {
    // Collect tab display data first to avoid borrow conflicts.
    let tab_info: Vec<(String, Option<String>)> = app
        .editor
        .docs
        .iter()
        .map(|doc| {
            let label = if doc.is_dirty() {
                format!(" {} \u{25CF}", doc.title) // ● dot for modified
            } else {
                format!(" {} ", doc.title)
            };
            let path_str = doc.path.as_ref().map(|p| p.display().to_string());
            (label, path_str)
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

    ui.horizontal_wrapped(|ui| {
        for (i, (label, path_str)) in tab_info.iter().enumerate() {
            let response = ui.selectable_label(i == app.editor.current_tab, label);
            if response.clicked() {
                actions.push(TabAction::Select(i));
            }
            response.context_menu(|ui| {
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
        }
    });

    // Execute deferred actions.
    for action in actions {
        match action {
            TabAction::Select(i) => app.editor.current_tab = i,
            TabAction::Close(i) => app.editor.close_tab(i),
            TabAction::CloseOthers(i) => {
                if i < app.editor.docs.len() {
                    app.editor.current_tab = i;
                    app.editor.close_others();
                }
            }
            TabAction::CloseAll => app.editor.close_all(),
            TabAction::CopyPath(path) => {
                ui.ctx().copy_text(path);
            }
        }
    }
}
