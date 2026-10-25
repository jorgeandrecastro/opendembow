//! Layout visibility: show / hide the instrument browser, the piano roll & mixer panel and the
//! on-screen keyboard. The buttons use a "panel" glyph (a window with one filled side) like
//! macOS / iOS sidebars. A hidden panel collapses into a thin strip with the same glyph in blue,
//! so a beginner can always find the way back.

use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Color32, Painter, Pos2, Rect, RichText, Rounding, Sense};

/// Which side of the window the panel lives on (drives the glyph).
#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Bottom,
}

/// Panel glyph: a rounded window; the filled part shows the panel when it is visible.
fn paint_glyph(p: &Painter, rect: Rect, side: Side, open: bool, hover: bool) {
    if hover {
        p.rect_filled(rect, 8.0, Color32::from_rgb(236, 236, 241));
    }
    let col = if !open {
        ACCENT
    } else if hover {
        INK
    } else {
        GREY
    };
    let r = Rect::from_center_size(rect.center(), vec2(16.0, 12.0));
    p.rect_stroke(r, 3.0, stroke(1.5, col));
    match side {
        Side::Left => {
            let x = r.left() + 5.5;
            if open {
                let fill = Rect::from_min_max(r.min + vec2(0.75, 0.75), pos2(x, r.bottom() - 0.75));
                p.rect_filled(fill, Rounding { nw: 2.0, sw: 2.0, ne: 0.0, se: 0.0 }, col);
            }
            p.line_segment([pos2(x, r.top() + 0.75), pos2(x, r.bottom() - 0.75)], stroke(1.5, col));
        }
        Side::Bottom => {
            let y = r.bottom() - 4.5;
            if open {
                let fill = Rect::from_min_max(pos2(r.left() + 0.75, y), r.max - vec2(0.75, 0.75));
                p.rect_filled(fill, Rounding { nw: 0.0, ne: 0.0, sw: 2.0, se: 2.0 }, col);
            }
            p.line_segment([pos2(r.left() + 0.75, y), pos2(r.right() - 0.75, y)], stroke(1.5, col));
        }
    }
}

/// Panel toggle button. Returns true when clicked. `tip` is shown on hover ("Hide ..." / "Show ...").
pub fn panel_button(ui: &mut egui::Ui, side: Side, open: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 22.0), Sense::click());
    paint_glyph(ui.painter(), rect, side, open, resp.hovered());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked()
}

/// Toggle floating above the panels at `pos` (optionally on a soft white chip with a light shadow).
pub fn float_toggle(ctx: &egui::Context, id: &str, pos: Pos2, side: Side, open: bool, tip: &str, chip: bool) -> bool {
    let mut clicked = false;
    egui::Area::new(egui::Id::new(id)).order(egui::Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        if chip {
            let r = Rect::from_min_size(pos, vec2(30.0, 22.0));
            ui.painter().rect_filled(r.translate(vec2(0.0, 1.5)).expand(1.0), 12.0, Color32::from_black_alpha(22));
            ui.painter().rect_filled(r, 11.0, Color32::WHITE);
        }
        clicked = panel_button(ui, side, open, tip);
    });
    clicked
}

/// Content of a collapsed strip: blue glyph + label. Returns true when the user wants the panel back.
pub fn collapsed_strip(ui: &mut egui::Ui, side: Side, label: &str, tip: &str) -> bool {
    let mut open = false;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if panel_button(ui, side, false, tip) {
            open = true;
        }
        let l = egui::Label::new(RichText::new(label).size(12.0).color(GREY)).sense(Sense::click());
        if ui.add(l).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            open = true;
        }
    });
    open
}

impl App {
    /// Ctrl+Shift+F: hide everything but the timeline, or bring everything back.
    pub fn toggle_focus(&mut self) {
        let any = self.show_browser || self.show_detail || self.show_keys;
        self.show_browser = !any;
        self.show_detail = !any;
        self.show_keys = !any;
    }

    pub fn ui_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.checkbox(&mut self.show_browser, "Instruments        Ctrl+B");
            ui.checkbox(&mut self.show_detail, "Piano roll & Mixer        Ctrl+J");
            ui.checkbox(&mut self.show_keys, "On-screen keyboard        Ctrl+Shift+K");
            ui.separator();
            if ui.button("Focus on the timeline        Ctrl+Shift+F").clicked() {
                self.show_browser = false;
                self.show_detail = false;
                self.show_keys = false;
                ui.close_menu();
            }
            if ui.button("Show everything").clicked() {
                self.show_browser = true;
                self.show_detail = true;
                self.show_keys = true;
                ui.close_menu();
            }
        });
    }
}