//! Curve widgets of the Mixer & Effects tab (0.11):
//! - the visual equalizer: three draggable points on the real response curve, spectrum behind it
//! - the compressor transfer curve: threshold and ratio handles, a live dot, a gain-reduction meter
//!
//! Gain reduction: when the engine calls `publish_gr` after each `process_block` the meter shows the
//! exact value. Until it does, the meter falls back to an estimate computed from the track level.

use crate::analyzer;
use crate::effects::{CompFx, EqFx, eq_response_db};
use crate::meters::to_db;
use crate::theme::*;
use opendembow_core::meter_bus::live_gr;
use opendembow_core::tap::Tap;
use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2,
};

/// Gain reduction to show for a track: the engine's value when it publishes one,
/// otherwise an estimate (static curve + attack / release) from the track level.
pub fn gain_reduction(ctx: &egui::Context, track: usize, comp: &CompFx, level_lin: f32) -> f32 {
    if let Some(v) = live_gr(track) {
        if v > 0.05 {
            ctx.request_repaint();
        }
        return v;
    }
    let id = egui::Id::unique(("gr_estimate", track));
    let mut gr: f32 = ctx.data(|d| d.get_temp(id)).unwrap_or(0.0);
    if !comp.on {
        gr = 0.0;
    } else {
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.1);
        let target =
            (to_db(level_lin) - comp.threshold).max(0.0) * (1.0 - 1.0 / comp.ratio.max(1.0));
        let tau = (if target > gr {
            comp.attack
        } else {
            comp.release
        } * 0.001)
            .max(0.001);
        gr += (target - gr) * (1.0 - (-dt / tau).exp());
    }
    ctx.data_mut(|d| d.insert_temp(id, gr));
    if gr > 0.05 {
        ctx.request_repaint();
    }
    gr
}

// ======================================================================
// Shared helpers
// ======================================================================

fn hz_text(f: f32) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}

/// Small dark read-out pill next to a handle.
fn tag(p: &egui::Painter, at: Pos2, bounds: Rect, text: String) {
    let g = p.layout_no_wrap(text, FontId::monospace(10.0), Color32::WHITE);
    let size = g.size() + vec2(10.0, 5.0);
    let mut min = pos2(at.x + 10.0, at.y - size.y - 6.0);
    if min.x + size.x > bounds.right() {
        min.x = at.x - 10.0 - size.x;
    }
    min.y = min.y.max(bounds.top() + 1.0);
    let r = Rect::from_min_size(min, size);
    p.rect_filled(r, 7.0, INK);
    p.galley(r.min + vec2(5.0, 2.5), g, Color32::WHITE);
}

fn handle(p: &egui::Painter, at: Pos2, active: bool, on: bool, col: Color32) {
    let c = if on { col } else { GREY };
    let r = if active { 7.5 } else { 6.0 };
    if active {
        p.circle_filled(at, r + 4.0, tint(c, 40));
    }
    p.circle_filled(at, r, Color32::WHITE);
    p.circle_stroke(at, r, stroke(2.2, c));
}

// ======================================================================
// Visual equalizer
// ======================================================================

const FMIN: f32 = 20.0;
const FMAX: f32 = 20000.0;
const EQ_DB: f32 = 18.0;
const SPEC_BOT: f32 = -80.0;

#[derive(Clone, Default)]
struct EqUi {
    drag: Option<usize>,
    smooth: Vec<f32>,
}

/// Visual EQ for `track`. Points: 0 = low shelf, 1 = mid peak, 2 = high shelf.
/// Drag a point (frequency + gain), scroll on the mid point for Q, double-click a point to reset it.
/// Returns true when a setting changed (touching a point also switches the EQ on).
pub fn eq_graph(
    ui: &mut egui::Ui,
    track: usize,
    eq: &mut EqFx,
    tap: &Tap,
    size: Vec2,
    col: Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let inner = Rect::from_min_max(
        pos2(rect.left() + 28.0, rect.top() + 8.0),
        pos2(rect.right() - 8.0, rect.bottom() - 16.0),
    );
    if inner.width() < 60.0 || inner.height() < 40.0 {
        return false;
    }
    let sid = egui::Id::unique(("eq_ui", track));
    let mut st: EqUi = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
    let sr = tap.sr().max(8000.0);
    let lr = (FMAX / FMIN).ln();
    let fx = |f: f32| inner.left() + (f / FMIN).ln() / lr * inner.width();
    let f_at = |x: f32| FMIN * ((x - inner.left()) / inner.width() * lr).exp();
    let dy = |d: f32| inner.center().y - d.clamp(-EQ_DB, EQ_DB) / EQ_DB * inner.height() * 0.5;
    let db_at =
        |y: f32| ((inner.center().y - y) / (inner.height() * 0.5) * EQ_DB).clamp(-EQ_DB, EQ_DB);
    let points = |e: &EqFx| {
        [
            pos2(fx(e.low_freq), dy(e.low)),
            pos2(fx(e.mid_freq), dy(e.mid)),
            pos2(fx(e.high_freq), dy(e.high)),
        ]
    };
    let nearest = |p: Pos2, e: &EqFx| -> Option<usize> {
        points(e)
            .iter()
            .enumerate()
            .map(|(i, q)| (i, q.distance(p)))
            .filter(|(_, d)| *d <= 14.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    };

    // ---- interaction ----
    let mut changed = false;
    let hover = resp.hover_pos();
    if resp.drag_started() {
        if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
            st.drag = nearest(p, eq);
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(p)) = (st.drag, resp.interact_pointer_pos()) {
            let f = f_at(p.x).clamp(FMIN, FMAX);
            let g = (db_at(p.y).clamp(-15.0, 15.0) * 10.0).round() / 10.0;
            match i {
                0 => {
                    eq.low_freq = f.clamp(40.0, 400.0);
                    eq.low = g;
                }
                1 => {
                    eq.mid_freq = f.clamp(200.0, 5000.0);
                    eq.mid = g;
                }
                _ => {
                    eq.high_freq = f.clamp(2000.0, 16000.0);
                    eq.high = g;
                }
            }
            eq.on = true;
            changed = true;
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
    }
    if resp.double_clicked() {
        if let Some(i) = resp.interact_pointer_pos().and_then(|p| nearest(p, eq)) {
            let d = EqFx::default();
            match i {
                0 => (eq.low, eq.low_freq) = (d.low, d.low_freq),
                1 => (eq.mid, eq.mid_freq, eq.mid_q) = (d.mid, d.mid_freq, d.mid_q),
                _ => (eq.high, eq.high_freq) = (d.high, d.high_freq),
            }
            changed = true;
        }
    }
    let hot = st.drag.or_else(|| hover.and_then(|h| nearest(h, eq)));
    if hot == Some(1) && st.drag.is_none() {
        let sc = ui.input(|i| i.smooth_scroll_delta.y);
        if sc != 0.0 {
            eq.mid_q = (eq.mid_q * (sc * 0.004).exp()).clamp(0.3, 6.0);
            eq.on = true;
            changed = true;
        }
    }
    if st.drag.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if hot.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    } else if hover.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }

    // ---- drawing ----
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 10.0, Color32::from_rgb(248, 248, 251));
    let line = Color32::from_rgb(236, 236, 241);
    for d in [-12.0f32, -6.0, 0.0, 6.0, 12.0] {
        let y = dy(d);
        let c = if d == 0.0 {
            Color32::from_rgb(214, 214, 222)
        } else {
            line
        };
        p.line_segment(
            [pos2(inner.left(), y), pos2(inner.right(), y)],
            stroke(1.0, c),
        );
        p.text(
            pos2(inner.left() - 5.0, y),
            Align2::RIGHT_CENTER,
            format!("{:+}", d as i32),
            FontId::proportional(9.0),
            GREY,
        );
    }
    for (f, label) in [
        (50.0, ""),
        (100.0, "100"),
        (200.0, ""),
        (500.0, ""),
        (1000.0, "1k"),
        (2000.0, ""),
        (5000.0, ""),
        (10000.0, "10k"),
    ] {
        let x = fx(f);
        p.line_segment(
            [pos2(x, inner.top()), pos2(x, inner.bottom())],
            stroke(1.0, line),
        );
        if !label.is_empty() {
            p.text(
                pos2(x, inner.bottom() + 3.0),
                Align2::CENTER_TOP,
                label,
                FontId::proportional(9.0),
                GREY,
            );
        }
    }
    let pp = ui.painter_at(inner);

    // master spectrum behind the curve (context: the track's own spectrum is not available)
    let cols = ((inner.width() / 3.0) as usize).max(8);
    let bins = analyzer::analyse(tap);
    let half = bins.len();
    let n = analyzer::N as f32;
    let fmax = FMAX.min(sr * 0.5 * 0.98);
    let ratio = fmax / FMIN;
    if st.smooth.len() != cols {
        st.smooth = vec![SPEC_BOT; cols];
    }
    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.1);
    let (a_up, a_down) = (1.0 - (-dt / 0.03).exp(), 1.0 - (-dt / 0.22).exp());
    let mut mesh = egui::epaint::Mesh::default();
    let mut loudest = SPEC_BOT;
    for i in 0..cols {
        let fa = FMIN * ratio.powf(i as f32 / cols as f32);
        let fb = FMIN * ratio.powf((i + 1) as f32 / cols as f32);
        let (ba, bb) = (fa * n / sr, fb * n / sr);
        let v = if bb - ba < 1.0 {
            let c = (ba + bb) * 0.5;
            let k0 = (c.floor() as usize).min(half - 2);
            let fr = c - k0 as f32;
            bins[k0] * (1.0 - fr) + bins[k0 + 1] * fr
        } else {
            let k0 = (ba.ceil() as usize).min(half - 1);
            let k1 = (bb.floor() as usize).min(half - 1).max(k0);
            bins[k0..=k1].iter().cloned().fold(f32::MIN, f32::max)
        };
        let s = &mut st.smooth[i];
        let v = v.clamp(SPEC_BOT - 10.0, 12.0);
        *s += (v - *s) * if v > *s { a_up } else { a_down };
        loudest = loudest.max(*s);
        let x = fx((fa * fb).sqrt());
        let y =
            inner.bottom() - ((*s - SPEC_BOT) / -SPEC_BOT).clamp(0.0, 1.0) * inner.height() * 0.85;
        mesh.colored_vertex(pos2(x, y), tint(INK, 34));
        mesh.colored_vertex(pos2(x, inner.bottom()), tint(INK, 3));
    }
    for i in 0..(cols as u32 - 1) {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        mesh.add_triangle(a, b, c);
        mesh.add_triangle(c, b, d);
    }
    pp.add(Shape::mesh(mesh));
    if loudest > SPEC_BOT + 0.5 {
        ui.ctx().request_repaint();
    }

    // response curve: real biquad magnitudes, filled towards 0 dB
    let curve_col = if eq.on { col } else { GREY };
    let steps = (inner.width() / 2.0) as usize;
    let mut curve = Vec::with_capacity(steps + 1);
    let mut fill = egui::epaint::Mesh::default();
    for i in 0..=steps {
        let f = FMIN * (FMAX / FMIN).powf(i as f32 / steps as f32);
        let x = fx(f);
        let y = dy(eq_response_db(eq, sr, f));
        curve.push(pos2(x, y));
        fill.colored_vertex(pos2(x, y), tint(curve_col, 60));
        fill.colored_vertex(pos2(x, dy(0.0)), tint(curve_col, 60));
    }
    for i in 0..steps as u32 {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        fill.add_triangle(a, b, c);
        fill.add_triangle(c, b, d);
    }
    pp.add(Shape::mesh(fill));
    pp.add(Shape::line(curve, stroke(2.2, curve_col)));

    // handles + read-outs
    let pts = points(eq);
    for (i, q) in pts.iter().enumerate() {
        handle(&p, *q, hot == Some(i), eq.on, col);
    }
    if let Some(i) = hot {
        let txt = match i {
            0 => format!("Low  {}  {:+.1} dB", hz_text(eq.low_freq), eq.low),
            1 => format!(
                "Mid  {}  {:+.1} dB  Q {:.2}",
                hz_text(eq.mid_freq),
                eq.mid,
                eq.mid_q
            ),
            _ => format!("High  {}  {:+.1} dB", hz_text(eq.high_freq), eq.high),
        };
        tag(&p, pts[i], inner, txt);
    }
    if st.drag.is_none() && hot.is_none() {
        resp.clone().on_hover_text("Drag a point: frequency and gain. Scroll on Mid: width (Q). Double-click a point: reset it.");
    }

    ui.ctx().data_mut(|d| d.insert_temp(sid, st));
    changed
}

// ======================================================================
// Compressor transfer curve + gain-reduction meter
// ======================================================================

const IN_MIN: f32 = -60.0;
const OUT_MAX: f32 = 12.0;

#[derive(Clone, Copy, Default)]
struct CompUi {
    drag: Option<usize>,
}

fn out_db(c: &CompFx, i: f32) -> f32 {
    (if i <= c.threshold {
        i
    } else {
        c.threshold + (i - c.threshold) / c.ratio.max(1.0)
    }) + c.makeup
}

/// Transfer curve (input dB -> output dB). Handle 0 (knee) moves the threshold, handle 1 (at 0 dB in) sets the ratio.
/// `level_lin` is the live input level (a dot rides the curve), `gr_db` the gain reduction shown in the corner.
pub fn comp_graph(
    ui: &mut egui::Ui,
    track: usize,
    comp: &mut CompFx,
    level_lin: f32,
    gr_db: f32,
    size: Vec2,
    col: Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let inner = Rect::from_min_max(
        pos2(rect.left() + 28.0, rect.top() + 8.0),
        pos2(rect.right() - 8.0, rect.bottom() - 16.0),
    );
    if inner.width() < 60.0 || inner.height() < 40.0 {
        return false;
    }
    let sid = egui::Id::unique(("comp_ui", track));
    let mut st: CompUi = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
    let xin = |d: f32| inner.left() + (d - IN_MIN) / -IN_MIN * inner.width();
    let yout = |d: f32| {
        inner.bottom() - (d.clamp(IN_MIN, OUT_MAX) - IN_MIN) / (OUT_MAX - IN_MIN) * inner.height()
    };
    let din = |x: f32| IN_MIN + (x - inner.left()) / inner.width() * -IN_MIN;
    let dout = |y: f32| IN_MIN + (inner.bottom() - y) / inner.height() * (OUT_MAX - IN_MIN);
    let handles = |c: &CompFx| {
        [
            pos2(xin(c.threshold), yout(c.threshold + c.makeup)),
            pos2(xin(0.0), yout(out_db(c, 0.0))),
        ]
    };
    let nearest = |p: Pos2, c: &CompFx| -> Option<usize> {
        handles(c)
            .iter()
            .enumerate()
            .map(|(i, q)| (i, q.distance(p)))
            .filter(|(_, d)| *d <= 14.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    };

    // ---- interaction ----
    let mut changed = false;
    let hover = resp.hover_pos();
    if resp.drag_started() {
        if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
            st.drag = nearest(p, comp);
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(p)) = (st.drag, resp.interact_pointer_pos()) {
            if i == 0 {
                comp.threshold = (din(p.x).clamp(-60.0, -1.0) * 2.0).round() / 2.0;
            } else {
                // output at 0 dB in, without makeup, gives the ratio: out = thr + (0 - thr) / ratio
                let base = dout(p.y) - comp.makeup;
                let span = base - comp.threshold;
                let ratio = if span <= 0.05 {
                    20.0
                } else {
                    -comp.threshold / span
                };
                comp.ratio = (ratio.clamp(1.0, 20.0) * 10.0).round() / 10.0;
            }
            comp.on = true;
            changed = true;
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
    }
    if resp.double_clicked() {
        if let Some(i) = resp.interact_pointer_pos().and_then(|p| nearest(p, comp)) {
            let d = CompFx::default();
            if i == 0 {
                comp.threshold = d.threshold;
            } else {
                comp.ratio = d.ratio;
            }
            changed = true;
        }
    }
    let hot = st.drag.or_else(|| hover.and_then(|h| nearest(h, comp)));
    if st.drag.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if hot.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    } else if hover.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    }

    // ---- drawing ----
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 10.0, Color32::from_rgb(248, 248, 251));
    let line = Color32::from_rgb(236, 236, 241);
    for d in [-48.0f32, -36.0, -24.0, -12.0, 0.0] {
        p.line_segment(
            [pos2(inner.left(), yout(d)), pos2(inner.right(), yout(d))],
            stroke(1.0, line),
        );
        p.line_segment(
            [pos2(xin(d), inner.top()), pos2(xin(d), inner.bottom())],
            stroke(1.0, line),
        );
    }
    for d in [-48.0f32, -24.0, 0.0] {
        p.text(
            pos2(inner.left() - 5.0, yout(d)),
            Align2::RIGHT_CENTER,
            format!("{}", d as i32),
            FontId::proportional(9.0),
            GREY,
        );
        p.text(
            pos2(xin(d), inner.bottom() + 3.0),
            Align2::CENTER_TOP,
            format!("{}", d as i32),
            FontId::proportional(9.0),
            GREY,
        );
    }
    let pp = ui.painter_at(inner);
    let curve_col = if comp.on { col } else { GREY };

    // 1:1 reference (dashed)
    pp.extend(Shape::dashed_line(
        &[pos2(xin(IN_MIN), yout(IN_MIN)), pos2(xin(0.0), yout(0.0))],
        stroke(1.0, tint(INK, 70)),
        4.0,
        4.0,
    ));

    // curve, with the gain-reduction area between it and the 1:1 line
    let steps = 60;
    let mut curve = Vec::with_capacity(steps + 1);
    let mut area = egui::epaint::Mesh::default();
    for i in 0..=steps {
        let d = IN_MIN + (-IN_MIN) * i as f32 / steps as f32;
        let (x, yc, yd) = (xin(d), yout(out_db(comp, d)), yout(d));
        curve.push(pos2(x, yc));
        area.colored_vertex(pos2(x, yd), tint(curve_col, 70));
        area.colored_vertex(pos2(x, yc.max(yd)), tint(curve_col, 70));
    }
    for i in 0..steps as u32 {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        area.add_triangle(a, b, c);
        area.add_triangle(c, b, d);
    }
    pp.add(Shape::mesh(area));
    pp.add(Shape::line(curve, Stroke::new(2.4_f32, curve_col)));

    // live dot: where the signal sits on the curve right now
    let lvl = to_db(level_lin);
    if lvl > IN_MIN {
        let li = lvl.min(0.0);
        let at = pos2(xin(li), yout(out_db(comp, li)));
        pp.line_segment(
            [pos2(at.x, inner.bottom()), at],
            stroke(1.0, tint(curve_col, 90)),
        );
        pp.circle_filled(at, 4.5, curve_col);
        pp.circle_stroke(at, 4.5, stroke(1.5, Color32::WHITE));
        ui.ctx().request_repaint();
    }
    p.text(
        pos2(inner.left() + 6.0, inner.top() + 4.0),
        Align2::LEFT_TOP,
        format!("GR {:.1} dB", gr_db),
        FontId::monospace(10.0),
        if gr_db > 0.05 {
            Color32::from_rgb(255, 149, 0)
        } else {
            GREY
        },
    );

    // handles + read-outs
    let hs = handles(comp);
    for (i, q) in hs.iter().enumerate() {
        handle(&p, *q, hot == Some(i), comp.on, col);
    }
    if let Some(i) = hot {
        let txt = if i == 0 {
            format!("Threshold {:.1} dB", comp.threshold)
        } else {
            format!("Ratio {:.1}:1", comp.ratio)
        };
        tag(&p, hs[i], inner, txt);
    }
    if st.drag.is_none() && hot.is_none() {
        resp.clone().on_hover_text(
            "Drag the knee: threshold. Drag the right end: ratio. The dot shows the live level.",
        );
    }

    ui.ctx().data_mut(|d| d.insert_temp(sid, st));
    changed
}

/// Vertical gain-reduction meter (0 to 24 dB, filling from the top). `size` includes a 14 px label.
pub fn gr_meter(ui: &mut egui::Ui, size: Vec2, gr_db: f32) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let bar = Rect::from_min_max(
        rect.min + vec2(0.0, 8.0),
        pos2(rect.right(), rect.bottom() - 16.0),
    );
    let p = ui.painter_at(rect);
    p.rect_filled(bar, 4.0, tint(INK, 14));
    let f = (gr_db / 24.0).clamp(0.0, 1.0);
    if f > 0.0 {
        let col = if gr_db > 12.0 {
            RED
        } else {
            Color32::from_rgb(255, 149, 0)
        };
        p.rect_filled(
            Rect::from_min_max(bar.min, pos2(bar.right(), bar.top() + f * bar.height())),
            4.0,
            col,
        );
    }
    for d in [6.0f32, 12.0, 18.0] {
        let y = bar.top() + d / 24.0 * bar.height();
        p.line_segment(
            [pos2(bar.left(), y), pos2(bar.right(), y)],
            stroke(1.0, tint(Color32::WHITE, 200)),
        );
    }
    p.text(
        pos2(rect.center().x, rect.bottom() - 6.0),
        Align2::CENTER_CENTER,
        "GR",
        FontId::proportional(9.0),
        GREY,
    );
    resp.on_hover_text(format!("Gain reduction: {:.1} dB", gr_db));
}
