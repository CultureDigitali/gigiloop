use std::{path::Path, sync::Arc};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PadSample {
    pub name: String,
    pub samples: Arc<[f32]>,
    pub sample_rate: u32,
    pub gain: f32,
    pub pan: f32,
    pub choke_group: Option<u8>,
}

impl PadSample {
    pub fn new(name: impl Into<String>, samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            name: name.into(),
            samples: samples.into(),
            sample_rate,
            gain: 1.0,
            pan: 0.0,
            choke_group: None,
        }
    }

    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain.clamp(0.0, 2.0);
        self
    }

    pub fn with_pan(mut self, pan: f32) -> Self {
        self.pan = pan.clamp(-1.0, 1.0);
        self
    }

    pub fn with_choke_group(mut self, choke_group: u8) -> Self {
        self.choke_group = Some(choke_group);
        self
    }
}

#[derive(Debug, Error)]
pub enum SampleLoadError {
    #[error("failed to open WAV: {0}")]
    Open(#[from] hound::Error),
    #[error("unsupported WAV with zero channels")]
    ZeroChannels,
}

pub fn load_wav_mono(path: &Path) -> Result<PadSample, SampleLoadError> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(SampleLoadError::ZeroChannels);
    }

    let channels = spec.channels as usize;
    let interleaved = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let max = ((1_i64 << (spec.bits_per_sample.saturating_sub(1))) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / max.max(1.0)))
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    let samples = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
            .collect()
    };

    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sample")
        .to_owned();

    Ok(PadSample::new(name, samples, spec.sample_rate))
}
