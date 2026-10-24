//! Édition de la vélocité du piano roll.
//!
//! Pas de piste dédiée : un petit badge « V » apparaît à droite de la note
//! survolée ou sélectionnée. Le remplissage du badge indique la vélocité ;
//! un clic ouvre un slider vertical (1..127). Si la note fait partie d'une
//! sélection multiple, la valeur est appliquée à tout le groupe.

use crate::pianoroll::ROW_H;
use crate::project::*;
use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Rounding, Sense};

const BADGE_W: f32 = 12.0;
const BADGE_H: f32 = 12.0;

/// État local (mémoire temporaire d'egui).
#[derive(Clone, Default)]
struct VelState {
    /// note dont le slider est ouvert
    open: Option<usize>,
    /// dernière note survolée (gardée tant que le curseur est sur son badge)
    hover: Option<usize>,
    /// vrai pendant qu'on déplace le slider (un seul checkpoint par geste)
    editing: bool,
}

impl App {
    /// À appeler après le dessin des notes.
    /// `sel` = notes sélectionnées, `hover` = note sous le curseur (le cas échéant).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn velocity_overlay(
        &mut self,
        ui: &mut egui::Ui,
        t: usize,
        c: usize,
        sel: &[usize],
        hover: Option<usize>,
        grid: Rect,
        top: i32,
        rows: i32,
        gx: &dyn Fn(f64) -> f32,
        col: Color32,
    ) {
        let sid = egui::Id::new("piano_roll_velocity_state");
        let mut st: VelState = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let painter = ui.painter_at(grid);

        // rectangle du badge d'une note : juste à droite de sa fin (ne gêne pas le redimensionnement)
        let badge_rect = |n: &Note| -> Rect {
            let y = grid.top() + (top - n.key as i32) as f32 * ROW_H;
            Rect::from_min_size(pos2(gx(n.start + n.len) + 1.0, y + (ROW_H - BADGE_H) * 0.5), vec2(BADGE_W, BADGE_H))
        };

        // le survol est conservé tant que le curseur est sur le badge de la dernière note survolée
        let keep = st.hover.and_then(|i| {
            let n = self.notes_ref(t, c)?.get(i)?;
            ptr.filter(|p| badge_rect(n).expand(2.0).contains(*p)).map(|_| i)
        });
        st.hover = hover.or(keep);

        let mut show: Vec<usize> = sel.to_vec();
        show.extend(st.hover);
        show.extend(st.open);
        show.sort_unstable();
        show.dedup();

        let mut open_info: Option<(usize, Rect)> = None;
        for i in show {
            let Some(n) = self.notes_ref(t, c).and_then(|v| v.get(i)).cloned() else { continue };
            if n.key as i32 > top || (n.key as i32) <= top - rows {
                continue;
            }
            let r = badge_rect(&n);
            if r.right() < grid.left() || r.left() > grid.right() {
                continue;
            }
            let resp = ui.interact(r, ui.id().with(("vel_badge", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                st.hover = Some(i);
            }
            if resp.clicked() {
                st.open = if st.open == Some(i) { None } else { Some(i) };
            }
            let is_open = st.open == Some(i);
            if !is_open {
                let v = (n.vel * 127.0).round().clamp(1.0, 127.0) as i32;
                resp.on_hover_text(format!("Velocity: {v}/127 - click to edit"));
            }

            // fond + remplissage proportionnel à la vélocité
            painter.rect_filled(r, Rounding::same(3.0), if is_open { col } else { Color32::WHITE });
            if !is_open {
                let h = r.height() * n.vel.clamp(0.0, 1.0);
                let fill = Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.right_bottom());
                painter.rect_filled(fill, Rounding::same(3.0), tint(col, 90));
            }
            painter.rect_stroke(r, Rounding::same(3.0), stroke(1.0, col));
            painter.text(r.center(), Align2::CENTER_CENTER, "V", FontId::proportional(9.5), if is_open { Color32::WHITE } else { INK });
            if is_open {
                open_info = Some((i, r));
            }
        }

        // ---- popup : slider vertical (1..127) ----
        if let Some((i, r)) = open_info {
            let cur = self.notes_ref(t, c).and_then(|v| v.get(i)).map_or(0.8, |n| n.vel);
            let before = (cur * 127.0).round().clamp(1.0, 127.0) as i32;
            let mut val = before;
            let area = egui::Area::new(egui::Id::new("vel_popup"))
                .fixed_pos(r.right_top() + vec2(4.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(RichText::new("Velocity").size(10.5).color(GREY));
                        ui.add(egui::Slider::new(&mut val, 1..=127).vertical());
                    });
                });
            if val != before {
                if !st.editing {
                    self.checkpoint();
                    st.editing = true;
                }
                let v = val as f32 / 127.0;
                let group: Vec<usize> = if sel.contains(&i) { sel.to_vec() } else { vec![i] };
                if let Some(notes) = self.notes_mut(t, c) {
                    for &k in &group {
                        if let Some(n) = notes.get_mut(k) {
                            n.vel = v;
                        }
                    }
                }
                self.push();
            }
            // fermeture : clic ailleurs ou Échap
            let (pressed, esc) = ui.input(|inp| (inp.pointer.any_pressed(), inp.key_pressed(egui::Key::Escape)));
            let outside = pressed && ptr.map_or(true, |p| !area.response.rect.contains(p) && !r.contains(p));
            if outside || esc {
                st.open = None;
            }
        }
        if !ui.input(|i| i.pointer.any_down()) {
            st.editing = false;
        }
        ui.ctx().data_mut(|d| d.insert_temp(sid, st));
    }
}