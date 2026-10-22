//! Recording with count-in: notes played live become a new MIDI clip.
//! The count-in is shown full screen as a big 3 - 2 - 1 with a click on every beat.

use crate::instruments::Instrument;
use crate::midi_in::MidiMsg;
use crate::project::*;
use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId};
use std::time::Instant;

/// Number of beats (and of numbers shown on screen) in the count-in.
pub const COUNT_IN_BEATS: u32 = 3;

pub enum RecState {
    Off,
    CountIn { t0: Instant, next: u32, from: f64 },
    Recording { from: f64, track: usize, notes: Vec<Note>, held: Vec<(u8, f64, f32)> },
}

impl App {
    pub fn is_recording(&self) -> bool {
        !matches!(self.rec, RecState::Off)
    }

    pub fn toggle_record(&mut self) {
        match self.rec {
            RecState::Off => self.start_record(),
            RecState::CountIn { .. } => self.cancel_count_in(),
            RecState::Recording { .. } => self.stop_record(),
        }
    }

    pub fn cancel_count_in(&mut self) {
        if matches!(self.rec, RecState::CountIn { .. }) {
            self.rec = RecState::Off;
            self.status = "Recording cancelled.".into();
        }
    }

    fn start_record(&mut self) {
        if self.live_inst().is_none() {
            self.status = "Select an instrument track to record on.".into();
            return;
        }
        if self.playing {
            self.engine.toggle_play();
        }
        let from = self.pos_beats.max(0.0);
        if self.count_in {
            self.rec = RecState::CountIn { t0: Instant::now(), next: 0, from };
        } else {
            self.begin_take(from);
        }
    }

    fn begin_take(&mut self, from: f64) {
        self.engine.set_pos(from, self.project.bpm, self.project.sr);
        self.engine.toggle_play();
        self.rec = RecState::Recording { from, track: self.sel_track, notes: Vec::new(), held: Vec::new() };
        self.status = "Recording... play your notes. Press R or Space to stop.".into();
    }

    /// Count-in click: the cowbell (first beat) / rim of the 808 Kit, played through the selected track.
    fn count_click(&self, beat: u32) {
        if let Some(kit) = Instrument::ALL.iter().copied().find(|i| i.name() == "808 Kit") {
            self.engine.audition(self.sel_track, kit, if beat == 0 { 56 } else { 37 }, self.project.sr);
        }
    }

    /// Called every frame: fires the clicks and starts the take when the count-in is over.
    pub fn rec_tick(&mut self, ctx: &egui::Context) {
        let bpm = self.project.bpm;
        let (clicks, start) = match &mut self.rec {
            RecState::CountIn { t0, next, from } => {
                let el = t0.elapsed().as_secs_f64() * bpm / 60.0;
                let mut clicks = Vec::new();
                while *next < COUNT_IN_BEATS && el >= *next as f64 {
                    clicks.push(*next);
                    *next += 1;
                }
                (clicks, if el >= COUNT_IN_BEATS as f64 { Some(*from) } else { None })
            }
            _ => return,
        };
        for k in clicks {
            self.count_click(k);
            self.status = format!("Count-in... {}", COUNT_IN_BEATS - k);
        }
        if let Some(f) = start {
            self.begin_take(f);
        }
        ctx.request_repaint();
    }

    /// Full-screen 3 - 2 - 1. Click anywhere (or press Esc) to cancel.
    pub fn ui_count_in(&mut self, ctx: &egui::Context) {
        let RecState::CountIn { t0, .. } = &self.rec else { return };
        let el = t0.elapsed().as_secs_f64() * self.project.bpm / 60.0;
        let k = (el.floor() as u32).min(COUNT_IN_BEATS - 1);
        let frac = (el - k as f64).clamp(0.0, 1.0) as f32;
        let number = COUNT_IN_BEATS - k;
        let screen = ctx.screen_rect();
        let mut cancel = false;

        egui::Area::new(egui::Id::new("count_in_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                let p = ui.painter();
                p.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 238));
                let c = rect.center();

                // the number pops in big and shrinks / fades during its beat
                let size = 340.0 - 90.0 * frac;
                let col = ACCENT.gamma_multiply(1.0 - 0.55 * frac);
                p.text(pos2(c.x, c.y - 40.0), Align2::CENTER_CENTER, number.to_string(), FontId::proportional(size), col);

                // one dot per beat
                let gap = 30.0;
                let x0 = c.x - gap * (COUNT_IN_BEATS as f32 - 1.0) / 2.0;
                for i in 0..COUNT_IN_BEATS {
                    let on = i <= k;
                    p.circle_filled(pos2(x0 + gap * i as f32, c.y + 190.0), 7.0, if on { ACCENT } else { ACCENT.gamma_multiply(0.2) });
                }
                p.text(
                    pos2(c.x, c.y + 235.0),
                    Align2::CENTER_CENTER,
                    "Get ready - recording starts after the count-in. Click or press Esc to cancel.",
                    FontId::proportional(15.0),
                    GREY,
                );
                if resp.clicked() {
                    cancel = true;
                }
            });
        if cancel {
            self.cancel_count_in();
        }
        let _ = vec2(0.0, 0.0);
    }

    pub fn rec_note_on(&mut self, track: usize, key: u8, vel: f32) {
        if let RecState::Recording { track: rt, held, .. } = &mut self.rec {
            if *rt == track && !held.iter().any(|h| h.0 == key) {
                held.push((key, self.pos_beats, vel));
            }
        }
    }

    pub fn rec_note_off(&mut self, key: u8) {
        if let RecState::Recording { notes, held, .. } = &mut self.rec {
            if let Some(i) = held.iter().position(|h| h.0 == key) {
                let (key, s, vel) = held.remove(i);
                notes.push(Note { start: s, len: (self.pos_beats - s).max(0.0625), key, vel });
            }
        }
    }

    pub fn stop_record(&mut self) {
        let RecState::Recording { from, track, mut notes, held } = std::mem::replace(&mut self.rec, RecState::Off) else { return };
        let pos = self.pos_beats;
        for (key, s, vel) in held {
            notes.push(Note { start: s, len: (pos - s).max(0.0625), key, vel });
        }
        if self.playing {
            self.engine.toggle_play();
        }
        if notes.is_empty() || track >= self.project.tracks.len() {
            self.status = "Nothing recorded.".into();
            return;
        }
        let origin = (from / 4.0).floor() * 4.0; // the clip starts on a bar line
        for n in notes.iter_mut() {
            n.start = (n.start - origin).max(0.0);
        }
        let end = notes.iter().map(|n| n.start + n.len).fold(0.0, f64::max);
        let len = ((end / 4.0).ceil() * 4.0).max(4.0);
        let count = notes.len();
        self.checkpoint();
        self.project.tracks[track].clips.push(Clip { name: "Recording".into(), start: origin, len, kind: ClipKind::Midi(notes) });
        let c = self.project.tracks[track].clips.len() - 1;
        self.select_clip(track, c);
        self.push();
        self.status = format!("Recorded {count} notes. Ctrl+Z to undo.");
    }

    /// Handles queued MIDI messages (called every frame).
    pub fn midi_poll(&mut self) {
        for m in self.midi.drain() {
            match m {
                MidiMsg::On(k, v) => {
                    self.rec_vel = v.max(0.05);
                    self.note_on(k);
                }
                MidiMsg::Off(k) => self.note_off(k),
            }
        }
    }
}