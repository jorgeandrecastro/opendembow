//! Piano roll: edit the notes of the selected MIDI clip.

use crate::project::*;
use crate::theme::*;
use crate::{App, Drag};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Sense};

const KEYS_W: f32 = 62.0;
const ROW_H: f32 = 15.0;

/// (label, length in beats): 1/32 ... whole note. Used for the grid and the note size.
pub const SIZES: [(&str, f64); 6] = [("1/32", 0.125), ("1/16", 0.25), ("1/8", 0.5), ("1/4", 1.0), ("1/2", 2.0), ("1", 4.0)];

pub fn size_label(v: f64) -> &'static str {
    SIZES.iter().find(|(_, s)| (s - v).abs() < 1e-9).map(|(l, _)| *l).unwrap_or("custom")
}

impl App {
    pub fn ui_roll(&mut self, ui: &mut egui::Ui) {
        let Some((t, c)) = self.sel_midi() else {
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 14.0, Color32::WHITE);
            p.rect_stroke(rect, 14.0, stroke(1.0, HAIR));
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Select a MIDI clip to edit its notes - or double-click an empty lane to create one",
                FontId::proportional(14.5),
                GREY,
            );
            return;
        };
        let title = self.project.tracks[t].clips[c].name.clone();
        let col = PALETTE[self.project.tracks[t].color % 8];

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(14.0);
            ui.label(RichText::new(title).strong());
            let hint = if self.pen {
                "Pen: click or drag to draw notes · drag a note to move · right-click: delete"
            } else {
                "Double-click: add note · drag: move · drag either edge: resize · Ctrl+click: cut"
            };
            ui.label(RichText::new(hint).size(11.5).color(GREY));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+").clicked() {
                    self.octave = (self.octave + 1).min(6);
                }
                ui.label(format!("Keyboard octave {}", self.octave));
                if ui.small_button("−").clicked() {
                    self.octave = (self.octave - 1).max(0);
                }
            });
        });

        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let rect = Rect::from_min_max(pos2(rect.left() + 14.0, rect.top()), rect.right_bottom());
        let keys_r = Rect::from_min_size(rect.min, vec2(KEYS_W, rect.height()));
        let grid = Rect::from_min_max(pos2(rect.left() + KEYS_W, rect.top()), rect.right_bottom());
        let rows = ((rect.height() / ROW_H).floor() as i32).max(1);
        let gr = ui.interact(grid, ui.id().with("roll"), Sense::click_and_drag());
        let kr = ui.interact(keys_r, ui.id().with("rollkeys"), Sense::click_and_drag());

        if gr.hovered() {
            let (zoom, scroll, hover) = ui.input(|i| (i.zoom_delta(), i.raw_scroll_delta, i.pointer.hover_pos()));
            if zoom != 1.0 {
                if let Some(p) = hover {
                    let anchor = self.pr_scroll + ((p.x - grid.left()) / self.pr_ppb) as f64;
                    self.pr_ppb = (self.pr_ppb * zoom).clamp(20.0, 400.0);
                    self.pr_scroll = (anchor - ((p.x - grid.left()) / self.pr_ppb) as f64).max(0.0);
                }
            } else {
                self.pr_scroll = (self.pr_scroll - (scroll.x / self.pr_ppb) as f64).max(0.0);
                self.pr_top += (scroll.y / ROW_H).round() as i32;
            }
        }
        self.pr_top = self.pr_top.clamp(rows - 1, 127);
        let (top, ppb, sx) = (self.pr_top, self.pr_ppb, self.pr_scroll);
        let gx = |b: f64| grid.left() + ((b - sx) as f32) * ppb;
        let beat_at = |x: f32| ((x - grid.left()) / ppb) as f64 + sx;
        let key_at = |y: f32| top - ((y - grid.top()) / ROW_H).floor() as i32;
        let g = self.grid;
        // hit test: (note index, edge) with edge 0 = body, 1 = right edge, 2 = left edge
        let hit = |notes: &Vec<Note>, p: Pos2| -> Option<(usize, u8)> {
            for (i, n) in notes.iter().enumerate().rev() {
                let y = grid.top() + (top - n.key as i32) as f32 * ROW_H;
                let r = Rect::from_min_max(pos2(gx(n.start), y), pos2(gx(n.start + n.len), y + ROW_H));
                if r.contains(p) {
                    let z = 6.0f32.min(r.width() / 3.0);
                    let edge = if p.x > r.right() - z {
                        1
                    } else if p.x < r.left() + z {
                        2
                    } else {
                        0
                    };
                    return Some((i, edge));
                }
            }
            None
        };

        // note editing
        if gr.drag_started() {
            // use where the mouse went down, not where it is after the drag threshold
            let origin = ui.input(|i| i.pointer.press_origin()).or_else(|| gr.interact_pointer_pos());
            if let Some(p) = origin {
                let h = self.notes_ref(t, c).and_then(|n| hit(n, p));
                if let Some((i, edge)) = h {
                    self.checkpoint();
                    self.sel_note = Some(i);
                    let n = self.notes_ref(t, c).map(|v| v[i].clone());
                    if let Some(n) = n {
                        self.drag = match edge {
                            1 => Drag::NoteResize { idx: i },
                            2 => Drag::NoteResizeL { idx: i, end: n.start + n.len },
                            _ => Drag::NoteMove { idx: i, grab_beat: beat_at(p.x) - n.start, grab_key: key_at(p.y) - n.key as i32 },
                        };
                        self.audition(t, n.key);
                    }
                } else if self.pen {
                    let key = key_at(p.y).clamp(0, 127) as u8;
                    let start = ((beat_at(p.x) / g).floor() * g).max(0.0);
                    self.checkpoint();
                    if let Some(n) = self.notes_mut(t, c) {
                        n.push(Note { start, len: g, key, vel: 0.8 });
                    }
                    if let Some(idx) = self.notes_ref(t, c).map(|v| v.len() - 1) {
                        self.sel_note = Some(idx);
                        self.drag = Drag::NoteResize { idx };
                    }
                    self.audition(t, key);
                    self.fit_clip(t, c);
                    self.push();
                }
            }
        }
        if gr.dragged() {
            if let Some(p) = gr.interact_pointer_pos() {
                match self.drag {
                    Drag::NoteMove { idx, grab_beat, grab_key } => {
                        let ns = (((beat_at(p.x) - grab_beat) / g).round() * g).max(0.0);
                        let nk = (key_at(p.y) - grab_key).clamp(0, 127) as u8;
                        let mut moved = false;
                        if let Some(n) = self.notes_mut(t, c).and_then(|v| v.get_mut(idx)) {
                            n.start = ns;
                            if n.key != nk {
                                n.key = nk;
                                moved = true;
                            }
                        }
                        if moved {
                            self.audition(t, nk);
                        }
                        self.fit_clip(t, c);
                        self.push();
                    }
                    Drag::NoteResize { idx } => {
                        let beat = beat_at(p.x);
                        let mut len = g;
                        if let Some(n) = self.notes_mut(t, c).and_then(|v| v.get_mut(idx)) {
                            n.len = (((beat - n.start) / g).round() * g).max(g);
                            len = n.len;
                        }
                        self.last_len = len;
                        self.fit_clip(t, c);
                        self.push();
                    }
                    Drag::NoteResizeL { idx, end } => {
                        let ns = (((beat_at(p.x) / g).round() * g).max(0.0)).min((end - g).max(0.0));
                        if let Some(n) = self.notes_mut(t, c).and_then(|v| v.get_mut(idx)) {
                            n.start = ns;
                            n.len = end - ns;
                        }
                        self.fit_clip(t, c);
                        self.push();
                    }
                    _ => {}
                }
            }
        }
        if gr.drag_stopped() && matches!(self.drag, Drag::NoteMove { .. } | Drag::NoteResize { .. } | Drag::NoteResizeL { .. }) {
            self.drag = Drag::None;
        }
        if gr.hovered() || gr.dragged() {
            let on_edge = ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|hp| self.notes_ref(t, c).and_then(|n| hit(n, hp)))
                .map_or(false, |(_, e)| e != 0)
                || matches!(self.drag, Drag::NoteResize { .. } | Drag::NoteResizeL { .. });
            if on_edge {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            } else if self.pen {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
        if gr.clicked() || gr.double_clicked() {
            if let Some(p) = gr.interact_pointer_pos() {
                let ctrl = ui.input(|i| i.modifiers.command);
                let h = self.notes_ref(t, c).and_then(|n| hit(n, p));
                match h {
                    // Ctrl+click on a note: cut it at the (snapped) click position
                    Some((i, _)) if ctrl => {
                        let cut = (beat_at(p.x) / g).round() * g;
                        self.sel_note = Some(i);
                        if !self.split_note(t, c, i, cut) {
                            self.status = "Ctrl+click inside a note, away from its edges, to cut it.".into();
                        }
                    }
                    Some((i, _)) => self.sel_note = Some(i),
                    // empty space: the pen adds with one click, the arrow needs a double-click
                    None if self.pen || gr.double_clicked() => {
                        let key = key_at(p.y).clamp(0, 127) as u8;
                        let start = ((beat_at(p.x) / g).floor() * g).max(0.0);
                        let len = self.last_len;
                        self.checkpoint();
                        if let Some(n) = self.notes_mut(t, c) {
                            n.push(Note { start, len, key, vel: 0.8 });
                        }
                        self.sel_note = self.notes_ref(t, c).map(|v| v.len() - 1);
                        self.audition(t, key);
                        self.fit_clip(t, c);
                        self.push();
                    }
                    None => self.sel_note = None,
                }
            }
        }
        if gr.secondary_clicked() {
            if let Some(p) = gr.interact_pointer_pos() {
                if let Some((i, _)) = self.notes_ref(t, c).and_then(|n| hit(n, p)) {
                    self.checkpoint();
                    if let Some(n) = self.notes_mut(t, c) {
                        n.remove(i);
                    }
                    self.sel_note = None;
                    self.push();
                }
            }
        }
        // clicking the key column plays the instrument
        let hk = if kr.is_pointer_button_down_on() {
            kr.interact_pointer_pos().map(|p| key_at(p.y).clamp(0, 127) as u8)
        } else {
            None
        };
        if hk != self.mouse_key {
            if let Some(k) = self.mouse_key {
                self.note_off(k);
            }
            if let Some(k) = hk {
                self.note_on(k);
            }
            self.mouse_key = hk;
        }

        // drawing
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 14.0, Color32::WHITE);
        p.rect_stroke(rect, 14.0, stroke(1.0, HAIR));
        let gp = ui.painter_at(grid);
        let kp = ui.painter_at(keys_r);
        for r in 0..rows {
            let key = top - r;
            if key < 0 {
                break;
            }
            let y = grid.top() + r as f32 * ROW_H;
            let black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
            let row = Rect::from_min_size(pos2(grid.left(), y), vec2(grid.width(), ROW_H));
            gp.rect_filled(row, 0.0, if black { Color32::from_rgb(246, 246, 249) } else { Color32::WHITE });
            let line = if key % 12 == 0 { Color32::from_rgb(214, 214, 220) } else { Color32::from_rgb(240, 240, 244) };
            gp.line_segment([pos2(row.left(), row.bottom()), pos2(row.right(), row.bottom())], stroke(1.0, line));
            // key column
            let down = self.down.contains(&(key as u8));
            let kr_row = Rect::from_min_size(pos2(keys_r.left(), y), vec2(KEYS_W, ROW_H));
            if black {
                kp.rect_filled(kr_row, 0.0, Color32::WHITE);
                let bk = Rect::from_min_size(kr_row.min, vec2(KEYS_W * 0.66, ROW_H));
                kp.rect_filled(bk, 0.0, if down { ACCENT } else { INK });
            } else {
                kp.rect_filled(kr_row, 0.0, if down { tint(ACCENT, 80) } else { Color32::WHITE });
                kp.line_segment([pos2(kr_row.left(), kr_row.bottom()), pos2(kr_row.right(), kr_row.bottom())], stroke(1.0, HAIR));
            }
            if key % 12 == 0 {
                kp.text(pos2(kr_row.right() - 6.0, kr_row.center().y), Align2::RIGHT_CENTER, format!("C{}", key / 12 - 1), FontId::proportional(10.5), GREY);
            }
        }
        let clip = &self.project.tracks[t].clips[c];
        let b0 = sx.floor() as i64;
        let b1 = (sx + (grid.width() / ppb) as f64).ceil() as i64;
        // grid subdivisions follow the chosen grid when there is room for them
        let steps = (1.0 / g).round().max(1.0) as i32;
        let show_sub = steps > 1 && ppb * (g as f32) >= 12.0;
        for b in b0..=b1 {
            let x = gx(b as f64);
            let bar = b.rem_euclid(4) == 0;
            gp.line_segment([pos2(x, grid.top()), pos2(x, grid.bottom())], stroke(1.0, if bar { Color32::from_rgb(205, 205, 212) } else { Color32::from_rgb(232, 232, 237) }));
            if show_sub {
                for q in 1..steps {
                    let xq = gx(b as f64 + q as f64 * g);
                    gp.line_segment([pos2(xq, grid.top()), pos2(xq, grid.bottom())], stroke(1.0, Color32::from_rgb(243, 243, 246)));
                }
            }
        }
        let ex = gx(clip.len);
        if ex < grid.right() {
            gp.rect_filled(Rect::from_min_max(pos2(ex.max(grid.left()), grid.top()), grid.right_bottom()), 0.0, tint(INK, 12));
        }
        if let ClipKind::Midi(notes) = &clip.kind {
            for (i, n) in notes.iter().enumerate() {
                if n.key as i32 > top || (n.key as i32) <= top - rows {
                    continue;
                }
                let y = grid.top() + (top - n.key as i32) as f32 * ROW_H;
                let r = Rect::from_min_max(pos2(gx(n.start), y + 1.0), pos2(gx(n.start + n.len), y + ROW_H - 1.0));
                gp.rect_filled(r, Rounding::same(4.0), tint(col, (110.0 + n.vel * 145.0) as u8));
                if self.sel_note == Some(i) {
                    gp.rect_stroke(r, Rounding::same(4.0), stroke(1.5, INK));
                }
            }
        }
        let hx = gx(self.pos_beats - clip.start);
        if hx >= grid.left() && hx <= grid.right() {
            gp.line_segment([pos2(hx, grid.top()), pos2(hx, grid.bottom())], stroke(2.0, RED));
        }
    }
}