//! Effects: per-track EQ, compressor, reverb and delay, plus the "Mixer & Effects" panel
//! (track strips, Master spectrum + dBFS meters, effect cards).
//!
//! The DSP (`FxProcessor`) does not depend on the UI. The engine owns one `FxProcessor` per track
//! and calls, on the track's stereo buffers, BEFORE volume/pan:
//!
//! ```ignore
//! fx_proc[t].process_block(&project.tracks[t].fx, &mut left, &mut right);
//! ```

use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, RichText, Rounding, Sense, Shape};
use std::f32::consts::PI;

// ======================================================================
// Settings (stored in every Track)
// ======================================================================

#[derive(Clone, PartialEq)]
pub struct EqFx {
    pub on: bool,
    pub low: f32,      // dB, shelf at 120 Hz
    pub mid: f32,      // dB, peak
    pub mid_freq: f32, // Hz
    pub high: f32,     // dB, shelf at 6 kHz
}

#[derive(Clone, PartialEq)]
pub struct CompFx {
    pub on: bool,
    pub threshold: f32, // dB
    pub ratio: f32,
    pub attack: f32,  // ms
    pub release: f32, // ms
    pub makeup: f32,  // dB
}

#[derive(Clone, PartialEq)]
pub struct ReverbFx {
    pub on: bool,
    pub mix: f32,
    pub size: f32,
    pub damp: f32,
}

#[derive(Clone, PartialEq)]
pub struct DelayFx {
    pub on: bool,
    pub time: f32, // ms
    pub feedback: f32,
    pub mix: f32,
}

#[derive(Clone, PartialEq, Default)]
pub struct FxSettings {
    pub eq: EqFx,
    pub comp: CompFx,
    pub reverb: ReverbFx,
    pub delay: DelayFx,
}

impl Default for EqFx {
    fn default() -> Self {
        Self { on: false, low: 0.0, mid: 0.0, mid_freq: 1000.0, high: 0.0 }
    }
}
impl Default for CompFx {
    fn default() -> Self {
        Self { on: false, threshold: -18.0, ratio: 3.0, attack: 10.0, release: 120.0, makeup: 0.0 }
    }
}
impl Default for ReverbFx {
    fn default() -> Self {
        Self { on: false, mix: 0.25, size: 0.5, damp: 0.5 }
    }
}
impl Default for DelayFx {
    fn default() -> Self {
        Self { on: false, time: 350.0, feedback: 0.35, mix: 0.25 }
    }
}

impl FxSettings {
    /// True when no effect is enabled.
    pub fn is_bypassed(&self) -> bool {
        !(self.eq.on || self.comp.on || self.reverb.on || self.delay.on)
    }

    /// Every parameter as ("effect.name", value). Booleans are 0 / 1. Used by the MCP server.
    pub fn params(&self) -> Vec<(&'static str, f64)> {
        let b = |x: bool| if x { 1.0 } else { 0.0 };
        vec![
            ("eq.on", b(self.eq.on)),
            ("eq.low", self.eq.low as f64),
            ("eq.mid", self.eq.mid as f64),
            ("eq.mid_freq", self.eq.mid_freq as f64),
            ("eq.high", self.eq.high as f64),
            ("comp.on", b(self.comp.on)),
            ("comp.threshold", self.comp.threshold as f64),
            ("comp.ratio", self.comp.ratio as f64),
            ("comp.attack", self.comp.attack as f64),
            ("comp.release", self.comp.release as f64),
            ("comp.makeup", self.comp.makeup as f64),
            ("reverb.on", b(self.reverb.on)),
            ("reverb.mix", self.reverb.mix as f64),
            ("reverb.size", self.reverb.size as f64),
            ("reverb.damp", self.reverb.damp as f64),
            ("delay.on", b(self.delay.on)),
            ("delay.time", self.delay.time as f64),
            ("delay.feedback", self.delay.feedback as f64),
            ("delay.mix", self.delay.mix as f64),
        ]
    }

    /// Set one parameter by name (see `params`). Returns false for an unknown name.
    pub fn set_param(&mut self, key: &str, v: f64) -> bool {
        let f = v as f32;
        let on = v >= 0.5;
        match key {
            "eq.on" => self.eq.on = on,
            "eq.low" => self.eq.low = f.clamp(-15.0, 15.0),
            "eq.mid" => self.eq.mid = f.clamp(-15.0, 15.0),
            "eq.mid_freq" => self.eq.mid_freq = f.clamp(200.0, 5000.0),
            "eq.high" => self.eq.high = f.clamp(-15.0, 15.0),
            "comp.on" => self.comp.on = on,
            "comp.threshold" => self.comp.threshold = f.clamp(-60.0, 0.0),
            "comp.ratio" => self.comp.ratio = f.clamp(1.0, 20.0),
            "comp.attack" => self.comp.attack = f.clamp(1.0, 100.0),
            "comp.release" => self.comp.release = f.clamp(20.0, 1000.0),
            "comp.makeup" => self.comp.makeup = f.clamp(0.0, 24.0),
            "reverb.on" => self.reverb.on = on,
            "reverb.mix" => self.reverb.mix = f.clamp(0.0, 1.0),
            "reverb.size" => self.reverb.size = f.clamp(0.0, 1.0),
            "reverb.damp" => self.reverb.damp = f.clamp(0.0, 1.0),
            "delay.on" => self.delay.on = on,
            "delay.time" => self.delay.time = f.clamp(20.0, 1000.0),
            "delay.feedback" => self.delay.feedback = f.clamp(0.0, 0.9),
            "delay.mix" => self.delay.mix = f.clamp(0.0, 1.0),
            _ => return false,
        }
        true
    }
}

// ======================================================================
// DSP
// ======================================================================

/// Transposed direct form II biquad, two channels.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: [f32; 2],
    z2: [f32; 2],
}

impl Biquad {
    fn identity() -> Self {
        Self { b0: 1.0, ..Default::default() }
    }
    fn set(&mut self, c: [f32; 5]) {
        self.b0 = c[0];
        self.b1 = c[1];
        self.b2 = c[2];
        self.a1 = c[3];
        self.a2 = c[4];
    }
    fn tick(&mut self, ch: usize, x: f32) -> f32 {
        let y = self.b0 * x + self.z1[ch];
        self.z1[ch] = self.b1 * x - self.a1 * y + self.z2[ch];
        self.z2[ch] = self.b2 * x - self.a2 * y;
        y
    }
}

fn norm(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> [f32; 5] {
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// RBJ cookbook peaking EQ.
fn peaking(sr: f32, f: f32, gain_db: f32, q: f32) -> [f32; 5] {
    let a = 10f32.powf(gain_db / 40.0);
    let w = 2.0 * PI * f.min(sr * 0.45) / sr;
    let (s, c) = (w.sin(), w.cos());
    let al = s / (2.0 * q);
    norm(1.0 + al * a, -2.0 * c, 1.0 - al * a, 1.0 + al / a, -2.0 * c, 1.0 - al / a)
}

fn low_shelf(sr: f32, f: f32, gain_db: f32) -> [f32; 5] {
    let a = 10f32.powf(gain_db / 40.0);
    let w = 2.0 * PI * f / sr;
    let (s, c) = (w.sin(), w.cos());
    let al = s / 2.0 * 2f32.sqrt();
    let sa = 2.0 * a.sqrt() * al;
    norm(
        a * ((a + 1.0) - (a - 1.0) * c + sa),
        2.0 * a * ((a - 1.0) - (a + 1.0) * c),
        a * ((a + 1.0) - (a - 1.0) * c - sa),
        (a + 1.0) + (a - 1.0) * c + sa,
        -2.0 * ((a - 1.0) + (a + 1.0) * c),
        (a + 1.0) + (a - 1.0) * c - sa,
    )
}

fn high_shelf(sr: f32, f: f32, gain_db: f32) -> [f32; 5] {
    let a = 10f32.powf(gain_db / 40.0);
    let w = 2.0 * PI * f.min(sr * 0.45) / sr;
    let (s, c) = (w.sin(), w.cos());
    let al = s / 2.0 * 2f32.sqrt();
    let sa = 2.0 * a.sqrt() * al;
    norm(
        a * ((a + 1.0) + (a - 1.0) * c + sa),
        -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
        a * ((a + 1.0) + (a - 1.0) * c - sa),
        (a + 1.0) - (a - 1.0) * c + sa,
        2.0 * ((a - 1.0) - (a + 1.0) * c),
        (a + 1.0) - (a - 1.0) * c - sa,
    )
}

struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}
impl Comb {
    fn new(n: usize) -> Self {
        Self { buf: vec![0.0; n.max(1)], i: 0, store: 0.0 }
    }
    fn tick(&mut self, x: f32, fb: f32, damp: f32) -> f32 {
        let o = self.buf[self.i];
        self.store = o * (1.0 - damp) + self.store * damp;
        self.buf[self.i] = x + self.store * fb;
        self.i += 1;
        if self.i >= self.buf.len() {
            self.i = 0;
        }
        o
    }
}

struct Allpass {
    buf: Vec<f32>,
    i: usize,
}
impl Allpass {
    fn new(n: usize) -> Self {
        Self { buf: vec![0.0; n.max(1)], i: 0 }
    }
    fn tick(&mut self, x: f32) -> f32 {
        let b = self.buf[self.i];
        let o = -x + b;
        self.buf[self.i] = x + b * 0.5;
        self.i += 1;
        if self.i >= self.buf.len() {
            self.i = 0;
        }
        o
    }
}

/// Freeverb-style stereo reverb.
struct Reverb {
    cl: Vec<Comb>,
    cr: Vec<Comb>,
    al: Vec<Allpass>,
    ar: Vec<Allpass>,
}
impl Reverb {
    fn new(sr: f32) -> Self {
        const COMB: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        const AP: [usize; 4] = [556, 441, 341, 225];
        const SPREAD: usize = 23;
        let k = sr / 44100.0;
        let s = |n: usize| (n as f32 * k) as usize;
        Self {
            cl: COMB.iter().map(|&n| Comb::new(s(n))).collect(),
            cr: COMB.iter().map(|&n| Comb::new(s(n + SPREAD))).collect(),
            al: AP.iter().map(|&n| Allpass::new(s(n))).collect(),
            ar: AP.iter().map(|&n| Allpass::new(s(n + SPREAD))).collect(),
        }
    }
    fn tick(&mut self, l: f32, r: f32, rv: &ReverbFx) -> (f32, f32) {
        let inp = (l + r) * 0.015;
        let fb = 0.7 + rv.size * 0.28;
        let damp = rv.damp * 0.4;
        let (mut wl, mut wr) = (0.0, 0.0);
        for c in &mut self.cl {
            wl += c.tick(inp, fb, damp);
        }
        for c in &mut self.cr {
            wr += c.tick(inp, fb, damp);
        }
        for a in &mut self.al {
            wl = a.tick(wl);
        }
        for a in &mut self.ar {
            wr = a.tick(wr);
        }
        (l + wl * rv.mix * 3.0, r + wr * rv.mix * 3.0)
    }
}

struct Delay {
    buf: [Vec<f32>; 2],
    w: usize,
}
impl Delay {
    fn new(sr: f32) -> Self {
        let n = sr as usize + 2; // up to 1 s
        Self { buf: [vec![0.0; n], vec![0.0; n]], w: 0 }
    }
    fn tick(&mut self, l: f32, r: f32, d: &DelayFx, sr: f32) -> (f32, f32) {
        let n = self.buf[0].len();
        let dl = ((d.time * 0.001 * sr) as usize).clamp(1, n - 1);
        let rd = (self.w + n - dl) % n;
        let (wl, wr) = (self.buf[0][rd], self.buf[1][rd]);
        // ping-pong: left feeds right and vice versa
        self.buf[0][self.w] = l + wr * d.feedback;
        self.buf[1][self.w] = r + wl * d.feedback;
        self.w = (self.w + 1) % n;
        (l + wl * d.mix, r + wr * d.mix)
    }
}

/// Effect chain of one track: EQ -> compressor -> reverb -> delay.
pub struct FxProcessor {
    sr: f32,
    eq: [Biquad; 3],
    gr: f32,
    att: f32,
    rel: f32,
    rev: Reverb,
    delay: Delay,
    last: Option<FxSettings>,
}

impl FxProcessor {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            eq: [Biquad::identity(); 3],
            gr: 0.0,
            att: 0.0,
            rel: 0.0,
            rev: Reverb::new(sr),
            delay: Delay::new(sr),
            last: None,
        }
    }

    fn refresh(&mut self, fx: &FxSettings) {
        if self.last.as_ref() == Some(fx) {
            return;
        }
        let sr = self.sr;
        self.eq[0].set(low_shelf(sr, 120.0, fx.eq.low));
        self.eq[1].set(peaking(sr, fx.eq.mid_freq, fx.eq.mid, 1.0));
        self.eq[2].set(high_shelf(sr, 6000.0, fx.eq.high));
        self.att = (-1.0 / (sr * fx.comp.attack.max(0.1) * 0.001)).exp();
        self.rel = (-1.0 / (sr * fx.comp.release.max(1.0) * 0.001)).exp();
        self.last = Some(fx.clone());
    }

    /// Process a block in place (cheap change detection once per block).
    pub fn process_block(&mut self, fx: &FxSettings, l: &mut [f32], r: &mut [f32]) {
        if fx.is_bypassed() {
            return;
        }
        self.refresh(fx);
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let (x, y) = self.tick(fx, *a, *b);
            *a = x;
            *b = y;
        }
    }

    fn tick(&mut self, fx: &FxSettings, mut l: f32, mut r: f32) -> (f32, f32) {
        if fx.eq.on {
            for b in self.eq.iter_mut() {
                l = b.tick(0, l);
                r = b.tick(1, r);
            }
        }
        if fx.comp.on {
            let peak = l.abs().max(r.abs()).max(1e-6);
            let level = 20.0 * peak.log10();
            let target = (level - fx.comp.threshold).max(0.0) * (1.0 - 1.0 / fx.comp.ratio.max(1.0));
            let k = if target > self.gr { self.att } else { self.rel };
            self.gr = k * self.gr + (1.0 - k) * target;
            let g = 10f32.powf((fx.comp.makeup - self.gr) / 20.0);
            l *= g;
            r *= g;
        }
        if fx.reverb.on {
            (l, r) = self.rev.tick(l, r, &fx.reverb);
        }
        if fx.delay.on {
            (l, r) = self.delay.tick(l, r, &fx.delay, self.sr);
        }
        (l, r)
    }
}

// ======================================================================
// UI: knob + effect cards + mixer strips
// ======================================================================

/// Rotary knob: drag vertically, double-click to reset. Returns true when the value changed.
fn knob(ui: &mut egui::Ui, label: &str, v: &mut f32, min: f32, max: f32, def: f32, fmt: impl Fn(f32) -> String, col: Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(50.0, 74.0), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let d = -resp.drag_delta().y / 160.0 * (max - min);
        let nv = (*v + d).clamp(min, max);
        if nv != *v {
            *v = nv;
            changed = true;
        }
    }
    if resp.double_clicked() && *v != def {
        *v = def;
        changed = true;
    }
    let _ = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
    let p = ui.painter_at(rect);
    let c = pos2(rect.center().x, rect.top() + 24.0);
    let r = 18.0;
    let t = ((*v - min) / (max - min)).clamp(0.0, 1.0);
    let arc = |a0: f32, a1: f32| -> Vec<Pos2> {
        (0..=32)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 32.0;
                pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect()
    };
    let (start, span) = (0.75 * PI, 1.5 * PI);
    p.add(Shape::line(arc(start, start + span), stroke(3.5, Color32::from_rgb(230, 230, 236))));
    if t > 0.003 {
        p.add(Shape::line(arc(start, start + span * t), stroke(3.5, col)));
    }
    let a = start + span * t;
    p.line_segment(
        [pos2(c.x + (r - 10.0) * a.cos(), c.y + (r - 10.0) * a.sin()), pos2(c.x + (r - 3.0) * a.cos(), c.y + (r - 3.0) * a.sin())],
        stroke(2.0, INK),
    );
    p.text(pos2(rect.center().x, rect.top() + 52.0), Align2::CENTER_CENTER, fmt(*v), FontId::proportional(11.0), INK);
    p.text(pos2(rect.center().x, rect.top() + 66.0), Align2::CENTER_CENTER, label, FontId::proportional(10.0), GREY);
    changed
}

fn db(v: f32) -> String {
    format!("{:+.1} dB", v)
}
fn hz(v: f32) -> String {
    if v >= 1000.0 {
        format!("{:.1} kHz", v / 1000.0)
    } else {
        format!("{:.0} Hz", v)
    }
}
fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}
fn ms(v: f32) -> String {
    format!("{:.0} ms", v)
}

/// Rounded white card with an ON/OFF pill. `body` returns true if a knob changed.
fn fx_card(ui: &mut egui::Ui, title: &str, on: &mut bool, col: Color32, body: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let mut changed = false;
    egui::Frame::none()
        .fill(Color32::WHITE)
        .rounding(Rounding::same(12.0))
        .stroke(stroke(1.0, if *on { col } else { HAIR }))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let txt = RichText::new(if *on { "ON" } else { "OFF" }).size(10.5).strong().color(if *on { Color32::WHITE } else { GREY });
                    let b = egui::Button::new(txt)
                        .fill(if *on { col } else { Color32::from_rgb(240, 240, 245) })
                        .stroke(egui::Stroke::NONE)
                        .rounding(Rounding::same(8.0))
                        .min_size(vec2(34.0, 18.0));
                    if ui.add(b).clicked() {
                        *on = !*on;
                        changed = true;
                    }
                    ui.label(RichText::new(title).strong().size(13.0));
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if body(ui) {
                        changed = true;
                        *on = true; // touching a knob switches the effect on
                    }
                });
            });
        });
    changed
}

impl App {
    /// "Mixer & Effects" tab: track strips | Master (spectrum + meters) | effects of the selected track.
    pub fn ui_fx(&mut self, ui: &mut egui::Ui) {
        let n = self.project.tracks.len();
        if n == 0 {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(RichText::new("Add a track to use the mixer and effects").color(GREY)));
            return;
        }
        let (w, h) = (ui.available_width(), ui.available_height());
        let tap = self.engine.tap.clone();
        ui.horizontal_top(|ui| {
            ui.add_space(14.0);
            let mw = (n as f32 * 70.0 + 8.0).min(w * 0.30);
            ui.allocate_ui_with_layout(vec2(mw, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.push_id("mixer_scroll", |ui| {
                    egui::ScrollArea::horizontal().auto_shrink([false, false]).show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            for i in 0..n {
                                self.mixer_strip(ui, i);
                            }
                        });
                    });
                });
            });
            ui.add_space(8.0);
            crate::analyzer::master_panel(ui, &tap, 340.0f32.min(w * 0.34), h);
            ui.add_space(8.0);
            let ew = ui.available_width();
            ui.allocate_ui_with_layout(vec2(ew, h), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.push_id("fx_scroll", |ui| self.ui_effects(ui, h));
            });
        });
    }

    fn mixer_strip(&mut self, ui: &mut egui::Ui, i: usize) {
        let tr = &self.project.tracks[i];
        let name: String = tr.name.chars().take(9).collect();
        let color = PALETTE[tr.color % 8];
        let (mut vol, mut pan, mut mute, mut solo) = (tr.volume, tr.pan, tr.mute, tr.solo);
        let selected = self.sel_track == i;
        // not clamped: an over (> 0 dBFS) lights the clip LED
        let level = self.meters.get(i).copied().unwrap_or(0.0);
        let (mut changed, mut select) = (false, false);
        let soft = Color32::from_rgb(240, 240, 245);

        egui::Frame::none()
            .fill(Color32::WHITE)
            .rounding(Rounding::same(12.0))
            .stroke(stroke(if selected { 1.5 } else { 1.0 }, if selected { ACCENT } else { HAIR }))
            .inner_margin(egui::Margin::same(7.0))
            .show(ui, |ui| {
                ui.set_width(50.0);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let lab = egui::Label::new(RichText::new(&name).size(11.0).strong().color(color)).sense(Sense::click());
                    if ui.add(lab).clicked() {
                        select = true;
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = 100.0;
                        let r = ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).vertical().show_value(false));
                        changed |= r.changed();
                        // dBFS meter: hover for the held peak, click to reset
                        crate::meters::meter(ui, vec2(8.0, 106.0), i, level, false);
                    });
                    ui.spacing_mut().slider_width = 42.0;
                    let r = ui.add(egui::Slider::new(&mut pan, -1.0..=1.0).show_value(false));
                    changed |= r.changed();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        let m = egui::Button::new(RichText::new("M").size(11.0).strong().color(if mute { Color32::WHITE } else { INK }))
                            .fill(if mute { RED } else { soft })
                            .stroke(egui::Stroke::NONE)
                            .min_size(vec2(20.0, 20.0));
                        if ui.add(m).clicked() {
                            mute = !mute;
                            changed = true;
                        }
                        let s = egui::Button::new(RichText::new("S").size(11.0).strong().color(if solo { Color32::WHITE } else { INK }))
                            .fill(if solo { ACCENT } else { soft })
                            .stroke(egui::Stroke::NONE)
                            .min_size(vec2(20.0, 20.0));
                        if ui.add(s).clicked() {
                            solo = !solo;
                            changed = true;
                        }
                    });
                });
            });

        if select {
            self.sel_track = i;
        }
        if changed {
            let ctx = ui.ctx().clone();
            self.edit_checkpoint(&ctx);
            let t = &mut self.project.tracks[i];
            t.volume = vol;
            t.pan = pan;
            t.mute = mute;
            t.solo = solo;
            self.push();
        }
    }

    fn ui_effects(&mut self, ui: &mut egui::Ui, h: f32) {
        let t = self.sel_track.min(self.project.tracks.len() - 1);
        let tr = &self.project.tracks[t];
        let (name, col) = (tr.name.clone(), PALETTE[tr.color % 8]);
        let mut fx = tr.fx.clone();
        let mut changed = false;
        let mut reset = false;

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Effects - {name}")).strong());
            ui.label(RichText::new("drag a knob, double-click to reset").size(11.5).color(GREY));
            if ui.small_button("Reset effects").clicked() {
                reset = true;
            }
        });
        egui::ScrollArea::horizontal().max_height((h - 28.0).max(80.0)).auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let eq = &mut fx.eq;
                changed |= fx_card(ui, "Equalizer", &mut eq.on, col, |ui| {
                    let a = knob(ui, "Low", &mut eq.low, -15.0, 15.0, 0.0, db, col);
                    let b = knob(ui, "Mid", &mut eq.mid, -15.0, 15.0, 0.0, db, col);
                    let mut lf = eq.mid_freq.ln();
                    let c = knob(ui, "Mid freq", &mut lf, 200f32.ln(), 5000f32.ln(), 1000f32.ln(), |v| hz(v.exp()), col);
                    if c {
                        eq.mid_freq = lf.exp();
                    }
                    let d = knob(ui, "High", &mut eq.high, -15.0, 15.0, 0.0, db, col);
                    a | b | c | d
                });
                let cp = &mut fx.comp;
                changed |= fx_card(ui, "Compressor", &mut cp.on, col, |ui| {
                    let a = knob(ui, "Thresh", &mut cp.threshold, -60.0, 0.0, -18.0, |v| format!("{:.0} dB", v), col);
                    let b = knob(ui, "Ratio", &mut cp.ratio, 1.0, 20.0, 3.0, |v| format!("{:.1}:1", v), col);
                    let c = knob(ui, "Attack", &mut cp.attack, 1.0, 100.0, 10.0, ms, col);
                    let d = knob(ui, "Release", &mut cp.release, 20.0, 1000.0, 120.0, ms, col);
                    let e = knob(ui, "Makeup", &mut cp.makeup, 0.0, 24.0, 0.0, |v| format!("{:.1} dB", v), col);
                    a | b | c | d | e
                });
                let rv = &mut fx.reverb;
                changed |= fx_card(ui, "Reverb", &mut rv.on, col, |ui| {
                    let a = knob(ui, "Mix", &mut rv.mix, 0.0, 1.0, 0.25, pct, col);
                    let b = knob(ui, "Size", &mut rv.size, 0.0, 1.0, 0.5, pct, col);
                    let c = knob(ui, "Damp", &mut rv.damp, 0.0, 1.0, 0.5, pct, col);
                    a | b | c
                });
                let dl = &mut fx.delay;
                changed |= fx_card(ui, "Delay", &mut dl.on, col, |ui| {
                    let a = knob(ui, "Time", &mut dl.time, 20.0, 1000.0, 350.0, ms, col);
                    let b = knob(ui, "Feedback", &mut dl.feedback, 0.0, 0.9, 0.35, pct, col);
                    let c = knob(ui, "Mix", &mut dl.mix, 0.0, 1.0, 0.25, pct, col);
                    a | b | c
                });
            });
        });

        if reset && fx != FxSettings::default() {
            fx = FxSettings::default();
            changed = true;
        }
        if changed {
            let ctx = ui.ctx().clone();
            self.edit_checkpoint(&ctx);
            self.project.tracks[t].fx = fx;
            self.push();
        }
    }
}