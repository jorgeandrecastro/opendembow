//! Urban drum kits (0.6): "Trap Kit" and "Reggaeton Kit".
//!
//! Same note map as `drums.rs` (notes are folded into 36..=59), so a pattern keeps working when you
//! swap kits. Kick, rim, snares, clap, toms and hats are redesigned here; crashes, rides, cowbell and
//! shakers fall back to the electronic kit of `drums.rs`.
//!
//! Trap Kit: hard kick with a sub tail, snappy snare, tight hats, long 808 toms, and key 59 (B3) is
//! a long 808 boom. Reggaeton Kit: boxy dembow kick, bright gated snare, short hats, toms are congas.

use crate::drums::{fold, highpass, metal, noise, pad, Pad};
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq)]
pub enum Style {
    Trap,
    Reggaeton,
}

/// Three quick noise bursts followed by a tail (band-passed noise).
fn clap(t: f32, n: f32, st: &mut [f32; 4], tail: f32) -> f32 {
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
        env += 1.2 * (-(t - 0.033) * tail).exp();
    }
    band * env * 0.5
}

/// Next sample of an urban drum hit, or `None` once it is over.
pub fn hit(style: Style, key: u8, t: f32, dt: f32, rng: &mut u32, st: &mut [f32; 4]) -> Option<f32> {
    let trap = style == Style::Trap;
    let n = noise(rng);
    let (s, dur) = match pad(key) {
        // Trap only: long 808 boom with a quick pitch drop, saturated.
        _ if trap && fold(key) == 59 => {
            let f = 45.0 + 70.0 * (-t * 30.0).exp();
            st[0] = (st[0] + TAU * f * dt) % TAU;
            ((st[0].sin() * (-t * 1.3).exp() * 2.0).tanh(), 2.0)
        }
        Pad::Kick => {
            let (f0, sweep, rate, decay, dur) = if trap { (48.0, 120.0, 45.0, 5.5, 0.7) } else { (55.0, 140.0, 38.0, 11.0, 0.4) };
            let f = f0 + sweep * (-t * rate).exp();
            st[0] = (st[0] + TAU * f * dt) % TAU;
            let body = st[0].sin() * (-t * decay).exp() + if trap { 0.0 } else { 0.2 * (2.0 * st[0]).sin() * (-t * 20.0).exp() };
            let click = n * (-t * 400.0).exp() * if trap { 0.25 } else { 0.3 };
            ((body * if trap { 1.4 } else { 1.2 }).tanh() + click, dur)
        }
        Pad::Rim => {
            let tone = (TAU * 480.0 * t).sin() + 0.7 * (TAU * 1250.0 * t).sin();
            ((tone * 0.55 + n * 0.3) * (-t * if trap { 140.0 } else { 90.0 }).exp(), 0.12)
        }
        Pad::Snare(tight) => {
            let k = if tight { 1.5 } else { 1.0 };
            if trap {
                let tone = (TAU * 220.0 * t).sin() * (-t * 28.0 * k).exp();
                let nz = highpass(&mut st[1], n, 0.22) * (-t * 16.0 * k).exp();
                (0.45 * tone + 0.85 * nz, 0.3 / k)
            } else {
                // dembow snare: bright body + noise tail cut by a short gate (gated-reverb feel)
                let tone = ((TAU * 250.0 * t).sin() + 0.6 * (TAU * 480.0 * t).sin()) * (-t * 30.0 * k).exp();
                let gate = ((0.2 / k - t) / 0.01).clamp(0.0, 1.0);
                let nz = highpass(&mut st[1], n, 0.2) * (-t * 7.0 * k).exp() * gate;
                (0.5 * tone + 0.7 * nz, 0.25 / k)
            }
        }
        Pad::Clap => (clap(t, n, st, if trap { 18.0 } else { 34.0 }), 0.45),
        Pad::Tom(base) => {
            if trap {
                // long 808 tom
                let f = base * 0.8 * (1.0 + 0.7 * (-t * 22.0).exp());
                st[0] = (st[0] + TAU * f * dt) % TAU;
                (st[0].sin() * (-t * 4.0).exp() + n * (-t * 200.0).exp() * 0.2, 1.2)
            } else {
                // conga-like: higher, short, with a slap
                let f = base * 1.6 * (1.0 + 0.15 * (-t * 40.0).exp());
                st[0] = (st[0] + TAU * f * dt) % TAU;
                (st[0].sin() * (-t * 14.0).exp() + n * (-t * 250.0).exp() * 0.3, 0.35)
            }
        }
        Pad::Hat(rate, dur) => {
            let (r, m) = if trap {
                (if rate > 20.0 { rate * 1.8 } else { rate * 1.3 }, metal(t) * 0.55 + n * 0.35)
            } else {
                (rate, metal(t) * 0.3 + n * 0.7)
            };
            (highpass(&mut st[1], m, 0.4) * (-t * r).exp(), dur)
        }
        // Crashes, rides, cowbell and shakers: the electronic kit sounds good with both styles.
        _ => return crate::drums::hit(trap, key, t, dt, rng, st),
    };
    if t >= dur {
        return None;
    }
    // 10 ms fade-out so that the end of the hit never clicks.
    Some(s * ((dur - t) * 100.0).min(1.0))
}