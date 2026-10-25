//! dBFS level meters: green / amber / red zones, peak hold line, clip LED.
//!
//! Hold state lives in egui's temporary memory, keyed by a "slot" (track index, or one of the
//! MASTER_* slots), so no change is needed in `App`.

use crate::theme::*;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Sense, Vec2};
use std::collections::HashMap;

pub const FLOOR_DB: f32 = -60.0;
/// Height reserved for the clip LED above the bar.
pub const CLIP_H: f32 = 6.0;
pub const MASTER_L: usize = usize::MAX - 1;
pub const MASTER_R: usize = usize::MAX;

const GREEN: Color32 = Color32::from_rgb(52, 199, 89);
const AMBER: Color32 = Color32::from_rgb(255, 204, 0);
/// Seconds the peak line stays put before it falls back.
const HOLD_SECS: f64 = 1.5;
/// How fast the peak line falls afterwards (dB per second).
const HOLD_FALL: f32 = 15.0;

/// Linear amplitude -> dBFS (-120 = silence).
pub fn to_db(lin: f32) -> f32 {
    if lin <= 1e-6 {
        -120.0
    } else {
        20.0 * lin.log10()
    }
}

/// dBFS -> 0..1 on the meter scale (-60 dB .. 0 dB).
pub fn db_norm(db: f32) -> f32 {
    ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0)
}

/// Linear amplitude -> 0..1 on the meter scale (handy for small level bars).
pub fn norm(lin: f32) -> f32 {
    db_norm(to_db(lin))
}

#[derive(Clone, Copy)]
struct Hold {
    /// Level shown (linear).
    lvl: f32,
    /// Held peak in dB.
    db: f32,
    /// Time of the last new peak.
    t: f64,
    clip: bool,
}

impl Default for Hold {
    fn default() -> Self {
        Self { lvl: 0.0, db: -120.0, t: 0.0, clip: false }
    }
}

#[derive(Clone, Default)]
struct Bank(HashMap<usize, Hold>);

fn bank_id() -> egui::Id {
    egui::Id::new("meter_bank")
}

/// Held peak of a slot, in dBFS (-120 = nothing yet).
pub fn peak_db(ctx: &egui::Context, slot: usize) -> f32 {
    ctx.data(|d| d.get_temp::<Bank>(bank_id())).and_then(|b| b.0.get(&slot).map(|h| h.db)).unwrap_or(-120.0)
}

/// Clears every peak hold and clip LED.
pub fn reset_all(ctx: &egui::Context) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Bank>(bank_id()).0.clear());
}

/// Vertical dBFS meter. `lin` is the linear peak level (it can exceed 1.0).
/// `fall`: apply a 20 dB/s falling edge (for raw peaks; track levels are already smoothed by the engine).
/// A click resets the peak hold and the clip LED.
pub fn meter(ui: &mut egui::Ui, size: Vec2, slot: usize, lin: f32, fall: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let ctx = ui.ctx().clone();
    let (now, dt) = ctx.input(|i| (i.time, i.stable_dt.min(0.1)));
    let clicked = resp.clicked();

    let h = ctx.data_mut(|d| {
        let h = d.get_temp_mut_or_default::<Bank>(bank_id()).0.entry(slot).or_default();
        h.lvl = if fall { lin.max(h.lvl * 10f32.powf(-dt)) } else { lin };
        let db = to_db(h.lvl);
        if db >= h.db {
            h.db = db;
            h.t = now;
        } else if now - h.t > HOLD_SECS {
            h.db = (h.db - HOLD_FALL * dt).max(db);
        }
        if h.lvl >= 1.0 {
            h.clip = true;
        }
        if clicked {
            h.db = -120.0;
            h.clip = false;
        }
        *h
    });

    // keep animating while something is moving
    if h.lvl > 1e-4 || to_db(h.lvl) < h.db - 0.05 && h.db > -100.0 {
        ctx.request_repaint();
    }

    let p = ui.painter_at(rect);
    let bar = Rect::from_min_max(pos2(rect.left(), rect.top() + CLIP_H), rect.right_bottom());
    p.rect_filled(bar, 3.0, tint(INK, 14));
    let y = |db: f32| bar.bottom() - db_norm(db) * bar.height();
    let lvl_db = to_db(h.lvl);
    for (lo, hi, col) in [(FLOOR_DB, -12.0, GREEN), (-12.0, -3.0, AMBER), (-3.0, 0.0, RED)] {
        let top = lvl_db.min(hi);
        if top > lo {
            p.rect_filled(Rect::from_min_max(pos2(bar.left(), y(top)), pos2(bar.right(), y(lo))), 0.0, col);
        }
    }
    if h.db > FLOOR_DB {
        let yy = y(h.db).clamp(bar.top() + 1.0, bar.bottom());
        p.line_segment([pos2(bar.left(), yy), pos2(bar.right(), yy)], stroke(2.0, if h.db >= 0.0 { RED } else { INK }));
    }
    let led = Rect::from_min_size(rect.min, vec2(rect.width(), 4.0));
    p.rect_filled(led, 2.0, if h.clip { RED } else { tint(INK, 14) });

    let tip = if h.db < -90.0 { "Peak: -∞ dBFS".to_string() } else { format!("Peak: {:+.1} dBFS - click to reset", h.db) };
    resp.on_hover_text(tip)
}

/// A few dB graduations, to place next to a `meter` of the same height.
pub fn scale(ui: &mut egui::Ui, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter_at(rect);
    let (top, bh) = (rect.top() + CLIP_H, rect.height() - CLIP_H);
    for db in [0.0f32, -12.0, -24.0, -48.0] {
        let y = top + (1.0 - db_norm(db)) * bh;
        let al = if db == 0.0 { Align2::LEFT_TOP } else { Align2::LEFT_CENTER };
        p.text(pos2(rect.left() + 2.0, y), al, format!("{}", db as i32), FontId::proportional(9.0), GREY);
    }
}