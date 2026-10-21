//! "Latin Percussion" (0.6): congas, bongos, timbales, cowbell, agogo, guiro, claves, shakers...
//!
//! Notes are folded into 36..=59 like the drum kits, but this kit has its own map:
//! 36 conga low, 37 conga mute, 38 conga high, 39 conga slap, 40 bongo low, 41 bongo high,
//! 42 timbale low, 43 timbale high, 44 paila (timbale rim), 45 cowbell, 46 guiro short,
//! 47 guiro long, 48 claves, 49 maracas, 50 shaker, 51 tambourine, 52 agogo high, 53 agogo low,
//! 54 triangle, 55 cabasa, 56 woodblock, 57 hand clap, 58 long shaker, 59 tumba (low conga).

use crate::drums::{fold, highpass, noise};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
enum Perc {
    /// Membrane (conga, bongo, timbale): pitch, decay rate, duration, noisy slap amount.
    Drum { f: f32, rate: f32, dur: f32, slap: f32 },
    /// Bell / metal: two inharmonic sines.
    Metal { f: f32, ratio: f32, rate: f32 },
    /// Filtered noise (shakers, guiro); `scr` > 0 adds a scraping modulation in Hz.
    Noise { rate: f32, attack: f32, dur: f32, hp: f32, scr: f32 },
    /// Wooden click (claves, woodblock).
    Click { f: f32, rate: f32 },
}

fn perc(k: i32) -> Perc {
    match k {
        36 => Perc::Drum { f: 150.0, rate: 9.0, dur: 0.5, slap: 0.1 },
        37 => Perc::Drum { f: 150.0, rate: 30.0, dur: 0.2, slap: 0.3 },
        38 => Perc::Drum { f: 230.0, rate: 11.0, dur: 0.45, slap: 0.1 },
        39 => Perc::Drum { f: 230.0, rate: 25.0, dur: 0.25, slap: 0.8 },
        40 => Perc::Drum { f: 330.0, rate: 18.0, dur: 0.3, slap: 0.15 },
        41 => Perc::Drum { f: 450.0, rate: 20.0, dur: 0.28, slap: 0.15 },
        42 => Perc::Drum { f: 260.0, rate: 12.0, dur: 0.4, slap: 0.25 },
        43 => Perc::Drum { f: 380.0, rate: 12.0, dur: 0.4, slap: 0.25 },
        44 => Perc::Metal { f: 1400.0, ratio: 1.6, rate: 40.0 },
        45 => Perc::Metal { f: 540.0, ratio: 1.48, rate: 9.0 },
        46 => Perc::Noise { rate: 25.0, attack: 0.01, dur: 0.15, hp: 0.25, scr: 70.0 },
        47 => Perc::Noise { rate: 7.0, attack: 0.02, dur: 0.5, hp: 0.25, scr: 55.0 },
        48 => Perc::Click { f: 2500.0, rate: 70.0 },
        49 => Perc::Noise { rate: 30.0, attack: 0.004, dur: 0.15, hp: 0.3, scr: 0.0 },
        50 => Perc::Noise { rate: 22.0, attack: 0.008, dur: 0.22, hp: 0.2, scr: 0.0 },
        51 => Perc::Noise { rate: 12.0, attack: 0.002, dur: 0.4, hp: 0.35, scr: 0.0 },
        52 => Perc::Metal { f: 900.0, ratio: 1.5, rate: 12.0 },
        53 => Perc::Metal { f: 600.0, ratio: 1.5, rate: 12.0 },
        54 => Perc::Metal { f: 3500.0, ratio: 2.7, rate: 3.5 },
        55 => Perc::Noise { rate: 35.0, attack: 0.003, dur: 0.12, hp: 0.4, scr: 0.0 },
        56 => Perc::Click { f: 1000.0, rate: 45.0 },
        57 => Perc::Noise { rate: 35.0, attack: 0.001, dur: 0.2, hp: 0.1, scr: 0.0 },
        58 => Perc::Noise { rate: 16.0, attack: 0.01, dur: 0.3, hp: 0.2, scr: 0.0 },
        _ => Perc::Drum { f: 100.0, rate: 8.0, dur: 0.6, slap: 0.1 },
    }
}

/// Next sample of a percussion hit, or `None` once it is over.
pub fn hit(key: u8, t: f32, dt: f32, rng: &mut u32, st: &mut [f32; 4]) -> Option<f32> {
    let n = noise(rng);
    let (s, dur) = match perc(fold(key)) {
        Perc::Drum { f, rate, dur, slap } => {
            let fr = f * (1.0 + 0.12 * (-t * 50.0).exp());
            st[0] = (st[0] + TAU * fr * dt) % TAU;
            let body = st[0].sin() + 0.25 * (2.0 * st[0]).sin() * (-t * 8.0).exp();
            ((body * (-t * rate).exp() + n * slap * (-t * 300.0).exp()) * 0.7, dur)
        }
        Perc::Metal { f, ratio, rate } => {
            let s = (TAU * f * t).sin() + 0.7 * (TAU * f * ratio * t).sin();
            (s * (-t * rate).exp() * 0.6, (7.0 / rate).clamp(0.1, 1.5))
        }
        Perc::Noise { rate, attack, dur, hp, scr } => {
            let h = highpass(&mut st[1], n, hp);
            let am = if scr > 0.0 { 0.6 + 0.4 * (TAU * scr * t).sin().signum() } else { 1.0 };
            (h * am * (t / attack).min(1.0) * (-t * rate).exp() * 0.7, dur)
        }
        Perc::Click { f, rate } => ((TAU * f * t).sin() * (-t * rate).exp() * 0.8 + n * (-t * 600.0).exp() * 0.15, 0.2),
    };
    if t >= dur {
        return None;
    }
    // 10 ms fade-out so that the end of the hit never clicks.
    Some(s * ((dur - t) * 100.0).min(1.0))
}