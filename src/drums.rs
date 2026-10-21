//! Synthesized drum kits, no samples: "Drum Kit" (acoustic style) and "808 Kit" (electronic).
//!
//! Every note is folded into 36..=59 (two octaves), so the whole keyboard plays drums:
//! 36 kick, 37 rim, 38/40 snare, 39 clap, 41/43/45/47/48/50 toms, 42/44 closed hat,
//! 46 open hat, 49/52/55/57 crash, 51/53/59 ride, 54/58 shaker, 56 cowbell.

use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub(crate) enum Pad {
    Kick,
    Rim,
    Snare(bool),     // tight
    Clap,
    Tom(f32),        // base frequency in Hz
    Hat(f32, f32),   // decay rate, duration in s
    Crash(f32, f32), // decay rate, duration in s
    Ride(bool),      // bell
    Cowbell,
    Shaker(f32), // decay rate
}

/// Folds any MIDI key into the 36..=59 drum range.
pub(crate) fn fold(key: u8) -> i32 {
    36 + (key as i32 - 36).rem_euclid(24)
}

pub(crate) fn pad(key: u8) -> Pad {
    match fold(key) {
        36 => Pad::Kick,
        37 => Pad::Rim,
        38 => Pad::Snare(false),
        39 => Pad::Clap,
        40 => Pad::Snare(true),
        41 => Pad::Tom(80.0),
        42 => Pad::Hat(60.0, 0.15),
        43 => Pad::Tom(100.0),
        44 => Pad::Hat(90.0, 0.08),
        45 => Pad::Tom(130.0),
        46 => Pad::Hat(7.0, 0.8),
        47 => Pad::Tom(160.0),
        48 => Pad::Tom(190.0),
        49 => Pad::Crash(1.6, 2.5),
        50 => Pad::Tom(220.0),
        51 => Pad::Ride(false),
        52 => Pad::Crash(2.2, 1.8),
        53 => Pad::Ride(true),
        54 => Pad::Shaker(18.0),
        55 => Pad::Crash(5.0, 0.9),
        56 => Pad::Cowbell,
        57 => Pad::Crash(1.3, 3.0),
        58 => Pad::Shaker(30.0),
        _ => Pad::Ride(false),
    }
}

/// White noise in [-1, 1) (xorshift32).
pub(crate) fn noise(rng: &mut u32) -> f32 {
    let mut x = *rng;
    if x == 0 {
        x = 0x1234_5678;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *rng = x;
    x as f32 / 2_147_483_648.0 - 1.0
}

/// One-pole high-pass (state in `st`, `a` = coefficient of the matching low-pass).
pub(crate) fn highpass(st: &mut f32, x: f32, a: f32) -> f32 {
    *st += a * (x - *st);
    x - *st
}

/// Six detuned square waves: the classic 808 "metallic" cymbal source.
pub(crate) fn metal(t: f32) -> f32 {
    const F: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
    F.iter().map(|f| if (TAU * f * t).sin() > 0.0 { 1.0 } else { -1.0 }).sum::<f32>() / 6.0
}

/// Next sample of a drum hit, or `None` once it is over.
///
/// `e808` selects the electronic kit, `t` is the time since the hit, `st` is per-voice filter
/// state (`st[0]` is a phase accumulator, `st[1]`/`st[2]` filter memories).
pub fn hit(e808: bool, key: u8, t: f32, dt: f32, rng: &mut u32, st: &mut [f32; 4]) -> Option<f32> {
    let e = e808;
    let n = noise(rng);
    let (s, dur) = match pad(key) {
        Pad::Kick => {
            let (f0, sweep, rate, dur) = if e { (42.0, 150.0, 28.0, 1.4) } else { (50.0, 110.0, 35.0, 0.45) };
            let f = f0 + sweep * (-t * rate).exp();
            st[0] = (st[0] + TAU * f * dt) % TAU;
            let body = st[0].sin() * (-t * if e { 2.6 } else { 9.0 }).exp();
            let click = n * (-t * 300.0).exp() * if e { 0.1 } else { 0.35 };
            let s = body + click;
            (if e { (s * 1.6).tanh() } else { s }, dur)
        }
        Pad::Rim => {
            let tone = (TAU * 440.0 * t).sin() + 0.6 * (TAU * 1800.0 * t).sin();
            ((tone * 0.6 + n * 0.4) * (-t * 110.0).exp(), 0.1)
        }
        Pad::Snare(tight) => {
            let k = if tight { 1.4 } else { 1.0 };
            let tone = ((TAU * 185.0 * t).sin() + 0.6 * (TAU * 330.0 * t).sin()) * (-t * 24.0 * k).exp();
            let nz = highpass(&mut st[1], n, 0.25) * (-t * if e { 14.0 } else { 17.0 } * k).exp();
            (0.55 * tone + 0.75 * nz, if tight { 0.22 } else { 0.35 })
        }
        Pad::Clap => {
            st[1] += 0.45 * (n - st[1]);
            st[2] += 0.08 * (n - st[2]);
            let band = (st[1] - st[2]) * 3.0;
            let mut env = 0.0;
            for b in [0.0f32, 0.011, 0.022] {
                if t >= b {
                    env += (-(t - b) * 160.0).exp();
                }
            }
            if t >= 0.033 {
                env += 1.2 * (-(t - 0.033) * if e { 18.0 } else { 26.0 }).exp();
            }
            (band * env * 0.5, 0.45)
        }
        Pad::Tom(base) => {
            let b = if e { base * 0.8 } else { base };
            let f = b * (1.0 + 0.7 * (-t * 22.0).exp());
            st[0] = (st[0] + TAU * f * dt) % TAU;
            let s = st[0].sin() * (-t * if e { 4.0 } else { 7.0 }).exp() + n * (-t * 200.0).exp() * 0.2;
            (s, if e { 1.2 } else { 0.7 })
        }
        Pad::Hat(rate, dur) => {
            let m = metal(t) * 0.7 + n * if e { 0.15 } else { 0.5 };
            (highpass(&mut st[1], m, 0.35) * (-t * rate).exp(), dur)
        }
        Pad::Crash(rate, dur) => {
            let m = metal(t) * 0.4 + n * 0.8;
            (highpass(&mut st[1], m, 0.3) * 0.8 * (-t * rate).exp(), dur)
        }
        Pad::Ride(bell) => {
            let f = if bell { 780.0 } else { 520.0 };
            let mut s = 0.0;
            for r in [1.0f32, 1.59, 2.14, 2.65, 3.2] {
                s += (TAU * f * r * t).sin() / r;
            }
            let tick = highpass(&mut st[1], n, 0.3) * (-t * 90.0).exp() * 0.5;
            (s * 0.35 * (-t * if bell { 2.5 } else { 3.5 }).exp() + tick, 1.6)
        }
        Pad::Cowbell => {
            let q = |f: f32| if (TAU * f * t).sin() > 0.0 { 1.0f32 } else { -1.0 };
            st[1] += 0.3 * (q(540.0) + q(800.0) - st[1]);
            (st[1] * 0.5 * (-t * if e { 9.0 } else { 12.0 }).exp(), 0.5)
        }
        Pad::Shaker(rate) => {
            let h = highpass(&mut st[1], n, 0.2);
            (h * 0.6 * (t / 0.008).min(1.0) * (-t * rate).exp(), 0.3)
        }
    };
    if t >= dur {
        return None;
    }
    // 10 ms fade-out so that the end of the hit never clicks.
    Some(s * ((dur - t) * 100.0).min(1.0))
}