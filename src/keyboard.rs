//! On-screen piano keyboard (always visible, like a hardware controller).

use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Rounding, Sense};

const LETTERS: [&str; 15] = ["A", "W", "S", "E", "D", "F", "T", "G", "Y", "H", "U", "J", "K", "O", "L"];

impl App {
    pub fn ui_keyboard(&mut self, ui: &mut egui::Ui) {
        let inst = self.live_inst();
        ui.horizontal(|ui| {
            ui.label(RichText::new("KEYBOARD").size(11.0).color(GREY).strong());
            match inst {
                Some((_, i)) => ui.label(RichText::new(i.name()).strong()),
                None => ui.label(RichText::new("Select an instrument track to play").color(GREY)),
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+").clicked() {
                    self.octave = (self.octave + 1).min(6);
                }
                ui.label(format!("Octave {}  (Z / X)", self.octave));
                if ui.small_button("−").clicked() {
                    self.octave = (self.octave - 1).max(0);
                }
            });
        });

        let size = vec2(ui.available_width(), ui.available_height().max(60.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let lo = (12 * self.octave).clamp(0, 72);
        let is_white = |m: i32| matches!(m % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11);
        let whites: Vec<i32> = (lo..=lo + 48).filter(|m| is_white(*m)).collect();
        let ww = rect.width() / whites.len() as f32;
        let bw = ww * 0.62;
        let mut wk: Vec<(Rect, i32)> = Vec::new();
        let mut bk: Vec<(Rect, i32)> = Vec::new();
        for (i, &m) in whites.iter().enumerate() {
            wk.push((Rect::from_min_size(pos2(rect.left() + i as f32 * ww, rect.top()), vec2(ww, rect.height())), m));
        }
        for m in lo..=lo + 48 {
            if !is_white(m) {
                let i = whites.iter().position(|&w| w == m - 1).unwrap_or(0);
                let x = rect.left() + (i as f32 + 1.0) * ww - bw / 2.0;
                bk.push((Rect::from_min_size(pos2(x, rect.top()), vec2(bw, rect.height() * 0.62)), m));
            }
        }
        let hit = if resp.is_pointer_button_down_on() {
            resp.interact_pointer_pos().and_then(|p| bk.iter().chain(wk.iter()).find(|(r, _)| r.contains(p)).map(|(_, m)| *m))
        } else {
            None
        };
        let hit = hit.map(|m| m as u8);
        if hit != self.kb_key {
            if let Some(k) = self.kb_key {
                self.note_off(k);
            }
            if let Some(k) = hit {
                self.note_on(k);
            }
            self.kb_key = hit;
        }

        let base = 12 * (self.octave + 1);
        let letter = |m: i32| if m >= base && m - base < 15 { Some(LETTERS[(m - base) as usize]) } else { None };
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 12.0, Color32::WHITE);
        for (r, m) in &wk {
            let on = self.down.contains(&(*m as u8));
            let rr = r.shrink2(vec2(0.5, 0.0));
            let round = Rounding { nw: 0.0, ne: 0.0, sw: 9.0, se: 9.0 };
            p.rect_filled(rr, round, if on { tint(ACCENT, 70) } else { Color32::WHITE });
            p.rect_stroke(rr, round, stroke(1.0, HAIR));
            if let Some(l) = letter(*m) {
                p.text(pos2(r.center().x, r.bottom() - 30.0), Align2::CENTER_CENTER, l, FontId::proportional(12.0), tint(ACCENT, 200));
            }
            if m % 12 == 0 {
                p.text(pos2(r.center().x, r.bottom() - 12.0), Align2::CENTER_CENTER, format!("C{}", m / 12 - 1), FontId::proportional(10.5), GREY);
            }
        }
        for (r, m) in &bk {
            let on = self.down.contains(&(*m as u8));
            p.rect_filled(*r, Rounding { nw: 0.0, ne: 0.0, sw: 5.0, se: 5.0 }, if on { ACCENT } else { INK });
            if let Some(l) = letter(*m) {
                p.text(pos2(r.center().x, r.bottom() - 12.0), Align2::CENTER_CENTER, l, FontId::proportional(10.5), Color32::from_rgb(200, 200, 206));
            }
        }
        if inst.is_none() {
            p.rect_filled(rect, 12.0, tint(Color32::WHITE, 150));
        }
    }
}
