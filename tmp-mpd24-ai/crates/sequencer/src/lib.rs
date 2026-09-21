#![forbid(unsafe_code)]

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TRACK_COUNT: usize = 16;
pub const DEFAULT_STEPS: usize = 16;
pub const PATTERN_SLOT_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchQuantization {
    Immediate,
    NextBeat,
    NextBar,
    NextTwoBars,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedPattern {
    pub slot: usize,
    pub quantization: LaunchQuantization,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternBank {
    pub slots: Vec<Pattern>,
    pub active_slot: usize,
    pub queued: Option<QueuedPattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternBankStatus {
    pub active_slot: usize,
    pub queued_slot: Option<usize>,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub active: bool,
    pub velocity: u8,
    pub probability: f32,
    pub micro_offset_micros: i32,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            active: false,
            velocity: 108,
            probability: 1.0,
            micro_offset_micros: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub pad: u8,
    pub length: usize,
    pub muted: bool,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    pub name: String,
    pub bpm: f64,
    pub total_steps: usize,
    pub swing: f32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledHit {
    pub pad: u8,
    pub velocity: u8,
}

#[derive(Debug, Error, PartialEq)]
pub enum SequencerError {
    #[error("step count must be one of 16, 32 or 64")]
    InvalidStepCount,
    #[error("BPM must be between 20 and 320")]
    InvalidBpm,
    #[error("track index out of range")]
    InvalidTrack,
    #[error("step index out of range")]
    InvalidStep,
    #[error("pattern slot index out of range")]
    InvalidPatternSlot,
}

impl Default for PatternBank {
    fn default() -> Self {
        let slots = (0..PATTERN_SLOT_COUNT)
            .map(|index| Pattern {
                name: format!("Pattern {}", (b'A' + index as u8) as char),
                ..Pattern::default()
            })
            .collect::<Vec<_>>();
        Self {
            slots,
            active_slot: 0,
            queued: None,
        }
    }
}

impl PatternBank {
    pub fn active(&self) -> &Pattern {
        &self.slots[self.active_slot]
    }

    pub fn active_mut(&mut self) -> &mut Pattern {
        &mut self.slots[self.active_slot]
    }

    pub fn status(&self) -> PatternBankStatus {
        PatternBankStatus {
            active_slot: self.active_slot,
            queued_slot: self.queued.as_ref().map(|queued| queued.slot),
            names: self
                .slots
                .iter()
                .map(|pattern| pattern.name.clone())
                .collect(),
        }
    }

    pub fn queue(
        &mut self,
        slot: usize,
        quantization: LaunchQuantization,
        playing: bool,
    ) -> Result<bool, SequencerError> {
        if slot >= self.slots.len() {
            return Err(SequencerError::InvalidPatternSlot);
        }
        if slot == self.active_slot {
            self.queued = None;
            return Ok(false);
        }
        if !playing || quantization == LaunchQuantization::Immediate {
            self.active_slot = slot;
            self.queued = None;
            return Ok(true);
        }
        self.queued = Some(QueuedPattern { slot, quantization });
        Ok(false)
    }

    pub fn commit_if_due(&mut self, absolute_step: u64) -> bool {
        let Some(queued) = self.queued.clone() else {
            return false;
        };
        let due = match queued.quantization {
            LaunchQuantization::Immediate | LaunchQuantization::NextBeat => true,
            LaunchQuantization::NextBar => absolute_step.is_multiple_of(16),
            LaunchQuantization::NextTwoBars => absolute_step.is_multiple_of(32),
        };
        if due {
            self.active_slot = queued.slot;
            self.queued = None;
            true
        } else {
            false
        }
    }

    pub fn copy_active_to(&mut self, slot: usize) -> Result<(), SequencerError> {
        if slot >= self.slots.len() {
            return Err(SequencerError::InvalidPatternSlot);
        }
        if slot == self.active_slot {
            return Ok(());
        }
        let mut copied = self.active().clone();
        copied.name = format!("Pattern {}", (b'A' + slot as u8) as char);
        self.slots[slot] = copied;
        Ok(())
    }

    pub fn replace_active(&mut self, pattern: Pattern) {
        self.slots[self.active_slot] = pattern;
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new(DEFAULT_STEPS)
    }
}

impl Pattern {
    pub fn new(total_steps: usize) -> Self {
        let total_steps = if matches!(total_steps, 16 | 32 | 64) {
            total_steps
        } else {
            DEFAULT_STEPS
        };
        Self {
            name: "Pattern A".into(),
            bpm: 92.0,
            total_steps,
            swing: 0.0,
            tracks: (0..TRACK_COUNT)
                .map(|pad| Track {
                    pad: pad as u8,
                    length: total_steps,
                    muted: false,
                    steps: vec![Step::default(); total_steps],
                })
                .collect(),
        }
    }

    pub fn demo() -> Self {
        let mut pattern = Self::new(16);
        pattern.name = "Demo Groove".into();
        pattern.bpm = 104.0;

        for step in [0, 4, 8, 12] {
            let _ = pattern.set_step(0, step, true, 118);
        }
        for step in [4, 12] {
            let _ = pattern.set_step(1, step, true, 108);
        }
        for step in (0_usize..16).step_by(2) {
            let velocity = if step.is_multiple_of(4) { 92 } else { 78 };
            let _ = pattern.set_step(2, step, true, velocity);
        }
        let _ = pattern.set_step(4, 14, true, 88);
        pattern
    }

    pub fn set_step_count(&mut self, total_steps: usize) -> Result<(), SequencerError> {
        if !matches!(total_steps, 16 | 32 | 64) {
            return Err(SequencerError::InvalidStepCount);
        }
        self.total_steps = total_steps;
        for track in &mut self.tracks {
            track.steps.resize(total_steps, Step::default());
            track.length = track.length.min(total_steps).max(1);
        }
        Ok(())
    }

    pub fn set_bpm(&mut self, bpm: f64) -> Result<(), SequencerError> {
        if !(20.0..=320.0).contains(&bpm) {
            return Err(SequencerError::InvalidBpm);
        }
        self.bpm = bpm;
        Ok(())
    }

    pub fn set_swing(&mut self, swing: f32) {
        self.swing = swing.clamp(0.0, 1.0);
    }

    pub fn set_step(
        &mut self,
        track: usize,
        step: usize,
        active: bool,
        velocity: u8,
    ) -> Result<(), SequencerError> {
        let track = self
            .tracks
            .get_mut(track)
            .ok_or(SequencerError::InvalidTrack)?;
        let slot = track
            .steps
            .get_mut(step)
            .ok_or(SequencerError::InvalidStep)?;
        slot.active = active;
        slot.velocity = velocity.clamp(1, 127);
        Ok(())
    }

    pub fn clear(&mut self) {
        for track in &mut self.tracks {
            for step in &mut track.steps {
                step.active = false;
            }
        }
    }

    pub fn hits_for_step(&self, global_step: usize, cycle: u64) -> Vec<ScheduledHit> {
        self.tracks
            .iter()
            .enumerate()
            .filter_map(|(track_index, track)| {
                if track.muted || track.length == 0 {
                    return None;
                }
                let local_step = global_step % track.length;
                let step = track.steps.get(local_step)?;
                if !step.active {
                    return None;
                }
                if !passes_probability(step.probability, cycle, global_step, track_index) {
                    return None;
                }
                Some(ScheduledHit {
                    pad: track.pad,
                    velocity: step.velocity,
                })
            })
            .collect()
    }

    pub fn step_duration(&self) -> Duration {
        Duration::from_secs_f64(60.0 / self.bpm / 4.0)
    }

    pub fn interval_after_step(&self, step_index: usize) -> Duration {
        let base = self.step_duration().as_secs_f64();
        let delay = base * 0.33 * self.swing as f64;
        let seconds = if step_index.is_multiple_of(2) {
            base + delay
        } else {
            (base - delay).max(base * 0.1)
        };
        Duration::from_secs_f64(seconds)
    }
}

fn passes_probability(
    probability: f32,
    cycle: u64,
    global_step: usize,
    track_index: usize,
) -> bool {
    let probability = probability.clamp(0.0, 1.0);
    if probability >= 1.0 {
        return true;
    }
    if probability <= 0.0 {
        return false;
    }
    let mut value = cycle
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((global_step as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((track_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    let normalized = (value as f64 / u64::MAX as f64) as f32;
    normalized <= probability
}
