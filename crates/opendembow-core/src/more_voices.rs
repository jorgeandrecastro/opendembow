//! More melodic voices: 8 pianos and 17 other instruments (organs, leads, basses,
//! plucked, pads, mallets, brass & winds). All synthesized, no samples.
//!
//! Same contract as `extra_voices.rs`: returns `(sample, attack_s, level)`; the caller applies
//! velocity, attack and the release envelope.

use crate::instruments::{Instrument, saw, square};
use std::f32::consts::TAU;

fn lp_a(fc: f32, dt: f32) -> f32 {
    1.0 - (-TAU * fc.min(0.45 / dt) * dt).exp()
}

/// White noise in -1..1 (xorshift).
fn noise(rng: &mut u32) -> f32 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 17;
    *rng ^= *rng << 5;
    (*rng as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// A string-like note: `n` partials, 1/k^`roll` amplitude, decay `d0 + d1*k` per second,
/// slight stretch (`inh`) of the upper partials like a real piano string.
fn string(f: f32, t: f32, n: u32, roll: f32, d0: f32, d1: f32, inh: f32) -> f32 {
    let mut s = 0.0;
    for k in 1..=n {
        let kf = k as f32;
        let fk = f * kf * (1.0 + inh * kf * kf).sqrt();
        if fk > 16000.0 {
            break;
        }
        s += (TAU * fk * t).sin() / kf.powf(roll) * (-(d0 + d1 * kf) * t).exp();
    }
    s
}

/// A sum of decaying inharmonic partials: `(ratio, amplitude, decay)`.
fn partials(f: f32, t: f32, parts: &[(f32, f32, f32)]) -> f32 {
    let mut s = 0.0;
    for &(r, a, d) in parts {
        if f * r < 16000.0 {
            s += a * (TAU * f * r * t).sin() * (-d * t).exp();
        }
    }
    s
}

/// `ph`, `lp`, `lp2` are the voice's own oscillator phases and filter memories, `rng` its noise.
#[allow(clippy::too_many_arguments)]
pub fn voice(
    inst: Instrument,
    f: f32,
    t: f32,
    dt: f32,
    ph: &mut [f32; 5],
    lp: &mut f32,
    lp2: &mut f32,
    rng: &mut u32,
) -> (f32, f32, f32) {
    let dp = f * dt;
    let a = TAU * f * t;
    match inst {
        // ------------------------------------------------ Pianos
        // Shorter, rounder than the grand piano, with a soft hammer thump.
        Instrument::UprightPiano => {
            let tone = (f / 261.63).powf(0.5);
            let s = (string(f, t, 8, 1.3, 0.9 * tone, 1.1 * tone, 0.0004)
                + string(f * 1.0008, t, 8, 1.3, 0.9 * tone, 1.1 * tone, 0.0004))
                * 0.5;
            let thump = 0.12 * (TAU * f * 0.5 * t).sin() * (-45.0 * t).exp();
            (s + thump, 0.003, 0.32)
        }
        // Many strong upper partials, slow decay: a bright, modern studio piano.
        Instrument::BrightPiano => {
            let tone = (f / 261.63).powf(0.35);
            let s = string(f, t, 16, 0.95, 0.5 * tone, 0.45 * tone, 0.0003);
            let click = 0.2 * (TAU * f * 6.0 * t).sin() * (-60.0 * t).exp();
            ((s + click) * 0.45, 0.002, 0.28)
        }
        // Two strings per note, tuned a little apart: the saloon piano.
        Instrument::HonkyTonk => {
            let tone = (f / 261.63).powf(0.45);
            let s = (string(f * 1.007, t, 9, 1.2, 1.0 * tone, 1.0 * tone, 0.0005)
                + string(f * 0.993, t, 9, 1.2, 1.0 * tone, 1.0 * tone, 0.0005))
                * 0.5;
            (s, 0.003, 0.3)
        }
        // Muted by felt: few partials, slow attack, long and dark.
        Instrument::FeltPiano => {
            let tone = (f / 261.63).powf(0.3);
            let s = string(f, t, 4, 1.6, 0.35 * tone, 0.55 * tone, 0.0002);
            *lp += lp_a(1800.0, dt) * (s - *lp);
            (*lp * 1.3, 0.012, 0.38)
        }
        // Tiny metal bars: inharmonic, short and glassy.
        Instrument::ToyPiano => {
            let s = partials(
                f,
                t,
                &[
                    (1.0, 1.0, 6.0),
                    (2.4, 0.55, 9.0),
                    (4.1, 0.35, 13.0),
                    (6.3, 0.2, 18.0),
                ],
            );
            let click = 0.25 * noise(rng) * (-120.0 * t).exp();
            (s + click, 0.001, 0.32)
        }
        // FM tine with a long bark, soft saturation: the classic electric piano.
        Instrument::Rhodes => {
            let idx = 2.2 * (-4.0 * t).exp() + 0.25;
            let body = (a + idx * a.sin()).sin() * (-1.0 * t).exp();
            let tine = 0.18 * (a * 8.0).sin() * (-12.0 * t).exp();
            (((body + tine) * 1.4).tanh() * 0.8, 0.003, 0.36)
        }
        // Reedy electric piano: even harmonics, a bit of growl and a slow tremolo.
        Instrument::Wurlitzer => {
            let trem = 1.0 + 0.06 * (TAU * 5.5 * t).sin();
            let s = ((a).sin() + 0.5 * (2.0 * a).sin() + 0.25 * (3.0 * a).sin())
                * (-1.7 * t).exp();
            *lp += lp_a(2600.0, dt) * ((s * 1.6).tanh() - *lp);
            (*lp * trem, 0.004, 0.36)
        }
        // DX-style FM piano: a bright 14x operator for the hammer, a soft body.
        Instrument::FmPiano => {
            let idx = 2.5 * (-5.0 * t).exp() + 0.5;
            let hammer = 0.5 * (-25.0 * t).exp() * (a * 14.0).sin();
            (
                (a + idx * a.sin() + hammer).sin() * (-1.2 * t).exp(),
                0.002,
                0.34,
            )
        }

        // ------------------------------------------------ Organs
        // Drawbars with a percussive 3rd harmonic, key click, vibrato and drive.
        Instrument::RockOrgan => {
            let bars = [
                (1.0, 1.0),
                (2.0, 1.0),
                (3.0, 0.8),
                (4.0, 0.6),
                (6.0, 0.4),
                (8.0, 0.3),
            ];
            let vib = 1.0 + 0.004 * (TAU * 6.5 * t).sin();
            let mut s: f32 = bars
                .iter()
                .map(|(h, amp)| amp * (TAU * f * vib * h * t).sin())
                .sum();
            s += 0.6 * (TAU * f * 3.0 * t).sin() * (-9.0 * t).exp();
            (((s / 3.6) * 1.8).tanh(), 0.004, 0.26)
        }
        // Two reeds slightly apart plus an octave square, bellows swell.
        Instrument::Accordion => {
            ph[0] = (ph[0] + dp).fract();
            ph[1] = (ph[1] + dp * 1.006).fract();
            ph[2] = (ph[2] + dp * 2.0).fract();
            let raw = 0.45 * saw(ph[0], dp) + 0.45 * saw(ph[1], dp * 1.006)
                + 0.25 * square(ph[2], dp * 2.0);
            *lp += lp_a(1900.0, dt) * (raw - *lp);
            let swell = 0.85 + 0.15 * (TAU * 4.5 * t).sin();
            (*lp * swell, 0.05, 0.28)
        }

        // ------------------------------------------------ Leads
        // A pulse whose width is swept by a slow LFO.
        Instrument::PwmLead => {
            ph[0] = (ph[0] + dp).fract();
            let w = 0.5 + 0.35 * (TAU * 0.9 * t).sin();
            let raw = (saw(ph[0], dp) - saw((ph[0] + w).fract(), dp)) * 0.8;
            *lp += lp_a(1800.0 + 3500.0 * (-3.0 * t).exp(), dt) * (raw - *lp);
            (*lp, 0.006, 0.26)
        }
        // Hard-sync: a slave saw resets with the master, its pitch sweeps down.
        Instrument::SyncLead => {
            let new = ph[0] + dp;
            if new >= 1.0 {
                ph[1] = 0.0;
            }
            ph[0] = new.fract();
            let ratio = 2.0 + 3.0 * (-1.2 * t).exp();
            ph[1] = (ph[1] + dp * ratio).fract();
            let raw = saw(ph[1], dp * ratio);
            *lp += lp_a(2500.0 + 4000.0 * (-3.0 * t).exp(), dt) * (raw - *lp);
            (*lp, 0.005, 0.24)
        }

        // ------------------------------------------------ Basses
        // Three detuned saws that beat against each other, with a sub.
        Instrument::ReeseBass => {
            let det = [0.994, 1.0, 1.006];
            let mut raw = 0.0;
            for i in 0..3 {
                ph[i] = (ph[i] + dp * det[i]).fract();
                raw += saw(ph[i], dp * det[i]) / 3.0;
            }
            let fc = 500.0 + 350.0 * (TAU * 0.4 * t).sin();
            let al = lp_a(fc, dt);
            *lp += al * (raw - *lp);
            *lp2 += al * (*lp - *lp2);
            (*lp2 * 1.4 + 0.45 * (a * 0.5).sin(), 0.006, 0.38)
        }
        // Saw + square through a low-pass that an LFO opens and closes.
        Instrument::WobbleBass => {
            ph[0] = (ph[0] + dp).fract();
            ph[1] = (ph[1] + dp * 0.5).fract();
            let raw = 0.7 * saw(ph[0], dp) + 0.5 * square(ph[1], dp * 0.5);
            let fc = 180.0 + 1900.0 * (0.5 + 0.5 * (TAU * 3.0 * t).sin());
            let al = lp_a(fc, dt);
            *lp += al * (raw - *lp);
            *lp2 += al * (*lp - *lp2);
            (*lp2 * 1.5, 0.004, 0.4)
        }

        // ------------------------------------------------ Plucked
        // Bright, nasal pluck with a metallic head: fast decay of the upper partials.
        Instrument::Banjo => {
            let s = string(f, t, 12, 0.6, 3.0, 1.4, 0.0006)
                + 0.3 * (a * 2.02).sin() * (-9.0 * t).exp();
            *lp += lp_a(5500.0, dt) * (s - *lp);
            (*lp * 0.7, 0.001, 0.3)
        }
        // Thumb piano: a tine with a "plink" partial and a wooden thump.
        Instrument::Kalimba => {
            let s = partials(f, t, &[(1.0, 1.0, 3.0), (5.4, 0.25, 9.0), (9.1, 0.1, 16.0)]);
            let thump = 0.15 * (a * 0.5).sin() * (-50.0 * t).exp();
            (s + thump, 0.001, 0.38)
        }
        // Plucked string with a pitch bend on the attack.
        Instrument::Koto => {
            let fb = f * (1.0 + 0.012 * (-30.0 * t).exp());
            let s = string(fb, t, 10, 0.9, 1.5, 0.8, 0.0004);
            let click = 0.15 * noise(rng) * (-150.0 * t).exp();
            (s * 0.9 + click, 0.001, 0.32)
        }

        // ------------------------------------------------ Pads
        // Slightly inharmonic sines with a slow shimmer: airy and glassy.
        Instrument::GlassPad => {
            let shim = 0.7 + 0.3 * (TAU * 0.35 * t).sin();
            let s = (a).sin()
                + 0.5 * (a * 2.0).sin()
                + 0.35 * (a * 3.01).sin() * shim
                + 0.2 * (a * 4.02).sin() * (1.0 - shim + 0.3);
            (s * 0.45, 0.6, 0.3)
        }
        // Three detuned saws with a sub square, the filter opens slowly.
        Instrument::AnalogPad => {
            let det = [0.996, 1.0, 1.004];
            let mut raw = 0.0;
            for i in 0..3 {
                ph[i] = (ph[i] + dp * det[i]).fract();
                raw += saw(ph[i], dp * det[i]) / 3.0;
            }
            ph[3] = (ph[3] + dp * 0.5).fract();
            raw += 0.25 * square(ph[3], dp * 0.5);
            let fc = 450.0 + 2600.0 * (1.0 - (-1.0 * t).exp()) + 300.0 * (TAU * 0.25 * t).sin();
            *lp += lp_a(fc, dt) * (raw - *lp);
            (*lp, 0.45, 0.24)
        }

        // ------------------------------------------------ Mallets
        Instrument::Glockenspiel => {
            let s = partials(f, t, &[(1.0, 1.0, 3.0), (2.76, 0.5, 5.0), (5.4, 0.3, 9.0)]);
            (s, 0.001, 0.3)
        }
        Instrument::Xylophone => {
            let s = partials(f, t, &[(1.0, 1.0, 10.0), (3.0, 0.6, 16.0), (6.0, 0.25, 25.0)]);
            let click = 0.2 * noise(rng) * (-200.0 * t).exp();
            (s + click, 0.001, 0.4)
        }
        // Tuned pan: octave and fifth partials, a gentle shimmer.
        Instrument::SteelDrum => {
            let shim = 1.0 + 0.05 * (TAU * 5.0 * t).sin();
            let s = partials(
                f,
                t,
                &[(1.0, 1.0, 2.5), (2.0, 0.6, 3.0), (3.0, 0.3, 4.0), (4.2, 0.15, 6.0)],
            );
            (s * shim, 0.002, 0.34)
        }

        // ------------------------------------------------ Brass & winds
        // Two close saws, bright filter that opens fast, delayed vibrato.
        Instrument::Trumpet => {
            let vib = 1.0 + 0.006 * (TAU * 5.5 * t).sin() * (1.0 - (-2.0 * t).exp());
            ph[0] = (ph[0] + dp * vib).fract();
            ph[1] = (ph[1] + dp * vib * 1.001).fract();
            let raw = (saw(ph[0], dp) + saw(ph[1], dp)) * 0.5;
            let al = lp_a(800.0 + 4200.0 * (1.0 - (-14.0 * t).exp()), dt);
            *lp += al * (raw - *lp);
            *lp2 += al * (*lp - *lp2);
            (*lp2, 0.035, 0.3)
        }
        // Reed: saw + square, breath noise, vibrato.
        Instrument::Saxophone => {
            let vib = 1.0 + 0.005 * (TAU * 5.0 * t).sin() * (1.0 - (-2.5 * t).exp());
            ph[0] = (ph[0] + dp * vib).fract();
            ph[1] = (ph[1] + dp * vib).fract();
            let raw = 0.6 * saw(ph[0], dp) + 0.4 * square(ph[1], dp) + 0.05 * noise(rng);
            *lp += lp_a(1300.0 + 1400.0 * (-5.0 * t).exp(), dt) * (raw - *lp);
            (*lp, 0.04, 0.5)
        }
        // Odd harmonics only: hollow, woody.
        Instrument::Clarinet => {
            let mut s = 0.0;
            for k in [1.0f32, 3.0, 5.0, 7.0, 9.0] {
                if f * k < 12000.0 {
                    s += (a * k).sin() / k.powf(0.9);
                }
            }
            s += 0.04 * noise(rng);
            *lp += lp_a(3200.0, dt) * (s - *lp);
            (*lp * 0.8, 0.03, 0.3)
        }

        _ => (0.0, 0.01, 0.0),
    }
}