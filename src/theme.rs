use eframe::egui;
use egui_extras::syntax_highlighting::CodeTheme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AppTheme {
    Light,
    #[default]
    Dark,
}

impl AppTheme {
    pub fn code_theme(self, font_size: f32) -> CodeTheme {
        match self {
            AppTheme::Light => CodeTheme::light(font_size),
            AppTheme::Dark => CodeTheme::dark(font_size),
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

    pub fn apply(self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // -- Global spacing / rounding --
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(10);
        style.spacing.menu_margin = egui::Margin::same(8);

        // Build visuals and assign into style.visuals BEFORE set_style.
        // (calling set_visuals + set_style separately causes the old visuals
        //  from the cloned style to overwrite the new ones.)
        match self {
            AppTheme::Dark => {
                let mut v = egui::Visuals::dark();

                // Base palette — deep blue-grey with translucent panels
                let bg_base = egui::Color32::from_rgb(22, 24, 33); // very dark blue
                let bg_panel = egui::Color32::from_rgba_unmultiplied(30, 33, 46, 240); // slight transparency
                let bg_widget = egui::Color32::from_rgb(40, 44, 62);
                let bg_hover = egui::Color32::from_rgb(50, 55, 78);
                let bg_active = egui::Color32::from_rgb(60, 66, 92);
                let accent = egui::Color32::from_rgb(100, 140, 255); // bright blue accent
                let accent_dim = egui::Color32::from_rgb(70, 100, 200);
                let text_primary = egui::Color32::from_rgb(220, 225, 240);
                let text_secondary = egui::Color32::from_rgb(150, 158, 180);
                let border = egui::Color32::from_rgba_unmultiplied(80, 90, 120, 100);

                v.panel_fill = bg_panel;
                v.window_fill = egui::Color32::from_rgba_unmultiplied(28, 31, 43, 245);
                v.faint_bg_color = egui::Color32::from_rgb(35, 38, 52);
                v.extreme_bg_color = bg_base;
                v.code_bg_color = egui::Color32::from_rgb(25, 27, 38);

                v.override_text_color = Some(text_primary);
                v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(100, 140, 255, 60);
                v.selection.stroke = egui::Stroke::new(1.0, accent);
                v.hyperlink_color = accent;

                // Widgets
                v.widgets.noninteractive.bg_fill = bg_panel;
                v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_secondary);
                v.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, border);
                v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);

                v.widgets.inactive.bg_fill = bg_widget;
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_primary);
                v.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, border);
                v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);

                v.widgets.hovered.bg_fill = bg_hover;
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent_dim);
                v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
                v.widgets.hovered.expansion = 1.0;

                v.widgets.active.bg_fill = bg_active;
                v.widgets.active.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
                v.widgets.active.bg_stroke = egui::Stroke::new(1.5, accent);
                v.widgets.active.corner_radius = egui::CornerRadius::same(6);

                v.widgets.open.bg_fill = bg_hover;
                v.widgets.open.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                v.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent_dim);
                v.widgets.open.corner_radius = egui::CornerRadius::same(6);

                // Window styling
                v.window_corner_radius = egui::CornerRadius::same(10);
                v.window_shadow = egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 2,
                    color: egui::Color32::from_black_alpha(80),
                };
                v.window_stroke = egui::Stroke::new(1.0, border);

                // Popups / menus
                v.popup_shadow = egui::epaint::Shadow {
                    offset: [0, 3],
                    blur: 12,
                    spread: 1,
                    color: egui::Color32::from_black_alpha(60),
                };
                v.menu_corner_radius = egui::CornerRadius::same(8);

                // Misc
                v.resize_corner_size = 10.0;
                v.clip_rect_margin = 3.0;
                v.interact_cursor = Some(egui::CursorIcon::PointingHand);

                style.visuals = v;
            }
            AppTheme::Light => {
                let mut v = egui::Visuals::light();

                let bg_base = egui::Color32::from_rgb(248, 249, 252);
                let bg_panel = egui::Color32::from_rgba_unmultiplied(242, 244, 250, 240);
                let bg_widget = egui::Color32::from_rgb(232, 235, 245);
                let bg_hover = egui::Color32::from_rgb(218, 224, 240);
                let bg_active = egui::Color32::from_rgb(200, 210, 235);
                let accent = egui::Color32::from_rgb(50, 100, 220);
                let accent_dim = egui::Color32::from_rgb(80, 120, 210);
                let text_primary = egui::Color32::from_rgb(30, 35, 50);
                let text_secondary = egui::Color32::from_rgb(100, 110, 130);
                let border = egui::Color32::from_rgba_unmultiplied(180, 190, 210, 120);

                v.panel_fill = bg_panel;
                v.window_fill = egui::Color32::from_rgba_unmultiplied(250, 251, 254, 248);
                v.faint_bg_color = egui::Color32::from_rgb(240, 242, 248);
                v.extreme_bg_color = bg_base;
                v.code_bg_color = egui::Color32::from_rgb(245, 247, 252);

                v.override_text_color = Some(text_primary);
                v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(50, 100, 220, 90);
                v.selection.stroke = egui::Stroke::new(1.0, accent);
                v.hyperlink_color = accent;

                v.widgets.noninteractive.bg_fill = bg_panel;
                v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_secondary);
                v.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, border);
                v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);

                v.widgets.inactive.bg_fill = bg_widget;
                v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_primary);
                v.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, border);
                v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);

                v.widgets.hovered.bg_fill = bg_hover;
                v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text_primary);
                v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent_dim);
                v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
                v.widgets.hovered.expansion = 1.0;

                v.widgets.active.bg_fill = bg_active;
                v.widgets.active.fg_stroke = egui::Stroke::new(1.5, text_primary);
                v.widgets.active.bg_stroke = egui::Stroke::new(1.5, accent);
                v.widgets.active.corner_radius = egui::CornerRadius::same(6);

                v.widgets.open.bg_fill = bg_hover;
                v.widgets.open.fg_stroke = egui::Stroke::new(1.0, text_primary);
                v.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent_dim);
                v.widgets.open.corner_radius = egui::CornerRadius::same(6);

                v.window_corner_radius = egui::CornerRadius::same(10);
                v.window_shadow = egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 2,
                    color: egui::Color32::from_black_alpha(30),
                };
                v.window_stroke = egui::Stroke::new(1.0, border);

                v.popup_shadow = egui::epaint::Shadow {
                    offset: [0, 3],
                    blur: 12,
                    spread: 1,
                    color: egui::Color32::from_black_alpha(20),
                };
                v.menu_corner_radius = egui::CornerRadius::same(8);

                v.resize_corner_size = 10.0;
                v.clip_rect_margin = 3.0;
                v.interact_cursor = Some(egui::CursorIcon::PointingHand);

                style.visuals = v;
            }
        }

        // Single set_style call — includes both spacing AND visuals.
        ctx.set_style(style);
    }

    /// Accent color for the current theme.
    pub fn accent(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgb(100, 140, 255),
            AppTheme::Light => egui::Color32::from_rgb(50, 100, 220),
        }
    }

    /// Subtle text color.
    pub fn text_dim(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgb(150, 158, 180),
            AppTheme::Light => egui::Color32::from_rgb(100, 110, 130),
        }
    }

    /// Tab active background — opaque enough to clearly distinguish the active tab.
    pub fn tab_active_bg(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgb(45, 52, 80),
            AppTheme::Light => egui::Color32::from_rgb(215, 225, 245),
        }
    }

    /// Tab active text — high contrast against the active bg.
    pub fn tab_active_text(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgb(240, 245, 255),
            AppTheme::Light => egui::Color32::from_rgb(20, 30, 60),
        }
    }

    /// Gutter background — just slightly different from code_bg_color so there's
    /// a subtle visual boundary without a jarring color jump.
    pub fn gutter_bg(self) -> egui::Color32 {
        match self {
            // code_bg_color is (25,27,38); gutter is a touch lighter
            AppTheme::Dark => egui::Color32::from_rgb(28, 30, 42),
            // code_bg_color is (245,247,252); gutter is a touch darker
            AppTheme::Light => egui::Color32::from_rgb(237, 239, 246),
        }
    }

    /// Gutter text color — muted but readable.
    pub fn gutter_text(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgb(100, 108, 135),
            AppTheme::Light => egui::Color32::from_rgb(150, 158, 175),
        }
    }

    pub fn current_line_bg(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
            AppTheme::Light => egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10),
        }
    }

    pub fn brace_match_bg(self) -> egui::Color32 {
        match self {
            AppTheme::Dark => egui::Color32::from_rgba_unmultiplied(100, 200, 255, 50),
            AppTheme::Light => egui::Color32::from_rgba_unmultiplied(0, 100, 200, 60),
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
