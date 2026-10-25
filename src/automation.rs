//! Automation de volume : bande sous les clips de chaque piste.
//!
//! - clic dans la bande : ajoute un point ; un point « fantôme » montre où il sera posé
//! - glisser un point : le déplace (magnétisme sur la grille ; Alt = libre)
//! - clic droit sur un point : le supprime
//! - repères 0 / 50 / 100 % toujours visibles, et la valeur s'accroche à ces trois repères
//!
//! La bande a sa propre zone d'interaction, posée par-dessus les lanes : elle ne
//! déclenche donc ni le déplacement / redimensionnement des clips, ni leurs curseurs.

use crate::arrangement::TRACK_H;
use crate::project::*;
use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke};

/// Hauteur de la bande d'automation, en bas de chaque piste.
pub(crate) const AUTO_H: f32 = 38.0;
/// Rose : distinct du playhead (rouge) et de l'accent de l'interface.
const AUTO: Color32 = Color32::from_rgb(240, 70, 130);
/// Rayon de capture d'un point, en pixels.
const HIT: f32 = 9.0;
/// Marge haut / bas de la courbe : 0 % et 100 % restent entièrement visibles.
const PAD: f32 = 9.0;
/// Écart (en valeur) dans lequel on s'accroche à 0 / 50 / 100 %.
const MAGNET: f32 = 0.03;
const PARAM: &str = "volume";

/// État local (mémoire temporaire d'egui).
#[derive(Clone, Copy, Default)]
struct AutoUi {
    /// point en cours de déplacement (piste, index)
    drag: Option<(usize, usize)>,
    /// point sous le curseur (piste, index)
    hover: Option<(usize, usize)>,
    /// point fantôme : (piste, temps, valeur) là où un clic poserait un point
    ghost: Option<(usize, f64, f32)>,
}

fn strip(lanes: Rect, ti: usize, sy: f32) -> Rect {
    let y = lanes.top() + ti as f32 * TRACK_H - sy + TRACK_H - AUTO_H;
    Rect::from_min_size(pos2(lanes.left(), y), vec2(lanes.width(), AUTO_H))
}

/// Zone de la courbe (avec marges haut / bas).
fn curve_area(s: Rect) -> Rect {
    s.shrink2(vec2(0.0, PAD))
}

fn y_of(a: Rect, v: f32) -> f32 {
    a.bottom() - v.clamp(0.0, 1.0) * a.height()
}

fn val_at(a: Rect, y: f32) -> f32 {
    ((a.bottom() - y) / a.height()).clamp(0.0, 1.0)
}

/// S'accroche à 0, 50 ou 100 % quand on en est très proche.
fn magnet(v: f32, on: bool) -> f32 {
    if !on {
        return v;
    }
    for t in [0.0f32, 0.5, 1.0] {
        if (v - t).abs() < MAGNET {
            return t;
        }
    }
    v
}

fn lane_points(t: &Track) -> &[AutomationPoint] {
    t.automations.iter().find(|l| l.parameter_id == PARAM).map(|l| l.points.as_slice()).unwrap_or(&[])
}

/// Petite étiquette sombre « 73% » à côté d'un point.
fn tag(p: &egui::Painter, at: Pos2, v: f32) {
    let r = Rect::from_min_size(pos2(at.x + 9.0, at.y - 18.0), vec2(38.0, 16.0));
    p.rect_filled(r, 8.0, INK);
    p.text(r.center(), Align2::CENTER_CENTER, format!("{:.0}%", v * 100.0), FontId::monospace(10.0), Color32::WHITE);
}

impl App {
    /// Insère un point (liste triée) et renvoie son index.
    fn auto_insert(&mut self, ti: usize, beat: f64, val: f32) -> usize {
        let t = &mut self.project.tracks[ti];
        let pos = match t.automations.iter().position(|l| l.parameter_id == PARAM) {
            Some(p) => p,
            None => {
                t.automations.push(AutomationLane { parameter_id: PARAM.to_string(), points: Vec::new() });
                t.automations.len() - 1
            }
        };
        let lane = &mut t.automations[pos];
        let idx = lane.points.partition_point(|p| p.beat <= beat);
        lane.points.insert(idx, AutomationPoint { beat, value: val });
        self.push();
        idx
    }

    /// Déplace un point sans dépasser ses voisins (l'ordre et l'index restent stables).
    fn auto_move(&mut self, ti: usize, di: usize, beat: f64, val: f32) {
        if let Some(l) = self.project.tracks.get_mut(ti).and_then(|t| t.automations.iter_mut().find(|l| l.parameter_id == PARAM)) {
            let lo = if di > 0 { l.points[di - 1].beat } else { 0.0 };
            let hi = l.points.get(di + 1).map_or(f64::MAX, |p| p.beat);
            if let Some(pt) = l.points.get_mut(di) {
                pt.beat = beat.max(lo).min(hi);
                pt.value = val;
            }
        }
        self.push();
    }

    fn auto_remove(&mut self, ti: usize, di: usize) {
        if let Some(l) = self.project.tracks.get_mut(ti).and_then(|t| t.automations.iter_mut().find(|l| l.parameter_id == PARAM)) {
            if di < l.points.len() {
                l.points.remove(di);
            }
        }
        self.push();
    }

    /// Interactions des bandes d'automation. À appeler avant le dessin, après les interactions des lanes.
    pub(crate) fn automation_interact(&mut self, ui: &mut egui::Ui, lanes: Rect, sx: f64, sy: f32, ppb: f32) {
        let sid = egui::Id::new("automation_ui_state");
        let mut st: AutoUi = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
        let alt = ui.input(|i| i.modifiers.alt);
        let grid: f64 = if ppb >= 64.0 { 0.25 } else if ppb >= 28.0 { 0.5 } else { 1.0 };
        let snap = |b: f64| if alt { b } else { (b / grid).round() * grid };
        let bx = |b: f64| lanes.left() + ((b - sx) as f32) * ppb;
        let beat_at = |x: f32| ((x - lanes.left()) / ppb) as f64 + sx;
        st.hover = None;
        st.ghost = None;

        for ti in 0..self.project.tracks.len() {
            let s = strip(lanes, ti, sy);
            let vis = s.intersect(lanes);
            if !(vis.width() > 1.0 && vis.height() > 1.0) {
                continue;
            }
            let area = curve_area(s);
            let resp = ui.interact(vis, ui.id().with(("auto", ti)), Sense::click_and_drag());
            let pts: Vec<AutomationPoint> = lane_points(&self.project.tracks[ti]).to_vec();
            let near = |p: Pos2| -> Option<usize> {
                pts.iter()
                    .enumerate()
                    .map(|(i, pt)| (i, pos2(bx(pt.beat), y_of(area, pt.value)).distance(p)))
                    .filter(|(_, d)| *d <= HIT)
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
            };

            // curseur + survol + point fantôme
            if let Some(p) = resp.hover_pos() {
                let n = near(p);
                st.hover = n.map(|i| (ti, i));
                if n.is_none() && st.drag.is_none() {
                    st.ghost = Some((ti, snap(beat_at(p.x)).max(0.0), magnet(val_at(area, p.y), !alt)));
                }
                let icon = if st.drag.is_some() {
                    CursorIcon::Grabbing
                } else if n.is_some() {
                    CursorIcon::Grab
                } else {
                    CursorIcon::Crosshair
                };
                ui.ctx().set_cursor_icon(icon);
            } else if resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }

            if resp.clicked() || resp.drag_started() {
                self.sel_track = ti;
            }

            // début de glisser : saisit le point le plus proche, ou en crée un sous le curseur
            if resp.drag_started() {
                let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| resp.interact_pointer_pos());
                if let Some(p) = origin {
                    self.checkpoint();
                    let idx = match near(p) {
                        Some(i) => i,
                        None => self.auto_insert(ti, snap(beat_at(p.x)).max(0.0), magnet(val_at(area, p.y), !alt)),
                    };
                    st.drag = Some((ti, idx));
                }
            }
            if resp.dragged() {
                if let (Some((dt, di)), Some(p)) = (st.drag, resp.interact_pointer_pos()) {
                    if dt == ti {
                        self.auto_move(ti, di, snap(beat_at(p.x)).max(0.0), magnet(val_at(area, p.y), !alt));
                    }
                }
            }
            if resp.drag_stopped() {
                st.drag = None;
            }

            // clic simple sur un endroit vide : nouveau point (là où se trouvait le fantôme)
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    if near(p).is_none() {
                        self.checkpoint();
                        self.auto_insert(ti, snap(beat_at(p.x)).max(0.0), magnet(val_at(area, p.y), !alt));
                        self.status = "Automation point added (right-click a point to delete it).".into();
                    }
                }
            }
            // clic droit sur un point : suppression
            if resp.secondary_clicked() {
                if let Some(i) = resp.interact_pointer_pos().and_then(near) {
                    self.checkpoint();
                    self.auto_remove(ti, i);
                    st.hover = None;
                }
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(sid, st));
    }

    /// Dessin des bandes d'automation (fond, repères 0/50/100 %, courbe, points, fantôme).
    pub(crate) fn draw_automation(&self, ui: &egui::Ui, lp: &egui::Painter, lanes: Rect, sx: f64, sy: f32, ppb: f32) {
        let st: AutoUi = ui.ctx().data_mut(|d| d.get_temp(egui::Id::new("automation_ui_state"))).unwrap_or_default();
        let bx = |b: f64| lanes.left() + ((b - sx) as f32) * ppb;

        for (ti, t) in self.project.tracks.iter().enumerate() {
            let s = strip(lanes, ti, sy);
            if s.bottom() < lanes.top() || s.top() > lanes.bottom() {
                continue;
            }
            let sp = lp.with_clip_rect(s.intersect(lanes));
            sp.rect_filled(s, 0.0, tint(AUTO, 8));
            sp.line_segment([s.left_top(), s.right_top()], stroke(1.0, HAIR));

            let area = curve_area(s);

            // repères 0 / 50 / 100 % (pointillés) + étiquettes à gauche
            for (v, label) in [(1.0f32, "100%"), (0.5, "50%"), (0.0, "0%")] {
                let y = y_of(area, v);
                let col = if v == 0.5 { tint(GREY, 55) } else { tint(GREY, 95) };
                sp.extend(Shape::dashed_line(&[pos2(s.left(), y), pos2(s.right(), y)], stroke(1.0, col), 3.0, 4.0));
                let lab = Rect::from_min_size(pos2(s.left() + 4.0, y - 6.0), vec2(28.0, 12.0));
                sp.rect_filled(lab, 3.0, tint(Color32::WHITE, 215));
                sp.text(lab.center(), Align2::CENTER_CENTER, label, FontId::proportional(9.0), tint(GREY, 210));
            }

            let pts = lane_points(t);
            if pts.is_empty() {
                sp.text(pos2(s.left() + 44.0, s.center().y), Align2::LEFT_CENTER, "Click to draw volume automation", FontId::proportional(10.5), tint(GREY, 150));
            } else {
                let pos: Vec<Pos2> = pts.iter().map(|p| pos2(bx(p.beat), y_of(area, p.value))).collect();
                // la valeur se prolonge à plat avant le premier et après le dernier point
                let mut line = Vec::with_capacity(pos.len() + 2);
                line.push(pos2(s.left(), pos[0].y));
                line.extend(pos.iter().copied());
                line.push(pos2(s.right(), pos[pos.len() - 1].y));
                for w in line.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    sp.add(Shape::convex_polygon(vec![a, b, pos2(b.x, area.bottom()), pos2(a.x, area.bottom())], tint(AUTO, 38), Stroke::NONE));
                }
                sp.add(Shape::line(line, stroke(1.8, AUTO)));

                for (i, p) in pos.iter().enumerate() {
                    if p.x < lanes.left() - 6.0 || p.x > lanes.right() + 6.0 {
                        continue;
                    }
                    let dragging = st.drag == Some((ti, i));
                    let active = dragging || st.hover == Some((ti, i));
                    let r = if active { 5.5 } else { 4.0 };
                    sp.circle_filled(*p, r, Color32::WHITE);
                    sp.circle_stroke(*p, r, stroke(2.0, AUTO));
                    if active {
                        tag(&sp, *p, pts[i].value);
                    }
                }
            }

            // point fantôme : là où le prochain clic posera un point
            if let Some((gt, gb, gv)) = st.ghost {
                if gt == ti {
                    let g = pos2(bx(gb), y_of(area, gv));
                    sp.line_segment([pos2(g.x, s.top() + 2.0), pos2(g.x, s.bottom() - 2.0)], stroke(1.0, tint(AUTO, 70)));
                    sp.circle_filled(g, 4.5, tint(AUTO, 90));
                    sp.circle_stroke(g, 4.5, stroke(1.5, tint(AUTO, 210)));
                    tag(&sp, g, gv);
                }
            }
        }
    }
}