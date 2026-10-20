//! MCP (Model Context Protocol) server over stdio.
//!
//! Start the app with `opendembow --mcp` (an MCP client such as Claude Desktop launches it as a
//! subprocess). stdout is reserved for JSON-RPC, so nothing else may print to it.
//! Requests are read on a background thread and executed on the UI thread (`App::mcp_poll`),
//! so the window always reflects what the assistant does and undo/redo keeps working.

use crate::instruments::Instrument;
use crate::project::*;
use crate::App;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub struct Call {
    pub tool: String,
    pub args: Value,
    pub reply: Sender<Result<Value, String>>,
}

pub fn start(ctx: eframe::egui::Context) -> Receiver<Call> {
    let (tx, rx) = channel();
    std::thread::spawn(move || serve(tx, ctx));
    rx
}

fn tool(name: &str, desc: &str, props: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": desc,
            "inputSchema": { "type": "object", "properties": props, "required": required } })
}

fn tools() -> Value {
    let fx_names: Vec<&str> = crate::effects::FxSettings::default().params().into_iter().map(|(k, _)| k).collect();
    let note = json!({ "type": "object", "properties": {
        "start": { "type": "number", "description": "beats from the start of the clip" },
        "length": { "type": "number", "description": "beats" },
        "key": { "type": "integer", "description": "MIDI note 0-127, 60 = middle C. On Drum Kit / 808 Kit tracks the key picks the drum: 36 kick, 37 rim, 38 snare, 39 clap, 40 tight snare, 41/43/45/47/48/50 toms, 42 closed hat, 44 pedal hat, 46 open hat, 49/52/55/57 crash, 51/53/59 ride, 54/58 shaker, 56 cowbell (notes outside 36-59 wrap around)" },
        "velocity": { "type": "number", "description": "0.0-1.0, default 0.8" } },
        "required": ["start", "length", "key"] });
    json!([
        tool("get_project", "Tempo, transport state, tracks, clips, mixer and effect settings.", json!({}), &[]),
        tool("list_instruments", "All built-in instruments with their category (including the Drum Kit and 808 Kit).", json!({}), &[]),
        tool("get_notes", "Notes of a MIDI clip.", json!({ "track": {"type":"integer"}, "clip": {"type":"integer"} }), &["track", "clip"]),
        tool("set_tempo", "Set the tempo (40-240 BPM).", json!({ "bpm": {"type":"number"} }), &["bpm"]),
        tool("add_track", "Add an instrument track (name from list_instruments, case-insensitive).", json!({ "instrument": {"type":"string"} }), &["instrument"]),
        tool("delete_track", "Delete a track.", json!({ "track": {"type":"integer"} }), &["track"]),
        tool("add_midi_clip", "Create an empty MIDI clip on an instrument track.", json!({
            "track": {"type":"integer"}, "start": {"type":"number","description":"beats on the timeline"},
            "length": {"type":"number","description":"beats, default 4"}, "name": {"type":"string"} }), &["track"]),
        tool("add_notes", "Add notes to a MIDI clip (the clip grows if needed). For drums use the General MIDI drum keys described in the note schema.", json!({
            "track": {"type":"integer"}, "clip": {"type":"integer"}, "notes": {"type":"array","items": note} }), &["track", "clip", "notes"]),
        tool("delete_clip", "Delete a clip.", json!({ "track": {"type":"integer"}, "clip": {"type":"integer"} }), &["track", "clip"]),
        tool("split_clip", "Split a clip at an absolute timeline position (beats).", json!({
            "track": {"type":"integer"}, "clip": {"type":"integer"}, "beat": {"type":"number"} }), &["track", "clip", "beat"]),
        tool("set_track", "Mixer settings of a track (any of volume 0-1, pan -1..1, mute, solo).", json!({
            "track": {"type":"integer"}, "volume": {"type":"number"}, "pan": {"type":"number"},
            "mute": {"type":"boolean"}, "solo": {"type":"boolean"} }), &["track"]),
        tool("set_effect", &format!("Set effect parameters of a track. Keys: {}. Booleans: true/false.", fx_names.join(", ")), json!({
            "track": {"type":"integer"}, "params": {"type":"object","description":"e.g. {\"eq.on\":true,\"eq.low\":3,\"reverb.mix\":0.3}"} }), &["track", "params"]),
        tool("transport", "Control playback.", json!({ "action": {"type":"string","enum":["play","pause","stop","rewind"]} }), &["action"]),
        tool("undo", "Undo the last change.", json!({}), &[]),
        tool("redo", "Redo.", json!({}), &[]),
    ])
}

fn tool_result(text: &str, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

fn serve(tx: Sender<Call>, ctx: eframe::egui::Context) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let Some(id) = msg.get("id").cloned() else { continue }; // notifications need no answer
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let result: Result<Value, (i64, String)> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or("2024-11-05"),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "opendembow", "version": "0.6" }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (rtx, rrx) = channel();
                if tx.send(Call { tool: name, args, reply: rtx }).is_err() {
                    break;
                }
                ctx.request_repaint();
                Ok(match rrx.recv_timeout(Duration::from_secs(10)) {
                    Ok(Ok(v)) => tool_result(&v.to_string(), false),
                    Ok(Err(e)) => tool_result(&e, true),
                    Err(_) => tool_result("OpenDembow did not answer in time", true),
                })
            }
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        let out = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err((code, m)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": m } }),
        };
        let mut so = std::io::stdout().lock();
        let _ = writeln!(so, "{out}");
        let _ = so.flush();
    }
}

// ---------------------------------------------------------------------------
// Execution on the UI thread
// ---------------------------------------------------------------------------

fn arg_f(a: &Value, k: &str) -> Option<f64> {
    a.get(k)?.as_f64()
}

fn arg_i(a: &Value, k: &str) -> Result<usize, String> {
    a.get(k).and_then(Value::as_u64).map(|x| x as usize).ok_or_else(|| format!("missing integer argument '{k}'"))
}

impl App {
    pub fn mcp_poll(&mut self) {
        let calls: Vec<Call> = match &self.mcp {
            Some(rx) => rx.try_iter().collect(),
            None => return,
        };
        for c in calls {
            let r = self.mcp_exec(&c.tool, &c.args);
            let _ = c.reply.send(r);
        }
    }

    fn mcp_track(&self, a: &Value) -> Result<usize, String> {
        let t = arg_i(a, "track")?;
        if t < self.project.tracks.len() {
            Ok(t)
        } else {
            Err(format!("track {t} does not exist (there are {} tracks)", self.project.tracks.len()))
        }
    }

    fn mcp_clip(&self, a: &Value) -> Result<(usize, usize), String> {
        let t = self.mcp_track(a)?;
        let c = arg_i(a, "clip")?;
        if c < self.project.tracks[t].clips.len() {
            Ok((t, c))
        } else {
            Err(format!("clip {c} does not exist on track {t}"))
        }
    }

    fn mcp_snapshot(&self) -> Value {
        let p = &self.project;
        let tracks: Vec<Value> = p
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let clips: Vec<Value> = t
                    .clips
                    .iter()
                    .enumerate()
                    .map(|(j, c)| {
                        json!({ "index": j, "name": c.name, "start": c.start, "length": c.len_beats(p.bpm, p.sr),
                                "type": match &c.kind { ClipKind::Midi(n) => format!("midi ({} notes)", n.len()), ClipKind::Audio(_) => "audio".to_string() } })
                    })
                    .collect();
                let fx: serde_json::Map<String, Value> = t.fx.params().into_iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
                json!({ "index": i, "name": t.name,
                        "type": match t.kind { TrackKind::Instrument(inst) => format!("instrument: {}", inst.name()), TrackKind::Audio => "audio".to_string() },
                        "volume": t.volume, "pan": t.pan, "mute": t.mute, "solo": t.solo, "effects": fx, "clips": clips })
            })
            .collect();
        json!({ "bpm": p.bpm, "playing": self.playing, "position_beats": self.pos_beats, "tracks": tracks })
    }

    pub fn mcp_exec(&mut self, tool: &str, a: &Value) -> Result<Value, String> {
        match tool {
            "get_project" => Ok(self.mcp_snapshot()),
            "list_instruments" => Ok(Value::Array(
                Instrument::ALL.iter().map(|i| json!({ "name": i.name(), "category": i.category().name() })).collect(),
            )),
            "get_notes" => {
                let (t, c) = self.mcp_clip(a)?;
                let notes = self.notes_ref(t, c).ok_or("not a MIDI clip")?;
                Ok(Value::Array(
                    notes.iter().map(|n| json!({ "start": n.start, "length": n.len, "key": n.key, "velocity": n.vel })).collect(),
                ))
            }
            "set_tempo" => {
                let b = arg_f(a, "bpm").ok_or("bpm required")?.clamp(40.0, 240.0);
                self.checkpoint();
                let pos = self.pos_beats;
                self.project.bpm = b;
                self.push();
                self.engine.set_pos(pos, b, self.project.sr);
                Ok(json!({ "bpm": b }))
            }
            "add_track" => {
                let want = a.get("instrument").and_then(Value::as_str).ok_or("instrument required")?.to_lowercase();
                let inst = Instrument::ALL
                    .iter()
                    .copied()
                    .find(|i| i.name().to_lowercase() == want)
                    .or_else(|| Instrument::ALL.iter().copied().find(|i| i.name().to_lowercase().contains(&want)))
                    .ok_or_else(|| format!("unknown instrument '{want}', see list_instruments"))?;
                self.add_track(inst);
                Ok(json!({ "track": self.project.tracks.len() - 1, "instrument": inst.name() }))
            }
            "delete_track" => {
                let t = self.mcp_track(a)?;
                self.delete_track(t);
                Ok(json!({ "deleted": t }))
            }
            "add_midi_clip" => {
                let t = self.mcp_track(a)?;
                if !matches!(self.project.tracks[t].kind, TrackKind::Instrument(_)) {
                    return Err("track is not an instrument track".into());
                }
                let start = arg_f(a, "start").unwrap_or(0.0).max(0.0);
                let len = arg_f(a, "length").unwrap_or(4.0).max(0.25);
                let name = a.get("name").and_then(Value::as_str).unwrap_or("Clip").to_string();
                self.checkpoint();
                self.project.tracks[t].clips.push(Clip { name, start, len, kind: ClipKind::Midi(Vec::new()) });
                let c = self.project.tracks[t].clips.len() - 1;
                self.select_clip(t, c);
                self.push();
                Ok(json!({ "track": t, "clip": c }))
            }
            "add_notes" => {
                let (t, c) = self.mcp_clip(a)?;
                if self.notes_ref(t, c).is_none() {
                    return Err("not a MIDI clip".into());
                }
                let arr = a.get("notes").and_then(Value::as_array).ok_or("notes array required")?;
                let mut notes = Vec::new();
                for (i, n) in arr.iter().enumerate() {
                    let start = arg_f(n, "start").ok_or_else(|| format!("note {i}: start required"))?.max(0.0);
                    let len = arg_f(n, "length").ok_or_else(|| format!("note {i}: length required"))?.max(0.0625);
                    let key = arg_f(n, "key").ok_or_else(|| format!("note {i}: key required"))?.clamp(0.0, 127.0) as u8;
                    let vel = arg_f(n, "velocity").unwrap_or(0.8).clamp(0.0, 1.0) as f32;
                    notes.push(Note { start, len, key, vel });
                }
                let added = notes.len();
                self.checkpoint();
                if let Some(v) = self.notes_mut(t, c) {
                    v.extend(notes);
                }
                self.fit_clip(t, c);
                self.push();
                Ok(json!({ "added": added }))
            }
            "delete_clip" => {
                let (t, c) = self.mcp_clip(a)?;
                self.checkpoint();
                self.project.tracks[t].clips.remove(c);
                self.fix_selection();
                self.push();
                Ok(json!({ "deleted": c }))
            }
            "split_clip" => {
                let (t, c) = self.mcp_clip(a)?;
                let beat = arg_f(a, "beat").ok_or("beat required")?;
                let n = self.split_clip_at(t, c, beat)?;
                Ok(json!({ "left": c, "right": n }))
            }
            "set_track" => {
                let t = self.mcp_track(a)?;
                self.checkpoint();
                let tr = &mut self.project.tracks[t];
                if let Some(v) = arg_f(a, "volume") {
                    tr.volume = v.clamp(0.0, 1.0) as f32;
                }
                if let Some(v) = arg_f(a, "pan") {
                    tr.pan = v.clamp(-1.0, 1.0) as f32;
                }
                if let Some(v) = a.get("mute").and_then(Value::as_bool) {
                    tr.mute = v;
                }
                if let Some(v) = a.get("solo").and_then(Value::as_bool) {
                    tr.solo = v;
                }
                self.push();
                Ok(self.mcp_snapshot())
            }
            "set_effect" => {
                let t = self.mcp_track(a)?;
                let obj = a.get("params").and_then(Value::as_object).ok_or("params object required")?;
                let mut fx = self.project.tracks[t].fx.clone();
                for (k, v) in obj {
                    let x = v.as_f64().or_else(|| v.as_bool().map(|b| if b { 1.0 } else { 0.0 })).ok_or_else(|| format!("'{k}' must be a number or boolean"))?;
                    if !fx.set_param(k, x) {
                        return Err(format!("unknown effect parameter '{k}'"));
                    }
                }
                self.checkpoint();
                self.project.tracks[t].fx = fx;
                self.push();
                Ok(json!({ "ok": true }))
            }
            "transport" => {
                match a.get("action").and_then(Value::as_str).unwrap_or("") {
                    "play" => {
                        if !self.playing {
                            self.engine.toggle_play();
                        }
                    }
                    "pause" => {
                        if self.playing {
                            self.engine.toggle_play();
                        }
                    }
                    "stop" => {
                        if self.playing {
                            self.engine.toggle_play();
                        }
                        self.rewind();
                    }
                    "rewind" => self.rewind(),
                    other => return Err(format!("unknown action '{other}'")),
                }
                Ok(json!({ "ok": true }))
            }
            "undo" => {
                self.undo();
                Ok(json!({ "ok": true }))
            }
            "redo" => {
                self.redo();
                Ok(json!({ "ok": true }))
            }
            other => Err(format!("unknown tool '{other}'")),
        }
    }
}