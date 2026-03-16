/// State for the Find & Replace panel.
#[derive(Default)]
pub struct FindState {
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub matches: Vec<usize>,
    pub selected_match: usize,
    pub show_panel: bool,
}

impl FindState {
    pub fn find_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected_match = (self.selected_match + 1) % self.matches.len();
        }
    }

    pub fn find_prev(&mut self) {
        if !self.matches.is_empty() {
            if self.selected_match == 0 {
                self.selected_match = self.matches.len() - 1;
            } else {
                self.selected_match -= 1;
            }
        }
    }
}

/// State for the Go to Line dialog.
#[derive(Default)]
pub struct GoToLineState {
    pub open: bool,
    pub input: String,
}

/// View-related settings (togglable panels, display options).
#[derive(Clone)]
pub struct ViewSettings {
    pub show_toolbar: bool,
    pub show_status_bar: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub show_whitespace: bool,
    pub font_size: f32,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            show_toolbar: true,
            show_status_bar: true,
            show_line_numbers: true,
            word_wrap: false,
            show_whitespace: false,
            font_size: 14.0,
        }
    }
}

impl ViewSettings {
    pub fn zoom_in(&mut self) {
        self.font_size = (self.font_size + 1.0).min(72.0);
    }

    pub fn zoom_out(&mut self) {
        self.font_size = (self.font_size - 1.0).max(6.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_next_wraps_around() {
        let mut state = FindState {
            matches: vec![0, 5, 10],
            selected_match: 2,
            ..Default::default()
        };
        state.find_next();
        assert_eq!(state.selected_match, 0);
    }

    #[test]
    fn find_prev_wraps_around() {
        let mut state = FindState {
            matches: vec![0, 5, 10],
            selected_match: 0,
            ..Default::default()
        };
        state.find_prev();
        assert_eq!(state.selected_match, 2);
    }

    #[test]
    fn find_next_noop_on_empty() {
        let mut state = FindState::default();
        state.find_next(); // should not panic
        assert_eq!(state.selected_match, 0);
    }

    #[test]
    fn zoom_clamps_at_bounds() {
        let mut view = ViewSettings { font_size: 72.0, ..Default::default() };
        view.zoom_in();
        assert_eq!(view.font_size, 72.0);

        view.font_size = 6.0;
        view.zoom_out();
        assert_eq!(view.font_size, 6.0);
    }

    #[test]
    fn default_view_settings() {
        let view = ViewSettings::default();
        assert!(view.show_toolbar);
        assert!(view.show_status_bar);
        assert_eq!(view.font_size, 14.0);
        assert!(!view.word_wrap);
    }
}
