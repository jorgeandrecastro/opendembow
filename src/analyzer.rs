//! Master tap (lock-free sample ring + peaks), FFT spectrum analyzer and the Master card.
//!
//! The audio thread only does atomic writes (no lock, no allocation); the FFT runs on the UI thread.

use crate::meters;
use crate::theme::*;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Rounding, Sense, Vec2};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::*};
use std::sync::{Arc, OnceLock};

// ======================================================================
// Tap
// ======================================================================

const TAP_SIZE: usize = 8192; // power of two
const MASK: usize = TAP_SIZE - 1;

pub struct Tap {
    buf: Vec<AtomicU32>,
    w: AtomicUsize,
    peak_l: AtomicU32,
    peak_r: AtomicU32,
    sr: AtomicU32,
}

impl Tap {
    pub fn new() -> Arc<Tap> {
        Arc::new(Tap {
            buf: (0..TAP_SIZE).map(|_| AtomicU32::new(0)).collect(),
            w: AtomicUsize::new(0),
            peak_l: AtomicU32::new(0),
            peak_r: AtomicU32::new(0),
            sr: AtomicU32::new(44100f32.to_bits()),
        })
    }

    pub fn set_sr(&self, sr: f32) {
        self.sr.store(sr.to_bits(), Relaxed);
    }

    pub fn sr(&self) -> f32 {
        f32::from_bits(self.sr.load(Relaxed))
    }

    // ---- audio thread ----
    #[inline]
    pub fn begin(&self) -> usize {
        self.w.load(Relaxed)
    }

    #[inline]
    pub fn put(&self, w: &mut usize, x: f32) {
        self.buf[*w & MASK].store(x.to_bits(), Relaxed);
        *w = w.wrapping_add(1);
    }

    /// Publishes the write index and the block peaks (positive floats compare like their bits).
    #[inline]
    pub fn end(&self, w: usize, pl: f32, pr: f32) {
        self.w.store(w, Release);
        self.peak_l.fetch_max(pl.to_bits(), Relaxed);
        self.peak_r.fetch_max(pr.to_bits(), Relaxed);
    }

    // ---- UI thread ----
    /// Copies the last `out.len()` samples (oldest first).
    pub fn snapshot(&self, out: &mut [f32]) {
        let n = out.len();
        let w = self.w.load(Acquire);
        for (i, o) in out.iter_mut().enumerate() {
            let idx = w.wrapping_sub(n).wrapping_add(i) & MASK;
            *o = f32::from_bits(self.buf[idx].load(Relaxed));
        }
    }

    /// Peaks (linear, L / R) since the previous call.
    pub fn take_peaks(&self) -> (f32, f32) {
        (f32::from_bits(self.peak_l.swap(0, Relaxed)), f32::from_bits(self.peak_r.swap(0, Relaxed)))
    }
}

// ======================================================================
// FFT (in-place radix-2)
// ======================================================================

fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f64, 0.0f64);
            for k in 0..len / 2 {
                let (a, b) = (i + k, i + k + len / 2);
                let tr = re[b] as f64 * cr - im[b] as f64 * ci;
                let ti = re[b] as f64 * ci + im[b] as f64 * cr;
                re[b] = (re[a] as f64 - tr) as f32;
                im[b] = (im[a] as f64 - ti) as f32;
                re[a] = (re[a] as f64 + tr) as f32;
                im[a] = (im[a] as f64 + ti) as f32;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

const N: usize = 4096;
const FMIN: f32 = 20.0;
const DB_TOP: f32 = 0.0;
const DB_BOT: f32 = -80.0;

fn hann() -> &'static [f32] {
    static W: OnceLock<Vec<f32>> = OnceLock::new();
    W.get_or_init(|| (0..N).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / N as f32).cos()).collect())
}

/// Magnitude of every bin in dBFS (a full-scale sine reads 0 dB).
fn analyse(tap: &Tap) -> Vec<f32> {
    let mut re = vec![0.0f32; N];
    tap.snapshot(&mut re);
    for (x, w) in re.iter_mut().zip(hann()) {
        *x *= *w;
    }
    let mut im = vec![0.0f32; N];
    fft(&mut re, &mut im);
    let k = 4.0 / N as f32; // sum(Hann) = N/2, and a sine splits into two bins
    (0..N / 2).map(|b| 20.0 * ((re[b] * re[b] + im[b] * im[b]).sqrt() * k + 1e-9).log10()).collect()
}

// ======================================================================
// UI
// ======================================================================

#[derive(Clone)]
struct Spec {
    smooth: Vec<f32>,
    hold: Vec<f32>,
    tilt: bool,
    peak: bool,
}

impl Default for Spec {
    fn default() -> Self {
        Self { smooth: Vec::new(), hold: Vec::new(), tilt: false, peak: true }
    }
}

const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

fn note_name(f: f32) -> String {
    let m = (69.0 + 12.0 * (f / 440.0).log2()).round() as i32;
    format!("{}{}", NOTES[m.rem_euclid(12) as usize], m.div_euclid(12) - 1)
}

fn hz_text(f: f32) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}

fn draw_spectrum(ui: &mut egui::Ui, tap: &Tap, size: Vec2, st: &mut Spec) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let sr = tap.sr().max(8000.0);
    let fmax = 20000f32.min(sr * 0.5 * 0.98);
    let ratio = fmax / FMIN;
    let inner = Rect::from_min_max(pos2(rect.left() + 26.0, rect.top() + 6.0), pos2(rect.right() - 6.0, rect.bottom() - 16.0));
    if inner.width() < 40.0 || inner.height() < 30.0 {
        return;
    }
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 10.0, Color32::from_rgb(248, 248, 251));

    let fx = |f: f32| inner.left() + (f / FMIN).ln() / ratio.ln() * inner.width();
    let dy = |d: f32| inner.top() + (DB_TOP - d.clamp(DB_BOT, DB_TOP)) / (DB_TOP - DB_BOT) * inner.height();

    // grid: one line every 20 dB, three frequency labels, the rest are faint guides
    let line = Color32::from_rgb(238, 238, 243);
    for i in 0..4 {
        let d = -20.0 * i as f32;
        let y = dy(d);
        p.line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], stroke(1.0, line));
        p.text(pos2(inner.left() - 5.0, y), Align2::RIGHT_CENTER, format!("{}", d as i32), FontId::proportional(9.0), GREY);
    }
    for (f, label) in [(50.0, ""), (100.0, "100"), (200.0, ""), (500.0, ""), (1000.0, "1k"), (2000.0, ""), (5000.0, ""), (10000.0, "10k")] {
        if f >= fmax {
            continue;
        }
        let x = fx(f);
        p.line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], stroke(1.0, line));
        if !label.is_empty() {
            p.text(pos2(x, inner.bottom() + 3.0), Align2::CENTER_TOP, label, FontId::proportional(9.0), GREY);
        }
    }

    // analysis -> one value per 2 px column
    let cols = ((inner.width() / 2.0) as usize).max(8);
    if st.smooth.len() != cols {
        st.smooth = vec![DB_BOT; cols];
        st.hold = vec![DB_BOT; cols];
    }
    let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.1);
    let (a_up, a_down) = (1.0 - (-dt / 0.03).exp(), 1.0 - (-dt / 0.22).exp());
    let bins = analyse(tap);
    let half = bins.len();
    let edge = |i: usize| FMIN * ratio.powf(i as f32 / cols as f32);
    let mut curve = Vec::with_capacity(cols);
    let mut peak_line = Vec::with_capacity(cols);
    let mut loudest = DB_BOT;
    for i in 0..cols {
        let (fa, fb) = (edge(i), edge(i + 1));
        let (ba, bb) = (fa * N as f32 / sr, fb * N as f32 / sr);
        let mut v = if bb - ba < 1.0 {
            let c = (ba + bb) * 0.5;
            let k0 = (c.floor() as usize).min(half - 2);
            let fr = c - k0 as f32;
            bins[k0] * (1.0 - fr) + bins[k0 + 1] * fr
        } else {
            let k0 = (ba.ceil() as usize).min(half - 1);
            let k1 = (bb.floor() as usize).min(half - 1).max(k0);
            bins[k0..=k1].iter().cloned().fold(f32::MIN, f32::max)
        };
        if st.tilt {
            v += 4.5 * ((fa * fb).sqrt() / 1000.0).log2();
        }
        let v = v.clamp(DB_BOT - 10.0, 12.0);
        let s = &mut st.smooth[i];
        *s += (v - *s) * if v > *s { a_up } else { a_down };
        let h = &mut st.hold[i];
        *h = if *s > *h { *s } else { (*h - 12.0 * dt).max(DB_BOT) };
        loudest = loudest.max(*s);
        let x = inner.left() + (i as f32 + 0.5) / cols as f32 * inner.width();
        curve.push(pos2(x, dy(*s)));
        peak_line.push(pos2(x, dy(*h)));
    }

    // soft gradient under the curve
    let mut mesh = egui::epaint::Mesh::default();
    for pt in &curve {
        mesh.colored_vertex(*pt, tint(ACCENT, 110));
        mesh.colored_vertex(pos2(pt.x, inner.bottom()), tint(ACCENT, 4));
    }
    for i in 0..(cols as u32 - 1) {
        let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        mesh.add_triangle(a, b, c);
        mesh.add_triangle(c, b, d);
    }
    let pp = ui.painter_at(inner);
    pp.add(egui::Shape::mesh(mesh));
    if st.peak {
        pp.add(egui::Shape::line(peak_line, stroke(1.0, tint(INK, 70))));
    }
    pp.add(egui::Shape::line(curve, stroke(1.6, ACCENT)));

    // hover read-out
    if let Some(h) = resp.hover_pos().filter(|h| inner.contains(*h)) {
        let t = (h.x - inner.left()) / inner.width();
        let f = FMIN * ratio.powf(t);
        let i = ((t * cols as f32) as usize).min(cols - 1);
        pp.line_segment([pos2(h.x, inner.top()), pos2(h.x, inner.bottom())], stroke(1.0, tint(INK, 45)));
        let txt = format!("{}  {}  {:.1} dB", hz_text(f), note_name(f), st.smooth[i]);
        let g = p.layout_no_wrap(txt, FontId::monospace(10.0), INK);
        let r = Rect::from_min_size(pos2(inner.right() - g.size().x - 14.0, inner.top() + 4.0), g.size() + vec2(8.0, 4.0));
        p.rect_filled(r, 6.0, tint(Color32::WHITE, 235));
        p.galley(r.min + vec2(4.0, 2.0), g, INK);
        ui.ctx().request_repaint();
    }

    // keep animating until the spectrum has fallen back to the floor
    if loudest > DB_BOT + 0.5 {
        ui.ctx().request_repaint();
    }
}

/// Pill toggle for the card header: tinted when on, grey when off, with a real hit area.
fn chip(ui: &mut egui::Ui, label: &str, on: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 22.0), Sense::click());
    let hover = resp.hovered();
    let (fill, fg) = if on {
        (tint(ACCENT, if hover { 52 } else { 34 }), ACCENT)
    } else {
        (if hover { Color32::from_rgb(228, 228, 235) } else { Color32::from_rgb(240, 240, 245) }, GREY)
    };
    let p = ui.painter();
    p.rect_filled(rect, 11.0, fill);
    p.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.5), fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked()
}

/// Peak read-out in the card header; a click resets every peak and the clip LEDs.
fn peak_readout(ui: &mut egui::Ui, pk: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(66.0, 22.0), Sense::click());
    let hover = resp.hovered();
    let col = if pk >= 0.0 { RED } else { GREY };
    if hover {
        ui.painter().rect_filled(rect, 11.0, Color32::from_rgb(240, 240, 245));
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, format!("{:+.1} dB", pk), FontId::proportional(11.5), col);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Highest peak. Click to reset").clicked() {
        meters::reset_all(ui.ctx());
    }
}

/// "Master" card: spectrum + stereo dBFS meters, `w` x `h` pixels.
pub fn master_panel(ui: &mut egui::Ui, tap: &Tap, w: f32, h: f32) {
    let sid = egui::Id::new("spectrum_state");
    let mut st: Spec = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
    let (pl, pr) = tap.take_peaks();

    ui.allocate_ui_with_layout(vec2(w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .rounding(Rounding::same(12.0))
            .stroke(stroke(1.0, HAIR))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(w - 22.0);
                ui.set_min_height(h - 22.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(RichText::new("Master").strong().size(13.0));
                    let pk = meters::peak_db(ui.ctx(), meters::MASTER_L).max(meters::peak_db(ui.ctx(), meters::MASTER_R));
                    if pk > -90.0 {
                        peak_readout(ui, pk);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if chip(ui, "Tilt", st.tilt, "+4.5 dB per octave: the spectrum looks like what you hear") {
                            st.tilt = !st.tilt;
                        }
                        if chip(ui, "Hold", st.peak, "Peak hold curve") {
                            st.peak = !st.peak;
                        }
                    });
                });

                let ah = ui.available_height().max(60.0);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let sw = (ui.available_width() - 48.0).max(80.0);
                    draw_spectrum(ui, tap, vec2(sw, ah), &mut st);
                    meters::meter(ui, vec2(8.0, ah), meters::MASTER_L, pl, true);
                    meters::meter(ui, vec2(8.0, ah), meters::MASTER_R, pr, true);
                    meters::scale(ui, vec2(20.0, ah));
                });
            });
    });
    ui.ctx().data_mut(|d| d.insert_temp(sid, st));
}