//! Effects: per-track EQ, compressor, reverb and delay (settings + DSP).
//!
//! The DSP (`FxProcessor`) does not depend on the UI. The engine owns one `FxProcessor` per track
//! and calls, on the track's stereo buffers, BEFORE volume/pan:
//!
//! ```ignore
//! fx_proc[t].process_block(&project.tracks[t].fx, &mut left, &mut right);
//! crate::meter_bus::publish_gr(t, fx_proc[t].gain_reduction_db()); // live gain-reduction meter
//! ```
//!
//! The Mixer & Effects panel lives in `mixer.rs`; the EQ / compressor curves in `curves.rs`.

use std::f32::consts::PI;

// ======================================================================
// Settings (stored in every Track)
// ======================================================================

#[derive(Clone, PartialEq)]
pub struct EqFx {
    pub on: bool,
    pub low: f32,       // dB, low shelf
    pub mid: f32,       // dB, peak
    pub mid_freq: f32,  // Hz
    pub high: f32,      // dB, high shelf
    pub low_freq: f32,  // Hz, shelf corner (0.11)
    pub mid_q: f32,     // peak width (0.11)
    pub high_freq: f32, // Hz, shelf corner (0.11)
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
        Self {
            on: false,
            low: 0.0,
            mid: 0.0,
            mid_freq: 1000.0,
            high: 0.0,
            low_freq: 120.0,
            mid_q: 1.0,
            high_freq: 6000.0,
        }
    }
}
impl Default for CompFx {
    fn default() -> Self {
        Self {
            on: false,
            threshold: -18.0,
            ratio: 3.0,
            attack: 10.0,
            release: 120.0,
            makeup: 0.0,
        }
    }
}
impl Default for ReverbFx {
    fn default() -> Self {
        Self {
            on: false,
            mix: 0.25,
            size: 0.5,
            damp: 0.5,
        }
    }
}
impl Default for DelayFx {
    fn default() -> Self {
        Self {
            on: false,
            time: 350.0,
            feedback: 0.35,
            mix: 0.25,
        }
    }
}

impl FxSettings {
    /// True when no effect is enabled.
    pub fn is_bypassed(&self) -> bool {
        !(self.eq.on || self.comp.on || self.reverb.on || self.delay.on)
    }

    /// Every parameter as ("effect.name", value). Booleans are 0 / 1. Used by the MCP server and the project file.
    pub fn params(&self) -> Vec<(&'static str, f64)> {
        let b = |x: bool| if x { 1.0 } else { 0.0 };
        vec![
            ("eq.on", b(self.eq.on)),
            ("eq.low", self.eq.low as f64),
            ("eq.mid", self.eq.mid as f64),
            ("eq.mid_freq", self.eq.mid_freq as f64),
            ("eq.high", self.eq.high as f64),
            ("eq.low_freq", self.eq.low_freq as f64),
            ("eq.mid_q", self.eq.mid_q as f64),
            ("eq.high_freq", self.eq.high_freq as f64),
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
            "eq.low_freq" => self.eq.low_freq = f.clamp(40.0, 400.0),
            "eq.mid_q" => self.eq.mid_q = f.clamp(0.3, 6.0),
            "eq.high_freq" => self.eq.high_freq = f.clamp(2000.0, 16000.0),
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
        Self {
            b0: 1.0,
            ..Default::default()
        }
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
    let al = s / (2.0 * q.max(0.1));
    norm(
        1.0 + al * a,
        -2.0 * c,
        1.0 - al * a,
        1.0 + al / a,
        -2.0 * c,
        1.0 - al / a,
    )
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

/// Magnitude (dB) of a normalised biquad `[b0, b1, b2, a1, a2]` at `f` Hz.
fn response_db(c: [f32; 5], sr: f32, f: f32) -> f32 {
    let w = 2.0 * PI * f.min(sr * 0.499) / sr;
    let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
    let (nr, ni) = (c[0] + c[1] * c1 + c[2] * c2, -(c[1] * s1 + c[2] * s2));
    let (dr, di) = (1.0 + c[3] * c1 + c[4] * c2, -(c[3] * s1 + c[4] * s2));
    10.0 * ((nr * nr + ni * ni) / (dr * dr + di * di).max(1e-20))
        .max(1e-20)
        .log10()
}

/// Frequency response of the whole EQ in dB at `f` Hz (what the equalizer curve draws).
pub fn eq_response_db(eq: &EqFx, sr: f32, f: f32) -> f32 {
    response_db(low_shelf(sr, eq.low_freq, eq.low), sr, f)
        + response_db(peaking(sr, eq.mid_freq, eq.mid, eq.mid_q), sr, f)
        + response_db(high_shelf(sr, eq.high_freq, eq.high), sr, f)
}

struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}
impl Comb {
    fn new(n: usize) -> Self {
        Self {
            buf: vec![0.0; n.max(1)],
            i: 0,
            store: 0.0,
        }
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
        Self {
            buf: vec![0.0; n.max(1)],
            i: 0,
        }
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
        Self {
            buf: [vec![0.0; n], vec![0.0; n]],
            w: 0,
        }
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

    /// Current gain reduction of the compressor in dB (>= 0). 0 when the compressor is off.
    pub fn gain_reduction_db(&self) -> f32 {
        self.gr
    }

    fn refresh(&mut self, fx: &FxSettings) {
        if self.last.as_ref() == Some(fx) {
            return;
        }
        let sr = self.sr;
        self.eq[0].set(low_shelf(sr, fx.eq.low_freq, fx.eq.low));
        self.eq[1].set(peaking(sr, fx.eq.mid_freq, fx.eq.mid, fx.eq.mid_q));
        self.eq[2].set(high_shelf(sr, fx.eq.high_freq, fx.eq.high));
        self.att = (-1.0 / (sr * fx.comp.attack.max(0.1) * 0.001)).exp();
        self.rel = (-1.0 / (sr * fx.comp.release.max(1.0) * 0.001)).exp();
        self.last = Some(fx.clone());
    }

    /// Process a block in place (cheap change detection once per block).
    pub fn process_block(&mut self, fx: &FxSettings, l: &mut [f32], r: &mut [f32]) {
        if fx.is_bypassed() {
            self.gr = 0.0;
            return;
        }
        self.refresh(fx);
        if !fx.comp.on {
            self.gr = 0.0;
        }
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
            let target =
                (level - fx.comp.threshold).max(0.0) * (1.0 - 1.0 / fx.comp.ratio.max(1.0));
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
