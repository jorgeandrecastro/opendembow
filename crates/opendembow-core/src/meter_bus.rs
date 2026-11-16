//! Hidden bridge for publishing compressor gain reduction from the audio thread to the UI.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

const MAX_TRACKS: usize = 128;
#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU32 = AtomicU32::new(0);
static GR: [AtomicU32; MAX_TRACKS] = [ZERO; MAX_TRACKS];
static GR_LIVE: AtomicBool = AtomicBool::new(false);

/// Publishes gain reduction in dB for one track.
pub fn publish_gr(track: usize, db: f32) {
    if let Some(value) = GR.get(track) {
        value.store(db.max(0.0).to_bits(), Relaxed);
        GR_LIVE.store(true, Relaxed);
    }
}

/// Returns the latest published value if the audio engine has published any.
pub fn live_gr(track: usize) -> Option<f32> {
    if GR_LIVE.load(Relaxed) {
        GR.get(track).map(|value| f32::from_bits(value.load(Relaxed)))
    } else {
        None
    }
}
