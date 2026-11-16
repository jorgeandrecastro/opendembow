//! Reusable audio engine, instruments, effects, and project format for OpenDembow.

pub mod audio_io;
pub mod drums;
pub mod drums_urban;
pub mod effects;
pub mod engine;
pub mod extra_voices;
pub mod instruments;
pub mod latin;
#[doc(hidden)]
pub mod meter_bus;
pub mod more_voices;
pub mod project;
pub mod project_io;
pub mod tap;

pub use engine::{Engine, device_sr, render_offline};
pub use instruments::{Category, Instrument, Voice};
pub use project::{
    AutomationLane, AutomationPoint, Clip, ClipKind, LoopRegion, Note, Project, Track, TrackKind,
};
