#![forbid(unsafe_code)]

pub mod demo;
pub mod engine;
pub mod sample;
pub mod sampler;

pub use engine::{AudioDeviceInfo, AudioEngineError, AudioEngineHandle};
pub use sample::{load_wav_mono, PadSample, SampleLoadError};
pub use sampler::{
    AudioCommand, PadPlaybackParams, SamplerCore, VelocityLayer, MAX_VELOCITY_LAYERS, MAX_VOICES,
    PAD_COUNT,
};
