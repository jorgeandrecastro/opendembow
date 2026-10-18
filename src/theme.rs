//! Visual identity: light, airy, rounded, a single blue accent.

use eframe::egui::{self, vec2, Color32, FontId, Rounding, Stroke};

pub const ACCENT: Color32 = Color32::from_rgb(0, 122, 255);
pub const RED: Color32 = Color32::from_rgb(255, 59, 48);
pub const INK: Color32 = Color32::from_rgb(28, 28, 30);
pub const GREY: Color32 = Color32::from_rgb(142, 142, 147);
pub const HAIR: Color32 = Color32::from_rgb(229, 229, 234);
pub const BG: Color32 = Color32::from_rgb(242, 242, 247);

/// Track colours (Apple system palette).
pub const PALETTE: [Color32; 8] = [
    Color32::from_rgb(0, 122, 255),
    Color32::from_rgb(52, 199, 89),
    Color32::from_rgb(255, 149, 0),
    Color32::from_rgb(255, 45, 85),
    Color32::from_rgb(175, 82, 222),
    Color32::from_rgb(90, 200, 250),
    Color32::from_rgb(255, 204, 0),
    Color32::from_rgb(88, 86, 214),
];

pub fn stroke(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

pub fn tint(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = BG;
    v.window_fill = Color32::WHITE;
    v.selection.bg_fill = ACCENT;
    v.window_rounding = Rounding::same(16.0);
    v.widgets.noninteractive.bg_stroke = stroke(1.0, HAIR);
    for (w, fill) in [
        (&mut v.widgets.inactive, Color32::WHITE),
        (&mut v.widgets.hovered, Color32::from_rgb(235, 235, 240)),
        (&mut v.widgets.active, Color32::from_rgb(220, 220, 228)),
    ] {
        w.weak_bg_fill = fill;
        w.bg_fill = fill;
        w.bg_stroke = stroke(1.0, HAIR);
        w.rounding = Rounding::same(9.0);
        w.fg_stroke.color = INK;
    }
    v.widgets.noninteractive.fg_stroke.color = INK;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(13.0, 6.0);
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}
