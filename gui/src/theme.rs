use egui::{Color32, Stroke, Rounding, Visuals, Margin, FontFamily, FontId};

/// Semantic color tokens for the Ajazz AK820 Max RGB Studio.
#[allow(dead_code)]
pub struct ThemeColors;

#[allow(dead_code)]
impl ThemeColors {
    // Surface & background
    pub const BG_MAIN: Color32 = Color32::from_rgb(11, 15, 25);
    pub const BG_PANEL: Color32 = Color32::from_rgb(19, 27, 46);
    pub const BG_CARD: Color32 = Color32::from_rgb(26, 35, 58);
    pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(35, 47, 76);
    pub const BG_KEYCAP_IDLE: Color32 = Color32::from_rgb(22, 29, 45);
    pub const BG_KEYCAP_SELECTED: Color32 = Color32::from_rgb(45, 60, 95);

    // Borders & strokes
    pub const BORDER_MUTED: Color32 = Color32::from_rgb(46, 61, 95);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(33, 44, 70);
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(0, 242, 254);

    // Typography
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 163, 184);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 116, 139);

    // Accents & Gradients
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 242, 254);
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
    pub const ACCENT_PINK: Color32 = Color32::from_rgb(244, 63, 94);
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);

    // Status colors
    pub const STATUS_SUCCESS: Color32 = Color32::from_rgb(16, 185, 129);
    pub const STATUS_WARNING: Color32 = Color32::from_rgb(245, 158, 11);
    pub const STATUS_ERROR: Color32 = Color32::from_rgb(239, 68, 68);

    // Audio Visualizer Gradient (10 Bands)
    pub const EQUALIZER_GRADIENT: [Color32; 10] = [
        Color32::from_rgb(0, 242, 254),   // 40 Hz (Sub-bass / Cyan)
        Color32::from_rgb(14, 215, 255),  // 80 Hz
        Color32::from_rgb(67, 180, 255),  // 160 Hz (Bass)
        Color32::from_rgb(125, 130, 255), // 320 Hz (Low mids)
        Color32::from_rgb(168, 85, 247),  // 640 Hz (Mids / Purple)
        Color32::from_rgb(200, 75, 225),  // 1.2 kHz
        Color32::from_rgb(230, 65, 175),  // 2.5 kHz (Upper mids)
        Color32::from_rgb(244, 63, 140),  // 5 kHz (Presence)
        Color32::from_rgb(244, 63, 94),   // 10 kHz (Brilliance / Pink)
        Color32::from_rgb(255, 75, 75),   // 16 kHz (Air / Red-Pink)
    ];
}

/// Applies the custom visual styling to the egui Context.
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Dark visuals base
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(ThemeColors::TEXT_PRIMARY);
    visuals.panel_fill = ThemeColors::BG_MAIN;
    visuals.window_fill = ThemeColors::BG_PANEL;
    visuals.faint_bg_color = ThemeColors::BG_CARD;
    visuals.extreme_bg_color = ThemeColors::BG_MAIN;
    visuals.window_stroke = Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED);

    // Widgets inactive
    visuals.widgets.inactive.bg_fill = ThemeColors::BG_CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(8.0);

    // Widgets hovered
    visuals.widgets.hovered.bg_fill = ThemeColors::BG_CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(8.0);

    // Widgets active / pressed
    visuals.widgets.active.bg_fill = ThemeColors::ACCENT_PURPLE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(8.0);

    // Widgets open
    visuals.widgets.open.bg_fill = ThemeColors::BG_CARD;
    visuals.widgets.open.bg_stroke = Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, ThemeColors::TEXT_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(8.0);

    // Selection highlight
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(0, 242, 254, 45);
    visuals.selection.stroke = Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN);

    // Spacing & padding
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.slider_width = 180.0;
    style.spacing.window_margin = Margin::same(16.0);

    // Typography
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (egui::TextStyle::Name("Subheading".into()), FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (egui::TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (egui::TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ].into();

    style.visuals = visuals;
    ctx.set_style(style);
}
