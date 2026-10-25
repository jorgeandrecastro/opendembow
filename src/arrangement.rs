//! Arrangement view: track headers (mixer strip), timeline and clips.

use crate::automation::AUTO_H;
use crate::instruments::Instrument;
use crate::loops::LOOP_H;
use crate::project::*;
use crate::theme::*;
use crate::{App, Drag};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Rounding, Sense};

const HDR_W: f32 = 250.0;
/// En-tête total : barre de loop (LOOP_H) + règle des mesures (30).
const RULER_H: f32 = LOOP_H + 30.0;
pub(crate) const TRACK_H: f32 = 112.0;
/// Zone de saisie du curseur rouge (playhead), en pixels de chaque côté.
const PH_GRAB: f32 = 5.0;

enum Act {
    Select(usize),
    Mute(usize),
    Solo(usize),
    Vol(usize, f32),
    Pan(usize, f32),
    Delete(usize),
    Inst(usize, Instrument),
    /// Déplace une piste (ancien index, nouvel index).
    Move(usize, usize),
}

/// Où atterrit un instrument glissé depuis le navigateur.
#[derive(Clone, Copy, PartialEq)]
enum DropAt {
    /// Remplace l'instrument de cette piste (les clips et les notes sont gardés).
    Replace(usize),
    /// Crée une nouvelle piste à cet emplacement (0 = tout en haut, nt = tout en bas).
    Insert(usize),
}

/// Ligne d'insertion bleue entre deux pistes, avec une étiquette.
fn draw_insert_line(p: &egui::Painter, rect: Rect, lanes: Rect, y: f32, label: &str) {
    let y = y.clamp(lanes.top() + 1.0, rect.bottom() - 3.0);
    let (x0, x1) = (rect.left() + 10.0, rect.right() - 10.0);
    p.line_segment([pos2(x0, y), pos2(x1, y)], stroke(5.0, tint(ACCENT, 40)));
    p.line_segment([pos2(x0, y), pos2(x1, y)], stroke(2.5, ACCENT));
    p.circle_filled(pos2(x0, y), 4.5, ACCENT);
    p.circle_filled(pos2(x1, y), 4.5, ACCENT);
    let tag = Rect::from_min_size(pos2(lanes.left() + 14.0, (y - 22.0).max(lanes.top() + 2.0)), vec2(150.0, 18.0));
    p.rect_filled(tag, 9.0, ACCENT);
    p.text(tag.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), Color32::WHITE);
}

/// Position en « mesure.temps.double-croche », comme dans la barre de transport.
fn bbt(b: f64) -> String {
    let b = b.max(0.0);
    format!("{}.{}.{}", (b / 4.0) as u32 + 1, (b % 4.0) as u32 + 1, ((b % 1.0) * 4.0) as u32 + 1)
}

/// Curseur horizontal (volume / pan). Double-clic : retour à `reset` s'il est donné.
#[allow(clippy::too_many_arguments)]
fn fader(ui: &mut egui::Ui, id: egui::Id, rect: Rect, v: f32, lo: f32, hi: f32, color: Color32, tip: &str, reset: Option<f32>) -> Option<f32> {
    let resp = ui.interact(rect, id, Sense::click_and_drag()).on_hover_text(tip);
    let mut out = None;
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let f = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            out = Some(lo + f * (hi - lo));
        }
    }
    if let (true, Some(r)) = (resp.double_clicked(), reset) {
        out = Some(r);
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    let f = ((out.unwrap_or(v) - lo) / (hi - lo)).clamp(0.0, 1.0);
    let p = ui.painter();
    let cy = rect.center().y;
    p.rect_filled(Rect::from_center_size(rect.center(), vec2(rect.width(), 4.0)), 2.0, HAIR);
    let kx = rect.left() + f * rect.width();
    p.rect_filled(Rect::from_min_max(pos2(rect.left(), cy - 2.0), pos2(kx, cy + 2.0)), 2.0, color);
    let r = if resp.hovered() || resp.dragged() { 8.0 } else { 7.0 };
    p.circle_filled(pos2(kx, cy), r, Color32::WHITE);
    p.circle_stroke(pos2(kx, cy), r, stroke(1.0, Color32::from_rgb(200, 200, 206)));
    out
}

fn pill(ui: &mut egui::Ui, id: egui::Id, rect: Rect, text: &str, on: bool, col: Color32, tip: &str) -> bool {
    let r = ui.interact(rect, id, Sense::click()).on_hover_text(tip);
    let (fill, fg) = if on {
        (col, Color32::WHITE)
    } else if r.hovered() {
        (Color32::from_rgb(224, 224, 231), INK)
    } else {
        (Color32::from_rgb(235, 235, 240), GREY)
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(11.5), fg);
    r.clicked()
}

impl App {
    pub fn ui_arrangement(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let lanes = Rect::from_min_max(pos2(rect.left() + HDR_W, rect.top() + RULER_H), rect.right_bottom());
        let loop_zone = Rect::from_min_max(pos2(rect.left() + HDR_W, rect.top()), pos2(rect.right(), rect.top() + LOOP_H));
        let ruler = Rect::from_min_max(pos2(rect.left() + HDR_W, rect.top() + LOOP_H), pos2(rect.right(), rect.top() + RULER_H));
        let nt = self.project.tracks.len();
        let (bpm, sr) = (self.project.bpm, self.project.sr);
        self.arr_rect = rect;
        self.lanes_rect = lanes;

        let lr = ui.interact(lanes, ui.id().with("lanes"), Sense::click_and_drag());
        let rr = ui.interact(ruler, ui.id().with("ruler"), Sense::click_and_drag());

        // zoom (pinch / Ctrl+wheel) and scroll (aussi au-dessus des bandes d'automation)
        if ui.rect_contains_pointer(lanes) || rr.hovered() {
            let (zoom, scroll, hover) = ui.input(|i| (i.zoom_delta(), i.raw_scroll_delta, i.pointer.hover_pos()));
            if zoom != 1.0 {
                if let Some(p) = hover {
                    let anchor = self.scroll_x + ((p.x - lanes.left()) / self.ppb) as f64;
                    self.ppb = (self.ppb * zoom).clamp(6.0, 240.0);
                    self.scroll_x = (anchor - ((p.x - lanes.left()) / self.ppb) as f64).max(0.0);
                }
            } else {
                self.scroll_x = (self.scroll_x - (scroll.x / self.ppb) as f64).max(0.0);
                let maxy = (nt as f32 * TRACK_H - lanes.height()).max(0.0);
                self.scroll_y = (self.scroll_y - scroll.y).clamp(0.0, maxy);
            }
        }
        let ppb = self.ppb;
        let visible = (lanes.width() / ppb) as f64;
        if self.playing && (self.pos_beats < self.scroll_x || self.pos_beats > self.scroll_x + visible) {
            self.scroll_x = self.pos_beats;
        }
        let (sx, sy) = (self.scroll_x, self.scroll_y);
        let bx = |b: f64| lanes.left() + ((b - sx) as f32) * ppb;
        let beat_at = |x: f32| ((x - lanes.left()) / ppb) as f64 + sx;
        let track_at = |y: f32| ((y - lanes.top() + sy) / TRACK_H).floor() as i64;
        let grid: f64 = if ppb >= 64.0 { 0.25 } else if ppb >= 28.0 { 0.5 } else { 1.0 };
        let free = ui.input(|i| i.modifiers.alt); // hold Alt: no snapping
        let snap = |b: f64| if free { b } else { (b / grid).round() * grid };
        let clip_at = |proj: &Project, p: Pos2| -> Option<(usize, usize)> {
            let ti = track_at(p.y);
            if ti < 0 {
                return None;
            }
            let ti = ti as usize;
            // la bande d'automation (bas de la piste) n'appartient pas aux clips
            if p.y - lanes.top() + sy - ti as f32 * TRACK_H >= TRACK_H - AUTO_H {
                return None;
            }
            let t = proj.tracks.get(ti)?;
            let b = beat_at(p.x);
            t.clips
                .iter()
                .position(|c| b >= c.start && b < c.start + c.len_beats(proj.bpm, proj.sr))
                .map(|ci| (ti, ci))
        };

        // which clip edge is under the pointer: Some((track, clip, is_left_edge))
        let edge_at = |proj: &Project, p: Pos2| -> Option<(usize, usize, bool)> {
            let (t, c) = clip_at(proj, p)?;
            let cl = &proj.tracks[t].clips[c];
            let (x0, x1) = (bx(cl.start), bx(cl.start + cl.len_beats(proj.bpm, proj.sr)));
            let z = 7.0f32.min((x1 - x0) / 3.0);
            if x1 - p.x <= z {
                Some((t, c, false))
            } else if p.x - x0 <= z {
                Some((t, c, true))
            } else {
                None
            }
        };

        // ---- curseur rouge (playhead) : saisissable à la main, dans la règle comme dans les lanes ----
        let scrub_id = egui::Id::new("playhead_scrub");
        let ptr = ui.input(|i| i.pointer.hover_pos());
        let hx0 = bx(self.pos_beats);
        let head_hover = ptr.map_or(false, |h| (h.x - hx0).abs() <= 9.0 && h.y >= ruler.top() && h.y <= ruler.bottom());
        let scrubbing_now: bool = ui.ctx().data_mut(|d| d.get_temp(scrub_id)).unwrap_or(false);
        let mut scrub_beat: Option<f64> = None;

        // curseurs de souris (la petite main)
        if rr.dragged() || scrubbing_now {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if rr.hovered() {
            ui.ctx().set_cursor_icon(if head_hover { CursorIcon::Grab } else { CursorIcon::PointingHand });
        } else if lr.hovered() || lr.dragged() {
            if let Some(h) = ptr {
                if (h.x - hx0).abs() <= PH_GRAB {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                } else if edge_at(&self.project, h).is_some() || matches!(self.drag, Drag::ClipEdge { .. }) {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                } else if matches!(self.drag, Drag::Clip { .. }) {
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                } else if clip_at(&self.project, h).is_some() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                } else if self.pen {
                    ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                }
            }
        }
        // clip survolé (léger éclaircissement)
        let hover_clip = if lr.hovered() && !lr.dragged() { ptr.and_then(|h| clip_at(&self.project, h)) } else { None };

        // interactions : clic / glisser dans la règle = déplacer le playhead (magnétisme sur la grille, Alt = libre)
        if rr.clicked() || rr.dragged() {
            if let Some(p) = rr.interact_pointer_pos() {
                let b = snap(beat_at(p.x)).max(0.0);
                self.engine.set_pos(b, bpm, sr);
                scrub_beat = Some(b);
            }
        }

        let mut acts: Vec<Act> = Vec::new();

        if lr.drag_started() {
            let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| lr.interact_pointer_pos());
            if let Some(p) = origin {
                if (p.x - hx0).abs() <= PH_GRAB {
                    // on attrape le curseur rouge
                    ui.ctx().data_mut(|d| d.insert_temp(scrub_id, true));
                } else if let Some((t, c, left)) = edge_at(&self.project, p) {
                    self.checkpoint();
                    self.select_clip(t, c);
                    self.edge_orig = Some(self.project.tracks[t].clips[c].clone());
                    self.drag = Drag::ClipEdge { t, c, left };
                } else if let Some((t, c)) = clip_at(&self.project, p) {
                    self.checkpoint();
                    self.select_clip(t, c);
                    let st = self.project.tracks[t].clips[c].start;
                    self.drag = Drag::Clip { t, c, grab: beat_at(p.x) - st };
                } else {
                    let ti = track_at(p.y);
                    if ti >= 0 && (ti as usize) < nt {
                        let ti = ti as usize;
                        self.sel_track = ti;
                        if self.pen && matches!(self.project.tracks[ti].kind, TrackKind::Instrument(_)) {
                            self.checkpoint();
                            let start = ((beat_at(p.x) / grid).floor() * grid).max(0.0);
                            let clip = Clip { name: "Clip".into(), start, len: grid.max(0.25), kind: ClipKind::Midi(Vec::new()) };
                            self.project.tracks[ti].clips.push(clip.clone());
                            let n = self.project.tracks[ti].clips.len() - 1;
                            self.select_clip(ti, n);
                            self.edge_orig = Some(clip);
                            self.drag = Drag::ClipEdge { t: ti, c: n, left: false };
                            self.push();
                        }
                    }
                }
            }
        }
        if lr.dragged() {
            if let Some(p) = lr.interact_pointer_pos() {
                let scrubbing: bool = ui.ctx().data_mut(|d| d.get_temp(scrub_id)).unwrap_or(false);
                if scrubbing {
                    let b = snap(beat_at(p.x)).max(0.0);
                    self.engine.set_pos(b, bpm, sr);
                    scrub_beat = Some(b);
                } else {
                    match self.drag {
                        Drag::Clip { t, c, grab } => {
                            let b = snap(beat_at(p.x) - grab).max(0.0);
                            self.project.tracks[t].clips[c].start = b;
                            self.push();
                        }
                        Drag::ClipEdge { t, c, left } => {
                            if let Some(orig) = self.edge_orig.clone() {
                                let b = snap(beat_at(p.x)).max(0.0);
                                if let Some(nc) = crate::edit::resize_clip(&orig, left, b, bpm, sr) {
                                    if let Some(slot) = self.project.tracks.get_mut(t).and_then(|tr| tr.clips.get_mut(c)) {
                                        *slot = nc;
                                    }
                                    self.push();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if lr.drag_stopped() {
            ui.ctx().data_mut(|d| d.insert_temp(scrub_id, false));
            if matches!(self.drag, Drag::Clip { .. } | Drag::ClipEdge { .. }) {
                self.drag = Drag::None;
                self.edge_orig = None;
            }
        }
        if lr.clicked() {
            if let Some(p) = lr.interact_pointer_pos() {
                let ctrl = ui.input(|i| i.modifiers.command);
                let ti = track_at(p.y);
                if ti >= 0 && (ti as usize) < nt {
                    let ti = ti as usize;
                    if let Some((t, c)) = clip_at(&self.project, p) {
                        self.select_clip(t, c);
                        if ctrl {
                            if let Err(e) = self.split_clip_at(t, c, snap(beat_at(p.x))) {
                                self.status = format!("Cannot cut: {e}");
                            }
                        }
                    } else {
                        self.sel_track = ti;
                        if self.pen {
                            let start = (beat_at(p.x) / 4.0).floor().max(0.0) * 4.0;
                            self.add_clip(ti, start, 4.0);
                        } else {
                            self.sel_clip = None;
                            self.sel_note = None;
                        }
                    }
                }
            }
        }
        if lr.double_clicked() {
            if let Some(p) = lr.interact_pointer_pos() {
                let ti = track_at(p.y);
                if clip_at(&self.project, p).is_none() && ti >= 0 && (ti as usize) < nt {
                    let ti = ti as usize;
                    if let TrackKind::Instrument(_) = self.project.tracks[ti].kind {
                        self.checkpoint();
                        let start = (beat_at(p.x) / 4.0).floor().max(0.0) * 4.0;
                        let n = self.project.tracks[ti].clips.len();
                        self.project.tracks[ti].clips.push(Clip { name: "Clip".into(), start, len: 4.0, kind: ClipKind::Midi(Vec::new()) });
                        self.select_clip(ti, n);
                        self.push();
                    }
                }
            }
        }

        // bandes d'automation : zones d'interaction posées par-dessus les lanes (voir automation.rs)
        self.automation_interact(ui, lanes, sx, sy, ppb);

        // drawing
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 14.0, Color32::WHITE);
        p.rect_stroke(rect, 14.0, stroke(1.0, HAIR));
        let lp = ui.painter_at(lanes);

        for i in 0..nt {
            let y = lanes.top() + i as f32 * TRACK_H - sy;
            let r = Rect::from_min_size(pos2(lanes.left(), y), vec2(lanes.width(), TRACK_H));
            let col = if i == self.sel_track {
                tint(ACCENT, 10)
            } else if i % 2 == 1 {
                Color32::from_rgb(250, 250, 252)
            } else {
                Color32::WHITE
            };
            lp.rect_filled(r, 0.0, col);
            lp.line_segment([pos2(r.left(), r.bottom()), pos2(r.right(), r.bottom())], stroke(1.0, HAIR));
        }

        let rp = ui.painter_at(ruler);
        let b0 = sx.floor() as i64;
        let b1 = (sx + visible).ceil() as i64;
        for b in b0..=b1 {
            let x = bx(b as f64);
            if b.rem_euclid(4) == 0 {
                lp.line_segment([pos2(x, lanes.top()), pos2(x, lanes.bottom())], stroke(1.0, Color32::from_rgb(214, 214, 220)));
                rp.line_segment([pos2(x, ruler.top() + 8.0), pos2(x, ruler.bottom())], stroke(1.0, HAIR));
                rp.text(pos2(x + 5.0, ruler.center().y), Align2::LEFT_CENTER, format!("{}", b / 4 + 1), FontId::proportional(12.0), Color32::from_rgb(110, 110, 120));
            } else if ppb >= 18.0 {
                lp.line_segment([pos2(x, lanes.top()), pos2(x, lanes.bottom())], stroke(1.0, Color32::from_rgb(238, 238, 242)));
                // petite graduation de temps dans la règle
                rp.line_segment([pos2(x, ruler.bottom() - 6.0), pos2(x, ruler.bottom())], stroke(1.0, HAIR));
            }
        }
        // repère de survol : montre où le clic placera le playhead
        if rr.hovered() && !rr.dragged() {
            if let Some(h) = ptr {
                let gxp = bx(snap(beat_at(h.x)).max(0.0));
                if gxp >= lanes.left() && gxp <= lanes.right() {
                    rp.line_segment([pos2(gxp, ruler.top() + 16.0), pos2(gxp, ruler.bottom())], stroke(1.0, tint(INK, 60)));
                    lp.line_segment([pos2(gxp, lanes.top()), pos2(gxp, lanes.bottom())], stroke(1.0, tint(INK, 22)));
                }
            }
        }
        p.line_segment([pos2(rect.left(), lanes.top()), pos2(rect.right(), lanes.top())], stroke(1.0, HAIR));
        p.text(pos2(rect.left() + 18.0, ruler.center().y), Align2::LEFT_CENTER, "TRACKS", FontId::proportional(11.0), GREY);
        if nt == 0 {
            p.text(lanes.center(), Align2::CENTER_CENTER, "Add an instrument from the left to begin", FontId::proportional(15.0), GREY);
        }

        // barre de loop (en haut), voile sur la région bouclée, bandes d'automation (voir loops.rs / automation.rs)
        self.ui_loop_zone(ui, loop_zone, sx, ppb);
        self.draw_loop_overlay(ui, &lp, lanes, sx, ppb);
        self.draw_automation(ui, &lp, lanes, sx, sy, ppb);

        // clips
        let spb = 60.0 / bpm * sr as f64;
        for (ti, t) in self.project.tracks.iter().enumerate() {
            let y = lanes.top() + ti as f32 * TRACK_H - sy;
            let col = PALETTE[t.color % 8];

            // piste vide : un indice discret pour savoir quoi faire
            if t.clips.is_empty() && matches!(t.kind, TrackKind::Instrument(_)) {
                lp.text(
                    pos2(lanes.left() + 16.0, y + (TRACK_H - AUTO_H) / 2.0),
                    Align2::LEFT_CENTER,
                    "Double-click to create a clip",
                    FontId::proportional(11.5),
                    tint(GREY, 130),
                );
            }

            for (ci, c) in t.clips.iter().enumerate() {
                let (x0, x1) = (bx(c.start), bx(c.start + c.len_beats(bpm, sr)));
                if x1 < lanes.left() || x0 > lanes.right() {
                    continue;
                }
                let hov = hover_clip == Some((ti, ci));
                let cr = Rect::from_min_max(pos2(x0, y + 7.0), pos2(x1, y + TRACK_H - AUTO_H - 7.0)); // laisse la place à l'automation en bas
                let cp = lp.with_clip_rect(cr.intersect(lanes));
                cp.rect_filled(cr, 8.0, tint(col, if hov { 88 } else { 64 }));
                if hov {
                    cp.rect_stroke(cr, 8.0, stroke(1.0, tint(col, 150)));
                }
                let hdr = Rect::from_min_size(cr.min, vec2(cr.width(), 18.0));
                cp.rect_filled(hdr, Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, col);
                cp.text(pos2(hdr.left() + 7.0, hdr.center().y), Align2::LEFT_CENTER, &c.name, FontId::proportional(11.0), Color32::WHITE);
                let body = Rect::from_min_max(pos2(cr.left(), cr.top() + 21.0), pos2(cr.right(), cr.bottom() - 4.0));
                match &c.kind {
                    ClipKind::Midi(notes) => {
                        if !notes.is_empty() {
                            let lo = notes.iter().map(|n| n.key).min().unwrap_or(0) as f32;
                            let hi = notes.iter().map(|n| n.key).max().unwrap_or(0) as f32;
                            let span = (hi - lo).max(11.0) + 1.0;
                            let h = (body.height() / span).clamp(2.0, 6.0);
                            for n in notes {
                                let ny = body.bottom() - (n.key as f32 - lo + 1.0) / span * body.height();
                                let nr = Rect::from_min_max(pos2(bx(c.start + n.start), ny), pos2(bx(c.start + n.start + n.len), ny + h));
                                cp.rect_filled(nr, 1.5, col);
                            }
                        }
                    }
                    ClipKind::Audio(data) => {
                        let mid = body.center().y;
                        let mut x = x0.max(lanes.left());
                        while x < x1.min(lanes.right()) {
                            let s0 = ((beat_at(x) - c.start) * spb).max(0.0) as usize;
                            let s1 = (((beat_at(x + 2.0) - c.start) * spb).max(0.0) as usize).max(s0 + 1).min(data.len());
                            if s0 >= data.len() {
                                break;
                            }
                            let stride = ((s1 - s0) / 48).max(1);
                            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                            let mut i = s0;
                            while i < s1 {
                                lo = lo.min(data[i]);
                                hi = hi.max(data[i]);
                                i += stride;
                            }
                            let hh = body.height() * 0.5;
                            cp.line_segment([pos2(x, mid - hi.min(1.0) * hh), pos2(x, (mid - lo.max(-1.0) * hh).max(mid - hi * hh + 1.0))], stroke(1.5, col));
                            x += 2.0;
                        }
                    }
                }
                if self.sel_clip == Some((ti, ci)) {
                    cp.rect_stroke(cr, 8.0, stroke(2.0, INK));
                    for gx in [cr.left() + 3.5, cr.right() - 3.5] {
                        cp.rect_filled(Rect::from_center_size(pos2(gx, cr.center().y + 8.0), vec2(3.0, 20.0)), 1.5, INK);
                    }
                }
            }
        }

        // track headers : dessinés même à moitié visibles (rognés), pour qu'aucune piste ne reste « fantôme »
        let reorder_id = egui::Id::new("track_reorder");
        let reorder_from: Option<usize> = ui.ctx().data_mut(|d| d.get_temp::<Option<usize>>(reorder_id)).flatten();
        let hdr_area = Rect::from_min_max(pos2(rect.left(), lanes.top()), pos2(lanes.left(), rect.bottom()));
        let hp = ui.painter_at(hdr_area);
        let old_clip = ui.clip_rect();
        ui.set_clip_rect(old_clip.intersect(hdr_area));
        for (i, t) in self.project.tracks.iter().enumerate() {
            let y = rect.top() + RULER_H + i as f32 * TRACK_H - sy;
            if y + TRACK_H < lanes.top() || y > rect.bottom() {
                continue;
            }
            let hr = Rect::from_min_size(pos2(rect.left() + 8.0, y + 5.0), vec2(HDR_W - 16.0, TRACK_H - 10.0));
            let col = PALETTE[t.color % 8];
            let hresp = ui.interact(hr, ui.id().with(("hdr", i)), Sense::click_and_drag());
            if hresp.clicked() {
                acts.push(Act::Select(i));
            }
            // glisser l'en-tête pour changer l'ordre des pistes (main ouverte au survol)
            if hresp.drag_started() {
                ui.ctx().data_mut(|d| d.insert_temp(reorder_id, Some(i)));
                acts.push(Act::Select(i));
            }
            if hresp.hovered() && reorder_from.is_none() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
            hresp.context_menu(|ui| {
                if ui.button("Delete track").clicked() {
                    acts.push(Act::Delete(i));
                    ui.close_menu();
                }
                if let TrackKind::Instrument(_) = t.kind {
                    ui.menu_button("Change instrument", |ui| {
                        for inst in Instrument::ALL {
                            if ui.button(inst.name()).clicked() {
                                acts.push(Act::Inst(i, inst));
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
            let sel = i == self.sel_track;
            hp.rect_filled(hr, 12.0, if sel { tint(ACCENT, 14) } else { Color32::from_rgb(250, 250, 252) });
            hp.rect_stroke(hr, 12.0, stroke(if sel { 1.5 } else { 1.0 }, if sel { ACCENT } else { HAIR }));
            hp.rect_filled(Rect::from_min_size(pos2(hr.left() + 8.0, hr.top() + 10.0), vec2(4.0, hr.height() - 20.0)), 2.0, col);
            hp.text(pos2(hr.left() + 20.0, hr.top() + 15.0), Align2::LEFT_CENTER, &t.name, FontId::proportional(13.5), INK);
            let sub = match t.kind {
                TrackKind::Instrument(inst) => inst.blurb(),
                TrackKind::Audio => "Audio",
            };
            hp.text(pos2(hr.left() + 20.0, hr.top() + 31.0), Align2::LEFT_CENTER, sub, FontId::proportional(11.0), GREY);
            let m = Rect::from_min_size(pos2(hr.right() - 92.0, hr.top() + 7.0), vec2(24.0, 20.0));
            let s = m.translate(vec2(28.0, 0.0));
            if pill(ui, ui.id().with(("m", i)), m, "M", t.mute, Color32::from_rgb(255, 149, 0), "Mute") {
                acts.push(Act::Mute(i));
            }
            if pill(ui, ui.id().with(("s", i)), s, "S", t.solo, Color32::from_rgb(255, 204, 0), "Solo") {
                acts.push(Act::Solo(i));
            }
            let xr = Rect::from_min_size(pos2(hr.right() - 34.0, hr.top() + 7.0), vec2(22.0, 20.0));
            let xresp = ui.interact(xr, ui.id().with(("x", i)), Sense::click()).on_hover_text("Delete track (Ctrl+Z to undo)");
            if xresp.hovered() {
                hp.rect_filled(xr, 6.0, tint(RED, 30));
            }
            hp.text(xr.center(), Align2::CENTER_CENTER, "×", FontId::proportional(17.0), if xresp.hovered() { RED } else { GREY });
            if xresp.clicked() {
                acts.push(Act::Delete(i));
            }
            let level = crate::meters::norm(self.meters.get(i).copied().unwrap_or(0.0));
            hp.rect_filled(Rect::from_min_size(pos2(hr.left() + 20.0, hr.bottom() - 33.0), vec2(96.0 * level, 3.0)), 1.5, Color32::from_rgb(52, 199, 89));
            let vr = Rect::from_min_size(pos2(hr.left() + 20.0, hr.bottom() - 27.0), vec2(96.0, 18.0));
            if let Some(v) = fader(ui, ui.id().with(("v", i)), vr, t.volume, 0.0, 1.25, col, "Volume", None) {
                acts.push(Act::Vol(i, v));
            }
            let pr = Rect::from_min_size(pos2(hr.right() - 92.0, hr.bottom() - 27.0), vec2(76.0, 18.0));
            if let Some(v) = fader(ui, ui.id().with(("p", i)), pr, t.pan, -1.0, 1.0, GREY, "Pan (double-click to center)", Some(0.0)) {
                acts.push(Act::Pan(i, v));
            }
            // la piste qu'on déplace est estompée
            if reorder_from == Some(i) {
                hp.rect_filled(hr, 12.0, tint(Color32::WHITE, 150));
            }
        }

        ui.set_clip_rect(old_clip);

        // ---- réorganiser les pistes : une ligne bleue montre où la piste ira ----
        if let Some(from) = reorder_from {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            ui.ctx().request_repaint();
            let released = ui.input(|i| i.pointer.any_released()) || !ui.input(|i| i.pointer.any_down());
            match ui.input(|i| i.pointer.latest_pos()) {
                Some(h) if !released => {
                    let rel = (h.y - lanes.top() + sy) / TRACK_H;
                    let gap = (rel.round() as i64).clamp(0, nt as i64) as usize;
                    let to = if gap > from { gap - 1 } else { gap };
                    if to != from {
                        draw_insert_line(&p, rect, lanes, lanes.top() + gap as f32 * TRACK_H - sy, "Move track here");
                    }
                    if let Some(t) = self.project.tracks.get(from) {
                        let gp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("track_drag_ghost")));
                        let r = Rect::from_min_size(h + vec2(14.0, 8.0), vec2(190.0, 40.0));
                        gp.rect_filled(r.translate(vec2(0.0, 3.0)).expand(2.0), 14.0, Color32::from_black_alpha(28));
                        gp.rect_filled(r, 12.0, Color32::WHITE);
                        gp.rect_stroke(r, 12.0, stroke(1.5, ACCENT));
                        gp.rect_filled(Rect::from_min_size(pos2(r.left() + 8.0, r.top() + 8.0), vec2(4.0, 24.0)), 2.0, PALETTE[t.color % 8]);
                        gp.text(pos2(r.left() + 20.0, r.center().y), Align2::LEFT_CENTER, &t.name, FontId::proportional(13.0), INK);
                    }
                }
                last => {
                    // relâché : on déplace la piste (si la souris est encore dans la fenêtre)
                    ui.ctx().data_mut(|d| d.insert_temp(reorder_id, None::<usize>));
                    if let Some(h) = last {
                        let rel = (h.y - lanes.top() + sy) / TRACK_H;
                        let gap = (rel.round() as i64).clamp(0, nt as i64) as usize;
                        let to = if gap > from { gap - 1 } else { gap };
                        if to != from {
                            acts.push(Act::Move(from, to));
                        }
                    }
                }
            }
        }

        for a in acts {
            match a {
                Act::Select(i) => {
                    self.sel_track = i;
                }
                Act::Mute(i) => {
                    self.project.tracks[i].mute ^= true;
                    self.push();
                }
                Act::Solo(i) => {
                    self.project.tracks[i].solo ^= true;
                    self.push();
                }
                Act::Vol(i, v) => {
                    self.project.tracks[i].volume = v;
                    self.push();
                }
                Act::Pan(i, v) => {
                    self.project.tracks[i].pan = if v.abs() < 0.06 { 0.0 } else { v };
                    self.push();
                }
                Act::Delete(i) => {
                    self.checkpoint();
                    self.project.tracks.remove(i);
                    self.fix_selection();
                    self.push();
                }
                Act::Inst(i, inst) => self.replace_instrument(i, inst),
                Act::Move(from, to) => {
                    let n = self.project.tracks.len();
                    if from != to && from < n && to < n {
                        self.checkpoint();
                        let tr = self.project.tracks.remove(from);
                        self.project.tracks.insert(to, tr);
                        // les sélections suivent les pistes
                        let map = |i: usize| -> usize {
                            if i == from {
                                to
                            } else if from < to && i > from && i <= to {
                                i - 1
                            } else if from > to && i >= to && i < from {
                                i + 1
                            } else {
                                i
                            }
                        };
                        self.sel_track = map(self.sel_track);
                        if let Some((t, c)) = self.sel_clip {
                            self.sel_clip = Some((map(t), c));
                        }
                        self.push();
                        self.status = "Track moved.".into();
                    }
                }
            }
        }

        // cible du dépôt pendant qu'on glisse un instrument depuis le navigateur
        if self.drag_inst.is_some() {
            p.rect_stroke(rect, 14.0, stroke(2.0, ACCENT));
            if let Some(h) = ui.input(|i| i.pointer.latest_pos()) {
                match self.drop_target(h) {
                    Some(DropAt::Replace(ti)) => {
                        let y = lanes.top() + ti as f32 * TRACK_H - sy;
                        lp.rect_filled(Rect::from_min_size(pos2(lanes.left(), y), vec2(lanes.width(), TRACK_H)), 0.0, tint(ACCENT, 30));
                        lp.text(pos2(lanes.left() + 18.0, y + TRACK_H / 2.0), Align2::LEFT_CENTER, "Drop to replace the instrument (notes are kept)", FontId::proportional(13.0), ACCENT);
                    }
                    Some(DropAt::Insert(at)) => {
                        draw_insert_line(&p, rect, lanes, lanes.top() + at as f32 * TRACK_H - sy, "New track here");
                    }
                    None => {}
                }
            }
        }

        // indicateur de défilement vertical (quand il y a plus de pistes que de place)
        let content_h = nt as f32 * TRACK_H;
        if content_h > lanes.height() + 1.0 {
            let th = (lanes.height() / content_h * lanes.height()).max(24.0);
            let ty = lanes.top() + (sy / content_h) * lanes.height();
            lp.rect_filled(Rect::from_min_size(pos2(lanes.right() - 7.0, ty), vec2(4.0, th)), 2.0, tint(INK, 60));
        }

        // ---- playhead : tête en drapeau + ligne + étiquette de position ----
        let ph = scrub_beat.unwrap_or(self.pos_beats);
        let hx = bx(ph);
        if hx >= lanes.left() - 8.0 && hx <= lanes.right() + 8.0 {
            let pp = p.with_clip_rect(Rect::from_min_max(pos2(lanes.left(), rect.top()), rect.right_bottom()));
            let active = scrub_beat.is_some() || scrubbing_now || head_hover;
            // halo + ligne
            pp.line_segment([pos2(hx, ruler.top() + 14.0), pos2(hx, lanes.bottom())], stroke(if active { 7.0 } else { 5.0 }, tint(RED, 26)));
            pp.line_segment([pos2(hx, ruler.top() + 14.0), pos2(hx, lanes.bottom())], stroke(1.6, RED));
            // tête : petit drapeau arrondi avec pointe vers le bas
            let top = ruler.top() + 3.0;
            let flag = Rect::from_min_size(pos2(hx - 7.0, top), vec2(14.0, 11.0));
            pp.rect_filled(flag, 3.0, RED);
            pp.add(egui::Shape::convex_polygon(
                vec![pos2(hx - 4.5, top + 10.0), pos2(hx + 4.5, top + 10.0), pos2(hx, top + 16.0)],
                RED,
                egui::Stroke::NONE,
            ));
            pp.line_segment([pos2(hx - 2.0, top + 3.0), pos2(hx - 2.0, top + 8.0)], stroke(1.0, tint(Color32::WHITE, 170)));
            pp.line_segment([pos2(hx + 2.0, top + 3.0), pos2(hx + 2.0, top + 8.0)], stroke(1.0, tint(Color32::WHITE, 170)));
            // étiquette de position quand on survole ou déplace le curseur
            if active {
                let (lw, lh) = (58.0, 18.0);
                let lx = if hx + 14.0 + lw > lanes.right() { hx - 14.0 - lw } else { hx + 14.0 };
                let tag = Rect::from_min_size(pos2(lx, ruler.top() + 6.0), vec2(lw, lh));
                pp.rect_filled(tag, 9.0, INK);
                pp.text(tag.center(), Align2::CENTER_CENTER, bbt(ph), FontId::monospace(11.0), Color32::WHITE);
            }
        }
    }
}

impl App {
    pub fn add_clip(&mut self, ti: usize, start: f64, len: f64) -> bool {
        if !matches!(self.project.tracks.get(ti).map(|t| t.kind), Some(TrackKind::Instrument(_))) {
            return false;
        }
        self.checkpoint();
        let n = self.project.tracks[ti].clips.len();
        self.project.tracks[ti].clips.push(Clip { name: "Clip".into(), start, len, kind: ClipKind::Midi(Vec::new()) });
        self.select_clip(ti, n);
        self.push();
        true
    }
}

impl App {
    /// Où irait un instrument lâché à `pos` : remplacer (milieu d'une piste d'instrument),
    /// insérer entre deux pistes (bords haut / bas d'une piste), ou annuler (à gauche de la timeline).
    fn drop_target(&self, pos: Pos2) -> Option<DropAt> {
        let arr = self.arr_rect;
        if pos.x < arr.left() || pos.y < arr.top() {
            return None;
        }
        let lanes = self.lanes_rect;
        let nt = self.project.tracks.len();
        let rel = (pos.y - lanes.top() + self.scroll_y) / TRACK_H;
        if rel < 0.0 {
            return Some(DropAt::Insert(0));
        }
        let ti = rel.floor() as usize;
        if ti >= nt {
            return Some(DropAt::Insert(nt));
        }
        let f = rel - ti as f32;
        let is_inst = matches!(self.project.tracks[ti].kind, TrackKind::Instrument(_));
        Some(if f < 0.25 {
            DropAt::Insert(ti)
        } else if f > 0.75 {
            DropAt::Insert(ti + 1)
        } else if is_inst {
            DropAt::Replace(ti)
        } else if f < 0.5 {
            DropAt::Insert(ti)
        } else {
            DropAt::Insert(ti + 1)
        })
    }

    /// Change l'instrument d'une piste en gardant ses clips et ses notes (comme dans Ableton).
    /// Le nom de la piste ne change que s'il était celui de l'ancien instrument.
    pub fn replace_instrument(&mut self, ti: usize, inst: Instrument) {
        if ti >= self.project.tracks.len() {
            return;
        }
        self.checkpoint();
        let t = &mut self.project.tracks[ti];
        if let TrackKind::Instrument(old) = t.kind {
            if t.name == old.name() {
                t.name = inst.name().into();
            }
        }
        t.kind = TrackKind::Instrument(inst);
        self.sel_track = ti;
        self.push();
        self.status = format!("Instrument changed to {} - notes and clips are kept.", inst.name());
    }

    /// Nouvelle piste d'instrument à l'emplacement `at`, avec un clip vide de 4 temps à `beat`.
    fn insert_track(&mut self, inst: Instrument, at: usize, beat: f64) {
        let at = at.min(self.project.tracks.len());
        self.checkpoint();
        let mut tr = Track::new(inst.name(), TrackKind::Instrument(inst), self.project.tracks.len() % 8);
        tr.clips.push(Clip { name: "Clip".into(), start: beat, len: 4.0, kind: ClipKind::Midi(Vec::new()) });
        self.project.tracks.insert(at, tr);
        self.select_clip(at, 0);
        self.push();
        self.status = format!("New track: {}", inst.name());
    }

    pub fn drop_instrument(&mut self, inst: Instrument, pos: Pos2) {
        let Some(target) = self.drop_target(pos) else {
            self.status = "Drop cancelled.".into();
            return;
        };
        match target {
            DropAt::Replace(ti) => self.replace_instrument(ti, inst),
            DropAt::Insert(at) => {
                let lanes = self.lanes_rect;
                let beat = if pos.x >= lanes.left() {
                    ((((pos.x - lanes.left()) / self.ppb) as f64 + self.scroll_x) / 4.0).floor().max(0.0) * 4.0
                } else {
                    0.0
                };
                self.insert_track(inst, at, beat);
            }
        }
    }
}