//! The song: tracks, clips, notes.

use crate::instruments::Instrument;
use std::sync::Arc;

#[derive(Clone)]
pub struct Note {
    /// Start in beats, relative to the clip.
    pub start: f64,
    pub len: f64,
    pub key: u8,
    pub vel: f32,
}

#[derive(Clone)]
pub enum ClipKind {
    Midi(Vec<Note>),
    Audio(Arc<Vec<f32>>),
}

#[derive(Clone)]
pub struct Clip {
    pub name: String,
    /// Start in beats on the timeline.
    pub start: f64,
    /// Length in beats (MIDI clips only; audio length is derived from the data).
    pub len: f64,
    pub kind: ClipKind,
}

impl Clip {
    pub fn len_beats(&self, bpm: f64, sr: f32) -> f64 {
        match &self.kind {
            ClipKind::Midi(_) => self.len,
            ClipKind::Audio(d) => d.len() as f64 / sr as f64 * bpm / 60.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TrackKind {
    Instrument(Instrument),
    Audio,
}

#[derive(Clone)]
pub struct Track {
    pub name: String,
    pub kind: TrackKind,
    pub color: usize,
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(name: &str, kind: TrackKind, color: usize) -> Self {
        Self { name: name.into(), kind, color, volume: 0.8, pan: 0.0, mute: false, solo: false, clips: Vec::new() }
    }
}

#[derive(Clone)]
pub struct Project {
    pub sr: f32,
    pub bpm: f64,
    pub tracks: Vec<Track>,
}

impl Project {
    pub fn end_beats(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.start + c.len_beats(self.bpm, self.sr))
            .fold(0.0, f64::max)
    }

    /// A small starter song so that pressing play makes sound immediately.
    pub fn demo(sr: f32) -> Self {
        let prog = [([57u8, 60, 64], 33u8), ([53, 57, 60], 29), ([60, 64, 67], 36), ([55, 59, 62], 31)];
        let (mut keys, mut bass) = (Vec::new(), Vec::new());
        for (i, (chord, root)) in prog.iter().enumerate() {
            let s = i as f64 * 4.0;
            for &k in chord {
                keys.push(Note { start: s, len: 3.8, key: k, vel: 0.7 });
            }
            bass.push(Note { start: s, len: 1.8, key: *root, vel: 0.9 });
            bass.push(Note { start: s + 2.0, len: 1.8, key: *root, vel: 0.8 });
        }
        let mut piano = Track::new("Grand Piano", TrackKind::Instrument(Instrument::Piano), 0);
        piano.clips.push(Clip { name: "Chords".into(), start: 0.0, len: 16.0, kind: ClipKind::Midi(keys) });
        let mut b = Track::new("Synth Bass", TrackKind::Instrument(Instrument::SynthBass), 1);
        b.clips.push(Clip { name: "Bass".into(), start: 0.0, len: 16.0, kind: ClipKind::Midi(bass) });
        Project { sr, bpm: 100.0, tracks: vec![piano, b] }
    }
}
