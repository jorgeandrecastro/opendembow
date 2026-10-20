//! Application chrome: icon, menu bar, transport bar, instrument browser, help window.

use crate::edit::AudioOp;
use crate::instruments::{Category, Instrument};
use crate::project::TrackKind;
use crate::theme::*;
use crate::{App, DetailTab};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, RichText, Rounding, Sense, Shape};
use std::f32::consts::TAU;

/// State of the instrument browser (kept in egui's memory, no change needed in `App`).
#[derive(Clone, Default)]
struct BrowserState {
    cat: Option<Category>,
    query: String,
    /// Scroll offset forced for the next frame (auto-scroll while dragging).
    force: Option<f32>,
}

fn short_name(c: Category) -> &'static str {
    match c {
        Category::Keys => "Keys",
        Category::Organs => "Organs",
        Category::Leads => "Leads",
        Category::Basses => "Basses",
        Category::Plucked => "Plucked",
        Category::Pads => "Pads",
        Category::Mallets => "Mallets",
        Category::Brass => "Brass",
        Category::Drums => "Drums",
    }
}

/// Tiny hand-drawn icon for each category.
fn draw_glyph(p: &Painter, r: Rect, cat: Category, col: Color32) {
    let c = r.center();
    let w = r.width() * 0.56;
    let h = r.height() * 0.46;
    let st = stroke(1.8, col);
    let wave = |f: fn(f32) -> f32| -> Vec<Pos2> {
        (0..=24)
            .map(|i| {
                let x = i as f32 / 24.0;
                pos2(c.x - w / 2.0 + x * w, c.y - f(x) * h / 2.0)
            })
            .collect()
    };
    match cat {
        Category::Keys => {
            for k in 0..3 {
                let x = c.x - w / 2.0 + k as f32 * (w / 3.0);
                p.rect_filled(Rect::from_min_size(pos2(x + 0.8, c.y - h / 2.0), vec2(w / 3.0 - 1.6, h)), 1.5, col);
            }
        }
        Category::Organs => {
            let hs = [0.55f32, 0.85, 1.0, 0.7];
            for (k, hh) in hs.iter().enumerate() {
                let x = c.x - w / 2.0 + k as f32 * (w / 4.0);
                let ph = h * hh;
                p.rect_filled(Rect::from_min_size(pos2(x + 0.8, c.y + h / 2.0 - ph), vec2(w / 4.0 - 1.6, ph)), 1.5, col);
            }
        }
        Category::Leads => {
            p.add(Shape::line(wave(|x| (x * 2.0).fract() * 2.0 - 1.0), st));
        }
        Category::Basses => {
            p.add(Shape::line(wave(|x| (x * TAU).sin()), st));
        }
        Category::Plucked => {
            p.add(Shape::line(wave(|x| (x * TAU * 3.0).sin() * (1.0 - x)), st));
        }
        Category::Pads => {
            p.add(Shape::line(wave(|x| (x * TAU * 1.5).sin()), st));
            p.add(Shape::line(wave(|x| (x * TAU * 1.5 + 1.2).sin()), stroke(1.8, col.gamma_multiply(0.45))));
        }
        Category::Mallets => {
            p.circle_stroke(c, h * 0.5, st);
            p.circle_filled(c, 2.2, col);
        }
        Category::Brass => {
            let pts = vec![
                pos2(c.x - w / 2.0, c.y - h * 0.14),
                pos2(c.x, c.y - h * 0.14),
                pos2(c.x + w / 2.0, c.y - h / 2.0),
                pos2(c.x + w / 2.0, c.y + h / 2.0),
                pos2(c.x, c.y + h * 0.14),
                pos2(c.x - w / 2.0, c.y + h * 0.14),
            ];
            p.add(Shape::closed_line(pts, st));
        }
        Category::Drums => {
            // a drum body with two crossed sticks above it
            let body = Rect::from_center_size(pos2(c.x, c.y + h * 0.18), vec2(w, h * 0.62));
            p.rect_stroke(body, 4.0, st);
            p.line_segment([pos2(c.x - w * 0.35, c.y - h * 0.55), pos2(c.x + w * 0.05, c.y - h * 0.02)], st);
            p.line_segment([pos2(c.x + w * 0.35, c.y - h * 0.55), pos2(c.x - w * 0.05, c.y - h * 0.02)], st);
        }
    }
}

/// Rounded tinted tile with the category glyph.
fn draw_tile(p: &Painter, rect: Rect, inst: Instrument) {
    let cat = inst.category();
    let col = PALETTE[cat as usize % 8];
    p.rect_filled(rect, 10.0, tint(col, 40));
    draw_glyph(p, rect, cat, col);
}

/// Floating card that follows the cursor while an instrument is dragged.
pub fn draw_ghost(ctx: &egui::Context, inst: Instrument) {
    if let Some(p) = ctx.pointer_latest_pos() {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("ghost")));
        let r = Rect::from_min_size(p + vec2(16.0, 12.0), vec2(200.0, 48.0));
        painter.rect_filled(r.translate(vec2(0.0, 4.0)).expand(2.0), 16.0, Color32::from_black_alpha(30));
        painter.rect_filled(r, 14.0, Color32::WHITE);
        painter.rect_stroke(r, 14.0, stroke(1.5, ACCENT));
        let tile = Rect::from_min_size(pos2(r.left() + 7.0, r.center().y - 17.0), vec2(34.0, 34.0));
        draw_tile(&painter, tile, inst);
        painter.text(pos2(tile.right() + 10.0, r.center().y - 7.0), Align2::LEFT_CENTER, inst.name(), FontId::proportional(13.5), INK);
        painter.text(pos2(tile.right() + 10.0, r.center().y + 9.0), Align2::LEFT_CENTER, inst.category().name(), FontId::proportional(11.0), GREY);
    }
}

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
                let has_clip = self.sel_clip.is_some();
                if ui.add_enabled(has_clip, egui::Button::new("Cut").shortcut_text("Ctrl+X")).clicked() {
                    self.cut_selection();
                    ui.close_menu();
                }
                if ui.add_enabled(has_clip, egui::Button::new("Copy").shortcut_text("Ctrl+C")).clicked() {
                    self.copy_selection();
                    ui.close_menu();
                }
                if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Paste").shortcut_text("Ctrl+V")).clicked() {
                    self.paste_clipboard();
                    ui.close_menu();
                }
                if ui.add_enabled(has_clip, egui::Button::new("Duplicate Clip").shortcut_text("Ctrl+D")).clicked() {
                    self.duplicate_clip();
                    ui.close_menu();
                }
                if ui.add_enabled(has_clip, egui::Button::new("Delete Selection").shortcut_text("Del")).clicked() {
                    self.delete_selected();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add_enabled(has_clip, egui::Button::new("Split at Playhead").shortcut_text("B")).clicked() {
                    self.split_at_playhead();
                    ui.close_menu();
                }
                if ui.add_enabled(has_clip, egui::Button::new("Trim Start to Playhead").shortcut_text("[")).clicked() {
                    self.trim_start_to_playhead();
                    ui.close_menu();
                }
                if ui.add_enabled(has_clip, egui::Button::new("Trim End to Playhead").shortcut_text("]")).clicked() {
                    self.trim_end_to_playhead();
                    ui.close_menu();
                }
                ui.separator();
                let audio = self.sel_is_audio();
                if ui.add_enabled(audio, egui::Button::new("Normalize Audio")).clicked() {
                    self.audio_process(AudioOp::Normalize);
                    ui.close_menu();
                }
                if ui.add_enabled(audio, egui::Button::new("Reverse Audio")).clicked() {
                    self.audio_process(AudioOp::Reverse);
                    ui.close_menu();
                }
            });
            ui.menu_button("Track", |ui| {
                ui.menu_button("Add Instrument", |ui| {
                    for c in Category::ALL {
                        ui.menu_button(c.name(), |ui| {
                            for inst in Instrument::ALL.iter().copied().filter(|i| i.category() == c) {
                                if ui.button(inst.name()).clicked() {
                                    self.add_track(inst);
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                });
                if ui.button("Add Audio Track (WAV)…").clicked() {
                    self.import_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Show Mixer & Effects").clicked() {
                    self.detail = DetailTab::Mixer;
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
            ui.label(RichText::new("0.6").color(GREY));
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
            ui.add_space(6.0);
            self.metronome.ui_button(ui, self.pos_beats, self.playing);
            ui.add_space(6.0);
            let b = self.pos_beats.max(0.0);
            let txt = format!("{}.{}.{}", (b / 4.0) as u32 + 1, (b % 4.0) as u32 + 1, ((b % 1.0) * 4.0) as u32 + 1);
            ui.label(RichText::new(txt).monospace().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let help = egui::Button::new(RichText::new("?").size(15.0).strong()).rounding(Rounding::same(15.0)).min_size(vec2(30.0, 30.0));
                if ui.add(help).on_hover_text("Help & shortcuts (F1)").clicked() {
                    self.show_help = true;
                }
                ui.menu_button("Export", |ui| {
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
                // Undo / redo live in the Edit menu and on Ctrl+Z / Ctrl+Shift+Z.
            });
        });
    }

    /// One instrument row: tinted glyph tile, name, description.
    fn browser_row(&mut self, ui: &mut egui::Ui, inst: Instrument) {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click_and_drag());
        let resp = resp.on_hover_cursor(egui::CursorIcon::Grab);
        let current = matches!(
            self.project.tracks.get(self.sel_track).map(|t| t.kind),
            Some(TrackKind::Instrument(i)) if i == inst
        );
        let p = ui.painter();
        if current {
            p.rect_filled(rect, 12.0, tint(ACCENT, 16));
        } else if resp.hovered() {
            p.rect_filled(rect, 12.0, Color32::from_rgb(243, 243, 247));
        }
        let tile = Rect::from_min_size(pos2(rect.left() + 8.0, rect.center().y - 17.0), vec2(34.0, 34.0));
        draw_tile(p, tile, inst);
        let tp = p.with_clip_rect(Rect::from_min_max(rect.min, pos2(rect.right() - 36.0, rect.bottom())));
        tp.text(pos2(tile.right() + 12.0, rect.center().y - 8.0), Align2::LEFT_CENTER, inst.name(), FontId::proportional(13.5), INK);
        tp.text(pos2(tile.right() + 12.0, rect.center().y + 9.0), Align2::LEFT_CENTER, inst.blurb(), FontId::proportional(11.0), GREY);
        if resp.hovered() && self.drag_inst.is_none() {
            let c = pos2(rect.right() - 20.0, rect.center().y);
            p.circle_filled(c, 10.0, ACCENT);
            p.line_segment([c - vec2(4.0, 0.0), c + vec2(4.0, 0.0)], stroke(1.6, Color32::WHITE));
            p.line_segment([c - vec2(0.0, 4.0), c + vec2(0.0, 4.0)], stroke(1.6, Color32::WHITE));
        }
        if resp.drag_started() {
            self.drag_inst = Some(inst);
        }
        if resp.clicked() {
            self.add_track(inst);
        }
    }

    pub fn ui_browser(&mut self, ui: &mut egui::Ui) {
        let sid = egui::Id::new("instrument_browser_state");
        let mut st: BrowserState = ui.ctx().data_mut(|d| d.get_temp(sid)).unwrap_or_default();
        let soft = Color32::from_rgb(240, 240, 245);

        egui::Frame::none()
            .fill(Color32::WHITE)
            .rounding(Rounding::same(14.0))
            .stroke(stroke(1.0, HAIR))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());

                // header
                ui.label(RichText::new("Instruments").size(17.0).strong().color(INK));
                ui.label(RichText::new(format!("{} sounds · drag onto the timeline", Instrument::ALL.len())).size(11.5).color(GREY));
                ui.add_space(8.0);

                // search pill
                egui::Frame::none()
                    .fill(soft)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔍").size(12.0).color(GREY));
                            let w = (ui.available_width() - 22.0).max(40.0);
                            ui.add(egui::TextEdit::singleline(&mut st.query).hint_text("Search").frame(false).desired_width(w));
                            if !st.query.is_empty() && ui.small_button("×").clicked() {
                                st.query.clear();
                            }
                        });
                    });
                ui.add_space(8.0);

                // category chips
                let chip = |ui: &mut egui::Ui, label: &str, sel: bool| -> bool {
                    let b = egui::Button::new(RichText::new(label).size(12.0).color(if sel { Color32::WHITE } else { INK }))
                        .fill(if sel { ACCENT } else { soft })
                        .stroke(egui::Stroke::NONE)
                        .rounding(Rounding::same(12.0))
                        .min_size(vec2(0.0, 24.0));
                    ui.add(b).clicked()
                };
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(5.0, 6.0);
                    if chip(ui, "All", st.cat.is_none()) {
                        st.cat = None;
                    }
                    for c in Category::ALL {
                        if chip(ui, short_name(c), st.cat == Some(c)) {
                            st.cat = Some(c);
                        }
                    }
                });
                ui.add_space(8.0);

                let cat = st.cat;
                let q = st.query.trim().to_lowercase();
                let matches = |inst: &Instrument| -> bool {
                    (cat.is_none() || Some(inst.category()) == cat)
                        && (q.is_empty()
                            || inst.name().to_lowercase().contains(&q)
                            || inst.blurb().to_lowercase().contains(&q)
                            || inst.category().name().to_lowercase().contains(&q))
                };

                // scrollable list
                let h = (ui.available_height() - 50.0).max(100.0);
                let mut area = egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]);
                if let Some(o) = st.force.take() {
                    area = area.vertical_scroll_offset(o);
                }
                let out = area.show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let mut shown = 0;
                    for c in Category::ALL {
                        let items: Vec<Instrument> =
                            Instrument::ALL.iter().copied().filter(|i| i.category() == c && matches(i)).collect();
                        if items.is_empty() {
                            continue;
                        }
                        if cat.is_none() {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.5, PALETTE[c as usize % 8]);
                                ui.label(RichText::new(c.name().to_uppercase()).size(10.5).color(GREY).strong());
                            });
                            ui.add_space(2.0);
                        }
                        for inst in items {
                            self.browser_row(ui, inst);
                            shown += 1;
                        }
                    }
                    if shown == 0 {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No instrument found").color(GREY));
                        });
                    }
                });

                // while dragging, the list keeps scrolling: wheel + auto-scroll near the edges
                if self.drag_inst.is_some() {
                    if let Some(p) = ui.ctx().pointer_latest_pos() {
                        let r = out.inner_rect;
                        if p.x >= r.left() && p.x <= r.right() && p.y >= r.top() - 40.0 && p.y <= r.bottom() + 40.0 {
                            let zone = 48.0;
                            let edge = if p.y < r.top() + zone {
                                -(1.0 - ((p.y - r.top()) / zone).clamp(0.0, 1.0))
                            } else if p.y > r.bottom() - zone {
                                1.0 - ((r.bottom() - p.y) / zone).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let wheel = ui.ctx().input(|i| i.raw_scroll_delta.y);
                            let d = edge * 14.0 - wheel;
                            if d != 0.0 {
                                st.force = Some((out.state.offset.y + d).max(0.0));
                            }
                        }
                    }
                    ui.ctx().request_repaint();
                }

                ui.add_space(8.0);
                let imp = egui::Button::new(RichText::new("Import WAV…").size(13.0).color(INK))
                    .fill(soft)
                    .stroke(egui::Stroke::NONE)
                    .rounding(Rounding::same(10.0));
                if ui.add_sized([ui.available_width(), 34.0], imp).clicked() {
                    self.import_dialog();
                }
            });

        ui.ctx().data_mut(|d| d.insert_temp(sid, st));
    }

    pub fn ui_help(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("OpenDembow Help")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("Quick start").size(15.0).strong());
                ui.label("1. Pick a category on the left, then drag an instrument onto the timeline (or click it).");
                ui.label("2. Double-click an empty lane to create a clip, then add notes in the piano roll.");
                ui.label("3. Play any instrument live with the on-screen keyboard or your computer keys.");
                ui.label("4. Press Space to play. Drag clips to move them. Use × on a track to delete it.");
                ui.label("5. Mixer & Effects tab: volume, pan, EQ, compressor, reverb, delay per track.");
                ui.label("6. Edit clips: Split (B), trim to playhead ( [ and ] ), cut / copy / paste.");
                ui.label("7. File > Export to bounce your song to WAV, stems or MIDI.");
                ui.label("8. Drums: pick Drum Kit or 808 Kit. C2 = kick, D2 = snare, E2 = clap, F#2 = closed hat, A#2 = open hat.");
                ui.add_space(10.0);
                ui.label(RichText::new("Shortcuts").size(15.0).strong());
                let rows = [
                    ("Space", "Play / pause"),
                    ("Home", "Rewind to start"),
                    ("M", "Metronome on / off (right-click it for options)"),
                    ("A W S E D F T G Y H U J K O L", "Play notes (piano layout)"),
                    ("Z / X", "Keyboard octave down / up"),
                    ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
                    ("Ctrl+C / Ctrl+X / Ctrl+V", "Copy / cut / paste clip or note (at playhead)"),
                    ("P / Esc", "Pen (draw / edit) mode on / off"),
                    ("B", "Split selected clip at the playhead"),
                    ("[  /  ]", "Trim clip start / end to the playhead"),
                    ("Ctrl+D", "Duplicate selected clip"),
                    ("Delete", "Delete selected note or clip"),
                    ("Pen + click / drag", "Draw a note (drag sets its length)"),
                    ("Double-click (arrow)", "Add a note"),
                    ("Drag clip / note edge", "Shorten or lengthen (Alt = no snap)"),
                    ("Pen + drag on a lane", "Draw a clip of any length"),
                    ("Ctrl+click clip / note", "Cut it at the click (B cuts at the playhead)"),
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
                ui.label(RichText::new("Tip: start with `opendembow --mcp` to let an AI assistant (MCP) control the project.").size(12.0));
                ui.label(RichText::new("OpenDembow 0.6 - made with Rust, egui and cpal.").color(GREY).size(12.0));
            });
        self.show_help = open;
    }
}