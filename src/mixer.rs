//! The "Mixer & Effects" tab: track strips, Master (spectrum + dBFS meters) and the effect cards
//! of the selected track. The EQ and compressor cards show their curves (see `curves.rs`).

use crate::App;
use crate::analyzer::Tap;
use crate::curves;
use crate::effects::FxSettings;
use crate::theme::*;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, RichText, Rounding, Sense, Shape, pos2, vec2,
};
use std::f32::consts::PI;

// ======================================================================
// Knob + effect card
// ======================================================================

/// Rotary knob: drag vertically, double-click to reset. Returns true when the value changed.
#[allow(clippy::too_many_arguments)]
fn knob(
    ui: &mut egui::Ui,
    label: &str,
    v: &mut f32,
    min: f32,
    max: f32,
    def: f32,
    fmt: impl Fn(f32) -> String,
    col: Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(50.0, 74.0), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let d = -resp.drag_delta().y / 160.0 * (max - min);
        let nv = (*v + d).clamp(min, max);
        if nv != *v {
            *v = nv;
            changed = true;
        }
    }
    if resp.double_clicked() && *v != def {
        *v = def;
        changed = true;
    }
    let _ = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
    let p = ui.painter_at(rect);
    let c = pos2(rect.center().x, rect.top() + 24.0);
    let r = 18.0;
    let t = ((*v - min) / (max - min)).clamp(0.0, 1.0);
    let arc = |a0: f32, a1: f32| -> Vec<Pos2> {
        (0..=32)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 32.0;
                pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect()
    };
    let (start, span) = (0.75 * PI, 1.5 * PI);
    p.add(Shape::line(
        arc(start, start + span),
        stroke(3.5, Color32::from_rgb(230, 230, 236)),
    ));
    if t > 0.003 {
        p.add(Shape::line(arc(start, start + span * t), stroke(3.5, col)));
    }
    let a = start + span * t;
    p.line_segment(
        [
            pos2(c.x + (r - 10.0) * a.cos(), c.y + (r - 10.0) * a.sin()),
            pos2(c.x + (r - 3.0) * a.cos(), c.y + (r - 3.0) * a.sin()),
        ],
        stroke(2.0, INK),
    );
    p.text(
        pos2(rect.center().x, rect.top() + 52.0),
        Align2::CENTER_CENTER,
        fmt(*v),
        FontId::proportional(11.0),
        INK,
    );
    p.text(
        pos2(rect.center().x, rect.top() + 66.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        GREY,
    );
    changed
}

fn db(v: f32) -> String {
    format!("{:+.1} dB", v)
}
fn hz(v: f32) -> String {
    if v >= 1000.0 {
        format!("{:.1} kHz", v / 1000.0)
    } else {
        format!("{:.0} Hz", v)
    }
}
fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}
fn ms(v: f32) -> String {
    format!("{:.0} ms", v)
}

/// Rounded white card with an ON/OFF pill. Returns (pill clicked, body changed something).
/// The caller flips the flag on a pill click and switches the effect on when the body changed.
fn fx_card(
    ui: &mut egui::Ui,
    title: &str,
    on: bool,
    col: Color32,
    body: impl FnOnce(&mut egui::Ui) -> bool,
) -> (bool, bool) {
    let (mut toggled, mut changed) = (false, false);
    egui::Frame::none()
        .fill(Color32::WHITE)
        .rounding(Rounding::same(12.0))
        .stroke(stroke(1.0, if on { col } else { HAIR }))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let txt = RichText::new(if on { "ON" } else { "OFF" })
                        .size(10.5)
                        .strong()
                        .color(if on { Color32::WHITE } else { GREY });
                    let b = egui::Button::new(txt)
                        .fill(if on {
                            col
                        } else {
                            Color32::from_rgb(240, 240, 245)
                        })
                        .stroke(egui::Stroke::NONE)
                        .rounding(Rounding::same(8.0))
                        .min_size(vec2(34.0, 18.0));
                    if ui.add(b).clicked() {
                        toggled = true;
                    }
                    ui.label(RichText::new(title).strong().size(13.0));
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    changed = body(ui);
                });
            });
        });
    (toggled, changed)
}

// ======================================================================
// Panel
// ======================================================================

impl App {
    /// "Mixer & Effects" tab: track strips | Master (spectrum + meters) | effects of the selected track.
    pub fn ui_fx(&mut self, ui: &mut egui::Ui) {
        let n = self.project.tracks.len();
        if n == 0 {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Add a track to use the mixer and effects").color(GREY))
            });
            return;
        }
        let (w, h) = (ui.available_width(), ui.available_height());
        let tap = self.engine.tap.clone();
        ui.horizontal_top(|ui| {
            ui.add_space(14.0);
            let mw = (n as f32 * 70.0 + 8.0).min(w * 0.30);
            ui.allocate_ui_with_layout(
                vec2(mw, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.push_id("mixer_scroll", |ui| {
                        egui::ScrollArea::horizontal()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    for i in 0..n {
                                        self.mixer_strip(ui, i);
                                    }
                                });
                            });
                    });
                },
            );
            ui.add_space(8.0);
            crate::analyzer::master_panel(ui, &tap, 340.0f32.min(w * 0.34), h);
            ui.add_space(8.0);
            let ew = ui.available_width();
            ui.allocate_ui_with_layout(
                vec2(ew, h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.push_id("fx_scroll", |ui| self.ui_effects(ui, h, &tap));
                },
            );
        });
    }

    fn mixer_strip(&mut self, ui: &mut egui::Ui, i: usize) {
        let tr = &self.project.tracks[i];
        let name: String = tr.name.chars().take(9).collect();
        let color = PALETTE[tr.color % 8];
        let (mut vol, mut pan, mut mute, mut solo) = (tr.volume, tr.pan, tr.mute, tr.solo);
        let selected = self.sel_track == i;
        // not clamped: an over (> 0 dBFS) lights the clip LED
        let level = self.meters.get(i).copied().unwrap_or(0.0);
        let (mut changed, mut select) = (false, false);
        let soft = Color32::from_rgb(240, 240, 245);

        egui::Frame::none()
            .fill(Color32::WHITE)
            .rounding(Rounding::same(12.0))
            .stroke(stroke(
                if selected { 1.5 } else { 1.0 },
                if selected { ACCENT } else { HAIR },
            ))
            .inner_margin(egui::Margin::same(7.0))
            .show(ui, |ui| {
                ui.set_width(50.0);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let lab =
                        egui::Label::new(RichText::new(&name).size(11.0).strong().color(color))
                            .sense(Sense::click());
                    if ui.add(lab).clicked() {
                        select = true;
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().slider_width = 100.0;
                        let r = ui.add(
                            egui::Slider::new(&mut vol, 0.0..=1.0)
                                .vertical()
                                .show_value(false),
                        );
                        changed |= r.changed();
                        // dBFS meter: hover for the held peak, click to reset
                        crate::meters::meter(ui, vec2(8.0, 106.0), i, level, false);
                    });
                    ui.spacing_mut().slider_width = 42.0;
                    let r = ui.add(egui::Slider::new(&mut pan, -1.0..=1.0).show_value(false));
                    changed |= r.changed();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        let m = egui::Button::new(
                            RichText::new("M").size(11.0).strong().color(if mute {
                                Color32::WHITE
                            } else {
                                INK
                            }),
                        )
                        .fill(if mute { RED } else { soft })
                        .stroke(egui::Stroke::NONE)
                        .min_size(vec2(20.0, 20.0));
                        if ui.add(m).clicked() {
                            mute = !mute;
                            changed = true;
                        }
                        let s = egui::Button::new(
                            RichText::new("S").size(11.0).strong().color(if solo {
                                Color32::WHITE
                            } else {
                                INK
                            }),
                        )
                        .fill(if solo { ACCENT } else { soft })
                        .stroke(egui::Stroke::NONE)
                        .min_size(vec2(20.0, 20.0));
                        if ui.add(s).clicked() {
                            solo = !solo;
                            changed = true;
                        }
                    });
                });
            });

        if select {
            self.sel_track = i;
        }
        if changed {
            let ctx = ui.ctx().clone();
            self.edit_checkpoint(&ctx);
            let t = &mut self.project.tracks[i];
            t.volume = vol;
            t.pan = pan;
            t.mute = mute;
            t.solo = solo;
            self.push();
        }
    }

    fn ui_effects(&mut self, ui: &mut egui::Ui, h: f32, tap: &Tap) {
        let t = self.sel_track.min(self.project.tracks.len() - 1);
        let tr = &self.project.tracks[t];
        let (name, col) = (tr.name.clone(), PALETTE[tr.color % 8]);
        let mut fx = tr.fx.clone();
        let level = self.meters.get(t).copied().unwrap_or(0.0);
        let mut changed = false;
        let mut reset = false;
        // graph height: as tall as the panel allows (title row, card header and margins taken out)
        let gh = (h - 28.0 - 66.0).clamp(96.0, 150.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Effects - {name}")).strong());
            ui.label(
                RichText::new("drag a knob or a curve point, double-click to reset")
                    .size(11.5)
                    .color(GREY),
            );
            if ui.small_button("Reset effects").clicked() {
                reset = true;
            }
        });
        egui::ScrollArea::horizontal()
            .max_height((h - 28.0).max(80.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // ---- Equalizer: curve + the same settings as knobs ----
                    let mut eq = fx.eq.clone();
                    let (tg, ch) = fx_card(ui, "Equalizer", eq.on, col, |ui| {
                        let mut c = curves::eq_graph(ui, t, &mut eq, tap, vec2(270.0, gh), col);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.horizontal(|ui| {
                                c |= knob(ui, "Low", &mut eq.low, -15.0, 15.0, 0.0, db, col);
                                c |= knob(ui, "Mid", &mut eq.mid, -15.0, 15.0, 0.0, db, col);
                                c |= knob(ui, "High", &mut eq.high, -15.0, 15.0, 0.0, db, col);
                            });
                            ui.horizontal(|ui| {
                                let mut lf = eq.mid_freq.ln();
                                if knob(
                                    ui,
                                    "Mid freq",
                                    &mut lf,
                                    200f32.ln(),
                                    5000f32.ln(),
                                    1000f32.ln(),
                                    |v| hz(v.exp()),
                                    col,
                                ) {
                                    eq.mid_freq = lf.exp();
                                    c = true;
                                }
                                c |= knob(
                                    ui,
                                    "Mid Q",
                                    &mut eq.mid_q,
                                    0.3,
                                    6.0,
                                    1.0,
                                    |v| format!("{:.2}", v),
                                    col,
                                );
                            });
                        });
                        c
                    });
                    if tg {
                        eq.on = !eq.on;
                    } else if ch {
                        eq.on = true; // touching a control switches the effect on
                    }
                    if tg || ch {
                        fx.eq = eq;
                        changed = true;
                    }

                    // ---- Compressor: transfer curve, gain-reduction meter and knobs ----
                    let mut cp = fx.comp.clone();
                    let gr = curves::gain_reduction(ui.ctx(), t, &cp, level);
                    let (tg, ch) = fx_card(ui, "Compressor", cp.on, col, |ui| {
                        let mut c =
                            curves::comp_graph(ui, t, &mut cp, level, gr, vec2(190.0, gh), col);
                        curves::gr_meter(ui, vec2(12.0, gh), gr);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.horizontal(|ui| {
                                c |= knob(
                                    ui,
                                    "Thresh",
                                    &mut cp.threshold,
                                    -60.0,
                                    0.0,
                                    -18.0,
                                    |v| format!("{:.0} dB", v),
                                    col,
                                );
                                c |= knob(
                                    ui,
                                    "Ratio",
                                    &mut cp.ratio,
                                    1.0,
                                    20.0,
                                    3.0,
                                    |v| format!("{:.1}:1", v),
                                    col,
                                );
                                c |= knob(ui, "Attack", &mut cp.attack, 1.0, 100.0, 10.0, ms, col);
                            });
                            ui.horizontal(|ui| {
                                c |= knob(
                                    ui,
                                    "Release",
                                    &mut cp.release,
                                    20.0,
                                    1000.0,
                                    120.0,
                                    ms,
                                    col,
                                );
                                c |= knob(
                                    ui,
                                    "Makeup",
                                    &mut cp.makeup,
                                    0.0,
                                    24.0,
                                    0.0,
                                    |v| format!("{:.1} dB", v),
                                    col,
                                );
                            });
                        });
                        c
                    });
                    if tg {
                        cp.on = !cp.on;
                    } else if ch {
                        cp.on = true;
                    }
                    if tg || ch {
                        fx.comp = cp;
                        changed = true;
                    }

                    // ---- Reverb ----
                    let mut rv = fx.reverb.clone();
                    let (tg, ch) = fx_card(ui, "Reverb", rv.on, col, |ui| {
                        let a = knob(ui, "Mix", &mut rv.mix, 0.0, 1.0, 0.25, pct, col);
                        let b = knob(ui, "Size", &mut rv.size, 0.0, 1.0, 0.5, pct, col);
                        let c = knob(ui, "Damp", &mut rv.damp, 0.0, 1.0, 0.5, pct, col);
                        a | b | c
                    });
                    if tg {
                        rv.on = !rv.on;
                    } else if ch {
                        rv.on = true;
                    }
                    if tg || ch {
                        fx.reverb = rv;
                        changed = true;
                    }

                    // ---- Delay ----
                    let mut dl = fx.delay.clone();
                    let (tg, ch) = fx_card(ui, "Delay", dl.on, col, |ui| {
                        let a = knob(ui, "Time", &mut dl.time, 20.0, 1000.0, 350.0, ms, col);
                        let b = knob(ui, "Feedback", &mut dl.feedback, 0.0, 0.9, 0.35, pct, col);
                        let c = knob(ui, "Mix", &mut dl.mix, 0.0, 1.0, 0.25, pct, col);
                        a | b | c
                    });
                    if tg {
                        dl.on = !dl.on;
                    } else if ch {
                        dl.on = true;
                    }
                    if tg || ch {
                        fx.delay = dl;
                        changed = true;
                    }
                });
            });

        if reset && fx != FxSettings::default() {
            fx = FxSettings::default();
            changed = true;
        }
        if changed {
            let ctx = ui.ctx().clone();
            self.edit_checkpoint(&ctx);
            self.project.tracks[t].fx = fx;
            self.push();
        }
    }
}
