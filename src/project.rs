//! The song: tracks, clips, notes.

use crate::effects::FxSettings;
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

/// Un point individuel d'une courbe d'automatisation
#[derive(Clone, Copy, Debug)]
pub struct AutomationPoint {
    /// Position temporelle en battements (beats) sur la timeline
    pub beat: f64,
    /// Valeur du paramètre (généralement normalisée entre 0.0 et 1.0, ou selon la plage du paramètre)
    pub value: f32,
}

/// Une ligne d'automatisation complète pour un paramètre donné sur une piste
#[derive(Clone, Debug)]
pub struct AutomationLane {
    /// Identifiant du paramètre ciblé (ex: "volume", "pan", "fx_reverb_mix", "cutoff")
    pub parameter_id: String,
    /// Liste des points clés de la courbe, triés par ordre de temps
    pub points: Vec<AutomationPoint>,
}

/// Plage de boucle, en beats, telle qu'elle est enregistrée dans le projet.
/// L'état « vivant » de la boucle reste dans `loops.rs` ; cette copie sert à la
/// sauvegarde / au chargement (voir `App::loop_region` et `App::set_loop_region`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LoopRegion {
    pub start: f64,
    pub end: f64,
    pub on: bool,
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
    /// EQ, compressor, reverb, delay.
    pub fx: FxSettings,
    /// Lignes d'automatisation de la piste
    pub automations: Vec<AutomationLane>,
}

impl Track {
    pub fn new(name: &str, kind: TrackKind, color: usize) -> Self {
        Self {
            name: name.into(),
            kind,
            color,
            volume: 0.8,
            pan: 0.0,
            mute: false,
            solo: false,
            clips: Vec::new(),
            fx: FxSettings::default(),
            automations: Vec::new(),
        }
    }

    /// Récupère la valeur interpolée d'un paramètre automatisé à un battement (beat) précis
    pub fn get_automation_value(&self, parameter_id: &str, current_beat: f64, default_value: f32) -> f32 {
        let Some(lane) = self.automations.iter().find(|l| l.parameter_id == parameter_id) else {
            return default_value;
        };
        if lane.points.is_empty() {
            return default_value;
        }

        if current_beat <= lane.points[0].beat {
            return lane.points[0].value;
        }
        if current_beat >= lane.points.last().unwrap().beat {
            return lane.points.last().unwrap().value;
        }

        for window in lane.points.windows(2) {
            let p1 = window[0];
            let p2 = window[1];
            if current_beat >= p1.beat && current_beat <= p2.beat {
                let t = (current_beat - p1.beat) / (p2.beat - p1.beat);
                return p1.value + (p2.value - p1.value) * t as f32;
            }
        }

        default_value
    }
}

#[derive(Clone)]
pub struct Project {
    pub sr: f32,
    pub bpm: f64,
    pub tracks: Vec<Track>,
    /// Plage de boucle sauvegardée avec le projet (`None` : aucune loop définie).
    pub loop_region: Option<LoopRegion>,
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

        // Dembow beat: kick on every beat, snares on the 3+3+2 accents, hi-hats on the eighths.
        let mut beat = Vec::new();
        for bar in 0..4 {
            let b0 = bar as f64 * 4.0;
            for i in 0..4 {
                beat.push(Note { start: b0 + i as f64, len: 0.2, key: 36, vel: 0.95 });
            }
            for s in [0.75, 1.5, 2.75, 3.5] {
                beat.push(Note { start: b0 + s, len: 0.2, key: 38, vel: 0.85 });
            }
            for i in 0..8 {
                beat.push(Note { start: b0 + i as f64 * 0.5, len: 0.1, key: 42, vel: if i % 2 == 0 { 0.6 } else { 0.4 } });
            }
        }
        let mut drums = Track::new("Reggaeton Kit", TrackKind::Instrument(Instrument::ReggaetonKit), 2);
        drums.clips.push(Clip { name: "Dembow".into(), start: 0.0, len: 16.0, kind: ClipKind::Midi(beat) });

        Project { sr, bpm: 100.0, tracks: vec![piano, b, drums], loop_region: None }
    }
}