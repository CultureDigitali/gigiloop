use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use ringbuf::{traits::*, HeapProd, HeapRb};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{demo::demo_kit, AudioCommand, SamplerCore};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Error)]
pub enum AudioEngineError {
    #[error("no default audio output device is available")]
    NoOutputDevice,
    #[error("failed to query output configurations: {0}")]
    Config(String),
    #[error("default output has no supported F32 PCM configuration")]
    NoF32Config,
    #[error("failed to build audio output stream: {0}")]
    Build(String),
    #[error("failed to start audio output stream: {0}")]
    Play(String),
    #[error("audio engine startup timed out")]
    StartupTimeout,
    #[error("audio command queue is full")]
    QueueFull,
    #[error("audio command queue is unavailable")]
    QueueUnavailable,
}

#[derive(Clone)]
pub struct AudioEngineHandle {
    producer: Arc<Mutex<Option<HeapProd<AudioCommand>>>>,
    device: AudioDeviceInfo,
}

impl AudioEngineHandle {
    pub fn start_default() -> Result<Self, AudioEngineError> {
        let rb = HeapRb::<AudioCommand>::new(1024);
        let (producer, consumer) = rb.split();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        thread::Builder::new()
            .name("mpd24-ai-audio-owner".into())
            .spawn(move || {
                let result = start_audio_stream(consumer);
                match result {
                    Ok((stream, info)) => {
                        let _ = ready_tx.send(Ok(info));
                        let _stream = stream;
                        loop {
                            thread::park_timeout(Duration::from_secs(60));
                        }
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
            })
            .map_err(|error| AudioEngineError::Build(error.to_string()))?;

        let device = ready_rx
            .recv_timeout(Duration::from_secs(8))
            .map_err(|_| AudioEngineError::StartupTimeout)??;

        Ok(Self {
            producer: Arc::new(Mutex::new(Some(producer))),
            device,
        })
    }

    pub fn device_info(&self) -> AudioDeviceInfo {
        self.device.clone()
    }

    pub fn send(&self, command: AudioCommand) -> Result<(), AudioEngineError> {
        let mut guard = self
            .producer
            .lock()
            .map_err(|_| AudioEngineError::QueueUnavailable)?;
        let producer = guard.as_mut().ok_or(AudioEngineError::QueueUnavailable)?;
        producer
            .try_push(command)
            .map_err(|_| AudioEngineError::QueueFull)
    }
}

fn start_audio_stream(
    mut consumer: ringbuf::HeapCons<AudioCommand>,
) -> Result<(cpal::Stream, AudioDeviceInfo), AudioEngineError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioEngineError::NoOutputDevice)?;
    let description = device
        .description()
        .map_err(|error| AudioEngineError::Config(error.to_string()))?;

    let mut ranges = device
        .supported_output_configs()
        .map_err(|error| AudioEngineError::Config(error.to_string()))?
        .filter(|range| range.sample_format() == SampleFormat::F32)
        .collect::<Vec<_>>();
    ranges.sort_by(|a, b| a.cmp_default_heuristics(b));
    let range = ranges.pop().ok_or(AudioEngineError::NoF32Config)?;
    let supported = range
        .try_with_standard_sample_rate()
        .unwrap_or_else(|| range.with_max_sample_rate());
    let channels = supported.channels();
    let sample_rate = supported.sample_rate();
    let config = supported.config();
    let mut sampler = SamplerCore::new(demo_kit(sample_rate), sample_rate);

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                while let Some(command) = consumer.try_pop() {
                    sampler.apply_command(command);
                }
                sampler.render(data, channels as usize);
            },
            |_error| {},
            None,
        )
        .map_err(|error| AudioEngineError::Build(error.to_string()))?;

    stream
        .play()
        .map_err(|error| AudioEngineError::Play(error.to_string()))?;

    Ok((
        stream,
        AudioDeviceInfo {
            name: description.name().to_owned(),
            sample_rate,
            channels,
        },
    ))
}
