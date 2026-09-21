use mpd24_ai_sequencer::{Pattern, SequencerError};

#[test]
fn supports_sixteen_thirty_two_and_sixty_four_steps() {
    let mut pattern = Pattern::new(16);
    assert_eq!(pattern.total_steps, 16);
    pattern.set_step_count(32).expect("32 steps");
    assert_eq!(pattern.total_steps, 32);
    assert!(pattern.tracks.iter().all(|track| track.steps.len() == 32));
    pattern.set_step_count(64).expect("64 steps");
    assert_eq!(pattern.total_steps, 64);
    assert_eq!(
        pattern.set_step_count(24),
        Err(SequencerError::InvalidStepCount)
    );
}

#[test]
fn active_steps_generate_hits_with_velocity() {
    let mut pattern = Pattern::new(16);
    pattern.set_step(0, 0, true, 121).expect("kick");
    pattern.set_step(1, 4, true, 93).expect("snare");
    assert_eq!(pattern.hits_for_step(0, 0)[0].velocity, 121);
    assert_eq!(pattern.hits_for_step(4, 0)[0].pad, 1);
}

#[test]
fn track_length_allows_polymetric_repetition() {
    let mut pattern = Pattern::new(16);
    pattern.tracks[2].length = 3;
    pattern.set_step(2, 0, true, 100).expect("hat");
    assert_eq!(pattern.hits_for_step(0, 0).len(), 1);
    assert_eq!(pattern.hits_for_step(3, 0).len(), 1);
    assert_eq!(pattern.hits_for_step(6, 0).len(), 1);
}

#[test]
fn swing_preserves_pair_duration() {
    let mut pattern = Pattern::new(16);
    pattern.set_bpm(120.0).expect("bpm");
    pattern.set_swing(0.75);
    let even = pattern.interval_after_step(0);
    let odd = pattern.interval_after_step(1);
    let pair = even + odd;
    let straight_pair = pattern.step_duration() * 2;
    let difference = pair.abs_diff(straight_pair);
    assert!(difference.as_micros() <= 2);
    assert!(even > odd);
}

#[test]
fn clear_deactivates_all_steps() {
    let mut pattern = Pattern::new(16);
    pattern.set_step(0, 0, true, 100).expect("step");
    pattern.clear();
    assert!(pattern.hits_for_step(0, 0).is_empty());
}

#[test]
fn demo_pattern_contains_a_playable_groove() {
    let pattern = Pattern::demo();
    assert_eq!(pattern.name, "Demo Groove");
    assert_eq!(pattern.total_steps, 16);
    assert!(pattern.hits_for_step(0, 0).iter().any(|hit| hit.pad == 0));
    assert!(pattern.hits_for_step(4, 0).iter().any(|hit| hit.pad == 1));
    assert!(pattern.hits_for_step(2, 0).iter().any(|hit| hit.pad == 2));
}
