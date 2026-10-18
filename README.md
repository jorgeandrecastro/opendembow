# OpenDembow 0.3

![OpenDembow Interface](images/image1.png)

A modern, elegant digital audio workstation written in Rust. Clean light interface, one accent colour,
rounded everything.

## Run
``` bash
cargo run --release
```   

Requires stable Rust (https://rustup.rs). On Linux: `sudo apt install libasound2-dev pkg-config`.

## Features
- Always-visible on-screen piano keyboard (4 octaves) + computer-keyboard playing (A W S E D F T G Y H U J K O L)
- Drag instruments from the browser onto the timeline (new track, or drop on a track to swap its instrument)
- Delete tracks with the × on the track header, the Track menu, or right-click
- Multi-track arrangement, piano roll, 8 built-in instruments, mixer strip per track
- Audio tracks: import WAV (drag & drop or menu)
- Menu bar (File / Edit / Track / Help) and a "?" help button (F1)
- Save / open projects (`.dembow`), undo / redo
- Export: mixdown WAV, stems (one WAV per track), MIDI file

## Shortcuts
Press F1 inside the app.

## Layout
- `engine.rs` real-time mixer, cpal output, offline bounce
- `instruments.rs` synth voices - `project.rs` data model - `project_io.rs` save/load + MIDI export
- `arrangement.rs`, `pianoroll.rs`, `keyboard.rs`, `chrome.rs`, `theme.rs` interface
- `files.rs` file commands - `audio_io.rs` WAV I/O

## Roadmap
Audio recording, MIDI input, effects (EQ, reverb, compressor), automation, dark theme, sample-based instruments.

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3) - see the [LICENSE](LICENSE) file for details.

# Copyright (C) 2026 Jorge Andre Castro