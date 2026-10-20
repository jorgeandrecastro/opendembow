//! Extra melodic voices (0.6): 808 Bass, Fingered Bass, Electric Guitar, Synth Pluck.
//!
//! Same contract as the voices inside `instruments.rs`: returns `(sample, attack_s, level)`;
//! the caller applies velocity, attack and the release envelope.

use crate::instruments::{saw, Instrument};
use std::f32::consts::TAU;

fn lp_a(fc: f32, dt: f32) -> f32 {
    1.0 - (-TAU * fc.min(0.45 / dt) * dt).exp()
}

/// `ph`, `lp`, `lp2` are the voice's own oscillator phases and filter memories.
pub fn voice(inst: Instrument, f: f32, t: f32, dt: f32, ph: &mut [f32; 5], lp: &mut f32, lp2: &mut f32) -> (f32, f32, f32) {
    let dp = f * dt;
    match inst {
        // Sub sine with a short pitch drop on the attack, saturated, slow decay (reggaeton / trap).
        Instrument::Bass808 => {
            let fd = f * (1.0 + 0.5 * (-t * 40.0).exp());
            ph[0] = (ph[0] + fd * dt).fract();
            let s = (TAU * ph[0]).sin();
            let sat = (s * (1.8 + 1.2 * (-t * 3.0).exp())).tanh();
            (sat * (-t * 0.6).exp(), 0.002, 0.55)
        }
        // Saw + sine through an envelope-driven low-pass, with a plucked "thump".
        Instrument::ElectricBass => {
            ph[0] = (ph[0] + dp).fract();
            let raw = 0.6 * saw(ph[0], dp) + 0.5 * (TAU * ph[0]).sin();
            *lp += lp_a(300.0 + 1800.0 * (-t * 8.0).exp(), dt) * (raw - *lp);
            (*lp * (0.35 + 0.65 * (-t * 2.5).exp()), 0.004, 0.45)
        }
        // Plucked harmonics, overdriven, then darkened.
        Instrument::ElectricGuitar => {
            let a = TAU * f * t;
            let mut s = 0.0;
            for k in 1..=10 {
                let kf = k as f32;
                if f * kf > 12000.0 {
                    break;
                }
                s += (a * kf).sin() / kf.powf(0.9) * (-(0.7 + 0.45 * kf) * t).exp();
            }
            *lp += lp_a(3500.0, dt) * ((s * 2.2).tanh() - *lp);
            (*lp, 0.003, 0.3)
        }
        // Two detuned saws, fast filter + amp decay, two-pole low-pass.
        Instrument::SynthPluck => {
            ph[0] = (ph[0] + dp * 0.996).fract();
            ph[1] = (ph[1] + dp * 1.004).fract();
            let raw = (saw(ph[0], dp) + saw(ph[1], dp)) * 0.5;
            let a = lp_a(500.0 + 7000.0 * (-t * 14.0).exp(), dt);
            *lp += a * (raw - *lp);
            *lp2 += a * (*lp - *lp2);
            (*lp2 * (0.25 + 0.75 * (-t * 5.0).exp()), 0.002, 0.32)
        }
        _ => (0.0, 0.01, 0.0),
    }
}