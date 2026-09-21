#![forbid(unsafe_code)]

use mpd24_ai_sequencer::Pattern;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GrooveOperation {
    Complete,
    Variation,
    Fill,
    Humanize,
    Simplify,
    Complexify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrooveParams {
    pub amount: f32,
    pub density: f32,
    pub complexity: f32,
    pub humanize: f32,
    pub seed: u64,
    pub locked_tracks: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateMetrics {
    pub active_hits: usize,
    pub changed_steps: usize,
    pub kick_hits: usize,
    pub snare_hits: usize,
    pub hat_hits: usize,
    pub microtimed_hits: usize,
    pub average_velocity: f32,
    pub local_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrooveCandidate {
    pub id: String,
    pub seed: u64,
    pub pattern: Pattern,
    pub metrics: CandidateMetrics,
}

impl Default for GrooveParams {
    fn default() -> Self {
        Self {
            amount: 0.35,
            density: 0.5,
            complexity: 0.5,
            humanize: 0.3,
            seed: 1,
            locked_tracks: Vec::new(),
        }
    }
}

pub fn generate(source: &Pattern, operation: GrooveOperation, params: &GrooveParams) -> Pattern {
    let mut output = source.clone();
    let normalized = normalize(params);
    match operation {
        GrooveOperation::Complete => complete(&mut output, &normalized),
        GrooveOperation::Variation => variation(&mut output, &normalized),
        GrooveOperation::Fill => fill(&mut output, &normalized),
        GrooveOperation::Humanize => humanize(&mut output, &normalized),
        GrooveOperation::Simplify => simplify(&mut output, &normalized),
        GrooveOperation::Complexify => complexify(&mut output, &normalized),
    }
    output
}

pub fn active_hit_count(pattern: &Pattern) -> usize {
    pattern
        .tracks
        .iter()
        .flat_map(|track| &track.steps)
        .filter(|step| step.active)
        .count()
}

pub fn generate_candidates(
    source: &Pattern,
    operation: GrooveOperation,
    params: &GrooveParams,
    count: usize,
) -> Vec<GrooveCandidate> {
    let count = count.clamp(1, 8);
    (0..count)
        .map(|index| {
            let seed = params.seed.wrapping_add(index as u64);
            let mut candidate_params = params.clone();
            candidate_params.seed = seed;
            let pattern = generate(source, operation, &candidate_params);
            let metrics = candidate_metrics(source, &pattern, operation, &candidate_params);
            GrooveCandidate {
                id: format!("candidate-{}", index + 1),
                seed,
                pattern,
                metrics,
            }
        })
        .collect()
}

pub fn best_local_candidate(candidates: &[GrooveCandidate]) -> Option<&GrooveCandidate> {
    candidates.iter().max_by(|left, right| {
        left.metrics
            .local_score
            .total_cmp(&right.metrics.local_score)
    })
}

fn candidate_metrics(
    source: &Pattern,
    candidate: &Pattern,
    operation: GrooveOperation,
    params: &GrooveParams,
) -> CandidateMetrics {
    let mut changed_steps = 0;
    let mut velocity_sum = 0.0_f32;
    let mut velocity_count = 0_usize;
    let mut microtimed_hits = 0;

    for (source_track, candidate_track) in source.tracks.iter().zip(&candidate.tracks) {
        for (before, after) in source_track.steps.iter().zip(&candidate_track.steps) {
            if before.active != after.active
                || before.velocity != after.velocity
                || before.micro_offset_micros != after.micro_offset_micros
            {
                changed_steps += 1;
            }
            if after.active {
                velocity_sum += after.velocity as f32;
                velocity_count += 1;
                if after.micro_offset_micros != 0 {
                    microtimed_hits += 1;
                }
            }
        }
    }

    let active_hits = active_hit_count(candidate);
    let source_hits = active_hit_count(source);
    let kick_hits = track_hit_count(candidate, 0);
    let snare_hits = track_hit_count(candidate, 1);
    let hat_hits = track_hit_count(candidate, 2) + track_hit_count(candidate, 3);
    let average_velocity = if velocity_count == 0 {
        0.0
    } else {
        velocity_sum / velocity_count as f32
    };

    let total_slots = candidate
        .total_steps
        .saturating_mul(candidate.tracks.len())
        .max(1);
    let density = active_hits as f32 / total_slots as f32;
    let changed_ratio = changed_steps as f32 / total_slots as f32;
    let target_density = (0.025 + params.density * 0.075).clamp(0.015, 0.12);
    let density_fit = 1.0 - ((density - target_density).abs() / 0.12).min(1.0);
    let structural = ((kick_hits > 0) as u8 as f32 + (snare_hits > 0) as u8 as f32) * 0.5;

    let operation_fit = match operation {
        GrooveOperation::Complete => {
            structural * 0.5 + (hat_hits > 0) as u8 as f32 * 0.25 + density_fit * 0.25
        }
        GrooveOperation::Variation => {
            let target = 0.015 + params.amount * 0.06;
            1.0 - ((changed_ratio - target).abs() / 0.12).min(1.0)
        }
        GrooveOperation::Fill => {
            let tail = tail_hit_count(candidate);
            (tail as f32 / 8.0).min(1.0) * 0.7 + structural * 0.3
        }
        GrooveOperation::Humanize => {
            let same_hits = active_hits == source_hits;
            (same_hits as u8 as f32) * 0.55
                + (microtimed_hits as f32 / active_hits.max(1) as f32).min(1.0) * 0.45
        }
        GrooveOperation::Simplify => {
            if source_hits == 0 {
                0.0
            } else {
                ((source_hits.saturating_sub(active_hits)) as f32 / source_hits as f32).min(1.0)
            }
        }
        GrooveOperation::Complexify => {
            ((active_hits.saturating_sub(source_hits)) as f32 / source_hits.max(1) as f32).min(1.0)
                * 0.7
                + density_fit * 0.3
        }
    };

    CandidateMetrics {
        active_hits,
        changed_steps,
        kick_hits,
        snare_hits,
        hat_hits,
        microtimed_hits,
        average_velocity,
        local_score: (operation_fit * 0.8 + density_fit * 0.2).clamp(0.0, 1.0),
    }
}

fn track_hit_count(pattern: &Pattern, track_index: usize) -> usize {
    pattern
        .tracks
        .get(track_index)
        .map(|track| track.steps.iter().filter(|step| step.active).count())
        .unwrap_or(0)
}

fn tail_hit_count(pattern: &Pattern) -> usize {
    let start = pattern.total_steps.saturating_sub(4);
    pattern
        .tracks
        .iter()
        .map(|track| {
            track
                .steps
                .iter()
                .skip(start)
                .filter(|step| step.active)
                .count()
        })
        .sum()
}

fn normalize(params: &GrooveParams) -> GrooveParams {
    GrooveParams {
        amount: params.amount.clamp(0.0, 1.0),
        density: params.density.clamp(0.0, 1.0),
        complexity: params.complexity.clamp(0.0, 1.0),
        humanize: params.humanize.clamp(0.0, 1.0),
        seed: params.seed,
        locked_tracks: params.locked_tracks.clone(),
    }
}

fn complete(pattern: &mut Pattern, params: &GrooveParams) {
    for bar_start in (0..pattern.total_steps).step_by(16) {
        if !is_locked(params, 0) {
            activate(pattern, 0, bar_start, 118);
            if params.amount > 0.45 {
                activate(pattern, 0, bar_start + 8, 103);
            }
        }
        if !is_locked(params, 1) {
            activate(pattern, 1, bar_start + 4, 108);
            activate(pattern, 1, bar_start + 12, 112);
        }
        if !is_locked(params, 2) {
            for local in (0..16).step_by(2) {
                let step = bar_start + local;
                if step >= pattern.total_steps {
                    break;
                }
                let probability = 0.45 + params.density * 0.5;
                if random01(params.seed, step as u64 + 0x2000) <= probability {
                    let velocity = if local.is_multiple_of(4) { 91 } else { 76 };
                    activate(pattern, 2, step, velocity);
                }
            }
        }
    }

    if params.complexity > 0.55 && !is_locked(params, 8) {
        for step in (3..pattern.total_steps).step_by(8) {
            if random01(params.seed, step as u64 + 0x8000) < params.amount * 0.6 {
                activate(pattern, 8, step, 72);
            }
        }
    }
}

fn variation(pattern: &mut Pattern, params: &GrooveParams) {
    for (track_index, track) in pattern.tracks.iter_mut().enumerate() {
        if is_locked(params, track_index) {
            continue;
        }
        let structural = matches!(track_index, 0 | 1);
        for (step_index, step) in track.steps.iter_mut().enumerate() {
            let salt = ((track_index as u64) << 32) | step_index as u64;
            if step.active {
                let remove_chance = params.amount * if structural { 0.08 } else { 0.2 };
                if random01(params.seed, salt) < remove_chance && !step_index.is_multiple_of(4) {
                    step.active = false;
                    continue;
                }
                step.velocity =
                    jitter_velocity(step.velocity, params.amount * 0.4, params.seed, salt + 1);
            } else {
                let track_weight = if structural {
                    0.04
                } else if matches!(track_index, 2 | 3 | 8 | 9) {
                    0.14
                } else {
                    0.08
                };
                if random01(params.seed, salt + 2) < params.amount * params.density * track_weight {
                    step.active = true;
                    step.velocity = 62 + (random01(params.seed, salt + 3) * 42.0).round() as u8;
                }
            }
        }
    }
}

fn fill(pattern: &mut Pattern, params: &GrooveParams) {
    let fill_start = pattern.total_steps.saturating_sub(4);
    for step in fill_start..pattern.total_steps {
        let local = step - fill_start;
        let track = 5 + local.min(3);
        if !is_locked(params, track) {
            activate(pattern, track, step, 92 + local as u8 * 6);
        }
        if params.amount > 0.45 && !is_locked(params, 9) && local.is_multiple_of(2) {
            activate(pattern, 9, step, 72);
        }
    }
    if !is_locked(params, 1) && pattern.total_steps > 0 {
        activate(pattern, 1, pattern.total_steps - 1, 121);
    }
}

fn humanize(pattern: &mut Pattern, params: &GrooveParams) {
    for (track_index, track) in pattern.tracks.iter_mut().enumerate() {
        if is_locked(params, track_index) {
            continue;
        }
        for (step_index, step) in track.steps.iter_mut().enumerate() {
            if !step.active {
                continue;
            }
            let salt = ((track_index as u64) << 32) | step_index as u64;
            let strength = (params.humanize * params.amount.max(0.15)).clamp(0.0, 1.0);
            step.velocity = jitter_velocity(step.velocity, strength, params.seed, salt);
            let offset = (random_signed(params.seed, salt + 0xA11) * 14_000.0 * strength) as i32;
            step.micro_offset_micros = offset.clamp(-18_000, 18_000);
        }
    }
}

fn simplify(pattern: &mut Pattern, params: &GrooveParams) {
    for (track_index, track) in pattern.tracks.iter_mut().enumerate() {
        if is_locked(params, track_index) {
            continue;
        }
        for (step_index, step) in track.steps.iter_mut().enumerate() {
            if !step.active || step_index.is_multiple_of(4) {
                continue;
            }
            let salt = ((track_index as u64) << 32) | step_index as u64;
            let weight = if track_index >= 2 { 0.7 } else { 0.35 };
            if random01(params.seed, salt) < params.amount * weight {
                step.active = false;
            }
        }
    }
}

fn complexify(pattern: &mut Pattern, params: &GrooveParams) {
    for track_index in 2..pattern.tracks.len().min(10) {
        if is_locked(params, track_index) {
            continue;
        }
        let track = &mut pattern.tracks[track_index];
        for (step_index, step) in track.steps.iter_mut().enumerate() {
            if step.active || step_index.is_multiple_of(4) {
                continue;
            }
            let salt = ((track_index as u64) << 32) | step_index as u64;
            let chance = params.amount * (0.06 + params.density * 0.1 + params.complexity * 0.12);
            if random01(params.seed, salt) < chance {
                step.active = true;
                step.velocity = 55 + (random01(params.seed, salt + 1) * 45.0) as u8;
                if params.humanize > 0.0 {
                    step.micro_offset_micros =
                        (random_signed(params.seed, salt + 2) * 8_000.0 * params.humanize) as i32;
                }
            }
        }
    }
}

fn activate(pattern: &mut Pattern, track: usize, step: usize, velocity: u8) {
    if step >= pattern.total_steps {
        return;
    }
    if let Some(slot) = pattern
        .tracks
        .get_mut(track)
        .and_then(|track| track.steps.get_mut(step))
    {
        slot.active = true;
        slot.velocity = velocity.clamp(1, 127);
    }
}

fn is_locked(params: &GrooveParams, track: usize) -> bool {
    params.locked_tracks.contains(&track)
}

fn jitter_velocity(velocity: u8, amount: f32, seed: u64, salt: u64) -> u8 {
    let delta = (random_signed(seed, salt) * 22.0 * amount) as i16;
    (velocity as i16 + delta).clamp(1, 127) as u8
}

fn random_signed(seed: u64, salt: u64) -> f32 {
    random01(seed, salt) * 2.0 - 1.0
}

fn random01(seed: u64, salt: u64) -> f32 {
    let mut value = seed.wrapping_add(salt).wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as f64 / u64::MAX as f64) as f32
}
