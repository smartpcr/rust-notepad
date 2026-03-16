use egui_extras::syntax_highlighting::CodeTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AppTheme {
    Light,
    Dark,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::Dark
    }
}

impl AppTheme {
    pub fn code_theme(self) -> CodeTheme {
        match self {
            AppTheme::Light => CodeTheme::light(),
            AppTheme::Dark => CodeTheme::dark(),
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            AppTheme::Light => AppTheme::Dark,
            AppTheme::Dark => AppTheme::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AppTheme::Light => "Light",
            AppTheme::Dark => "Dark",
        }
    }

    pub fn apply(self, ctx: &eframe::egui::Context) {
        match self {
            AppTheme::Light => ctx.set_visuals(eframe::egui::Visuals::light()),
            AppTheme::Dark => ctx.set_visuals(eframe::egui::Visuals::dark()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_switches_between_variants() {
        assert_eq!(AppTheme::Dark.toggle(), AppTheme::Light);
        assert_eq!(AppTheme::Light.toggle(), AppTheme::Dark);
    }

    #[test]
    fn label_returns_display_name() {
        assert_eq!(AppTheme::Dark.label(), "Dark");
        assert_eq!(AppTheme::Light.label(), "Light");
    }

    #[test]
    fn default_is_dark() {
        assert_eq!(AppTheme::default(), AppTheme::Dark);
    }
}
