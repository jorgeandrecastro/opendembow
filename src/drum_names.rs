//! Names of the drum pads shown in the piano roll.

use crate::instruments::Category;
use crate::App;

#[derive(Clone, Copy, PartialEq)]
pub enum DrumMap {
    Kit,
    Trap,
    Reggaeton,
    Latin,
}

pub fn name(map: DrumMap, key: i32) -> Option<&'static str> {
    if !(36..=59).contains(&key) {
        return None;
    }
    Some(match map {
        DrumMap::Latin => match key {
            36 => "Conga low", 37 => "Conga mute", 38 => "Conga high", 39 => "Conga slap",
            40 => "Bongo low", 41 => "Bongo high", 42 => "Timbale low", 43 => "Timbale high",
            44 => "Paila", 45 => "Cowbell", 46 => "Guiro short", 47 => "Guiro long",
            48 => "Claves", 49 => "Maracas", 50 => "Shaker", 51 => "Tambourine",
            52 => "Agogo high", 53 => "Agogo low", 54 => "Triangle", 55 => "Cabasa",
            56 => "Woodblock", 57 => "Hand clap", 58 => "Long shaker", _ => "Tumba",
        },
        _ => {
            let tom = |n: &'static str, c: &'static str| if map == DrumMap::Reggaeton { c } else { n };
            match key {
                36 => "Kick", 37 => "Rim", 38 => "Snare", 39 => "Clap", 40 => "Tight snare",
                41 => tom("Tom 1", "Conga 1"), 43 => tom("Tom 2", "Conga 2"),
                45 => tom("Tom 3", "Conga 3"), 47 => tom("Tom 4", "Conga 4"),
                48 => tom("Tom 5", "Conga 5"), 50 => tom("Tom 6", "Conga 6"),
                42 => "Closed hat", 44 => "Pedal hat", 46 => "Open hat",
                49 => "Crash 1", 52 => "Crash 2", 55 => "Crash 3", 57 => "Crash 4",
                51 => "Ride", 53 => "Ride bell", 54 => "Shaker", 56 => "Cowbell", 58 => "Shaker 2",
                _ => if map == DrumMap::Trap { "808 boom" } else { "Ride 2" },
            }
        }
    })
}

impl App {
    /// The drum map of a track, or `None` if it is not a drum track.
    pub fn drum_map(&self, t: usize) -> Option<DrumMap> {
        let crate::project::TrackKind::Instrument(i) = self.project.tracks.get(t)?.kind else { return None };
        if i.category() != Category::Drums {
            return None;
        }
        let n = i.name();
        Some(if n.contains("Latin") {
            DrumMap::Latin
        } else if n.contains("Trap") {
            DrumMap::Trap
        } else if n.contains("Reggaeton") {
            DrumMap::Reggaeton
        } else {
            DrumMap::Kit
        })
    }
}