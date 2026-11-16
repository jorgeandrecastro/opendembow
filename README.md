
# OpenDembow 0.13

![OpenDembow Interface](images/image1.png)

![OpenDembow Interface](images/image4.png)

![OpenDembow Interface](images/image2.png)

![OpenDembow Interface](images/image3.png)

![OpenDembow Interface](images/image5.png)


A modern, elegant digital audio workstation written in Rust. Clean light interface, one accent colour, rounded everything - a calmer layout where you can hide what you do not use, see what you hear, and loops that never slip.

## Run

```bash
cargo run --release
```

Requires stable Rust (https://rustup.rs).

On Linux:

```bash
sudo apt install libasound2-dev pkg-config
```

These dependencies are used for audio output and MIDI input.

Dependencies in `Cargo.toml`:

```toml
eframe = "0.37"
cpal = "0.18"
rfd = "0.18"
serde_json = "1.0.154"
midir = "0.11"
opendembow-core = { version = "0.13.0", path = "crates/opendembow-core" }
```

## Crates and modularity

The repository is a Cargo workspace with two packages:

- **`opendembow-core`** (`crates/opendembow-core`): reusable audio engine,
  synthesized instruments and drums, effects, project model and persistence,
  WAV I/O, and MIDI export. Its modules are public and the crate can be used
  independently from the desktop interface.
- **`opendembow`** (repository root): the desktop application and user
  interface, which depends on `opendembow-core`.

To use the core from another Rust project, add `opendembow-core = "0.13"` to
`Cargo.toml`. The crate README describes its main entry points. To publish the
library package to crates.io, run `cargo publish -p opendembow-core` from the
workspace root.

### Develop, test, and build the core crate

Run these commands from the workspace root:

```bash
# Fast compile check while developing
cargo check -p opendembow-core

# Run the crate's tests
cargo test -p opendembow-core

# Build an optimized library
cargo build -p opendembow-core --release
```

To check or test the entire workspace instead, use `cargo check --workspace`
or `cargo test --workspace`.

## What's new in 0.13

- **Drum-focused piano roll:** Drum tracks now use a dedicated pad grid instead of piano keys, with named rows for each drum sound and a clearer visual layout for programming beats.
- **25 new synthesized instruments:** Upright, Bright, Honky-tonk, Felt and Toy Pianos, Rhodes, Wurlitzer and FM Piano; Rock Organ and Accordion; PWM and Sync Leads; Reese and Wobble Basses; Banjo, Kalimba and Koto; Glass and Analog Pads; Glockenspiel, Xylophone and Steel Drum; Trumpet, Saxophone and Clarinet. These bring the built-in instrument total to 58, with no samples required.

## What's new in 0.12

- **Dependency upgrades:** Migrated to the latest versions of project dependencies and Rust crates.
- **Build fixes:** Fixed compilation errors and compatibility issues introduced by dependency updates.
- **Code cleanup:** Resolved compiler warnings and improved overall code quality.
- **CI/CD improvements:** Updated the GitHub Actions workflows to ensure reliable cross-platform builds on Linux, Windows, and macOS.
- **Linux compatibility:** Switched the Linux CI build environment to Ubuntu 22.04 to ensure compatibility with older GNU C Library (glibc) versions, including Debian 13.
- **Release workflow:** Improved automated release builds and artifact generation for all supported platforms.


## Features

- **58 built-in synthesized instruments** in 9 categories: Keys, Organs, Leads, Basses, Plucked, Pads & Strings, Mallets & Bells, Brass & Winds, Drums & Percussion.
- **25 additional synthesized voices in 0.13**: eight piano variants and new sounds across Organs, Leads, Basses, Plucked, Pads & Strings, Mallets & Bells, and Brass & Winds.
- **Sample-accurate loops** (0.10): the loop runs inside the audio engine, so the playhead never leaves it while the loop is on.
- **Master spectrum analyzer and dBFS meters** (0.9): see the whole mix in real time.
- **Hideable panels and Focus mode** (0.9): instruments, piano roll & mixer and keyboard can be hidden; View menu, panel icons and shortcuts.
- **Recording with count-in** (0.7): press `R` or the red button, hear a 4-beat count-in, then play - the notes you play become a new MIDI clip (undo with Ctrl+Z).
- **MIDI input** (0.7): plug in a MIDI keyboard or controller and pick it in Track > MIDI Input; it plays the selected track and records like the computer keyboard.
- **Drum-focused piano roll** (0.13): drum tracks use a dedicated pad grid instead of piano keys, with each row named for its sound.
- **Favourite instruments** (0.7): click the star on an instrument, then use the ★ Favourites filter in the browser; favourites are saved in `~/.opendembow_favorites`.
- **Drums** (0.6): five kits - **Drum Kit**, **808 Kit**, **Trap Kit**, **Reggaeton Kit** (dembow) and **Latin Percussion** (congas, bongos, timbales, shakers).
- **Instruments added in 0.6**: 808 Bass, Fingered Bass, Electric Guitar, Synth Pluck.
- Ableton-style instrument browser: search box, category filters, favourites, instruments grouped by category.
- Drag instruments from the browser onto the timeline (new track, or drop on a track to swap its instrument).
- Metronome click: toggle with the transport button or `M`, accented first beat, volume, beats per bar, latency compensation (right-click).
- On-screen piano keyboard (4 octaves, can be hidden) + computer-keyboard playing (A W S E D F T G Y H U J K O L).
- Delete tracks with the × on the track header, the Track menu, or right-click.
- Multi-track arrangement, piano roll.
- **Mixer & Effects tab**: one strip per track (volume, pan, mute, solo, dBFS meter), the **Master** card and, for the selected track, a 3-band **equalizer**, a **compressor**, a stereo **reverb** and a ping-pong **delay**; effects run before volume / pan, are included in the mixdown, the stems and the live output, and are saved in the project.
- **Clip editing**: split at the playhead, trim start / end, cut / copy / paste clips and notes, clip length, normalize and reverse audio.
- **MCP server**: let an AI assistant build and mix your song (it knows the drum map too).
- Audio tracks: import WAV (drag & drop or menu).
- Menu bar (File / Edit / Track / View / Help) and a "?" help button (F1).
- Save / open projects (`.dembow`: tracks, clips, effects, volume automation and loop range), undo / redo (Edit menu, Ctrl+Z).
- Export: mixdown WAV, stems (one WAV per track), MIDI file.
- **Volume Automation** (0.7, redesigned in 0.8): click directly in the dedicated bottom lane of any track to add automation points and draw smooth volume curves.
- **Loops** (0.8): Ableton-style loop bar above the timeline.
- **Scale-Aware Piano Roll** (0.7): select a root note and scale (Major, Minor, Pentatonic, Blues) to colour in-scale notes with a subtle green tint, shade out-of-scale notes, and show key labels.

## Master: what is it for?

The Master is the final output: every track is summed into it before it reaches your speakers and the exported WAV.

- **Meters**: they show the total level of the song. If they touch 0 dB the clip LED lights up and the sound may distort - lower the track volumes.
- **Spectrum**: it shows where the energy is, bass on the left and treble on the right. A tall curve on the left means a lot of bass, a low curve on the right means a dull mix.

Click the peak value in the card header (or a meter) to reset the peaks and the clip LED.

## Layout and panels

Three parts of the window can be hidden to give the timeline more room. Each one has a small panel icon; hover it to read *Hide ...*.

| Panel | Icon position | Shortcut |
|---|---|---|
| Instruments (left) | Top right of the browser card | `Ctrl+B` |
| Piano roll & Mixer (bottom) | On the top edge of the panel | `Ctrl+J` |
| On-screen keyboard (bottom) | On the top edge of the panel | `Ctrl+Shift+K` |
| Focus on the timeline / show everything | View menu | `Ctrl+Shift+F` |

A hidden panel leaves a thin strip with a blue icon: click it to bring the panel back. The panel layout is not saved in the project (every panel is visible at start-up).

## Recording and MIDI input

1. Select an instrument track (click its header, or drop an instrument on the timeline).
2. Optional: connect a MIDI keyboard and choose it in **Track > MIDI Input** (the first device found is connected at start-up; **Refresh devices** rescans).
3. Press `R` or the red ● button. A 4-beat count-in plays (switch it off in **Track > Count-in before recording**; `Esc` cancels it).
4. Play: on the on-screen keyboard, your computer keys or the MIDI controller. The rest of the song plays along.
5. Press `R` or `Space` to stop. A new clip called "Recording" appears on the track, starting on a bar line.

Notes: the velocity of a MIDI controller is stored in the clip but not yet used in the live sound; there is no automatic quantization while recording, but you can quantize afterwards in the piano roll (select the notes, `Ctrl+U`).

## Piano roll

Select a MIDI clip (or double-click an empty lane to create one) to edit its notes in the bottom panel. On drum tracks, the piano roll switches to a dedicated grid of named drum-pad rows instead of piano keys, making each hit easy to identify.

**Drawing and selecting**

- Double-click, or use the pen (`P`) and click / drag, to draw a note; the pen's Grid and Size menus set the snap and the length.
- Drag on empty space to draw a selection rectangle; `Shift` keeps the current selection.
- `Shift+click` a note to add or remove it; click a note to select only that one.
- Hold `Alt` to turn the grid snap off while dragging.

**Editing a selection**

- Drag a note to move the whole selection; drag a note's left or right edge to resize (the right edge resizes the whole selection).
- `↑ ↓` / `Shift+↑ ↓` transpose by a semitone / an octave, `← →` nudge by one grid step, `Shift+← →` change the length.
- `Ctrl+U` quantizes, `Ctrl+D` duplicates, `Delete` removes, `Ctrl+C` / `Ctrl+X` / `Ctrl+V` copy, cut and paste at the playhead.
- `Ctrl+click` a note cuts it at the click.

**Velocity**: hover or select a note and click the **V** badge that appears right after it. A vertical slider (1-127) sets the velocity; if the note is part of the selection, every selected note gets the same value.

**Scale**: pick a root and a scale next to the clip name to highlight the notes of the scale (rows, keys and key labels).

## Loops

1. Drag in the thin bar above the bar numbers to choose the part that repeats (it snaps to the beat; hold `Alt` for free positioning).
2. Move the brace by dragging its body, resize it by dragging an edge.
3. Click the brace, press `Ctrl+L` or press the **Loop** button to turn the loop on or off. `Ctrl+Shift+L` loops the selected clip.
4. Right-click the bar to loop the selected clip or to loop 1 / 2 / 4 / 8 bars from the playhead.

Notes: the loop is handled inside the audio engine, so the jump back to the start is sample-accurate and the beat grid never drifts, whatever the frame rate of the interface. While the loop is on, the playhead never leaves it. Starting the playhead after the loop end plays on normally. Notes still ringing when the loop wraps are released. The loop does not apply to the exported mixdown. The loop range is saved with the project and restored when you open it (a project where you never touched the loop saves none).

## Automation

Each track has a volume automation lane under its clips, with 0 / 50 / 100 % guides.

- **Click** in the lane to add a point (a ghost point shows where it will land and its value).
- **Drag** a point to move it; it stays between its neighbours and snaps to the grid and to 0 / 50 / 100 % (`Alt` = free).
- **Right-click** a point to delete it.
- Before the first point and after the last one, the value stays flat.
- The points are saved in the project file.

## Drums

Add a drum kit track (**Drum Kit**, **808 Kit**, **Trap Kit** or **Reggaeton Kit**) and play it with the on-screen keyboard, your computer keys or the piano roll. In the piano roll, drum tracks show named pad rows in place of piano keys.

Notes are folded into two octaves (MIDI 36-59), so every key of the keyboard plays a drum. The piano roll shows these names next to each row:

| MIDI key | Note | Sound |
|---|---|---|
| 36 | C2 | Kick |
| 37 | C#2 | Rim / side stick |
| 38 / 40 | D2 / E2 | Snare / tight snare |
| 39 | D#2 | Clap |
| 41 / 43 / 45 / 47 / 48 / 50 | F2 ... D3 | Toms (low to high) |
| 42 / 44 | F#2 | Closed hi-hat |
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
| `Space` | Play / pause |
| `R` | Record (count-in, then play; `R` or `Space` to stop) |
| `Home` | Rewind to the start |
| `M` | Metronome on / off |
| `P` | Pen mode on / off |
| `Esc` | Leave pen mode, cancel a count-in, deselect notes |
| `B` | Split the selected clip at the playhead |
| `[` / `]` | Trim clip start / end to the playhead |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste the selected notes or clip (pasted at the playhead) |
| `Ctrl+D` | Duplicate the selected notes (or the clip when no note is selected) |
| `Delete` | Delete the selected notes (or the clip when no note is selected) |
| `Ctrl+L` | Loop on / off |
| `Ctrl+Shift+L` | Loop the selected clip |
| `Ctrl+B` | Show / hide the instruments |
| `Ctrl+J` | Show / hide the piano roll & mixer |
| `Ctrl+Shift+K` | Show / hide the on-screen keyboard |
| `Ctrl+Shift+F` | Focus on the timeline / show everything |
| Drag in the loop bar | Draw / move / resize the loop (right-click for more) |
| Click / drag in a track's automation lane | Add / move a volume automation point |
| Right-click an automation point | Delete the point |
| Drag the red playhead | Move the playhead (`Alt` = no snap) |

Piano roll only (mouse over the roll):

| Key | Action |
|---|---|
| `Ctrl+A` | Select all notes |
| `Shift+click` / drag a rectangle | Add to the selection / select several notes |
| `Alt` (held) | Disable the grid snap |
| `↑ ↓` / `Shift+↑ ↓` | Transpose by a semitone / an octave |
| `← →` / `Shift+← →` | Nudge by one grid step / change the note length |
| `Ctrl+U` | Quantize the selected notes to the grid |
| Click the **V** badge of a note | Set the velocity of the note (or of the whole selection) |

Press F1 inside the app for all shortcuts.

## MCP (Model Context Protocol)

Start the app with `--mcp`: it then speaks MCP over stdio while keeping its window. Example for Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "opendembow": {
      "command": "/path/to/opendembow",
      "args": ["--mcp"]
    }
  }
}
```

Tools: `get_project`, `list_instruments`, `get_notes`, `set_tempo`, `add_track`, `delete_track`, `add_midi_clip`, `add_notes`, `delete_clip`, `split_clip`, `set_track`, `set_effect`, `transport`, `undo`, `redo`.

Every change goes through the normal undo history and appears live in the window.

## Project files

A `.dembow` project is plain text (tab separated). The `bpm` line may be followed by a `loop<TAB>start<TAB>end<TAB>on` line (in beats).

Each track line is followed by its effect settings (`fx<TAB>reverb.mix<TAB>0.3` ...) when they differ from the defaults, then by its automation points (`auto<TAB>volume<TAB>beat<TAB>value`, one per point); older projects without these lines still open.

Audio clips are stored next to the project as `<name>.audioN.wav`.

Favourite instruments are not part of a project: they are stored per user in `~/.opendembow_favorites` (one instrument name per line).

The piano roll's note selection and scale choice, the analyzer options (Hold / Tilt) and the panel layout are not saved.

## Layout of the source

- `engine.rs` real-time mixer (one effect chain per track), cpal output, sample-accurate loop wrapping in the audio callback, master tap for the analyzer, offline bounce.
- `analyzer.rs` lock-free master tap, FFT spectrum and the Master card - `meters.rs` dBFS meters, peak hold, clip LED.
- `view.rs` show / hide panels, panel icons, collapsed strips, View menu.
- `instruments.rs` synth voices and instrument categories - `project.rs` data model - `project_io.rs` save/load (loop range, effects, automation) + MIDI export.
- `drums.rs` Drum Kit and 808 Kit - `drums_urban.rs` Trap Kit and Reggaeton Kit - `latin.rs` Latin Percussion - `extra_voices.rs` 808 Bass, Fingered Bass, Electric Guitar, Synth Pluck.
- `drum_names.rs` pad names shown in the piano roll (one map per kit family).
- `arrangement.rs` timeline: track headers, ruler, clips, playhead - `loops.rs` loop bar and loop region - `automation.rs` volume automation lanes.
- `pianoroll.rs` note editing, multi-selection, scales - `velocity.rs` the V badge and its velocity slider.
- `keyboard.rs`, `chrome.rs` (menus, instrument browser, favourites, help), `theme.rs` interface.
- `effects.rs` effect settings and DSP (`FxProcessor`) - `mixer.rs` Mixer & Effects panel - `curves.rs` visual equalizer, compressor transfer curve and gain-reduction meter.
- `edit.rs` split / trim / clipboard / audio tools and the bottom-panel tabs.
- `record.rs` recording with count-in - `midi_in.rs` MIDI keyboard input (midir).
- `favorites.rs` favourite instruments (saved in the home folder).
- `mcp.rs` MCP server.
- `metronome.rs` metronome click (own audio stream) and its transport button.
- `files.rs` file commands (new / open / save, exports) - `audio_io.rs` WAV I/O.

## Adding an instrument

1. Add a variant to `Instrument` in `instruments.rs` and to `Instrument::ALL` (append at the end to keep old projects compatible).
2. Fill in `name`, `blurb`, `category` and `release_rate`.
3. Write its sound in `Voice::next` (or in `extra_voices.rs` / `drums.rs` to keep files short).

## Roadmap

Next (0.13.x): automation of pan and effect parameters, MIDI velocity in the live sound, saving the panel layout, LUFS and stereo correlation meters, waveforms with a dB scale.

Later: step sequencer, audio recording, quantization while recording, fold-to-scale in the piano roll, dark theme, a "Designer" synth with filter / envelopes / LFO, sample-based instruments, instrument preview on hover, drag-to-trim clip edges, send effects.

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3) - see the [LICENSE](LICENSE) file for details.

Copyright (C) 2026 Jorge Andre Castro
