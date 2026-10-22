//! Metronome: an audible click that follows the transport.
//!
//! It runs on its own small audio stream and never touches the engine. The UI tells it
//! where the playhead is; the audio callback then keeps perfect time by itself
//! (sample-accurate) until the playhead jumps (seek / rewind / tempo change / drift).

use crate::theme::{ACCENT, GREY};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use eframe::egui::{self, pos2, vec2, Color32, RichText, Rounding, Sense, Shape, Stroke};
use std::f32::consts::{PI, TAU};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

/// State shared between the UI thread and the audio callback.
struct Shared {
    enabled: AtomicBool,
    running: AtomicBool,
    accent: AtomicBool,
    bpm: AtomicU64,    // f64 bits
    volume: AtomicU32, // f32 bits
    per_bar: AtomicU32,
    target: AtomicU64, // f64 bits: beat the click clock must jump to
    generation: AtomicU32,
    beat_now: AtomicU64, // f64 bits: where the click clock currently is
}

pub struct Metronome {
    shared: Arc<Shared>,
    _stream: Option<cpal::Stream>,
    pub error: Option<String>,
    pub enabled: bool,
    pub volume: f32,
    pub per_bar: u32,
    pub accent: bool,
    /// Latency compensation in milliseconds (positive = clicks earlier).
    pub offset_ms: f32,
    was_live: bool,
    need_sync: bool,
    bad_frames: u32,
}

fn build_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let sr = config.sample_rate.0 as f32;
    let channels = (config.channels as usize).max(1);

    let mut beat = 0.0f64;
    let mut last_idx: i64 = -1;
    let mut last_gen = 0u32;
    let mut t: u32 = u32::MAX; // samples since the click started (MAX = silent)
    let mut freq = 1000.0f32;
    let mut amp = 0.0f32;

    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                // the UI asked us to jump to a new position
                let gen = shared.generation.load(Relaxed);
                if gen != last_gen {
                    last_gen = gen;
                    beat = f64::from_bits(shared.target.load(Relaxed));
                    last_idx = (beat - 1e-9).floor() as i64;
                }
                let running = shared.running.load(Relaxed) && shared.enabled.load(Relaxed);
                let bpm = f64::from_bits(shared.bpm.load(Relaxed)).clamp(20.0, 400.0);
                let vol = f32::from_bits(shared.volume.load(Relaxed)) * 0.5;
                let per_bar = shared.per_bar.load(Relaxed).max(1) as i64;
                let accent_on = shared.accent.load(Relaxed);
                let inc = bpm / 60.0 / sr as f64;

                for frame in data.chunks_mut(channels) {
                    if running {
                        let idx = beat.floor() as i64;
                        if idx > last_idx {
                            last_idx = idx;
                            let first = idx.rem_euclid(per_bar) == 0 && accent_on;
                            freq = if first { 1700.0 } else { 1150.0 };
                            amp = if first { 1.0 } else { 0.7 };
                            t = 0;
                        }
                        beat += inc;
                    }
                    let mut s = 0.0f32;
                    if t != u32::MAX {
                        let tt = t as f32 / sr;
                        let env = (-tt * 190.0).exp();
                        if env < 0.001 {
                            t = u32::MAX;
                        } else {
                            let a = TAU * freq * tt;
                            s = (a.sin() + 0.35 * (a * 2.0).sin()) * env * amp * vol;
                            t += 1;
                        }
                    }
                    let v = T::from_sample(s);
                    for out in frame.iter_mut() {
                        *out = v;
                    }
                }
                if running {
                    shared.beat_now.store(beat.to_bits(), Relaxed);
                }
            },
            |e| eprintln!("metronome audio error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

fn open(shared: Arc<Shared>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| "No audio output device".to_string())?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let fmt = supported.sample_format();
    let config = supported.config();
    let stream = match fmt {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, shared)?,
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, shared)?,
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, shared)?,
        other => return Err(format!("Unsupported sample format: {other:?}")),
    };
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

impl Metronome {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            enabled: AtomicBool::new(false),
            running: AtomicBool::new(false),
            accent: AtomicBool::new(true),
            bpm: AtomicU64::new(100.0f64.to_bits()),
            volume: AtomicU32::new(0.6f32.to_bits()),
            per_bar: AtomicU32::new(4),
            target: AtomicU64::new(0.0f64.to_bits()),
            generation: AtomicU32::new(0),
            beat_now: AtomicU64::new(0.0f64.to_bits()),
        });
        let (stream, error) = match open(shared.clone()) {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };
        Self {
            shared,
            _stream: stream,
            error,
            enabled: false,
            volume: 0.6,
            per_bar: 4,
            accent: true,
            offset_ms: 0.0,
            was_live: false,
            need_sync: false,
            bad_frames: 0,
        }
    }

    pub fn toggle(&mut self) {
        if self.error.is_none() {
            self.enabled = !self.enabled;
            self.need_sync = true;
        }
    }

    fn resync(&mut self, target: f64) {
        self.shared.target.store(target.to_bits(), Relaxed);
        self.shared.generation.fetch_add(1, Relaxed);
        self.need_sync = false;
        self.bad_frames = 0;
    }

    /// Call once per frame with the transport state.
    pub fn update(&mut self, playing: bool, pos_beats: f64, bpm: f64) {
        let s = self.shared.clone();
        s.enabled.store(self.enabled, Relaxed);
        s.accent.store(self.accent, Relaxed);
        s.bpm.store(bpm.to_bits(), Relaxed);
        s.volume.store(self.volume.to_bits(), Relaxed);
        s.per_bar.store(self.per_bar, Relaxed);

        let live = playing && self.enabled;
        s.running.store(live, Relaxed);
        if !live {
            self.was_live = false;
            self.bad_frames = 0;
            return;
        }

        let target = (pos_beats + self.offset_ms as f64 / 1000.0 * bpm / 60.0).max(0.0);
        if !self.was_live || self.need_sync {
            self.resync(target);
        } else {
            // follow seeks / rewinds / tempo jumps; ignore tiny buffer-size noise (~45 ms)
            let mine = f64::from_bits(s.beat_now.load(Relaxed));
            let tol = 0.045 * bpm / 60.0;
            if (target - mine).abs() > tol {
                self.bad_frames += 1;
                if self.bad_frames >= 3 {
                    self.resync(target);
                }
            } else {
                self.bad_frames = 0;
            }
        }
        self.was_live = true;
    }

    /// Round transport button: click = on/off, right-click = options.
    pub fn ui_button(&mut self, ui: &mut egui::Ui, pos_beats: f64, playing: bool) {
        let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 32.0), Sense::click());
        let p = ui.painter().clone();
        let usable = self.error.is_none();
        let on = self.enabled && usable;
        let fill = if on {
            ACCENT
        } else if resp.hovered() {
            Color32::from_rgb(228, 228, 234)
        } else {
            Color32::from_rgb(238, 238, 243)
        };
        let fg = if on { Color32::WHITE } else { GREY };
        p.rect_filled(rect, Rounding::same(18.0), fill);

        // metronome icon: body + swinging pendulum
        let c = rect.center();
        let body = vec![
            pos2(c.x - 6.5, c.y + 8.0),
            pos2(c.x + 6.5, c.y + 8.0),
            pos2(c.x + 3.0, c.y - 8.0),
            pos2(c.x - 3.0, c.y - 8.0),
        ];
        p.add(Shape::closed_line(body, Stroke::new(1.6_f32, fg)));
        let live = on && playing;
        let ang = if live { (pos_beats.rem_euclid(2.0) as f32 * PI).sin() * 0.55 } else { 0.0 };
        let pivot = pos2(c.x, c.y + 6.0);
        let dir = vec2(ang.sin(), -ang.cos());
        p.line_segment([pivot, pivot + dir * 15.0], Stroke::new(1.6_f32, fg));
        p.circle_filled(pivot + dir * 9.0, 2.2, fg);

        // beat flash (yellow on the first beat of the bar)
        if live {
            let flash = (1.0 - pos_beats.rem_euclid(1.0) as f32 * 5.0).clamp(0.0, 1.0);
            let first = self.accent && (pos_beats.floor() as i64).rem_euclid(self.per_bar.max(1) as i64) == 0;
            let col = if first { (255, 214, 10) } else { (255, 255, 255) };
            p.circle_filled(
                pos2(rect.right() - 7.0, rect.top() + 7.0),
                3.0,
                Color32::from_rgba_unmultiplied(col.0, col.1, col.2, (flash * 255.0) as u8),
            );
        }

        let tip = match &self.error {
            Some(e) => format!("Metronome unavailable: {e}"),
            None => "Metronome (M) - right-click for options".to_string(),
        };
        let resp = resp.on_hover_text(tip);
        if resp.clicked() {
            self.toggle();
        }
        resp.context_menu(|ui| {
            ui.label(RichText::new("Metronome").strong());
            ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume"));
            ui.horizontal(|ui| {
                ui.label("Beats per bar");
                for n in [2u32, 3, 4, 5, 6, 7] {
                    if ui.selectable_label(self.per_bar == n, n.to_string()).clicked() {
                        self.per_bar = n;
                    }
                }
            });
            ui.checkbox(&mut self.accent, "Accent the first beat");
            if ui.add(egui::Slider::new(&mut self.offset_ms, -80.0..=80.0).text("Latency (ms)")).changed() {
                self.need_sync = true;
            }
        });
    }
}