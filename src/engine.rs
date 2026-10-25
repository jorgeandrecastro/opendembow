//! Real-time mixer, audio output and offline bounce.

use crate::analyzer::Tap;
use crate::effects::FxProcessor;
use crate::instruments::{Instrument, Voice};
use crate::project::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

fn soft(x: f32) -> f32 {
    let a = x.abs();
    if a < 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((a - 0.8) / 0.2).tanh())
    }
}

pub struct Mixer {
    pub voices: Vec<Voice>,
    pub meters: Vec<f32>,
    pub master: f32,
    l: Vec<f32>,
    r: Vec<f32>,
    peaks: Vec<f32>,
    // Per-track stereo buffers (before volume/pan) and one effect chain per track.
    tl: Vec<Vec<f32>>,
    tr: Vec<Vec<f32>>,
    fx: Vec<FxProcessor>,
    fx_sr: f32,
}

impl Mixer {
    pub fn new() -> Self {
        Mixer {
            voices: Vec::new(),
            meters: Vec::new(),
            master: 0.0,
            l: Vec::new(),
            r: Vec::new(),
            peaks: Vec::new(),
            tl: Vec::new(),
            tr: Vec::new(),
            fx: Vec::new(),
            fx_sr: 0.0,
        }
    }

    /// Renders `n` frames starting at sample `pos` into the internal L/R buffers.
    pub fn render(&mut self, p: &Project, pos: usize, playing: bool, n: usize) {
        let Mixer { voices, meters, master, l, r, peaks, tl, tr, fx, fx_sr } = self;
        l.clear();
        l.resize(n, 0.0);
        r.clear();
        r.resize(n, 0.0);
        let count = p.tracks.len();
        peaks.clear();
        peaks.resize(count, 0.0);
        if meters.len() != count {
            *meters = vec![0.0; count];
        }

        // One FxProcessor per track (all recreated only when the sample rate changes).
        if *fx_sr != p.sr {
            fx.clear();
            *fx_sr = p.sr;
        }
        while fx.len() < count {
            fx.push(FxProcessor::new(p.sr));
        }
        fx.truncate(count);

        // Per-track buffers, zeroed.
        tl.resize_with(count, Vec::new);
        tr.resize_with(count, Vec::new);
        tl.truncate(count);
        tr.truncate(count);
        for b in tl.iter_mut().chain(tr.iter_mut()) {
            b.clear();
            b.resize(n, 0.0);
        }

        let any_solo = p.tracks.iter().any(|t| t.solo);
        let spb = 60.0 / p.bpm * p.sr as f64;
        let dt = 1.0 / p.sr;

        if playing {
            for (ti, t) in p.tracks.iter().enumerate() {
                if t.mute || (any_solo && !t.solo) {
                    continue;
                }
                for c in &t.clips {
                    match &c.kind {
                        ClipKind::Midi(notes) => {
                            if let TrackKind::Instrument(inst) = t.kind {
                                for nn in notes {
                                    if nn.start >= c.len {
                                        continue;
                                    }
                                    let s = ((c.start + nn.start) * spb) as usize;
                                    if s >= pos && s < pos + n {
                                        let hold = ((nn.len * spb) as usize).max(1);
                                        voices.push(Voice::new(ti, inst, nn.key, nn.vel, s - pos, hold));
                                    }
                                }
                            }
                        }
                        ClipKind::Audio(data) => {
                            let start = (c.start * spb) as usize;
                            if pos + n > start && pos < start + data.len() {
                                for i in 0..n {
                                    let a = pos + i;
                                    if a >= start && a - start < data.len() {
                                        let s = data[a - start];
                                        // Dry signal, no gain: effects come first.
                                        tl[ti][i] += s;
                                        tr[ti][i] += s;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        voices.retain_mut(|v| {
            if p.tracks.get(v.track).is_none() {
                return false;
            }
            for i in 0..n {
                if v.delay > 0 {
                    v.delay -= 1;
                    continue;
                }
                if v.hold != usize::MAX {
                    if v.hold == 0 {
                        v.release();
                    } else {
                        v.hold -= 1;
                    }
                }
                match v.next(dt) {
                    Some(x) => {
                        tl[v.track][i] += x;
                        tr[v.track][i] += x;
                    }
                    None => return false,
                }
            }
            true
        });

        // Track effects first, THEN automated volume/pan/mute/solo, THEN sum into the master bus.
        for ti in 0..count {
            let t = &p.tracks[ti];
            fx[ti].process_block(&t.fx, &mut tl[ti], &mut tr[ti]);

            if t.mute || (any_solo && !t.solo) {
                peaks[ti] = 0.0;
                continue;
            }

            let mut pk = 0.0f32;
            for i in 0..n {
                // Position in beats for this exact sample
                let current_beat = (pos + i) as f64 / spb;

                // Automated values (or the slider values when there is no automation)
                let vol = t.get_automation_value("volume", current_beat, t.volume);
                let pan = t.get_automation_value("pan", current_beat, t.pan);

                let ang = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
                let gl = vol * ang.cos() * std::f32::consts::SQRT_2;
                let gr = vol * ang.sin() * std::f32::consts::SQRT_2;

                let a = tl[ti][i] * gl;
                let b = tr[ti][i] * gr;
                l[i] += a;
                r[i] += b;
                pk = pk.max(a.abs()).max(b.abs());
            }
            peaks[ti] = pk;
        }

        for (m, pk) in meters.iter_mut().zip(peaks.iter()) {
            *m = (*m * 0.88).max(*pk);
        }
        let mut mp = 0.0f32;
        for i in 0..n {
            mp = mp.max(l[i].abs()).max(r[i].abs());
        }
        *master = (*master * 0.88).max(mp);
    }
}

/// Bounces the whole project to interleaved stereo.
pub fn render_offline(p: &Project) -> Vec<f32> {
    let mut m = Mixer::new();
    let end = (p.end_beats() * 60.0 / p.bpm * p.sr as f64) as usize + (p.sr * 3.0) as usize;
    let mut out = Vec::with_capacity(end * 2);
    let mut pos = 0;
    while pos < end {
        let n = 512.min(end - pos);
        m.render(p, pos, true, n);
        for i in 0..n {
            out.push(soft(m.l[i]));
            out.push(soft(m.r[i]));
        }
        pos += n;
    }
    out
}

pub struct Shared {
    pub project: Arc<Project>,
    pub pos: usize,
    pub playing: bool,
    pub mixer: Mixer,
}

pub struct Engine {
    shared: Arc<Mutex<Shared>>,
    pub error: Option<String>,
    /// Lock-free tap of the master output (spectrum + peaks), read by the UI.
    pub tap: Arc<Tap>,
    _stream: Option<cpal::Stream>,
}

pub fn device_sr() -> f32 {
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.default_output_config().ok())
        .map(|c| c.sample_rate().0 as f32)
        .unwrap_or(44100.0)
}

impl Engine {
    pub fn new(project: Project) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            project: Arc::new(project),
            pos: 0,
            playing: false,
            mixer: Mixer::new(),
        }));
        let tap = Tap::new();
        match Self::open(shared.clone(), tap.clone()) {
            Ok(s) => Engine { shared, error: None, tap, _stream: Some(s) },
            Err(e) => Engine { shared, error: Some(e), tap, _stream: None },
        }
    }

    fn open(shared: Arc<Mutex<Shared>>, tap: Arc<Tap>) -> Result<cpal::Stream, String> {
        let dev = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "No audio output device found".to_string())?;
        let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
        if cfg.sample_format() != cpal::SampleFormat::F32 {
            return Err("Unsupported audio format (f32 required)".into());
        }
        tap.set_sr(cfg.sample_rate().0 as f32);
        let channels = cfg.channels() as usize;
        let stream = dev
            .build_output_stream(
                &cfg.into(),
                move |data: &mut [f32], _| {
                    let mut g = shared.lock().unwrap();
                    let s = &mut *g;
                    let n = data.len() / channels;
                    let proj = s.project.clone();
                    s.mixer.render(&proj, s.pos, s.playing, n);
                    if s.playing {
                        s.pos += n;
                    }
                    let mut w = tap.begin();
                    let (mut pl, mut pr) = (0.0f32, 0.0f32);
                    for (i, frame) in data.chunks_mut(channels).enumerate() {
                        let (xl, xr) = (s.mixer.l[i], s.mixer.r[i]);
                        // peaks BEFORE the soft limiter, so an over (> 0 dBFS) is visible
                        pl = pl.max(xl.abs());
                        pr = pr.max(xr.abs());
                        let (l, r) = (soft(xl), soft(xr));
                        tap.put(&mut w, (l + r) * 0.5);
                        if channels == 1 {
                            frame[0] = (l + r) * 0.5;
                        } else {
                            frame[0] = l;
                            frame[1] = r;
                            for x in frame.iter_mut().skip(2) {
                                *x = 0.0;
                            }
                        }
                    }
                    tap.end(w, pl, pr);
                },
                |e| eprintln!("audio error: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    pub fn set_project(&self, p: &Project) {
        self.shared.lock().unwrap().project = Arc::new(p.clone());
    }

    pub fn toggle_play(&self) {
        let mut g = self.shared.lock().unwrap();
        g.playing = !g.playing;
        if !g.playing {
            for v in g.mixer.voices.iter_mut().filter(|v| !v.live) {
                v.release();
            }
        }
    }

    pub fn set_pos(&self, beats: f64, bpm: f64, sr: f32) {
        let mut g = self.shared.lock().unwrap();
        g.pos = (beats.max(0.0) * 60.0 / bpm * sr as f64) as usize;
        for v in g.mixer.voices.iter_mut().filter(|v| !v.live) {
            v.release();
        }
    }

    pub fn note_on(&self, track: usize, inst: Instrument, key: u8) {
        self.shared.lock().unwrap().mixer.voices.push(Voice::live(track, inst, key, 0.85));
    }

    pub fn note_off(&self, key: u8) {
        for v in self.shared.lock().unwrap().mixer.voices.iter_mut() {
            if v.live && v.key == key && !v.released() {
                v.release();
            }
        }
    }

    /// Short preview of a note (used when editing in the piano roll).
    pub fn audition(&self, track: usize, inst: Instrument, key: u8, sr: f32) {
        let hold = (0.25 * sr) as usize;
        self.shared.lock().unwrap().mixer.voices.push(Voice::new(track, inst, key, 0.8, 0, hold));
    }

    /// (position in samples, playing, per-track meters, master meter)
    pub fn status(&self) -> (usize, bool, Vec<f32>, f32) {
        let g = self.shared.lock().unwrap();
        (g.pos, g.playing, g.mixer.meters.clone(), g.mixer.master)
    }
}