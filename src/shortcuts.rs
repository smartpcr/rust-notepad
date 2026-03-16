use eframe::egui;

/// Notepad++ style keyboard shortcuts.
pub struct Shortcuts;

impl Shortcuts {
    // File
    pub fn new_tab() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::N)
    }
    pub fn open() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::O)
    }
    pub fn save() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S)
    }
    pub fn save_as() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            egui::Key::S,
        )
    }
    pub fn close_tab() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::W)
    }

    // Search
    pub fn find() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::F)
    }
    pub fn replace() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::H)
    }
    pub fn go_to_line() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::G)
    }

    // View
    pub fn toggle_word_wrap() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::Z)
    }

    // Tab navigation
    pub fn next_tab() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Tab)
    }
    pub fn prev_tab() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            egui::Key::Tab,
        )
    }

    // Zoom
    pub fn zoom_in() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Equals)
    }
    pub fn zoom_out() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Minus)
    }

    // Find next/prev
    pub fn find_next() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F3)
    }
    pub fn find_prev() -> egui::KeyboardShortcut {
        egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::F3)
    }
}

/// Format a shortcut for display in menus (e.g. "Ctrl+S").
pub fn shortcut_text(s: &egui::KeyboardShortcut) -> String {
    let mut parts = Vec::new();
    if s.modifiers.ctrl {
        parts.push("Ctrl");
    }
    if s.modifiers.shift {
        parts.push("Shift");
    }
    if s.modifiers.alt {
        parts.push("Alt");
    }
    parts.push(key_name(s.logical_key));
    parts.join("+")
}

fn key_name(key: egui::Key) -> &'static str {
    match key {
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::S => "S",
        egui::Key::W => "W",
        egui::Key::F => "F",
        egui::Key::H => "H",
        egui::Key::G => "G",
        egui::Key::Z => "Z",
        egui::Key::Tab => "Tab",
        egui::Key::F3 => "F3",
        egui::Key::Equals => "+",
        egui::Key::Minus => "-",
        _ => "?",
    }
}

/// Render a menu item with label left-aligned and shortcut right-aligned.
pub fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &egui::KeyboardShortcut) -> bool {
    let shortcut_str = shortcut_text(shortcut);
    let min_width = 220.0;

    let response = ui.horizontal(|ui| {
        ui.set_min_width(min_width);
        let r = ui.button(label);
        // Push shortcut text to the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(&shortcut_str);
        });
        r
    });
    response.inner.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_text_ctrl_s() {
        let text = shortcut_text(&Shortcuts::save());
        assert!(text.contains("Ctrl"));
        assert!(text.contains("S"));
    }

    #[test]
    fn shortcut_text_ctrl_shift_s() {
        let text = shortcut_text(&Shortcuts::save_as());
        assert!(text.contains("Ctrl"));
        assert!(text.contains("Shift"));
        assert!(text.contains("S"));
    }

    #[test]
    fn shortcut_text_alt_z() {
        let text = shortcut_text(&Shortcuts::toggle_word_wrap());
        assert!(text.contains("Alt"));
        assert!(text.contains("Z"));
    }
}
