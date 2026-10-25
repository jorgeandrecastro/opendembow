//! Loops façon Ableton : une barre en haut de la timeline (la « brace ») où l'on choisit
//! la partie qui se répète.
//!
//! - glisser dans la barre : dessine une nouvelle zone de loop
//! - glisser la brace : la déplace ; glisser ses bords : la redimensionne (Alt = sans magnétisme)
//! - clic sur la brace ou Ctrl+L : loop on / off ; Ctrl+Maj+L : boucle le clip sélectionné
//! - clic droit sur la barre : menu (on/off, clip sélectionné, 1 / 2 / 4 / 8 mesures depuis le playhead)
//!
//! L'état est gardé dans la mémoire temporaire d'egui (rien à changer dans `App`).
//! Le bouclage se fait dans `loop_tick`, une fois par image : quand le playhead franchit la fin
//! de la zone, on le replace au début en reportant le petit dépassement (pas de dérive).

use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Pos2, Rect, RichText, Rounding, Sense};

/// Hauteur de la barre de loop (au-dessus de la règle des mesures).
pub(crate) const LOOP_H: f32 = 16.0;
/// Ambre : distinct du playhead (rouge), de l'accent (bleu) et de l'automation (rose).
const AMBER: Color32 = Color32::from_rgb(255, 159, 10);
/// Longueur minimale d'une loop, en temps.
const MIN_LEN: f64 = 1.0;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Left,
    Right,
    Body,
    New,
}

#[derive(Clone, Copy)]
struct DragInfo {
    mode: Mode,
    /// décalage entre le curseur et le début de la loop (déplacement)
    grab: f64,
    /// point d'ancrage (nouvelle zone)
    anchor: f64,
    /// longueur de la loop au début du glisser (déplacement)
    len: f64,
}

#[derive(Clone, Copy)]
struct LoopState {
    on: bool,
    start: f64,
    end: f64,
    /// position du playhead à l'image précédente
    last: f64,
    drag: Option<DragInfo>,
}

impl Default for LoopState {
    fn default() -> Self {
        Self { on: false, start: 0.0, end: 8.0, last: 0.0, drag: None }
    }
}

enum MenuAct {
    Toggle,
    Clip,
    Bars(u32),
}

fn sid() -> egui::Id {
    egui::Id::new("loop_state")
}

fn get(ctx: &egui::Context) -> LoopState {
    ctx.data_mut(|d| d.get_temp::<LoopState>(sid())).unwrap_or_default()
}

fn put(ctx: &egui::Context, st: LoopState) {
    ctx.data_mut(|d| d.insert_temp(sid(), st));
}

fn bars(st: &LoopState) -> String {
    format!("bars {}-{}", (st.start / 4.0).floor() as i64 + 1, ((st.end / 4.0).ceil() as i64).max(1))
}

impl App {
    /// Début / fin (en temps) du clip sélectionné.
    fn sel_clip_span(&self) -> Option<(f64, f64)> {
        let (t, c) = self.sel_clip?;
        let clip = self.project.tracks.get(t)?.clips.get(c)?;
        Some((clip.start, clip.start + clip.len_beats(self.project.bpm, self.project.sr)))
    }

    pub(crate) fn toggle_loop(&mut self, ctx: &egui::Context) {
        let mut st = get(ctx);
        st.on = !st.on;
        self.status = if st.on { format!("Loop on ({}).", bars(&st)) } else { "Loop off.".into() };
        put(ctx, st);
    }

    /// Ctrl+Maj+L : la loop prend la taille du clip sélectionné et s'active.
    pub(crate) fn loop_to_selection(&mut self, ctx: &egui::Context) {
        match self.sel_clip_span() {
            Some((s, e)) if e - s >= MIN_LEN => {
                let mut st = get(ctx);
                st.start = s;
                st.end = e;
                st.on = true;
                self.status = format!("Looping the selected clip ({}).", bars(&st));
                put(ctx, st);
            }
            _ => self.status = "Select a clip to loop it.".into(),
        }
    }

    /// À appeler une fois par image (dans `update`, après le calcul de `pos_beats`).
    pub(crate) fn loop_tick(&mut self, ctx: &egui::Context) {
        let mut st = get(ctx);
        let pos = self.pos_beats;
        if st.on && self.playing && st.end > st.start && st.last < st.end && pos >= st.end && pos - st.last < 2.0 {
            // le playhead vient de franchir la fin : retour au début, en gardant le dépassement
            let target = (st.start + (pos - st.end)).min(st.end - 0.001);
            self.engine.set_pos(target, self.project.bpm, self.project.sr);
            self.pos_beats = target;
            st.last = target;
        } else {
            st.last = pos;
        }
        put(ctx, st);
    }

    /// Bouton « Loop » de la barre de transport.
    pub(crate) fn loop_button(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let on = get(&ctx).on;
        let b = egui::Button::new(RichText::new("Loop").size(12.5).color(if on { Color32::WHITE } else { INK }))
            .fill(if on { AMBER } else { Color32::from_rgb(243, 243, 247) })
            .rounding(Rounding::same(14.0))
            .min_size(vec2(52.0, 28.0));
        if ui.add(b).on_hover_text("Loop on / off (Ctrl+L). Set the range in the bar above the timeline.").clicked() {
            self.toggle_loop(&ctx);
        }
    }

    /// Voile ambre sur la région bouclée (à dessiner avant les clips).
    pub(crate) fn draw_loop_overlay(&self, ui: &egui::Ui, lp: &egui::Painter, lanes: Rect, sx: f64, ppb: f32) {
        let st = get(ui.ctx());
        if !st.on {
            return;
        }
        let bx = |b: f64| lanes.left() + ((b - sx) as f32) * ppb;
        let (x0, x1) = (bx(st.start), bx(st.end));
        let r = Rect::from_min_max(pos2(x0.max(lanes.left()), lanes.top()), pos2(x1.min(lanes.right()), lanes.bottom()));
        if r.width() > 0.0 {
            lp.rect_filled(r, 0.0, tint(AMBER, 16));
        }
        for x in [x0, x1] {
            if x >= lanes.left() && x <= lanes.right() {
                lp.line_segment([pos2(x, lanes.top()), pos2(x, lanes.bottom())], stroke(1.5, tint(AMBER, 170)));
            }
        }
    }

    /// La barre de loop : dessin + interactions (à appeler après le fond blanc de l'arrangement).
    pub(crate) fn ui_loop_zone(&mut self, ui: &mut egui::Ui, zone: Rect, sx: f64, ppb: f32) {
        let ctx = ui.ctx().clone();
        let mut st = get(&ctx);
        let bx = |b: f64| zone.left() + ((b - sx) as f32) * ppb;
        let beat_at = |x: f32| ((x - zone.left()) / ppb) as f64 + sx;
        // magnétisme : au temps (ou à la mesure quand on est très dézoomé), Alt = libre
        let step = if ui.input(|i| i.modifiers.alt) { 0.0 } else if ppb >= 14.0 { 1.0 } else { 4.0 };
        let snap = |b: f64| if step > 0.0 { (b / step).round() * step } else { b };

        let resp = ui.interact(zone, ui.id().with("loop_zone"), Sense::click_and_drag());
        let (x0, x1) = (bx(st.start), bx(st.end));
        let hw = 7.0f32.min(((x1 - x0) / 3.0).max(2.0));
        let hit = |p: Pos2| -> Mode {
            if p.x >= x0 - 3.0 && p.x <= x0 + hw {
                Mode::Left
            } else if p.x >= x1 - hw && p.x <= x1 + 3.0 {
                Mode::Right
            } else if p.x > x0 && p.x < x1 {
                Mode::Body
            } else {
                Mode::New
            }
        };

        // curseur
        let mode_now = st.drag.map(|d| d.mode).or_else(|| resp.hover_pos().map(|p| hit(p)));
        match mode_now {
            Some(Mode::Left) | Some(Mode::Right) => ctx.set_cursor_icon(CursorIcon::ResizeHorizontal),
            Some(Mode::Body) => ctx.set_cursor_icon(if st.drag.is_some() { CursorIcon::Grabbing } else { CursorIcon::Grab }),
            Some(Mode::New) => ctx.set_cursor_icon(CursorIcon::Crosshair),
            None => {}
        }

        // glisser : nouvelle zone / déplacer / redimensionner
        if resp.drag_started() {
            let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| resp.interact_pointer_pos());
            if let Some(p) = origin {
                let b = beat_at(p.x);
                st.drag = Some(DragInfo { mode: hit(p), grab: b - st.start, anchor: snap(b).max(0.0), len: st.end - st.start });
                st.on = true;
            }
        }
        if resp.dragged() {
            if let (Some(d), Some(p)) = (st.drag, resp.interact_pointer_pos()) {
                let b = beat_at(p.x);
                match d.mode {
                    Mode::Left => st.start = snap(b).max(0.0).min(st.end - MIN_LEN),
                    Mode::Right => st.end = snap(b).max(st.start + MIN_LEN),
                    Mode::Body => {
                        st.start = snap(b - d.grab).max(0.0);
                        st.end = st.start + d.len;
                    }
                    Mode::New => {
                        let c = snap(b).max(0.0);
                        let (lo, hi) = (d.anchor.min(c), d.anchor.max(c));
                        st.start = lo;
                        st.end = hi.max(lo + MIN_LEN);
                    }
                }
            }
        }
        if resp.drag_stopped() {
            st.drag = None;
            self.status = format!("Loop {}.", bars(&st));
        }
        // clic sur la brace : on / off
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if hit(p) == Mode::Body {
                    st.on = !st.on;
                    self.status = if st.on { format!("Loop on ({}).", bars(&st)) } else { "Loop off.".into() };
                }
            }
        }

        // menu contextuel
        let sel_span = self.sel_clip_span();
        let play_bar = (self.pos_beats / 4.0).floor() * 4.0;
        let on_now = st.on;
        let mut act: Option<MenuAct> = None;
        resp.context_menu(|ui| {
            if ui.add(egui::Button::new(if on_now { "Loop off" } else { "Loop on" }).shortcut_text("Ctrl+L")).clicked() {
                act = Some(MenuAct::Toggle);
                ui.close_menu();
            }
            if ui.add_enabled(sel_span.is_some(), egui::Button::new("Loop selected clip").shortcut_text("Ctrl+Shift+L")).clicked() {
                act = Some(MenuAct::Clip);
                ui.close_menu();
            }
            ui.separator();
            for n in [1u32, 2, 4, 8] {
                let label = if n == 1 { "Loop 1 bar from here".to_string() } else { format!("Loop {n} bars from here") };
                if ui.button(label).clicked() {
                    act = Some(MenuAct::Bars(n));
                    ui.close_menu();
                }
            }
        });
        match act {
            Some(MenuAct::Toggle) => st.on = !st.on,
            Some(MenuAct::Clip) => {
                if let Some((s, e)) = sel_span {
                    st.start = s;
                    st.end = e.max(s + MIN_LEN);
                    st.on = true;
                }
            }
            Some(MenuAct::Bars(n)) => {
                st.start = play_bar;
                st.end = play_bar + 4.0 * n as f64;
                st.on = true;
            }
            None => {}
        }

        // ---- dessin ----
        let (x0, x1) = (bx(st.start), bx(st.end));
        let p = ui.painter_at(zone);
        p.rect_filled(zone, Rounding { nw: 0.0, ne: 14.0, sw: 0.0, se: 0.0 }, Color32::from_rgb(248, 248, 251));
        p.line_segment([zone.left_bottom(), zone.right_bottom()], stroke(1.0, HAIR));
        if x1 >= zone.left() && x0 <= zone.right() {
            let col = if st.on { AMBER } else { Color32::from_rgb(178, 178, 188) };
            let r = Rect::from_min_max(pos2(x0.max(zone.left()), zone.top() + 3.0), pos2(x1.min(zone.right()), zone.bottom() - 3.0));
            p.rect_filled(r, 4.0, tint(col, if st.on { 90 } else { 60 }));
            p.rect_stroke(r, 4.0, stroke(1.2, col));
            // poignées aux deux bouts
            if x0 >= zone.left() {
                p.rect_filled(Rect::from_min_size(r.min, vec2(5.0, r.height())), 3.0, col);
            }
            if x1 <= zone.right() {
                p.rect_filled(Rect::from_min_max(pos2(r.right() - 5.0, r.top()), r.right_bottom()), 3.0, col);
            }
            if r.width() > 90.0 {
                let txt_col = if st.on { Color32::from_rgb(150, 92, 0) } else { GREY };
                p.text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, format!("Loop · {}", bars(&st)), FontId::proportional(10.0), txt_col);
            }
        }
        resp.clone().on_hover_text("Drag to set the loop · click the brace to turn it on/off · right-click for more");

        put(&ctx, st);
    }
}