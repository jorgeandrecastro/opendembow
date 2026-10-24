//! Clip editing: split, trim, cut / copy / paste, audio tools, and the tab bar of the bottom panel.

use crate::project::*;
use crate::theme::*;
use crate::pianoroll::{size_label, SIZES};
use crate::{App, DetailTab};
use eframe::egui::{self, vec2, Color32, RichText, Rounding};
use std::sync::Arc;

/// What Copy / Cut put in the clipboard.
#[derive(Clone)]
pub enum Clipboard {
    Clip(Clip),
    Notes(Vec<Note>),
}

#[derive(Clone, Copy)]
pub enum AudioOp {
    Normalize,
    Reverse,
}

fn snap(b: f64) -> f64 {
    (b * 4.0).round() / 4.0
}

/// Cut a clip `cut` beats after its own start. Returns (left, right), or None if `cut` is not inside the clip.
pub fn split_data(clip: &Clip, cut: f64, bpm: f64, sr: f32) -> Option<(Clip, Clip)> {
    let total = clip.len_beats(bpm, sr);
    if cut <= 0.001 || cut >= total - 0.001 {
        return None;
    }
    match &clip.kind {
        ClipKind::Midi(notes) => {
            let (mut l, mut r) = (Vec::new(), Vec::new());
            for n in notes {
                let end = n.start + n.len;
                if n.start < cut {
                    let mut a = n.clone();
                    a.len = end.min(cut) - n.start;
                    l.push(a);
                }
                if end > cut {
                    let from = n.start.max(cut);
                    let mut b = n.clone();
                    b.start = from - cut;
                    b.len = end - from;
                    r.push(b);
                }
            }
            let left = Clip { name: clip.name.clone(), start: clip.start, len: cut, kind: ClipKind::Midi(l) };
            let right = Clip { name: clip.name.clone(), start: clip.start + cut, len: clip.len - cut, kind: ClipKind::Midi(r) };
            Some((left, right))
        }
        ClipKind::Audio(d) => {
            let i = (cut * 60.0 / bpm * sr as f64).round() as usize;
            if i == 0 || i >= d.len() {
                return None;
            }
            let left = Clip { name: clip.name.clone(), start: clip.start, len: 0.0, kind: ClipKind::Audio(Arc::new(d[..i].to_vec())) };
            let right = Clip { name: clip.name.clone(), start: clip.start + cut, len: 0.0, kind: ClipKind::Audio(Arc::new(d[i..].to_vec())) };
            Some((left, right))
        }
    }
}

/// New version of `orig` after dragging its left or right edge to the absolute position `beat`.
/// Always computed from the original clip, so dragging back restores everything.
/// MIDI clips can grow on both sides; audio clips can only be trimmed (the sound is not stretched).
pub fn resize_clip(orig: &Clip, left: bool, beat: f64, bpm: f64, sr: f32) -> Option<Clip> {
    let total = orig.len_beats(bpm, sr);
    let end = orig.start + total;
    let min = 0.25;
    if left {
        let b = beat.min(end - min);
        let cut = b - orig.start;
        if cut > 0.0005 {
            split_data(orig, cut, bpm, sr).map(|(_, r)| r)
        } else if cut < -0.0005 {
            match &orig.kind {
                ClipKind::Midi(notes) => {
                    let b = b.max(0.0);
                    let shift = orig.start - b;
                    let moved = notes
                        .iter()
                        .map(|n| {
                            let mut n = n.clone();
                            n.start += shift;
                            n
                        })
                        .collect();
                    Some(Clip { name: orig.name.clone(), start: b, len: total + shift, kind: ClipKind::Midi(moved) })
                }
                ClipKind::Audio(_) => Some(orig.clone()),
            }
        } else {
            Some(orig.clone())
        }
    } else {
        let new_len = (beat - orig.start).max(min);
        if new_len < total - 0.0005 {
            split_data(orig, new_len, bpm, sr).map(|(l, _)| l)
        } else if new_len > total + 0.0005 {
            match &orig.kind {
                ClipKind::Midi(_) => {
                    let mut c = orig.clone();
                    c.len = new_len;
                    Some(c)
                }
                ClipKind::Audio(_) => Some(orig.clone()),
            }
        } else {
            Some(orig.clone())
        }
    }
}

impl App {
    // ---------- undo grouping for continuous edits (knobs, faders, length) ----------

    /// Call once per frame: when the mouse is up, the next edit opens a new undo step.
    pub fn edit_begin_frame(&self, ctx: &egui::Context) {
        if !ctx.input(|i| i.pointer.any_down()) {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("edit_undo_open"), false));
        }
    }

    /// Create an undo step only once per drag.
    pub fn edit_checkpoint(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("edit_undo_open");
        let open: bool = ctx.data_mut(|d| d.get_temp(id)).unwrap_or(false);
        if !open {
            self.checkpoint();
            ctx.data_mut(|d| d.insert_temp(id, true));
        }
    }

    // ---------- split / trim ----------

    pub fn sel_is_audio(&self) -> bool {
        self.sel_clip
            .and_then(|(t, c)| self.project.tracks.get(t)?.clips.get(c))
            .map_or(false, |c| matches!(c.kind, ClipKind::Audio(_)))
    }

    /// Split clip (t, c) at an absolute timeline position. Returns the index of the new right-hand clip.
    pub fn split_clip_at(&mut self, t: usize, c: usize, beat: f64) -> Result<usize, String> {
        let clip = self.project.tracks.get(t).and_then(|tr| tr.clips.get(c)).ok_or("no such clip")?.clone();
        let beat = if matches!(clip.kind, ClipKind::Midi(_)) { snap(beat) } else { beat };
        let (left, right) =
            split_data(&clip, beat - clip.start, self.project.bpm, self.project.sr).ok_or("the playhead is not inside the clip")?;
        self.checkpoint();
        let tr = &mut self.project.tracks[t];
        tr.clips[c] = left;
        tr.clips.push(right);
        let n = tr.clips.len() - 1;
        self.select_clip(t, c);
        self.push();
        Ok(n)
    }

    /// Keep only the right (or left) part of a clip, cut at an absolute position.
    pub fn trim_clip(&mut self, t: usize, c: usize, beat: f64, keep_right: bool) -> Result<(), String> {
        let clip = self.project.tracks.get(t).and_then(|tr| tr.clips.get(c)).ok_or("no such clip")?.clone();
        let beat = if matches!(clip.kind, ClipKind::Midi(_)) { snap(beat) } else { beat };
        let (left, right) =
            split_data(&clip, beat - clip.start, self.project.bpm, self.project.sr).ok_or("the playhead is not inside the clip")?;
        self.checkpoint();
        self.project.tracks[t].clips[c] = if keep_right { right } else { left };
        self.select_clip(t, c);
        self.push();
        Ok(())
    }

    /// Cut note `i` of a MIDI clip at `cut` beats (relative to the clip). False if `cut` is not inside the note.
    pub fn split_note(&mut self, t: usize, c: usize, i: usize, cut: f64) -> bool {
        let Some(n) = self.notes_ref(t, c).and_then(|v| v.get(i)).cloned() else { return false };
        if cut <= n.start + 0.01 || cut >= n.start + n.len - 0.01 {
            return false;
        }
        self.checkpoint();
        if let Some(v) = self.notes_mut(t, c) {
            v[i].len = cut - n.start;
            let mut right = n.clone();
            right.start = cut;
            right.len = n.start + n.len - cut;
            v.push(right);
        }
        self.push();
        true
    }

    /// Multiply the time of every note of the selected MIDI clip (0.5 = compress, 2.0 = stretch).
    pub fn time_scale_clip(&mut self, f: f64) {
        let Some((t, c)) = self.sel_clip else { return };
        if self.notes_ref(t, c).is_none() {
            self.status = "Select a MIDI clip first.".into();
            return;
        }
        self.checkpoint();
        if let Some(v) = self.notes_mut(t, c) {
            for n in v.iter_mut() {
                n.start *= f;
                n.len *= f;
            }
        }
        if let Some(cl) = self.project.tracks.get_mut(t).and_then(|tr| tr.clips.get_mut(c)) {
            cl.len = (cl.len * f).max(0.25);
        }
        self.push();
        self.status = if f < 1.0 { "Clip compressed.".into() } else { "Clip stretched.".into() };
    }

    pub fn split_at_playhead(&mut self) {
        // a selected note under the playhead is cut first
        if let (Some((t, c)), Some(i)) = (self.sel_clip, self.sel_note) {
            let cut = self.project.tracks.get(t).and_then(|tr| tr.clips.get(c)).map(|cl| self.pos_beats - cl.start);
            if let Some(cut) = cut {
                if self.split_note(t, c, i, cut) {
                    self.status = "Note cut at the playhead.".into();
                    return;
                }
            }
        }
        self.status = match self.sel_clip {
            None => "Select a clip first.".into(),
            Some((t, c)) => match self.split_clip_at(t, c, self.pos_beats) {
                Ok(_) => "Clip split at the playhead.".into(),
                Err(e) => format!("Cannot split: {e}"),
            },
        };
    }

    pub fn trim_start_to_playhead(&mut self) {
        self.status = match self.sel_clip {
            None => "Select a clip first.".into(),
            Some((t, c)) => match self.trim_clip(t, c, self.pos_beats, true) {
                Ok(()) => "Clip start trimmed to the playhead.".into(),
                Err(e) => format!("Cannot trim: {e}"),
            },
        };
    }

    pub fn trim_end_to_playhead(&mut self) {
        self.status = match self.sel_clip {
            None => "Select a clip first.".into(),
            Some((t, c)) => match self.trim_clip(t, c, self.pos_beats, false) {
                Ok(()) => "Clip end trimmed to the playhead.".into(),
                Err(e) => format!("Cannot trim: {e}"),
            },
        };
    }

    // ---------- clipboard ----------

    /// Copy the selected note, or else the selected clip.
    pub fn copy_selection(&mut self) -> bool {
        let Some((t, c)) = self.sel_clip else {
            self.status = "Nothing selected to copy.".into();
            return false;
        };
        if let Some(i) = self.sel_note {
            if let Some(n) = self.notes_ref(t, c).and_then(|v| v.get(i)).cloned() {
                self.clipboard = Some(Clipboard::Notes(vec![n]));
                self.status = "Note copied.".into();
                return true;
            }
        }
        match self.project.tracks.get(t).and_then(|tr| tr.clips.get(c)).cloned() {
            Some(clip) => {
                self.clipboard = Some(Clipboard::Clip(clip));
                self.status = "Clip copied. Move the playhead and press Ctrl+V.".into();
                true
            }
            None => false,
        }
    }

    pub fn cut_selection(&mut self) {
        if self.copy_selection() {
            self.delete_selected();
            self.status = "Cut.".into();
        }
    }

    pub fn paste_clipboard(&mut self) {
        match self.clipboard.clone() {
            None => self.status = "Clipboard is empty.".into(),
            Some(Clipboard::Notes(ns)) => {
                let Some((t, c)) = self.sel_midi() else {
                    self.status = "Select a MIDI clip to paste notes into.".into();
                    return;
                };
                let start = self.project.tracks[t].clips[c].start;
                let rel = snap(self.pos_beats - start).max(0.0);
                let base = ns.iter().map(|n| n.start).fold(f64::MAX, f64::min);
                self.checkpoint();
                if let Some(v) = self.notes_mut(t, c) {
                    for n in &ns {
                        let mut n = n.clone();
                        n.start = rel + (n.start - base);
                        v.push(n);
                    }
                }
                self.sel_note = self.notes_ref(t, c).map(|v| v.len() - 1);
                self.fit_clip(t, c);
                self.push();
                self.status = "Note pasted.".into();
            }
            Some(Clipboard::Clip(clip)) => {
                let want_midi = matches!(clip.kind, ClipKind::Midi(_));
                let fits = |tr: &Track| matches!(tr.kind, TrackKind::Instrument(_)) == want_midi;
                let t = if self.project.tracks.get(self.sel_track).map_or(false, fits) {
                    Some(self.sel_track)
                } else {
                    self.project.tracks.iter().position(fits)
                };
                let Some(t) = t else {
                    self.status = "No compatible track to paste into.".into();
                    return;
                };
                self.checkpoint();
                let mut copy = clip;
                copy.start = snap(self.pos_beats).max(0.0);
                self.project.tracks[t].clips.push(copy);
                let n = self.project.tracks[t].clips.len() - 1;
                self.select_clip(t, n);
                self.push();
                self.status = "Clip pasted at the playhead.".into();
            }
        }
    }

    // ---------- audio tools ----------

    pub fn audio_process(&mut self, op: AudioOp) {
        let Some((t, c)) = self.sel_clip else { return };
        let data = match self.project.tracks.get(t).and_then(|tr| tr.clips.get(c)).map(|cl| &cl.kind) {
            Some(ClipKind::Audio(d)) => d.clone(),
            _ => {
                self.status = "Select an audio clip first.".into();
                return;
            }
        };
        let mut v: Vec<f32> = (*data).clone();
        match op {
            AudioOp::Reverse => v.reverse(),
            AudioOp::Normalize => {
                let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                if peak > 1e-6 {
                    let g = 0.97 / peak;
                    v.iter_mut().for_each(|x| *x *= g);
                }
            }
        }
        self.checkpoint();
        self.project.tracks[t].clips[c].kind = ClipKind::Audio(Arc::new(v));
        self.push();
        self.status = match op {
            AudioOp::Reverse => "Audio reversed.".into(),
            AudioOp::Normalize => "Audio normalized.".into(),
        };
    }

    pub fn toggle_pen(&mut self) {
        self.pen = !self.pen;
        self.status = if self.pen {
            "Pen on: click or drag in the piano roll to draw notes. Pick Grid / Size, or just drag. P or Esc to leave.".into()
        } else {
            "Pen off.".into()
        };
    }

    // ---------- bottom panel: tabs + pen options ----------

    pub fn ui_detail(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.edit_begin_frame(&ctx);
        let soft = Color32::from_rgb(240, 240, 245);
        let has_clip = self.sel_clip.is_some();
        let can_paste = self.clipboard.is_some();
        let audio = self.sel_is_audio();
        let midi_len = self.sel_clip.and_then(|(t, c)| {
            let clip = self.project.tracks.get(t)?.clips.get(c)?;
            if let ClipKind::Midi(_) = &clip.kind {
                Some(clip.len)
            } else {
                None
            }
        });
        let is_midi = midi_len.is_some();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.add_space(14.0);
            let chip = |ui: &mut egui::Ui, label: &str, sel: bool| -> bool {
                let b = egui::Button::new(RichText::new(label).size(12.5).color(if sel { Color32::WHITE } else { INK }))
                    .fill(if sel { ACCENT } else { soft })
                    .stroke(egui::Stroke::NONE)
                    .rounding(Rounding::same(12.0))
                    .min_size(vec2(0.0, 24.0));
                ui.add(b).clicked()
            };
            if chip(ui, "Piano roll", self.detail == DetailTab::Roll) {
                self.detail = DetailTab::Roll;
            }
            if chip(ui, "Mixer & Effects", self.detail == DetailTab::Mixer) {
                self.detail = DetailTab::Mixer;
            }
            ui.add_space(6.0);
            if pen_button(ui, self.pen) {
                self.toggle_pen();
            }

            if self.pen {
                // options only appear while the pen is active
                ui.add_space(6.0);
                ui.label(RichText::new("Grid").size(11.5).color(GREY));
                egui::ComboBox::new("pen_grid", "").width(62.0).selected_text(size_label(self.grid)).show_ui(ui, |ui| {
                    for (lab, v) in SIZES {
                        ui.selectable_value(&mut self.grid, v, lab);
                    }
                });
                ui.label(RichText::new("Size").size(11.5).color(GREY));
                let before = self.last_len;
                egui::ComboBox::new("pen_size", "").width(62.0).selected_text(size_label(self.last_len)).show_ui(ui, |ui| {
                    for (lab, v) in SIZES {
                        ui.selectable_value(&mut self.last_len, v, lab);
                    }
                });
                if (self.last_len - before).abs() > 1e-9 {
                    // the chosen size also applies to the selected note
                    if let (Some((t, c)), Some(i)) = (self.sel_clip, self.sel_note) {
                        let v = self.last_len;
                        self.checkpoint();
                        if let Some(n) = self.notes_mut(t, c).and_then(|x| x.get_mut(i)) {
                            n.len = v;
                        }
                        self.fit_clip(t, c);
                        self.push();
                    }
                }
                ui.menu_button("Clip", |ui| {
                    if menu_item(ui, "Split at playhead", "B", has_clip) {
                        self.split_at_playhead();
                    }
                    if menu_item(ui, "Trim start to playhead", "[", has_clip) {
                        self.trim_start_to_playhead();
                    }
                    if menu_item(ui, "Trim end to playhead", "]", has_clip) {
                        self.trim_end_to_playhead();
                    }
                    ui.separator();
                    if menu_item(ui, "Compress ½", "", is_midi) {
                        self.time_scale_clip(0.5);
                    }
                    if menu_item(ui, "Stretch ×2", "", is_midi) {
                        self.time_scale_clip(2.0);
                    }
                    ui.separator();
                    if menu_item(ui, "Cut", "Ctrl+X", has_clip) {
                        self.cut_selection();
                    }
                    if menu_item(ui, "Copy", "Ctrl+C", has_clip) {
                        self.copy_selection();
                    }
                    if menu_item(ui, "Paste", "Ctrl+V", can_paste) {
                        self.paste_clipboard();
                    }
                    if menu_item(ui, "Delete", "Del", has_clip) {
                        self.delete_selected();
                    }
                    ui.separator();
                    if menu_item(ui, "Normalize audio", "", audio) {
                        self.audio_process(AudioOp::Normalize);
                    }
                    if menu_item(ui, "Reverse audio", "", audio) {
                        self.audio_process(AudioOp::Reverse);
                    }
                });
            } else {
                ui.label(RichText::new("Click the pen to edit").size(11.5).color(GREY));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.pen {
                    if let (Some((t, c)), Some(mut len)) = (self.sel_clip, midi_len) {
                        let r = ui.add(egui::DragValue::new(&mut len).range(0.25..=1024.0).speed(0.25).suffix(" beats"));
                        ui.label(RichText::new("Clip length").size(11.5).color(GREY));
                        if r.changed() {
                            self.edit_checkpoint(&ctx);
                            self.project.tracks[t].clips[c].len = len;
                            self.push();
                        }
                    }
                }
            });
        });
        ui.add_space(4.0);
        match self.detail {
            DetailTab::Roll => self.ui_roll(ui),
            DetailTab::Mixer => self.ui_fx(ui),
        }
    }
}

/// Menu entry with an optional shortcut hint; closes the menu when clicked.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    let mut b = egui::Button::new(label);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
    }
    let clicked = ui.add_enabled(enabled, b).clicked();
    if clicked {
        ui.close_menu();
    }
    clicked
}

/// Pen icon drawn by hand (no font glyph needed). Returns true when clicked.
pub fn pen_button(ui: &mut egui::Ui, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 24.0), egui::Sense::click());
    let resp = resp.on_hover_text("Pen: draw and edit (P). Click again or press Esc to leave.");
    let bg = if active {
        ACCENT
    } else if resp.hovered() {
        Color32::from_rgb(228, 228, 234)
    } else {
        Color32::from_rgb(240, 240, 245)
    };
    let fg = if active { Color32::WHITE } else { INK };
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 12.0, bg);
    let c = rect.center();
    let d = vec2(1.0, -1.0).normalized();
    let n = vec2(1.0, 1.0).normalized();
    let tip = c - d * 6.5;
    let body = vec![tip, tip + d * 4.0 + n * 2.6, tip + d * 13.0 + n * 2.6, tip + d * 13.0 - n * 2.6, tip + d * 4.0 - n * 2.6];
    p.add(egui::Shape::closed_line(body, stroke(1.6, fg)));
    p.line_segment([tip + d * 4.0 + n * 2.6, tip + d * 4.0 - n * 2.6], stroke(1.2, fg));
    resp.clicked()
}