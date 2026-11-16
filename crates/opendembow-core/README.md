# opendembow-core

The reusable audio and project library used by the OpenDembow desktop
application. It provides the audio engine, synthesized instruments and drum
kits, effects, project model, project persistence, WAV I/O, and MIDI export.

Add it to a Rust project with:

```toml
[dependencies]
opendembow-core = "0.13"
```

The library modules are public. The main entry points are `Project`,
`Instrument`, `Engine`, `render_offline`, and the helpers in `project_io` and
`audio_io`.

```rust
use opendembow_core::{Instrument, Project, render_offline};

fn main() {
    let project = Project::demo(44_100.0);
    let stereo_samples = render_offline(&project);
    let available_instruments = Instrument::ALL;
}
```

The `Engine` opens the system's default audio output device. Use
`render_offline` when audio-device access is not wanted.
