//! MIDI keyboard / controller input (midir). Events are queued and handled on the UI thread.

use midir::{MidiInput, MidiInputConnection};
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum MidiMsg {
    On(u8, f32),
    Off(u8),
}

pub struct MidiIn {
    ctx: eframe::egui::Context,
    tx: Sender<MidiMsg>,
    rx: Receiver<MidiMsg>,
    conn: Option<MidiInputConnection<()>>,
    pub ports: Vec<String>,
    pub current: Option<String>,
}

fn parse(m: &[u8]) -> Option<MidiMsg> {
    if m.len() < 3 {
        return None;
    }
    match m[0] & 0xF0 {
        0x90 if m[2] > 0 => Some(MidiMsg::On(m[1], m[2] as f32 / 127.0)),
        0x90 | 0x80 => Some(MidiMsg::Off(m[1])),
        _ => None,
    }
}

impl MidiIn {
    /// Lists the devices and connects to the first one, if any.
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (tx, rx) = channel();
        let mut m = Self { ctx, tx, rx, conn: None, ports: Vec::new(), current: None };
        m.refresh();
        if let Some(first) = m.ports.first().cloned() {
            let _ = m.connect(&first);
        }
        m
    }

    pub fn refresh(&mut self) {
        self.ports = MidiInput::new("OpenDembow list")
            .map(|m| m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect::<Vec<_>>())
            .unwrap_or_default();
    }

    pub fn connect(&mut self, name: &str) -> Result<(), String> {
        self.conn = None;
        self.current = None;
        let input = MidiInput::new("OpenDembow").map_err(|e| e.to_string())?;
        let port = input
            .ports()
            .into_iter()
            .find(|p| input.port_name(p).map_or(false, |n| n == name))
            .ok_or("device not found")?;
        let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
        let conn = input
            .connect(
                &port,
                "opendembow-in",
                move |_, msg, _| {
                    if let Some(m) = parse(msg) {
                        let _ = tx.send(m);
                        ctx.request_repaint();
                    }
                },
                (),
            )
            .map_err(|e| e.to_string())?;
        self.conn = Some(conn);
        self.current = Some(name.to_string());
        Ok(())
    }

    pub fn drain(&self) -> Vec<MidiMsg> {
        self.rx.try_iter().collect()
    }
}