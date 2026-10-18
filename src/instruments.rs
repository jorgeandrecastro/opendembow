//! Built-in instruments: eight synthesized voices, no samples required.

use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instrument {
    Piano,
    ElectricPiano,
    Organ,
    SynthLead,
    SynthBass,
    Pluck,
    Strings,
    Bell,
}

impl Instrument {
    pub const ALL: [Instrument; 8] = [
        Instrument::Piano,
        Instrument::ElectricPiano,
        Instrument::Organ,
        Instrument::SynthLead,
        Instrument::SynthBass,
        Instrument::Pluck,
        Instrument::Strings,
        Instrument::Bell,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Instrument::Piano => "Grand Piano",
            Instrument::ElectricPiano => "Electric Piano",
            Instrument::Organ => "Drawbar Organ",
            Instrument::SynthLead => "Synth Lead",
            Instrument::SynthBass => "Synth Bass",
            Instrument::Pluck => "Pluck",
            Instrument::Strings => "Strings Pad",
            Instrument::Bell => "Bell",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Instrument::Piano => "Warm, two-string piano",
            Instrument::ElectricPiano => "FM tine, mellow",
            Instrument::Organ => "Classic drawbars",
            Instrument::SynthLead => "Detuned saws, bright",
            Instrument::SynthBass => "Saw + sub, punchy",
            Instrument::Pluck => "Short, plucked string",
            Instrument::Strings => "Slow, lush ensemble",
            Instrument::Bell => "Glassy inharmonic bell",
        }
    }

    fn release_rate(self) -> f32 {
        match self {
            Instrument::Piano => 14.0,
            Instrument::ElectricPiano => 10.0,
            Instrument::Organ => 40.0,
            Instrument::SynthLead => 14.0,
            Instrument::SynthBass => 30.0,
            Instrument::Pluck => 12.0,
            Instrument::Strings => 3.5,
            Instrument::Bell => 6.0,
        }
    }
}

fn polyblep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        2.0 * x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + 2.0 * x + 1.0
    } else {
        0.0
    }
}

fn saw(ph: f32, dp: f32) -> f32 {
    2.0 * ph - 1.0 - polyblep(ph, dp)
}

fn square(ph: f32, dp: f32) -> f32 {
    let s = if ph < 0.5 { 1.0 } else { -1.0 };
    s + polyblep(ph, dp) - polyblep((ph + 0.5).fract(), dp)
}

/// One sounding note.
pub struct Voice {
    pub track: usize,
    pub key: u8,
    pub live: bool,
    pub delay: usize,
    pub hold: usize,
    inst: Instrument,
    freq: f32,
    t: f32,
    vel: f32,
    rel: Option<f32>,
    ph: [f32; 3],
    lp: f32,
}

impl Voice {
    pub fn new(track: usize, inst: Instrument, key: u8, vel: f32, delay: usize, hold: usize) -> Self {
        Self {
            track,
            key,
            live: false,
            delay,
            hold,
            inst,
            freq: 440.0 * 2f32.powf((key as f32 - 69.0) / 12.0),
            t: 0.0,
            vel,
            rel: None,
            ph: [0.0; 3],
            lp: 0.0,
        }
    }

    /// A note held by the player (keyboard / mouse) until `release`.
    pub fn live(track: usize, inst: Instrument, key: u8, vel: f32) -> Self {
        let mut v = Self::new(track, inst, key, vel, 0, usize::MAX);
        v.live = true;
        v
    }

    pub fn released(&self) -> bool {
        self.rel.is_some()
    }

    pub fn release(&mut self) {
        if self.rel.is_none() {
            self.rel = Some(self.t);
        }
    }

    /// Next sample, or `None` when the voice has finished.
    pub fn next(&mut self, dt: f32) -> Option<f32> {
        let t = self.t;
        self.t += dt;
        if t > 12.0 {
            return None;
        }
        let f = self.freq;
        let dp = f * dt;
        let lp_a = |fc: f32| 1.0 - (-TAU * fc.min(0.45 / dt) * dt).exp();
        let (s, attack, level) = match self.inst {
            Instrument::Piano => {
                let tone = (f / 261.63).powf(0.4);
                let mut s = 0.0;
                for k in 1..=10 {
                    let kf = k as f32;
                    let fk = f * kf * (1.0 + 0.0003 * kf * kf).sqrt();
                    if fk > 16000.0 {
                        break;
                    }
                    let env = (-(0.55 + 0.75 * kf) * tone * t).exp();
                    let a = TAU * fk * t;
                    s += env / kf.powf(1.2) * (a.sin() + (a * 1.0013).sin()) * 0.5;
                }
                (s, 0.004, 0.3)
            }
            Instrument::ElectricPiano => {
                let a = TAU * f * t;
                let idx = 1.6 * (-3.0 * t).exp();
                let tine = 0.12 * (a * 7.0).sin() * (-10.0 * t).exp();
                (((a + idx * a.sin()).sin() + tine) * (-1.4 * t).exp(), 0.003, 0.35)
            }
            Instrument::Organ => {
                let bars = [(1.0, 1.0), (2.0, 0.8), (3.0, 0.5), (4.0, 0.4), (6.0, 0.25), (8.0, 0.2)];
                let s: f32 = bars.iter().map(|(h, a)| a * (TAU * f * h * t).sin()).sum();
                (s / 3.2, 0.008, 0.28)
            }
            Instrument::SynthLead => {
                self.ph[0] = (self.ph[0] + dp * 0.997).fract();
                self.ph[1] = (self.ph[1] + dp * 1.003).fract();
                let raw = (saw(self.ph[0], dp) + saw(self.ph[1], dp)) * 0.5;
                self.lp += lp_a(900.0 + 4000.0 * (-3.0 * t).exp()) * (raw - self.lp);
                (self.lp, 0.006, 0.28)
            }
            Instrument::SynthBass => {
                self.ph[0] = (self.ph[0] + dp).fract();
                self.ph[1] = (self.ph[1] + dp * 0.5).fract();
                let raw = 0.7 * saw(self.ph[0], dp) + 0.6 * square(self.ph[1], dp * 0.5);
                self.lp += lp_a(250.0 + 1600.0 * (-9.0 * t).exp()) * (raw - self.lp);
                (self.lp * (0.75 + 0.25 * (-5.0 * t).exp()), 0.004, 0.4)
            }
            Instrument::Pluck => {
                let mut s = 0.0;
                for k in 1..=12 {
                    let kf = k as f32;
                    if f * kf > 16000.0 {
                        break;
                    }
                    s += (TAU * f * kf * t).sin() / kf * (-(2.0 + 1.8 * kf) * t).exp();
                }
                (s, 0.002, 0.3)
            }
            Instrument::Strings => {
                let det = [0.995, 1.0, 1.005];
                let mut raw = 0.0;
                for i in 0..3 {
                    self.ph[i] = (self.ph[i] + dp * det[i]).fract();
                    raw += saw(self.ph[i], dp) / 3.0;
                }
                self.lp += lp_a(2000.0) * (raw - self.lp);
                (self.lp, 0.35, 0.25)
            }
            Instrument::Bell => {
                let parts = [(1.0, 1.0, 0.5), (2.76, 0.6, 0.9), (5.4, 0.4, 1.4), (8.93, 0.3, 2.0)];
                let mut s = 0.0;
                for (r, a, d) in parts {
                    if f * r < 16000.0 {
                        s += a * (TAU * f * r * t).sin() * (-d * t).exp();
                    }
                }
                (s, 0.002, 0.3)
            }
        };
        let mut g = (t / attack).min(1.0) * self.vel * level;
        if let Some(tr) = self.rel {
            let r = (-(t - tr) * self.inst.release_rate()).exp();
            if r < 0.001 {
                return None;
            }
            g *= r;
        }
        Some(s * g)
    }
}
