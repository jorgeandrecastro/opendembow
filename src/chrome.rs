//! Application chrome: icon, menu bar, transport bar, instrument browser, help window.

use crate::instruments::Instrument;
use crate::theme::*;
use crate::App;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, RichText, Rounding, Sense};

/// Procedural app icon: a blue rounded square with a tiny piano keyboard.
pub fn app_icon() -> egui::IconData {
    let n = 256usize;
    let mut rgba = vec![0u8; n * n * 4];
    let inside = |x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32| -> bool {
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    for py in 0..n {
        for px in 0..n {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            if !inside(x, y, 8.0, 8.0, 248.0, 248.0, 56.0) {
                continue;
            }
            let t = y / n as f32;
            let mut c = [10.0 - 10.0 * t, 132.0 - 42.0 * t, 255.0 - 25.0 * t];
            for k in 0..4 {
                let x0 = 44.0 + k as f32 * 43.0;
                if inside(x, y, x0 + 2.0, 96.0, x0 + 41.0, 206.0, 8.0) {
                    c = [255.0, 255.0, 255.0];
                }
            }
            for k in 0..3 {
                let cx = 44.0 + (k as f32 + 1.0) * 43.0;
                if inside(x, y, cx - 13.0, 96.0, cx + 13.0, 160.0, 5.0) {
                    c = [28.0, 28.0, 30.0];
                }
            }
            let i = (py * n + px) * 4;
            rgba[i] = c[0] as u8;
            rgba[i + 1] = c[1] as u8;
            rgba[i + 2] = c[2] as u8;
            rgba[i + 3] = 255;
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}

impl App {
    pub fn ui_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    self.new_project();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Open Project…").shortcut_text("Ctrl+O")).clicked() {
                    self.open_project_dialog();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Save Project").shortcut_text("Ctrl+S")).clicked() {
                    self.save_project(false);
                    ui.close_menu();
                }
                if ui.button("Save Project As…").clicked() {
                    self.save_project(true);
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Import Audio…").shortcut_text("Ctrl+I")).clicked() {
                    self.import_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Export Mixdown (WAV)…").shortcut_text("Ctrl+E")).clicked() {
                    self.export_mixdown();
                    ui.close_menu();
                }
                if ui.button("Export Stems (one WAV per track)…").clicked() {
                    self.export_stems();
                    ui.close_menu();
                }
                if ui.button("Export MIDI…").clicked() {
                    self.export_midi_dialog();
                    ui.close_menu();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo").shortcut_text("Ctrl+Z")).clicked() {
                    self.undo();
                    ui.close_menu();
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo").shortcut_text("Ctrl+Shift+Z")).clicked() {
                    self.redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add_enabled(self.sel_clip.is_some(), egui::Button::new("Duplicate Clip").shortcut_text("Ctrl+D")).clicked() {
                    self.duplicate_clip();
                    ui.close_menu();
                }
                if ui.add_enabled(self.sel_clip.is_some(), egui::Button::new("Delete Selection").shortcut_text("Del")).clicked() {
                    self.delete_selected();
                    ui.close_menu();
                }
            });
            ui.menu_button("Track", |ui| {
                ui.menu_button("Add Instrument", |ui| {
                    for inst in Instrument::ALL {
                        if ui.button(inst.name()).clicked() {
                            self.add_track(inst);
                            ui.close_menu();
                        }
                    }
                });
                if ui.button("Add Audio Track (WAV)…").clicked() {
                    self.import_dialog();
                    ui.close_menu();
                }
                ui.separator();
                let has = !self.project.tracks.is_empty();
                if ui.add_enabled(has, egui::Button::new("Delete Selected Track")).clicked() {
                    self.delete_track(self.sel_track);
                    ui.close_menu();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.add(egui::Button::new("Help & Shortcuts").shortcut_text("F1")).clicked() {
                    self.show_help = true;
                    ui.close_menu();
                }
                if ui.button("About OpenDembow").clicked() {
                    self.show_help = true;
                    ui.close_menu();
                }
            });
        });
    }

    pub fn ui_top(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("OpenDembow").size(18.0).strong());
            ui.label(RichText::new("0.3").color(GREY));
            ui.add_space(20.0);
            let rw = egui::Button::new(RichText::new("⏮").size(16.0)).rounding(Rounding::same(18.0)).min_size(vec2(40.0, 32.0));
            if ui.add(rw).clicked() {
                self.rewind();
            }
            let icon = if self.playing { "⏸" } else { "⏵" };
            let play = egui::Button::new(RichText::new(icon).size(18.0).color(Color32::WHITE))
                .fill(ACCENT)
                .rounding(Rounding::same(18.0))
                .min_size(vec2(52.0, 32.0));
            if ui.add(play).clicked() {
                self.engine.toggle_play();
            }
            ui.add_space(10.0);
            let mut bpm = self.project.bpm;
            if ui.add(egui::DragValue::new(&mut bpm).range(40.0..=240.0).speed(0.5).suffix(" BPM")).changed() {
                let b = self.pos_beats;
                self.project.bpm = bpm;
                self.push();
                self.engine.set_pos(b, bpm, self.project.sr);
            }
            let b = self.pos_beats.max(0.0);
            let txt = format!("{}.{}.{}", (b / 4.0) as u32 + 1, (b % 4.0) as u32 + 1, ((b % 1.0) * 4.0) as u32 + 1);
            ui.label(RichText::new(txt).monospace().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let help = egui::Button::new(RichText::new("?").size(15.0).strong()).rounding(Rounding::same(15.0)).min_size(vec2(30.0, 30.0));
                if ui.add(help).on_hover_text("Help & shortcuts (F1)").clicked() {
                    self.show_help = true;
                }
                ui.menu_button("Export ▾", |ui| {
                    if ui.button("Mixdown (WAV)…").clicked() {
                        self.export_mixdown();
                        ui.close_menu();
                    }
                    if ui.button("Stems (one WAV per track)…").clicked() {
                        self.export_stems();
                        ui.close_menu();
                    }
                    if ui.button("MIDI file…").clicked() {
                        self.export_midi_dialog();
                        ui.close_menu();
                    }
                });
                if ui.button("Save").clicked() {
                    self.save_project(false);
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("↷")).clicked() {
                    self.redo();
                }
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("↶")).clicked() {
                    self.undo();
                }
            });
        });
    }

    pub fn ui_browser(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("INSTRUMENTS").size(11.0).color(GREY).strong());
        ui.label(RichText::new("Drag onto the timeline, or click to add").size(11.5).color(GREY));
        for (i, inst) in Instrument::ALL.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click_and_drag());
            let resp = resp.on_hover_cursor(egui::CursorIcon::Grab);
            let p = ui.painter();
            let fill = if resp.hovered() { Color32::from_rgb(235, 235, 240) } else { Color32::WHITE };
            p.rect_filled(rect, 11.0, fill);
            p.rect_stroke(rect, 11.0, stroke(1.0, HAIR));
            p.circle_filled(pos2(rect.left() + 18.0, rect.center().y), 6.0, PALETTE[i % 8]);
            p.text(pos2(rect.left() + 34.0, rect.top() + 15.0), Align2::LEFT_CENTER, inst.name(), FontId::proportional(13.5), INK);
            p.text(pos2(rect.left() + 34.0, rect.top() + 32.0), Align2::LEFT_CENTER, inst.blurb(), FontId::proportional(11.0), GREY);
            p.text(pos2(rect.right() - 14.0, rect.center().y), Align2::CENTER_CENTER, "⠿", FontId::proportional(14.0), GREY);
            if resp.drag_started() {
                self.drag_inst = Some(*inst);
            }
            if resp.clicked() {
                self.add_track(*inst);
            }
        }
        ui.add_space(8.0);
        ui.label(RichText::new("AUDIO").size(11.0).color(GREY).strong());
        if ui.add_sized([ui.available_width(), 34.0], egui::Button::new("Import WAV…")).clicked() {
            self.import_dialog();
        }
    }

    pub fn ui_help(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("OpenDembow Help")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("Quick start").size(15.0).strong());
                ui.label("1. Drag an instrument from the left onto the timeline (or click it).");
                ui.label("2. Double-click an empty lane to create a clip, then add notes in the piano roll.");
                ui.label("3. Play any instrument live with the on-screen keyboard or your computer keys.");
                ui.label("4. Press Space to play. Drag clips to move them. Use × on a track to delete it.");
                ui.label("5. File > Export to bounce your song to WAV, stems or MIDI.");
                ui.add_space(10.0);
                ui.label(RichText::new("Shortcuts").size(15.0).strong());
                let rows = [
                    ("Space", "Play / pause"),
                    ("Home", "Rewind to start"),
                    ("A W S E D F T G Y H U J K O L", "Play notes (piano layout)"),
                    ("Z / X", "Keyboard octave down / up"),
                    ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
                    ("Ctrl+D", "Duplicate selected clip"),
                    ("Delete", "Delete selected note or clip"),
                    ("Right-click note", "Delete note"),
                    ("Right-click track", "Change instrument / delete track"),
                    ("Ctrl+S / Ctrl+O", "Save / open project"),
                    ("Ctrl+I / Ctrl+E", "Import audio / export mixdown"),
                    ("Pinch or Ctrl+wheel", "Zoom the timeline or piano roll"),
                    ("F1", "This window"),
                ];
                egui::Grid::new("shortcuts").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                    for (k, d) in rows {
                        ui.label(RichText::new(k).monospace().strong());
                        ui.label(d);
                        ui.end_row();
                    }
                });
                ui.add_space(10.0);
                ui.label(RichText::new("OpenDembow 0.3 - made with Rust, egui and cpal.").color(GREY).size(12.0));
            });
        self.show_help = open;
    }
}
