//! Lock-free tap of the master output for spectrum and peak analysis.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::*};
use std::sync::Arc;

const TAP_SIZE: usize = 8192;
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

    #[inline]
    pub fn begin(&self) -> usize {
        self.w.load(Relaxed)
    }

    #[inline]
    pub fn put(&self, w: &mut usize, x: f32) {
        self.buf[*w & MASK].store(x.to_bits(), Relaxed);
        *w = w.wrapping_add(1);
    }

    /// Publishes the write index and block peaks.
    #[inline]
    pub fn end(&self, w: usize, pl: f32, pr: f32) {
        self.w.store(w, Release);
        self.peak_l.fetch_max(pl.to_bits(), Relaxed);
        self.peak_r.fetch_max(pr.to_bits(), Relaxed);
    }

    /// Copies the most recent samples into `out`, oldest first.
    pub fn snapshot(&self, out: &mut [f32]) {
        let n = out.len();
        let w = self.w.load(Acquire);
        for (i, o) in out.iter_mut().enumerate() {
            let idx = w.wrapping_sub(n).wrapping_add(i) & MASK;
            *o = f32::from_bits(self.buf[idx].load(Relaxed));
        }
    }

    /// Returns linear left and right peaks since the previous call.
    pub fn take_peaks(&self) -> (f32, f32) {
        (
            f32::from_bits(self.peak_l.swap(0, Relaxed)),
            f32::from_bits(self.peak_r.swap(0, Relaxed)),
        )
    }
}
