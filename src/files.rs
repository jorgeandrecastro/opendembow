//! File commands: new / open / save project, export mixdown, stems and MIDI.
use crate::{audio_io, engine, project_io, App};

impl App {
    pub fn new_project(&mut self) {
        self.checkpoint();
        self.project.tracks.clear();
        self.sel_track = 0;
        self.sel_clip = None;
        self.sel_note = None;
        self.project_path = None;
        self.push();
        self.engine.set_pos(0.0, self.project.bpm, self.project.sr);
        self.status = "New project. Drag an instrument onto the timeline to begin.".into();
    }

    pub fn delete_track(&mut self, i: usize) {
        if i < self.project.tracks.len() {
            self.checkpoint();
            self.project.tracks.remove(i);
            self.fix_selection();
            self.push();
            self.status = "Track deleted (Ctrl+Z to undo)".into();
        }
    }

    pub fn save_project(&mut self, save_as: bool) {
        let path = match (&self.project_path, save_as) {
            (Some(p), false) => Some(p.clone()),
            _ => rfd::FileDialog::new()
                .add_filter("OpenDembow project", &["dembow"])
                .set_file_name("Untitled.dembow")
                .save_file()
                .map(|p| p.with_extension("dembow")),
        };
        if let Some(p) = path {
            match project_io::save_project(&self.project, &p) {
                Ok(()) => {
                    self.status = format!("Saved {}", p.display());
                    self.project_path = Some(p);
                }
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }
    }

    pub fn open_project_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("OpenDembow project", &["dembow"]).pick_file() else { return };
        match project_io::load_project(&p, self.project.sr) {
            Ok(proj) => {
                self.project = proj;
                self.undo.clear();
                self.redo.clear();
                self.sel_track = 0;
                self.sel_clip = None;
                self.sel_note = None;
                self.push();
                self.engine.set_pos(0.0, self.project.bpm, self.project.sr);
                self.status = format!("Opened {}", p.display());
                self.project_path = Some(p);
            }
            Err(e) => self.status = format!("Could not open project: {e}"),
        }
    }

    pub fn export_mixdown(&mut self) {
        if self.project.tracks.is_empty() {
            self.status = "Nothing to export yet".into();
            return;
        }
        if let Some(p) = rfd::FileDialog::new().add_filter("WAV audio", &["wav"]).set_file_name("OpenDembow.wav").save_file() {
            let data = engine::render_offline(&self.project);
            self.status = match audio_io::save_wav_stereo(&p, &data, self.project.sr as u32) {
                Ok(()) => format!("Exported mixdown to {}", p.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }

    pub fn export_stems(&mut self) {
        if self.project.tracks.is_empty() {
            self.status = "Nothing to export yet".into();
            return;
        }
        let Some(dir) = rfd::FileDialog::new().pick_folder() else { return };
        let n = self.project.tracks.len();
        for i in 0..n {
            let mut p = self.project.clone();
            for (j, t) in p.tracks.iter_mut().enumerate() {
                t.mute = j != i;
                t.solo = false;
            }
            let data = engine::render_offline(&p);
            let name: String = self.project.tracks[i]
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' })
                .collect();
            let path = dir.join(format!("{:02} - {}.wav", i + 1, name));
            if let Err(e) = audio_io::save_wav_stereo(&path, &data, p.sr as u32) {
                self.status = format!("Stem export failed: {e}");
                return;
            }
        }
        self.status = format!("Exported {n} stems to {}", dir.display());
    }

    pub fn export_midi_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("MIDI file", &["mid"]).set_file_name("OpenDembow.mid").save_file() {
            self.status = match project_io::export_midi(&self.project, &p) {
                Ok(()) => format!("Exported MIDI to {}", p.display()),
                Err(e) => format!("MIDI export failed: {e}"),
            };
        }
    }
}
