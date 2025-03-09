mod audio_graph;
mod engine;
mod nodes;

use nih_plug::prelude::*;

pub use audio_graph::AudioGraphNode;
pub use engine::{RustAudioEngine, RustAudioEngineParams};
pub use nodes::{GainProcessor, SineGenerator};

nih_export_clap!(engine::RustAudioEngine);
