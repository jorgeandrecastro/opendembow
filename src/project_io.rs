//! Project save / load (plain text + WAV side files) and Standard MIDI File export.

use crate::effects::FxSettings;
use crate::instruments::Instrument;
use crate::project::*;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

fn write_wav_f32(path: &Path, data: &[f32], sr: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: sr, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in data {
        w.write_sample(s).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/// Saves `<name>.dembow`; audio clips are written next to it as `<name>.audioN.wav`.
///
/// Effects are stored as `fx<TAB>effect.param<TAB>value` lines right after their `track` line
/// (only when the track's effects differ from the defaults).
pub fn save_project(p: &Project, path: &Path) -> Result<(), String> {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "project".into());
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut out = String::new();
    let mut n_audio = 0;
    let _ = writeln!(out, "OPENDEMBOW\t1");
    let _ = writeln!(out, "bpm\t{}", p.bpm);
    for t in &p.tracks {
        let kind = match t.kind {
            TrackKind::Instrument(i) => i.name().to_string(),
            TrackKind::Audio => "audio".to_string(),
        };
        let _ = writeln!(out, "track\t{}\t{}\t{}\t{}\t{}\t{}\t{}", kind, t.color, t.volume, t.pan, t.mute as u8, t.solo as u8, clean(&t.name));
        if t.fx != FxSettings::default() {
            for (key, v) in t.fx.params() {
                let _ = writeln!(out, "fx\t{}\t{}", key, v as f32);
            }
        }
        for c in &t.clips {
            match &c.kind {
                ClipKind::Midi(notes) => {
                    let _ = writeln!(out, "clip\tmidi\t{}\t{}\t{}", c.start, c.len, clean(&c.name));
                    for n in notes {
                        let _ = writeln!(out, "note\t{}\t{}\t{}\t{}", n.start, n.len, n.key, n.vel);
                    }
                }
                ClipKind::Audio(data) => {
                    let file = format!("{stem}.audio{n_audio}.wav");
                    n_audio += 1;
                    write_wav_f32(&dir.join(&file), data, p.sr as u32)?;
                    let _ = writeln!(out, "clip\taudio\t{}\t{}\t{}", c.start, file, clean(&c.name));
                }
            }
        }
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}

pub fn load_project(path: &Path, sr: f32) -> Result<Project, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut lines = text.lines();
    if !lines.next().unwrap_or("").starts_with("OPENDEMBOW") {
        return Err("Not an OpenDembow project".into());
    }
    let mut p = Project { sr, bpm: 100.0, tracks: Vec::new() };
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        match f.first().copied() {
            Some("bpm") => p.bpm = f.get(1).and_then(|s| s.parse().ok()).unwrap_or(100.0),
            Some("track") if f.len() >= 8 => {
                let kind = if f[1] == "audio" {
                    TrackKind::Audio
                } else {
                    TrackKind::Instrument(Instrument::ALL.iter().copied().find(|i| i.name() == f[1]).unwrap_or(Instrument::Piano))
                };
                let mut t = Track::new(f[7], kind, f[2].parse().unwrap_or(0));
                t.volume = f[3].parse().unwrap_or(0.8);
                t.pan = f[4].parse().unwrap_or(0.0);
                t.mute = f[5] == "1";
                t.solo = f[6] == "1";
                p.tracks.push(t);
            }
            // Effect parameter of the last track. Old projects have no such lines (defaults stay);
            // unknown names are ignored by `set_param`, values are clamped there.
            Some("fx") if f.len() >= 3 => {
                if let (Some(t), Ok(v)) = (p.tracks.last_mut(), f[2].parse::<f64>()) {
                    if v.is_finite() {
                        t.fx.set_param(f[1], v);
                    }
                }
            }
            Some("clip") if f.len() >= 5 => {
                if let Some(t) = p.tracks.last_mut() {
                    let start = f[2].parse().unwrap_or(0.0);
                    if f[1] == "midi" {
                        t.clips.push(Clip { name: f[4].into(), start, len: f[3].parse().unwrap_or(4.0), kind: ClipKind::Midi(Vec::new()) });
                    } else {
                        let data = crate::audio_io::load_wav(&dir.join(f[3]), sr)?;
                        t.clips.push(Clip { name: f[4].into(), start, len: 0.0, kind: ClipKind::Audio(Arc::new(data)) });
                    }
                }
            }
            Some("note") if f.len() >= 5 => {
                if let Some(Clip { kind: ClipKind::Midi(n), .. }) = p.tracks.last_mut().and_then(|t| t.clips.last_mut()) {
                    n.push(Note {
                        start: f[1].parse().unwrap_or(0.0),
                        len: f[2].parse().unwrap_or(0.25),
                        key: f[3].parse().unwrap_or(60),
                        vel: f[4].parse().unwrap_or(0.8),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(p)
}

fn vlq(mut v: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 5];
    let mut i = 0;
    buf[0] = (v & 0x7f) as u8;
    v >>= 7;
    while v > 0 {
        i += 1;
        buf[i] = ((v & 0x7f) | 0x80) as u8;
        v >>= 7;
    }
    for j in (0..=i).rev() {
        out.push(buf[j]);
    }
}

/// Exports every instrument track as a track of a type-1 Standard MIDI File.
pub fn export_midi(p: &Project, path: &Path) -> Result<(), String> {
    const PPQ: f64 = 480.0;
    let mut tracks: Vec<Vec<u8>> = Vec::new();
    let mut t0 = Vec::new();
    let us = (60_000_000.0 / p.bpm) as u32;
    vlq(0, &mut t0);
    t0.extend([0xFFu8, 0x51, 0x03, (us >> 16) as u8, (us >> 8) as u8, us as u8]);
    vlq(0, &mut t0);
    t0.extend([0xFFu8, 0x2F, 0x00]);
    tracks.push(t0);
    for (ti, t) in p.tracks.iter().enumerate() {
        let TrackKind::Instrument(_) = t.kind else { continue };
        let mut ev: Vec<(u32, u8, u8, u8)> = Vec::new();
        for c in &t.clips {
            if let ClipKind::Midi(notes) = &c.kind {
                for n in notes {
                    let s = ((c.start + n.start) * PPQ).round() as u32;
                    let e = ((c.start + n.start + n.len) * PPQ).round() as u32;
                    ev.push((s, 1, n.key, (n.vel * 127.0).clamp(1.0, 127.0) as u8));
                    ev.push((e.max(s + 1), 0, n.key, 0));
                }
            }
        }
        if ev.is_empty() {
            continue;
        }
        ev.sort_by_key(|e| (e.0, e.1));
        let ch = (ti % 16) as u8;
        let mut tr = Vec::new();
        let name: Vec<u8> = t.name.bytes().take(100).collect();
        vlq(0, &mut tr);
        tr.extend([0xFFu8, 0x03, name.len() as u8]);
        tr.extend(name);
        let mut last = 0u32;
        for (tick, kind, key, vel) in ev {
            vlq(tick - last, &mut tr);
            last = tick;
            tr.push(if kind == 1 { 0x90 | ch } else { 0x80 | ch });
            tr.push(key);
            tr.push(vel);
        }
        vlq(0, &mut tr);
        tr.extend([0xFFu8, 0x2F, 0x00]);
        tracks.push(tr);
    }
    if tracks.len() == 1 {
        return Err("There are no MIDI notes to export".into());
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend(b"MThd");
    out.extend(6u32.to_be_bytes());
    out.extend(1u16.to_be_bytes());
    out.extend((tracks.len() as u16).to_be_bytes());
    out.extend(480u16.to_be_bytes());
    for tr in tracks {
        out.extend(b"MTrk");
        out.extend((tr.len() as u32).to_be_bytes());
        out.extend(tr);
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}