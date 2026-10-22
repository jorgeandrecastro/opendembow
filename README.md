# OpenDembow 0.7

![OpenDembow Interface](images/image1.png)
![OpenDembow Interface](images/image2.png)

A modern, elegant digital audio workstation written in Rust. Clean light interface, one accent colour,
rounded everything.

## Run
``` bash
cargo run --release
```

Requires stable Rust (https://rustup.rs). On Linux: `sudo apt install libasound2-dev pkg-config` (used for both audio output and MIDI input).

Dependencies to add to `Cargo.toml`:

```toml
serde_json = "1"   # MCP server
midir = "0.10"     # MIDI input (new in 0.7)
```

## Features
- **33 built-in synthesized instruments** in 9 categories: Keys, Organs, Leads, Basses, Plucked, Pads & Strings, Mallets & Bells, Brass & Winds, Drums & Percussion
- **Recording with count-in** (new in 0.7): press `R` or the red button, hear a 4-beat count-in, then play - the notes you play become a new MIDI clip (undo with Ctrl+Z)
- **MIDI input** (new in 0.7): plug in a MIDI keyboard or controller and pick it in Track > MIDI Input; it plays the selected track and records like the computer keyboard
- **Drum names in the piano roll** (new in 0.7): on a drum track every row shows the name of its pad (Kick, Snare, Closed hat, Conga slap...), following the map of the kit
- **Favourite instruments** (new in 0.7): click the star on an instrument, then use the ★ Favourites filter in the browser; favourites are saved in `~/.opendembow_favorites`
- **Drums** (0.6): five kits - **Drum Kit** (acoustic style), **808 Kit** (electronic), **Trap Kit**, **Reggaeton Kit** (dembow) and **Latin Percussion** (congas, bongos, timbales, shakers) - played with a General MIDI style note map (see below)
- **Instruments added in 0.6**: 808 Bass, Fingered Bass, Electric Guitar, Synth Pluck
- Ableton-style instrument browser: search box, category filters, favourites, instruments grouped by category, and the instrument of the selected track highlighted
- Drag instruments from the browser onto the timeline (new track, or drop on a track to swap its instrument); the list keeps scrolling while you drag
- Metronome click: toggle with the transport button or `M`, accented first beat, volume, beats per bar, latency compensation (right-click)
- Always-visible on-screen piano keyboard (4 octaves) + computer-keyboard playing (A W S E D F T G Y H U J K O L)
- Delete tracks with the × on the track header, the Track menu, or right-click
- Multi-track arrangement, piano roll
- **Mixer & Effects tab**: one strip per track (volume, pan, mute, solo, level meter) and, for the selected track, a 3-band **equalizer**, a **compressor**, a stereo **reverb** and a ping-pong **delay**; effects run before volume / pan, are included in the mixdown, the stems and the live output, and are saved in the project
- **Clip editing**: split at the playhead, trim start / end, cut / copy / paste clips and notes, clip length, normalize and reverse audio
- **MCP server**: let an AI assistant build and mix your song (it knows the drum map too)
- Audio tracks: import WAV (drag & drop or menu)
- Menu bar (File / Edit / Track / Help) and a "?" help button (F1)
- Save / open projects (`.dembow`), undo / redo (Edit menu, Ctrl+Z)
- Export: mixdown WAV, stems (one WAV per track), MIDI file

## Recording and MIDI input
1. Select an instrument track (click its header, or drop an instrument on the timeline).
2. Optional: connect a MIDI keyboard and choose it in **Track > MIDI Input** (the first device found is connected at start-up; **Refresh devices** rescans).
3. Press `R` or the red ● button. A 4-beat count-in plays (switch it off in **Track > Count-in before recording**; `Esc` cancels it).
4. Play: on the on-screen keyboard, your computer keys or the MIDI controller. The rest of the song plays along.
5. Press `R` or `Space` to stop. A new clip called "Recording" appears on the track, starting on a bar line.

Notes: the velocity of a MIDI controller is stored in the clip but not yet used by the live sound; there is no quantization yet (edit with the piano roll grid).

## Drums
Add a drum kit track (**Drum Kit**, **808 Kit**, **Trap Kit** or **Reggaeton Kit**) and play it with the on-screen keyboard, your computer keys or the piano roll.
Notes are folded into two octaves (MIDI 36-59), so every key of the keyboard plays a drum. The piano roll shows these names next to each row:

| MIDI key | Note | Sound |
|---|---|---|
| 36 | C2 | Kick |
| 37 | C#2 | Rim / side stick |
| 38 / 40 | D2 / E2 | Snare / tight snare |
| 39 | D#2 | Clap |
| 41 / 43 / 45 / 47 / 48 / 50 | F2 ... D3 | Toms (low to high) |
| 42 / 44 | F#2 / G#2 | Closed hi-hat / pedal hi-hat |
| 46 | A#2 | Open hi-hat |
| 49 / 52 / 55 / 57 | C#3 / E3 / G#3 / A3 | Crash cymbals |
| 51 / 53 / 59 | D#3 / F3 / B3 | Ride / ride bell |
| 54 / 58 | F#3 / A#3 | Tambourine-style shaker / shaker |
| 56 | G#3 | Cowbell |

Notes below 36 or above 59 wrap around (60 is a kick again); the piano roll shades those rows. Drum hits end by themselves, so note length does not cut them.

All four kits share this map, so a pattern keeps working when you swap kits. What changes is the sound:
- **Trap Kit**: hard kick with a sub tail, snappy snare, tight hats, long 808 toms; key 59 (B3) is a long **808 boom**.
- **Reggaeton Kit**: boxy dembow kick, bright snare with a short gate, short hats, the toms are **congas**.

**Latin Percussion** has its own map (same two octaves):

| MIDI key | Sound | MIDI key | Sound |
|---|---|---|---|
| 36 / 38 | Conga low / high | 47 | Guiro long |
| 37 / 39 | Conga mute / slap | 48 | Claves |
| 40 / 41 | Bongo low / high | 49 | Maracas |
| 42 / 43 | Timbale low / high | 50 / 58 | Shaker / long shaker |
| 44 | Paila (timbale rim) | 51 | Tambourine |
| 45 | Cowbell | 52 / 53 | Agogo high / low |
| 46 | Guiro short | 54 / 55 | Triangle / cabasa |
| 56 | Woodblock | 57 / 59 | Hand clap / tumba |

The demo song contains a **dembow** beat on the Reggaeton Kit: kick on every beat, snares on the classic 3+3+2 accents, hi-hats on the eighth notes.

## Editing shortcuts
| Key | Action |
|---|---|
| `R` | Record (count-in, then play; `R` or `Space` to stop) |
| `B` | Split the selected clip at the playhead |
| `[` / `]` | Trim clip start / end to the playhead |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste the selected note or clip (pasted at the playhead) |
| `Ctrl+D` | Duplicate clip |

Press F1 inside the app for all shortcuts.

## MCP (Model Context Protocol)
Start the app with `--mcp`: it then speaks MCP over stdio while keeping its window. Example for Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "opendembow": { "command": "/path/to/opendembow", "args": ["--mcp"] }
  }
}
```

Tools: `get_project`, `list_instruments`, `get_notes`, `set_tempo`, `add_track`, `delete_track`, `add_midi_clip`, `add_notes`,
`delete_clip`, `split_clip`, `set_track`, `set_effect`, `transport`, `undo`, `redo`.
Every change goes through the normal undo history and appears live in the window.

## Project files
A `.dembow` project is plain text (tab separated). Each track line is followed by its effect settings
(`fx<TAB>reverb.mix<TAB>0.3` ...) when they differ from the defaults; older projects without effects still open.
Audio clips are stored next to the project as `<name>.audioN.wav`.

Favourite instruments are not part of a project: they are stored per user in `~/.opendembow_favorites` (one instrument name per line).

## Layout
- `engine.rs` real-time mixer (one effect chain per track), cpal output, offline bounce
- `instruments.rs` synth voices and instrument categories - `project.rs` data model - `project_io.rs` save/load + MIDI export
- `drums.rs` Drum Kit and 808 Kit - `drums_urban.rs` Trap Kit and Reggaeton Kit - `latin.rs` Latin Percussion - `extra_voices.rs` 808 Bass, Fingered Bass, Electric Guitar, Synth Pluck
- `drum_names.rs` pad names shown in the piano roll (one map per kit family)
- `arrangement.rs`, `pianoroll.rs`, `keyboard.rs`, `chrome.rs` (menus, instrument browser, favourites), `theme.rs` interface
- `effects.rs` effect settings, DSP (`FxProcessor`) and the Mixer & Effects panel
- `edit.rs` split / trim / clipboard / audio tools and the bottom-panel tabs
- `record.rs` recording with count-in - `midi_in.rs` MIDI keyboard input (midir)
- `favorites.rs` favourite instruments (saved in the home folder)
- `mcp.rs` MCP server
- `metronome.rs` metronome click (own audio stream) and its transport button
- `files.rs` file commands - `audio_io.rs` WAV I/O

## Adding an instrument
1. Add a variant to `Instrument` in `instruments.rs` and to `Instrument::ALL` (append at the end to keep old projects compatible).
2. Fill in `name`, `blurb`, `category` and `release_rate`.
3. Write its sound in `Voice::next` (or in `extra_voices.rs` / `drums.rs` to keep files short).

## Roadmap
Step sequencer, audio recording, recording quantization, MIDI velocity in the live sound, automation, dark theme,
sample-based instruments, instrument preview on hover, drag-to-trim clip edges, send effects.

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3) - see the [LICENSE](LICENSE) file for details.

# Copyright (C) 2026 Jorge Andre Castro