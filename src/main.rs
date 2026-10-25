//! OpenDembow 0.9 - a modern, elegant DAW.

mod arrangement;
mod audio_io;
mod automation;
mod chrome;
mod drum_names;
mod drums;
mod drums_urban;
mod edit;
mod effects;
mod engine;
mod extra_voices;
mod favorites;
mod files;
mod instruments;
mod keyboard;
mod latin;
mod loops;
mod mcp;
mod metronome;
mod midi_in;
mod pianoroll;
mod project;
mod project_io;
mod record;
mod theme;
mod velocity;
mod analyzer;
mod meters;

use eframe::egui::{self, Color32, Key, Rect};
use engine::Engine;
use instruments::Instrument;
use project::*;
use std::{collections::HashSet, path::PathBuf};
use theme::*;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 920.0])
            .with_min_inner_size([1000.0, 720.0])
            .with_icon(std::sync::Arc::new(chrome::app_icon())),
        ..Default::default()
    };
    eframe::run_native("OpenDembow", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

/// What the mouse is currently dragging.
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    Clip { t: usize, c: usize, grab: f64 },
    NoteMove { idx: usize, grab_beat: f64, grab_key: i32 },
    NoteResize { idx: usize },
    /// Drag the left edge of a note (its end stays fixed).
    NoteResizeL { idx: usize, end: f64 },
    /// Drag the left / right edge of a clip.
    ClipEdge { t: usize, c: usize, left: bool },
}

/// Tab shown in the bottom panel.
#[derive(Clone, Copy, PartialEq)]
enum DetailTab {
    Roll,
    Mixer,
}

struct App {
    project: Project,
    engine: Engine,
    project_path: Option<PathBuf>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    sel_track: usize,
    sel_clip: Option<(usize, usize)>,
    sel_note: Option<usize>,
    /// Multi-selection of notes in the piano roll (the last one is `sel_note`).
    sel_notes: Vec<usize>,
    // arrangement view
    ppb: f32,
    scroll_x: f64,
    scroll_y: f32,
    arr_rect: Rect,
    lanes_rect: Rect,
    // piano roll view
    pr_ppb: f32,
    pr_scroll: f64,
    pr_top: i32,
    last_len: f64,
    /// Piano roll snap, in beats.
    grid: f64,
    drag: Drag,
    drag_inst: Option<Instrument>,
    octave: i32,
    down: HashSet<u8>,
    mouse_key: Option<u8>,
    kb_key: Option<u8>,
    status: String,
    show_help: bool,
    playing: bool,
    pos_beats: f64,
    meters: Vec<f32>,
    metronome: metronome::Metronome,
    // 0.5
    detail: DetailTab,
    clipboard: Option<edit::Clipboard>,
    /// Pen (draw / edit) mode.
    pen: bool,
    /// The clip as it was when an edge drag began.
    edge_orig: Option<Clip>,
    mcp: Option<std::sync::mpsc::Receiver<mcp::Call>>,
    // 0.7
    /// Recording state (off / count-in / recording).
    rec: record::RecState,
    /// Velocity of the next live note (set by MIDI input, 0.8 for the computer keyboard).
    rec_vel: f32,
    /// Play a 4-beat count-in before recording.
    count_in: bool,
    /// MIDI keyboard / controller input.
    midi: midi_in::MidiIn,
    /// Favourite instruments (by name).
    favs: HashSet<String>,
}

fn piano_key(k: Key) -> Option<u8> {
    Some(match k {
        Key::A => 0,
        Key::W => 1,
        Key::S => 2,
        Key::E => 3,
        Key::D => 4,
        Key::F => 5,
        Key::T => 6,
        Key::G => 7,
        Key::Y => 8,
        Key::H => 9,
        Key::U => 10,
        Key::J => 11,
        Key::K => 12,
        Key::O => 13,
        Key::L => 14,
        _ => return None,
    })
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        theme::apply(&cc.egui_ctx);
        let project = Project::demo(engine::device_sr());
        let engine = Engine::new(project.clone());
        let mcp = if std::env::args().any(|a| a == "--mcp") { Some(mcp::start(cc.egui_ctx.clone())) } else { None };
        let status = engine.error.clone().unwrap_or_else(|| {
            if mcp.is_some() {
                "MCP server active (stdio). An AI assistant can now control this project.".into()
            } else {
                "Welcome to OpenDembow. Press Space to play, R to record, or press ? for help.".into()
            }
        });
        let mut app = Self {
            project,
            engine,
            project_path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            sel_track: 0,
            sel_clip: None,
            sel_note: None,
            sel_notes: Vec::new(),
            ppb: 36.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            arr_rect: Rect::NOTHING,
            lanes_rect: Rect::NOTHING,
            pr_ppb: 90.0,
            pr_scroll: 0.0,
            pr_top: 72,
            last_len: 0.5,
            grid: 0.25,
            drag: Drag::None,
            drag_inst: None,
            octave: 3,
            down: HashSet::new(),
            mouse_key: None,
            kb_key: None,
            status,
            show_help: false,
            playing: false,
            pos_beats: 0.0,
            meters: Vec::new(),
            metronome: metronome::Metronome::new(),
            detail: DetailTab::Roll,
            clipboard: None,
            pen: false,
            edge_orig: None,
            mcp,
            rec: record::RecState::Off,
            rec_vel: 0.8,
            count_in: true,
            midi: midi_in::MidiIn::start(cc.egui_ctx.clone()),
            favs: favorites::load(),
        };
        app.select_clip(0, 0);
        app
    }

    // ---------- project plumbing ----------
    fn push(&self) {
        self.engine.set_project(&self.project);
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn fix_selection(&mut self) {
        let nt = self.project.tracks.len();
        self.sel_track = self.sel_track.min(nt.saturating_sub(1));
        if let Some((t, c)) = self.sel_clip {
            if self.project.tracks.get(t).map_or(true, |tr| c >= tr.clips.len()) {
                self.sel_clip = None;
            }
        }
        self.sel_note = None;
        self.sel_notes.clear();
    }

    fn undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.project, p));
            self.fix_selection();
            self.push();
        }
    }

    fn redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.project, p));
            self.fix_selection();
            self.push();
        }
    }

    fn notes_ref(&self, t: usize, c: usize) -> Option<&Vec<Note>> {
        match &self.project.tracks.get(t)?.clips.get(c)?.kind {
            ClipKind::Midi(n) => Some(n),
            _ => None,
        }
    }

    fn notes_mut(&mut self, t: usize, c: usize) -> Option<&mut Vec<Note>> {
        match &mut self.project.tracks.get_mut(t)?.clips.get_mut(c)?.kind {
            ClipKind::Midi(n) => Some(n),
            _ => None,
        }
    }

    /// The selected clip, if it is a MIDI clip.
    fn sel_midi(&self) -> Option<(usize, usize)> {
        let (t, c) = self.sel_clip?;
        self.notes_ref(t, c).map(|_| (t, c))
    }

    /// Indices of the selected notes (multi-selection, or the single selected note), sorted and deduplicated.
    fn selected_note_indices(&self) -> Vec<usize> {
        let mut sel = self.sel_notes.clone();
        if sel.is_empty() {
            sel.extend(self.sel_note);
        }
        sel.sort_unstable();
        sel.dedup();
        sel
    }

    fn fit_clip(&mut self, t: usize, c: usize) {
        let end = self.notes_ref(t, c).map_or(0.0, |n| n.iter().map(|x| x.start + x.len).fold(0.0, f64::max));
        if let Some(clip) = self.project.tracks.get_mut(t).and_then(|tr| tr.clips.get_mut(c)) {
            clip.len = clip.len.max((end / 4.0).ceil() * 4.0);
        }
    }

    fn select_clip(&mut self, t: usize, c: usize) {
        self.sel_track = t;
        self.sel_clip = Some((t, c));
        self.sel_note = None;
        self.sel_notes.clear();
        self.pr_scroll = 0.0;
        let avg = self.notes_ref(t, c).and_then(|n| {
            if n.is_empty() {
                None
            } else {
                Some(n.iter().map(|x| x.key as i32).sum::<i32>() / n.len() as i32)
            }
        });
        if let Some(a) = avg {
            self.pr_top = a + 9;
        }
    }

    // ---------- commands ----------
    fn add_track(&mut self, inst: Instrument) {
        self.checkpoint();
        let color = self.project.tracks.len() % 8;
        self.project.tracks.push(Track::new(inst.name(), TrackKind::Instrument(inst), color));
        self.sel_track = self.project.tracks.len() - 1;
        self.sel_clip = None;
        self.sel_note = None;
        self.sel_notes.clear();
        self.push();
        self.status = format!("Added track: {}. Double-click its lane to create a clip.", inst.name());
    }

    fn import_audio(&mut self, path: PathBuf) {
        match audio_io::load_wav(&path, self.project.sr) {
            Ok(data) => {
                self.checkpoint();
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Audio".into());
                let color = self.project.tracks.len() % 8;
                let mut tr = Track::new(&name, TrackKind::Audio, color);
                tr.clips.push(Clip { name: name.clone(), start: self.pos_beats.floor(), len: 0.0, kind: ClipKind::Audio(std::sync::Arc::new(data)) });
                self.project.tracks.push(tr);
                self.sel_track = self.project.tracks.len() - 1;
                self.push();
                self.status = format!("Imported {name}");
            }
            Err(e) => self.status = format!("Could not import: {e}"),
        }
    }

    fn import_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).pick_file() {
            self.import_audio(p);
        }
    }

    /// Delete the selected notes (all of the multi-selection), or else the selected clip.
    fn delete_selected(&mut self) {
        let Some((t, c)) = self.sel_clip else { return };
        let sel = self.selected_note_indices();
        self.checkpoint();
        if sel.is_empty() {
            if t < self.project.tracks.len() && c < self.project.tracks[t].clips.len() {
                self.project.tracks[t].clips.remove(c);
            }
            self.sel_clip = None;
        } else {
            if let Some(n) = self.notes_mut(t, c) {
                for &i in sel.iter().rev() {
                    if i < n.len() {
                        n.remove(i);
                    }
                }
            }
            self.sel_note = None;
            self.sel_notes.clear();
        }
        self.push();
    }

    /// Ctrl+D: duplicate the selected notes if there are any, otherwise the selected clip.
    fn duplicate_clip(&mut self) {
        if self.duplicate_notes() {
            return;
        }
        if let Some((t, c)) = self.sel_clip {
            self.checkpoint();
            let mut copy = self.project.tracks[t].clips[c].clone();
            copy.start += copy.len_beats(self.project.bpm, self.project.sr);
            self.project.tracks[t].clips.push(copy);
            let n = self.project.tracks[t].clips.len() - 1;
            self.select_clip(t, n);
            self.push();
        }
    }

    // ---------- live playing ----------
    fn live_inst(&self) -> Option<(usize, Instrument)> {
        match self.project.tracks.get(self.sel_track)?.kind {
            TrackKind::Instrument(i) => Some((self.sel_track, i)),
            _ => None,
        }
    }

    fn note_on(&mut self, key: u8) {
        // velocity comes from the MIDI controller when there is one, 0.8 otherwise
        let vel = std::mem::replace(&mut self.rec_vel, 0.8);
        if let Some((t, i)) = self.live_inst() {
            self.engine.note_on(t, i, key);
            self.down.insert(key);
            self.rec_note_on(t, key, vel);
        }
    }

    fn note_off(&mut self, key: u8) {
        self.down.remove(&key);
        self.engine.note_off(key);
        self.rec_note_off(key);
    }

    fn audition(&self, t: usize, key: u8) {
        if let Some(TrackKind::Instrument(i)) = self.project.tracks.get(t).map(|x| x.kind) {
            self.engine.audition(t, i, key, self.project.sr);
        }
    }

    fn rewind(&mut self) {
        self.engine.set_pos(0.0, self.project.bpm, self.project.sr);
        self.scroll_x = 0.0;
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(p) = dropped.into_iter().find_map(|f| f.path) {
            self.import_audio(p);
        }
        // typing in a text field (search box, number entry) must not trigger shortcuts
        if ctx.wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        // Ctrl+C/X/V can arrive both as Key and as Copy/Cut/Paste events: handle one per frame
        let mut clip_done = false;
        for e in events {
            match &e {
                egui::Event::Copy => {
                    if !clip_done {
                        self.copy_selection();
                        clip_done = true;
                    }
                    continue;
                }
                egui::Event::Cut => {
                    if !clip_done {
                        self.cut_selection();
                        clip_done = true;
                    }
                    continue;
                }
                egui::Event::Paste(_) => {
                    if !clip_done {
                        self.paste_clipboard();
                        clip_done = true;
                    }
                    continue;
                }
                _ => {}
            }
            let egui::Event::Key { key, pressed, repeat, modifiers, .. } = e else { continue };
            if repeat {
                continue;
            }
            if !modifiers.command && !modifiers.alt {
                if let Some(off) = piano_key(key) {
                    let n = (12 * (self.octave + 1) + off as i32).clamp(0, 127) as u8;
                    if pressed {
                        self.note_on(n);
                    } else {
                        self.note_off(n);
                    }
                    continue;
                }
            }
            if !pressed {
                continue;
            }
            match key {
                Key::Space => {
                    if self.is_recording() {
                        self.toggle_record()
                    } else {
                        self.engine.toggle_play()
                    }
                }
                Key::R if !modifiers.command => self.toggle_record(),
                Key::Home => self.rewind(),
                Key::M => self.metronome.toggle(),
                Key::F1 => self.show_help = !self.show_help,
                Key::Delete | Key::Backspace => self.delete_selected(),
                Key::P if !modifiers.command => self.toggle_pen(),
                Key::Escape => {
                    self.pen = false;
                    self.cancel_count_in();
                }
                Key::B if !modifiers.command => self.split_at_playhead(),
                Key::OpenBracket => self.trim_start_to_playhead(),
                Key::CloseBracket => self.trim_end_to_playhead(),
                Key::C if modifiers.command => {
                    if !clip_done {
                        self.copy_selection();
                        clip_done = true;
                    }
                }
                Key::X if modifiers.command => {
                    if !clip_done {
                        self.cut_selection();
                        clip_done = true;
                    }
                }
                Key::V if modifiers.command => {
                    if !clip_done {
                        self.paste_clipboard();
                        clip_done = true;
                    }
                }
                Key::Z if modifiers.command => {
                    if modifiers.shift { self.redo() } else { self.undo() }
                }
                Key::Z => self.octave = (self.octave - 1).max(0),
                Key::X => self.octave = (self.octave + 1).min(6),
                Key::D if modifiers.command => self.duplicate_clip(),
                Key::L if modifiers.command && modifiers.shift => self.loop_to_selection(ctx),
                Key::L if modifiers.command => self.toggle_loop(ctx),
                Key::I if modifiers.command => self.import_dialog(),
                Key::E if modifiers.command => self.export_mixdown(),
                Key::S if modifiers.command => self.save_project(modifiers.shift),
                Key::O if modifiers.command => self.open_project_dialog(),
                _ => {}
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let (pos, playing, meters, _master) = self.engine.status();
        self.playing = playing;
        self.pos_beats = pos as f64 / self.project.sr as f64 * self.project.bpm / 60.0;
        self.loop_tick(ctx);
        self.metronome.update(playing, self.pos_beats, self.project.bpm);
        let active = playing || meters.iter().any(|m| *m > 0.002);
        self.meters = meters;
        if active || !self.down.is_empty() {
            ctx.request_repaint();
        }
        self.midi_poll();
        self.rec_tick(ctx);
        self.mcp_poll();
        self.ui_count_in(ctx);
        if self.mcp.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        self.handle_input(ctx);

        let bar = |y: f32| egui::Frame::none().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(18.0, y));
        egui::TopBottomPanel::top("menu").frame(bar(2.0)).show(ctx, |ui| self.ui_menu(ui));
        egui::TopBottomPanel::top("top").frame(bar(10.0)).show(ctx, |ui| self.ui_top(ui));
        egui::TopBottomPanel::bottom("status")
            .exact_height(28.0)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(18.0, 5.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(&self.status).color(GREY).size(12.0));
            });
        egui::TopBottomPanel::bottom("keyboard")
            .exact_height(150.0)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 14.0, right: 14.0, top: 6.0, bottom: 8.0 }))
            .show(ctx, |ui| self.ui_keyboard(ui));
        egui::SidePanel::left("browser")
            .exact_width(300.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(14.0, 14.0)))
            .show(ctx, |ui| self.ui_browser(ui));
        egui::TopBottomPanel::bottom("detail")
            .exact_height(285.0)
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 0.0, right: 14.0, top: 0.0, bottom: 10.0 }))
            .show(ctx, |ui| self.ui_detail(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin { left: 0.0, right: 14.0, top: 14.0, bottom: 10.0 }))
            .show(ctx, |ui| self.ui_arrangement(ui));
        self.ui_help(ctx);

        // dragging an instrument from the browser: ghost + drop
        if let Some(inst) = self.drag_inst {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            chrome::draw_ghost(ctx, inst);
            if ctx.input(|i| i.pointer.any_released()) {
                if let Some(p) = ctx.pointer_latest_pos() {
                    self.drop_instrument(inst, p);
                }
                self.drag_inst = None;
            }
            ctx.request_repaint();
        }
    }
}