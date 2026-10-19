# OpenDembow 0.5

![OpenDembow Interface](images/image1.png)
![OpenDembow Interface](images/image2.png)

A modern, elegant digital audio workstation written in Rust. Clean light interface, one accent colour,
rounded everything.

## Run
``` bash
cargo run --release
```

Requires stable Rust (https://rustup.rs). On Linux: `sudo apt install libasound2-dev pkg-config`.

Add the MCP dependency to `Cargo.toml`:

```toml
serde_json = "1"
```

## Features
- **24 built-in synthesized instruments** in 8 categories: Keys, Organs, Leads, Basses, Plucked, Pads & Strings, Mallets & Bells, Brass & Winds
- Ableton-style instrument browser: search box, category filters, instruments grouped by category, and the instrument of the selected track highlighted
- Drag instruments from the browser onto the timeline (new track, or drop on a track to swap its instrument); the list keeps scrolling while you drag
- Metronome click: toggle with the transport button or `M`, accented first beat, volume, beats per bar, latency compensation (right-click)
- Always-visible on-screen piano keyboard (4 octaves) + computer-keyboard playing (A W S E D F T G Y H U J K O L)
- Delete tracks with the × on the track header, the Track menu, or right-click
- Multi-track arrangement, piano roll
- **Mixer & Effects tab** (new): one strip per track (volume, pan, mute, solo, level meter) and, for the selected track, a 3-band **equalizer**, a **compressor**, a stereo **reverb** and a ping-pong **delay**
- **Clip editing** (new): split at the playhead, trim start / end, cut / copy / paste clips and notes, clip length, normalize and reverse audio
- **MCP server** (new): let an AI assistant build and mix your song
- Audio tracks: import WAV (drag & drop or menu)
- Menu bar (File / Edit / Track / Help) and a "?" help button (F1)
- Save / open projects (`.dembow`), undo / redo (Edit menu, Ctrl+Z)
- Export: mixdown WAV, stems (one WAV per track), MIDI file

## Editing shortcuts
| Key | Action |
|---|---|
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

## Layout
- `engine.rs` real-time mixer, cpal output, offline bounce
- `instruments.rs` synth voices and instrument categories - `project.rs` data model - `project_io.rs` save/load + MIDI export
- `arrangement.rs`, `pianoroll.rs`, `keyboard.rs`, `chrome.rs` (menus, instrument browser), `theme.rs` interface
- `effects.rs` effect settings, DSP (`FxProcessor`) and the Mixer & Effects panel
- `edit.rs` split / trim / clipboard / audio tools and the bottom-panel tabs
- `mcp.rs` MCP server
- `metronome.rs` metronome click (own audio stream) and its transport button
- `files.rs` file commands - `audio_io.rs` WAV I/O

## Adding an instrument
1. Add a variant to `Instrument` in `instruments.rs` and to `Instrument::ALL` (append at the end to keep old projects compatible).
2. Fill in `name`, `blurb`, `category` and `release_rate`.
3. Write its sound in `Voice::next`.

## Roadmap
Count-in before recording, audio recording, MIDI input, automation, dark theme, sample-based instruments,
instrument preview on hover, favourites, drag-to-trim clip edges, send effects.

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3) - see the [LICENSE](LICENSE) file for details.

# Copyright (C) 2026 Jorge Andre Castro