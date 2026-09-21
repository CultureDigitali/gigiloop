use std::sync::Arc;

use crate::PadSample;

pub const PAD_COUNT: usize = 16;
pub const MAX_VOICES: usize = 64;
pub const MAX_VELOCITY_LAYERS: usize = 8;

#[derive(Debug, Clone)]
pub struct VelocityLayer {
    pub min_velocity: u8,
    pub max_velocity: u8,
    pub samples: Arc<[PadSample]>,
    pub round_robin: bool,
}

impl VelocityLayer {
    pub fn new(
        min_velocity: u8,
        max_velocity: u8,
        samples: Vec<PadSample>,
        round_robin: bool,
    ) -> Self {
        let min_velocity = min_velocity.clamp(1, 127);
        Self {
            min_velocity,
            max_velocity: max_velocity.clamp(min_velocity, 127),
            samples: samples.into(),
            round_robin,
        }
    }

    fn matches(&self, velocity: u8) -> bool {
        velocity >= self.min_velocity && velocity <= self.max_velocity && !self.samples.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadPlaybackParams {
    pub pitch_semitones: f32,
    pub reverse: bool,
    pub start: f32,
    pub end: f32,
    pub choke_group: Option<u8>,
}

impl Default for PadPlaybackParams {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            reverse: false,
            start: 0.0,
            end: 1.0,
            choke_group: None,
        }
    }
}

impl PadPlaybackParams {
    fn normalized(self) -> Self {
        let start = self.start.clamp(0.0, 0.999);
        let end = self.end.clamp(start + 0.001, 1.0);
        Self {
            pitch_semitones: self.pitch_semitones.clamp(-24.0, 24.0),
            reverse: self.reverse,
            start,
            end,
            choke_group: self.choke_group.filter(|group| *group > 0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Trigger {
        pad: u8,
        velocity: u8,
    },
    SetMasterGain(f32),
    SetPadGain {
        pad: u8,
        gain: f32,
    },
    SetPadPan {
        pad: u8,
        pan: f32,
    },
    SetPadPlayback {
        pad: u8,
        params: PadPlaybackParams,
    },
    SetPadVelocityLayers {
        pad: u8,
        layers: Arc<[VelocityLayer]>,
    },
    ReplacePad {
        pad: u8,
        sample: PadSample,
    },
    StopAll,
}

#[derive(Debug, Clone)]
struct Voice {
    active: bool,
    samples: Option<Arc<[f32]>>,
    position: f64,
    step: f64,
    min_position: f64,
    max_position: f64,
    left_gain: f32,
    right_gain: f32,
    choke_group: Option<u8>,
    age: u64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            samples: None,
            position: 0.0,
            step: 1.0,
            min_position: 0.0,
            max_position: 0.0,
            left_gain: 0.0,
            right_gain: 0.0,
            choke_group: None,
            age: 0,
        }
    }
}

pub struct SamplerCore {
    pads: [PadSample; PAD_COUNT],
    voices: [Voice; MAX_VOICES],
    pad_gains: [f32; PAD_COUNT],
    pad_pans: [f32; PAD_COUNT],
    pad_playback: [PadPlaybackParams; PAD_COUNT],
    pad_velocity_layers: [Arc<[VelocityLayer]>; PAD_COUNT],
    pad_round_robin_counters: [[usize; MAX_VELOCITY_LAYERS]; PAD_COUNT],
    retired_samples: Vec<Arc<[f32]>>,
    master_gain: f32,
    output_sample_rate: u32,
    voice_age: u64,
}

impl SamplerCore {
    pub fn new(pads: [PadSample; PAD_COUNT], output_sample_rate: u32) -> Self {
        let pad_playback = std::array::from_fn(|index| PadPlaybackParams {
            choke_group: pads[index].choke_group,
            ..PadPlaybackParams::default()
        });
        Self {
            pads,
            voices: std::array::from_fn(|_| Voice::default()),
            pad_gains: [1.0; PAD_COUNT],
            pad_pans: [0.0; PAD_COUNT],
            pad_playback,
            pad_velocity_layers: std::array::from_fn(|_| Arc::from([])),
            pad_round_robin_counters: [[0; MAX_VELOCITY_LAYERS]; PAD_COUNT],
            retired_samples: Vec::new(),
            master_gain: 0.86,
            output_sample_rate,
            voice_age: 0,
        }
    }

    pub fn apply_command(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::Trigger { pad, velocity } => self.trigger(pad as usize, velocity),
            AudioCommand::SetMasterGain(gain) => self.master_gain = gain.clamp(0.0, 1.5),
            AudioCommand::SetPadGain { pad, gain } => {
                if let Some(slot) = self.pad_gains.get_mut(pad as usize) {
                    *slot = gain.clamp(0.0, 2.0);
                }
            }
            AudioCommand::SetPadPan { pad, pan } => {
                if let Some(slot) = self.pad_pans.get_mut(pad as usize) {
                    *slot = pan.clamp(-1.0, 1.0);
                }
            }
            AudioCommand::SetPadPlayback { pad, params } => {
                if let Some(slot) = self.pad_playback.get_mut(pad as usize) {
                    *slot = params.normalized();
                }
            }
            AudioCommand::SetPadVelocityLayers { pad, layers } => {
                if layers.len() <= MAX_VELOCITY_LAYERS {
                    if let Some(slot) = self.pad_velocity_layers.get_mut(pad as usize) {
                        *slot = layers;
                        self.pad_round_robin_counters[pad as usize] = [0; MAX_VELOCITY_LAYERS];
                    }
                }
            }
            AudioCommand::ReplacePad { pad, sample } => {
                if let Some(slot) = self.pads.get_mut(pad as usize) {
                    let previous = std::mem::replace(slot, sample);
                    self.retired_samples.push(previous.samples);
                }
            }
            AudioCommand::StopAll => {
                for voice in &mut self.voices {
                    voice.active = false;
                    voice.samples = None;
                }
                self.retired_samples.clear();
            }
        }
    }

    pub fn trigger(&mut self, pad: usize, velocity: u8) {
        if pad >= PAD_COUNT || velocity == 0 {
            return;
        }

        let layer_choice = self.pad_velocity_layers[pad]
            .iter()
            .position(|layer| layer.matches(velocity));
        let sample_index = layer_choice.map(|layer_index| {
            let layer = &self.pad_velocity_layers[pad][layer_index];
            if layer.round_robin && layer.samples.len() > 1 {
                self.pad_round_robin_counters[pad][layer_index] % layer.samples.len()
            } else {
                0
            }
        });
        if let Some(layer_index) = layer_choice {
            if self.pad_velocity_layers[pad][layer_index].round_robin
                && self.pad_velocity_layers[pad][layer_index].samples.len() > 1
            {
                self.pad_round_robin_counters[pad][layer_index] =
                    self.pad_round_robin_counters[pad][layer_index].wrapping_add(1);
            }
        }
        let sample = match (layer_choice, sample_index) {
            (Some(layer_index), Some(sample_index)) => {
                &self.pad_velocity_layers[pad][layer_index].samples[sample_index]
            }
            _ => &self.pads[pad],
        };
        if sample.samples.is_empty() || sample.sample_rate == 0 || self.output_sample_rate == 0 {
            return;
        }

        let playback = self.pad_playback[pad].normalized();
        if let Some(group) = playback.choke_group {
            for voice in &mut self.voices {
                if voice.active && voice.choke_group == Some(group) {
                    voice.active = false;
                }
            }
        }

        self.voice_age = self.voice_age.wrapping_add(1);
        let velocity_gain = velocity as f32 / 127.0;
        let pan = (sample.pan + self.pad_pans[pad]).clamp(-1.0, 1.0);
        let left_pan = if pan > 0.0 { 1.0 - pan } else { 1.0 };
        let right_pan = if pan < 0.0 { 1.0 + pan } else { 1.0 };
        let base_gain = velocity_gain * sample.gain * self.pad_gains[pad];
        let sample_len = sample.samples.len() as f64;
        let min_position = (playback.start as f64 * sample_len)
            .floor()
            .clamp(0.0, (sample_len - 1.0).max(0.0));
        let max_position = (playback.end as f64 * sample_len)
            .ceil()
            .clamp(min_position + 1.0, sample_len);
        let pitch_ratio = 2.0_f64.powf(playback.pitch_semitones as f64 / 12.0);
        let base_step = sample.sample_rate as f64 / self.output_sample_rate as f64 * pitch_ratio;
        let (position, step) = if playback.reverse {
            (max_position - 1.0, -base_step)
        } else {
            (min_position, base_step)
        };
        let slot = self
            .voices
            .iter()
            .position(|voice| !voice.active)
            .or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, voice)| voice.age)
                    .map(|(index, _)| index)
            })
            .unwrap_or(0);

        self.voices[slot] = Voice {
            active: true,
            samples: Some(Arc::clone(&sample.samples)),
            position,
            step,
            min_position,
            max_position,
            left_gain: base_gain * left_pan,
            right_gain: base_gain * right_pan,
            choke_group: playback.choke_group,
            age: self.voice_age,
        };
    }

    pub fn render(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);
        if channels == 0 {
            return;
        }

        for frame in output.chunks_mut(channels) {
            let mut left = 0.0_f32;
            let mut right = 0.0_f32;

            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }

                let Some(sample) = voice.samples.as_ref() else {
                    voice.active = false;
                    continue;
                };
                if voice.position < voice.min_position || voice.position >= voice.max_position {
                    voice.active = false;
                    voice.samples = None;
                    continue;
                }
                let index = voice.position as usize;

                let value = sample[index];
                left += value * voice.left_gain;
                right += value * voice.right_gain;
                voice.position += voice.step;
            }

            frame[0] = soft_clip(left * self.master_gain);
            if channels > 1 {
                frame[1] = soft_clip(right * self.master_gain);
                let extra_value = (frame[0] + frame[1]) * 0.5;
                for extra in &mut frame[2..] {
                    *extra = extra_value;
                }
            }
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }

    pub fn pad_name(&self, pad: usize) -> Option<&str> {
        self.pads.get(pad).map(|sample| sample.name.as_str())
    }
}

fn soft_clip(value: f32) -> f32 {
    value / (1.0 + value.abs())
}
