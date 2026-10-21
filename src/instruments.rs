//! Built-in instruments: 33 synthesized voices grouped in categories, no samples required.
//! Drum kits live in `drums.rs` and `drums_urban.rs`, percussion in `latin.rs`,
//! the 0.6 melodic additions in `extra_voices.rs`.

use std::f32::consts::TAU;

/// Browser categories (like Ableton's sidebar).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Keys,
    Organs,
    Leads,
    Basses,
    Plucked,
    Pads,
    Mallets,
    Brass,
    Drums,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Keys,
        Category::Organs,
        Category::Leads,
        Category::Basses,
        Category::Plucked,
        Category::Pads,
        Category::Mallets,
        Category::Brass,
        Category::Drums,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Keys => "Keys",
            Category::Organs => "Organs",
            Category::Leads => "Leads",
            Category::Basses => "Basses",
            Category::Plucked => "Plucked",
            Category::Pads => "Pads & Strings",
            Category::Mallets => "Mallets & Bells",
            Category::Brass => "Brass & Winds",
            Category::Drums => "Drums & Percussion",
        }
    }
}

// NOTE: keep the first 8 variants (and their order in ALL) unchanged so that
// projects saved with an older version still load the same instruments.
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
    // --- new in 0.4 ---
    Clavinet,
    Harpsichord,
    ChurchOrgan,
    SquareLead,
    SuperSaw,
    SubBass,
    AcidBass,
    Guitar,
    Harp,
    WarmPad,
    Choir,
    Marimba,
    MusicBox,
    Vibraphone,
    Brass,
    Flute,
    // --- new in 0.6 ---
    DrumKit,
    Kit808,
    Bass808,
    ElectricBass,
    ElectricGuitar,
    SynthPluck,
    TrapKit,
    ReggaetonKit,
    LatinPerc,
}

impl Instrument {
    pub const ALL: [Instrument; 33] = [
        Instrument::Piano,
        Instrument::ElectricPiano,
        Instrument::Organ,
        Instrument::SynthLead,
        Instrument::SynthBass,
        Instrument::Pluck,
        Instrument::Strings,
        Instrument::Bell,
        Instrument::Clavinet,
        Instrument::Harpsichord,
        Instrument::ChurchOrgan,
        Instrument::SquareLead,
        Instrument::SuperSaw,
        Instrument::SubBass,
        Instrument::AcidBass,
        Instrument::Guitar,
        Instrument::Harp,
        Instrument::WarmPad,
        Instrument::Choir,
        Instrument::Marimba,
        Instrument::MusicBox,
        Instrument::Vibraphone,
        Instrument::Brass,
        Instrument::Flute,
        Instrument::DrumKit,
        Instrument::Kit808,
        Instrument::Bass808,
        Instrument::ElectricBass,
        Instrument::ElectricGuitar,
        Instrument::SynthPluck,
        Instrument::TrapKit,
        Instrument::ReggaetonKit,
        Instrument::LatinPerc,
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
            Instrument::Clavinet => "Clavinet",
            Instrument::Harpsichord => "Harpsichord",
            Instrument::ChurchOrgan => "Church Organ",
            Instrument::SquareLead => "Square Lead",
            Instrument::SuperSaw => "Super Saw",
            Instrument::SubBass => "Sub Bass",
            Instrument::AcidBass => "Acid Bass",
            Instrument::Guitar => "Nylon Guitar",
            Instrument::Harp => "Harp",
            Instrument::WarmPad => "Warm Pad",
            Instrument::Choir => "Choir Ahh",
            Instrument::Marimba => "Marimba",
            Instrument::MusicBox => "Music Box",
            Instrument::Vibraphone => "Vibraphone",
            Instrument::Brass => "Brass Section",
            Instrument::Flute => "Flute",
            Instrument::DrumKit => "Drum Kit",
            Instrument::Kit808 => "808 Kit",
            Instrument::Bass808 => "808 Bass",
            Instrument::ElectricBass => "Fingered Bass",
            Instrument::ElectricGuitar => "Electric Guitar",
            Instrument::SynthPluck => "Synth Pluck",
            Instrument::TrapKit => "Trap Kit",
            Instrument::ReggaetonKit => "Reggaeton Kit",
            Instrument::LatinPerc => "Latin Percussion",
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
            Instrument::Clavinet => "Funky, bright and snappy",
            Instrument::Harpsichord => "Baroque, crisp and dry",
            Instrument::ChurchOrgan => "Big pipe organ, slow attack",
            Instrument::SquareLead => "Hollow chiptune-style lead",
            Instrument::SuperSaw => "Five fat detuned saws",
            Instrument::SubBass => "Deep, clean sine sub",
            Instrument::AcidBass => "Squelchy resonant saw",
            Instrument::Guitar => "Soft nylon-string pluck",
            Instrument::Harp => "Airy, long ringing strings",
            Instrument::WarmPad => "Slow, dark, evolving pad",
            Instrument::Choir => "Vocal 'ahh' formants",
            Instrument::Marimba => "Woody mallet, short",
            Instrument::MusicBox => "Tiny, sparkling comb",
            Instrument::Vibraphone => "Metal bars with tremolo",
            Instrument::Brass => "Swelling brass stack",
            Instrument::Flute => "Breathy, soft wind",
            Instrument::DrumKit => "Acoustic-style kit, GM note map",
            Instrument::Kit808 => "Electronic kit, long 808 kick",
            Instrument::Bass808 => "Booming saturated sub",
            Instrument::ElectricBass => "Round fingered bass",
            Instrument::ElectricGuitar => "Overdriven plucked guitar",
            Instrument::SynthPluck => "Short, bright saw pluck",
            Instrument::TrapKit => "Hard trap drums, 808 boom on B3",
            Instrument::ReggaetonKit => "Dembow kick, gated snare, congas",
            Instrument::LatinPerc => "Congas, bongos, timbales, shakers",
        }
    }

    pub fn category(self) -> Category {
        match self {
            Instrument::Piano | Instrument::ElectricPiano | Instrument::Clavinet | Instrument::Harpsichord => Category::Keys,
            Instrument::Organ | Instrument::ChurchOrgan => Category::Organs,
            Instrument::SynthLead | Instrument::SquareLead | Instrument::SuperSaw => Category::Leads,
            Instrument::SynthBass | Instrument::SubBass | Instrument::AcidBass | Instrument::Bass808 | Instrument::ElectricBass => {
                Category::Basses
            }
            Instrument::Pluck | Instrument::Guitar | Instrument::Harp | Instrument::ElectricGuitar | Instrument::SynthPluck => {
                Category::Plucked
            }
            Instrument::Strings | Instrument::WarmPad | Instrument::Choir => Category::Pads,
            Instrument::Bell | Instrument::Marimba | Instrument::MusicBox | Instrument::Vibraphone => Category::Mallets,
            Instrument::Brass | Instrument::Flute => Category::Brass,
            Instrument::DrumKit | Instrument::Kit808 | Instrument::TrapKit | Instrument::ReggaetonKit | Instrument::LatinPerc => Category::Drums,
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
            Instrument::Clavinet => 25.0,
            Instrument::Harpsichord => 40.0,
            Instrument::ChurchOrgan => 18.0,
            Instrument::SquareLead => 14.0,
            Instrument::SuperSaw => 10.0,
            Instrument::SubBass => 25.0,
            Instrument::AcidBass => 28.0,
            Instrument::Guitar => 16.0,
            Instrument::Harp => 8.0,
            Instrument::WarmPad => 2.5,
            Instrument::Choir => 3.0,
            Instrument::Marimba => 12.0,
            Instrument::MusicBox => 8.0,
            Instrument::Vibraphone => 5.0,
            Instrument::Brass => 12.0,
            Instrument::Flute => 10.0,
            // Drums are one-shots: they end by themselves, note-off must not cut them.
            Instrument::DrumKit | Instrument::Kit808 | Instrument::TrapKit | Instrument::ReggaetonKit | Instrument::LatinPerc => 0.0,
            Instrument::Bass808 => 18.0,
            Instrument::ElectricBass => 22.0,
            Instrument::ElectricGuitar => 14.0,
            Instrument::SynthPluck => 20.0,
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

pub(crate) fn saw(ph: f32, dp: f32) -> f32 {
    2.0 * ph - 1.0 - polyblep(ph, dp)
}

pub(crate) fn square(ph: f32, dp: f32) -> f32 {
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
    ph: [f32; 5],
    lp: f32,
    lp2: f32,
    /// Noise generator and filter memories (drums).
    rng: u32,
    st: [f32; 4],
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
            ph: [0.0; 5],
            lp: 0.0,
            lp2: 0.0,
            rng: 0x9E37_79B9 ^ (key as u32 + 1).wrapping_mul(0x85EB_CA6B),
            st: [0.0; 4],
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

            // ------------------------------------------------ Keys
            Instrument::Clavinet => {
                let a = TAU * f * t;
                let mut s = 0.0;
                for k in 1..=8 {
                    let kf = k as f32;
                    if f * kf > 16000.0 {
                        break;
                    }
                    s += (a * kf).sin() / kf.powf(0.7) * (-(3.0 + 2.5 * kf) * t).exp();
                }
                (s * 0.8, 0.002, 0.3)
            }
            Instrument::Harpsichord => {
                let a = TAU * f * t;
                let mut s = 0.0;
                for k in 1..=14 {
                    let kf = k as f32;
                    if f * kf > 16000.0 {
                        break;
                    }
                    s += (a * kf).sin() / kf * (-(2.5 + 0.4 * kf) * t).exp();
                }
                (s * 0.9, 0.001, 0.3)
            }

            // ------------------------------------------------ Organs
            Instrument::ChurchOrgan => {
                let bars = [(0.5, 0.6), (1.0, 1.0), (2.0, 0.7), (3.0, 0.4), (4.0, 0.5), (5.0, 0.2), (8.0, 0.2)];
                let trem = 1.0 + 0.03 * (TAU * 5.0 * t).sin();
                let s: f32 = bars.iter().map(|(h, a)| a * (TAU * f * h * t).sin()).sum();
                (s / 3.6 * trem, 0.03, 0.26)
            }

            // ------------------------------------------------ Leads
            Instrument::SquareLead => {
                self.ph[0] = (self.ph[0] + dp).fract();
                self.ph[1] = (self.ph[1] + dp * 1.004).fract();
                let raw = (square(self.ph[0], dp) + square(self.ph[1], dp * 1.004)) * 0.5;
                self.lp += lp_a(1500.0 + 3500.0 * (-4.0 * t).exp()) * (raw - self.lp);
                (self.lp, 0.005, 0.22)
            }
            Instrument::SuperSaw => {
                let det = [0.985, 0.993, 1.0, 1.007, 1.015];
                let mut raw = 0.0;
                for i in 0..5 {
                    self.ph[i] = (self.ph[i] + dp * det[i]).fract();
                    raw += saw(self.ph[i], dp) / 5.0;
                }
                self.lp += lp_a(3000.0 + 5000.0 * (-2.0 * t).exp()) * (raw - self.lp);
                (self.lp, 0.01, 0.28)
            }

            // ------------------------------------------------ Basses
            Instrument::SubBass => {
                let a = TAU * f * t;
                let s = a.sin() + 0.25 * (2.0 * a).sin() * (-6.0 * t).exp();
                (s, 0.006, 0.5)
            }
            Instrument::AcidBass => {
                self.ph[0] = (self.ph[0] + dp).fract();
                let raw = saw(self.ph[0], dp);
                let a = lp_a(200.0 + 2800.0 * (-7.0 * t).exp());
                self.lp += a * (raw - self.lp);
                self.lp2 += a * (self.lp - self.lp2);
                (self.lp2 * 1.8, 0.003, 0.4)
            }

            // ------------------------------------------------ Plucked
            Instrument::Guitar => {
                let a = TAU * f * t;
                let mut s = 0.0;
                for k in 1..=14 {
                    let kf = k as f32;
                    if f * kf > 16000.0 {
                        break;
                    }
                    s += (a * kf).sin() / kf.powf(1.1) * (-(1.2 + 0.9 * kf) * t).exp();
                }
                (s, 0.002, 0.32)
            }
            Instrument::Harp => {
                let a = TAU * f * t;
                let mut s = 0.0;
                for k in 1..=8 {
                    let kf = k as f32;
                    if f * kf > 16000.0 {
                        break;
                    }
                    s += (a * kf).sin() / kf.powf(1.3) * (-(1.0 + 0.5 * kf) * t).exp();
                }
                (s, 0.002, 0.34)
            }

            // ------------------------------------------------ Pads & strings
            Instrument::WarmPad => {
                let det = [0.997, 1.0, 1.003];
                let mut raw = 0.0;
                for i in 0..3 {
                    self.ph[i] = (self.ph[i] + dp * det[i]).fract();
                    raw += saw(self.ph[i], dp) / 3.0;
                }
                raw += 0.4 * (TAU * f * t).sin();
                self.lp += lp_a(600.0 + 900.0 * (1.0 - (-0.8 * t).exp())) * (raw - self.lp);
                (self.lp, 0.6, 0.25)
            }
            Instrument::Choir => {
                let a = TAU * f * t;
                let mut s = 0.0;
                for k in 1..=24 {
                    let kf = k as f32;
                    let fk = f * kf;
                    if fk > 5000.0 {
                        break;
                    }
                    let w = (-((fk - 800.0) / 350.0).powi(2)).exp()
                        + 0.7 * (-((fk - 1200.0) / 300.0).powi(2)).exp()
                        + 0.15 * (-((fk - 2600.0) / 500.0).powi(2)).exp();
                    s += w / kf.sqrt() * ((a * kf).sin() + (a * kf * 1.004).sin()) * 0.5;
                }
                let norm = (200.0 / f.max(200.0)).sqrt();
                (s * 0.35 * norm, 0.25, 0.2)
            }

            // ------------------------------------------------ Mallets & bells
            Instrument::Marimba => {
                let a = TAU * f * t;
                let mut s = a.sin() * (-6.0 * t).exp();
                if f * 4.0 < 16000.0 {
                    s += 0.3 * (a * 4.0).sin() * (-14.0 * t).exp();
                }
                if f * 9.2 < 16000.0 {
                    s += 0.1 * (a * 9.2).sin() * (-25.0 * t).exp();
                }
                (s, 0.001, 0.4)
            }
            Instrument::MusicBox => {
                let parts = [(1.0, 1.0, 2.0), (2.0, 0.3, 3.5), (4.1, 0.25, 5.0)];
                let mut s = 0.0;
                for (r, a, d) in parts {
                    if f * r < 16000.0 {
                        s += a * (TAU * f * r * t).sin() * (-d * t).exp();
                    }
                }
                (s, 0.001, 0.3)
            }
            Instrument::Vibraphone => {
                let a = TAU * f * t;
                let trem = 0.8 + 0.2 * (TAU * 5.0 * t).sin();
                let mut s = a.sin() * (-0.9 * t).exp();
                if f * 4.0 < 16000.0 {
                    s += 0.15 * (a * 4.0).sin() * (-3.0 * t).exp();
                }
                (s * trem, 0.002, 0.35)
            }

            // ------------------------------------------------ Brass & winds
            Instrument::Brass => {
                self.ph[0] = (self.ph[0] + dp).fract();
                self.ph[1] = (self.ph[1] + dp * 1.002).fract();
                let raw = (saw(self.ph[0], dp) + saw(self.ph[1], dp)) * 0.5;
                self.lp += lp_a(400.0 + 3000.0 * (1.0 - (-8.0 * t).exp())) * (raw - self.lp);
                (self.lp, 0.06, 0.3)
            }
            Instrument::Flute => {
                let a = TAU * f * t;
                let vib = 0.25 * (1.0 - (-2.0 * t).exp()) * (TAU * 5.0 * t).sin();
                let tone = (a + vib).sin() + 0.15 * (2.0 * (a + vib)).sin() + 0.05 * (3.0 * (a + vib)).sin();
                let noise = ((t * 12345.678).sin() * 43758.5453).fract() - 0.5;
                (tone + 0.06 * noise, 0.08, 0.3)
            }

            // ------------------------------------------------ Drums (0.6, see drums.rs)
            Instrument::DrumKit | Instrument::Kit808 => {
                match crate::drums::hit(self.inst == Instrument::Kit808, self.key, t, dt, &mut self.rng, &mut self.st) {
                    Some(x) => (x, 0.0005, 0.6),
                    None => return None, // a drum hit ends by itself: no 12 s voice
                }
            }

            Instrument::TrapKit | Instrument::ReggaetonKit => {
                let style = if self.inst == Instrument::TrapKit { crate::drums_urban::Style::Trap } else { crate::drums_urban::Style::Reggaeton };
                match crate::drums_urban::hit(style, self.key, t, dt, &mut self.rng, &mut self.st) {
                    Some(x) => (x, 0.0005, 0.6),
                    None => return None,
                }
            }
            Instrument::LatinPerc => match crate::latin::hit(self.key, t, dt, &mut self.rng, &mut self.st) {
                Some(x) => (x, 0.0005, 0.6),
                None => return None,
            },

            // ------------------------------------------------ Extra voices (0.6, see extra_voices.rs)
            Instrument::Bass808 | Instrument::ElectricBass | Instrument::ElectricGuitar | Instrument::SynthPluck => {
                crate::extra_voices::voice(self.inst, f, t, dt, &mut self.ph, &mut self.lp, &mut self.lp2)
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