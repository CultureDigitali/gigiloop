use mpd24_ai_groove_engine::{
    active_hit_count, best_local_candidate, generate, generate_candidates, GrooveOperation,
    GrooveParams,
};
use mpd24_ai_sequencer::Pattern;

fn params(seed: u64) -> GrooveParams {
    GrooveParams {
        amount: 0.75,
        density: 0.7,
        complexity: 0.7,
        humanize: 0.8,
        seed,
        locked_tracks: Vec::new(),
    }
}

#[test]
fn complete_turns_an_empty_pattern_into_a_groove() {
    let source = Pattern::new(16);
    let result = generate(&source, GrooveOperation::Complete, &params(10));
    assert!(active_hit_count(&result) > 6);
    assert!(result.hits_for_step(0, 0).iter().any(|hit| hit.pad == 0));
    assert!(result.hits_for_step(4, 0).iter().any(|hit| hit.pad == 1));
}

#[test]
fn groove_lock_preserves_locked_tracks_exactly() {
    let source = Pattern::demo();
    let mut settings = params(20);
    settings.locked_tracks = vec![0, 1];
    let result = generate(&source, GrooveOperation::Variation, &settings);
    assert_eq!(result.tracks[0], source.tracks[0]);
    assert_eq!(result.tracks[1], source.tracks[1]);
}

#[test]
fn humanize_changes_expression_without_moving_notes_between_steps() {
    let source = Pattern::demo();
    let result = generate(&source, GrooveOperation::Humanize, &params(30));
    for (source_track, result_track) in source.tracks.iter().zip(&result.tracks) {
        for (source_step, result_step) in source_track.steps.iter().zip(&result_track.steps) {
            assert_eq!(source_step.active, result_step.active);
        }
    }
    assert_ne!(result, source);
}

#[test]
fn fill_adds_activity_near_the_end() {
    let source = Pattern::demo();
    let result = generate(&source, GrooveOperation::Fill, &params(40));
    let tail_hits = result
        .tracks
        .iter()
        .flat_map(|track| &track.steps[12..16])
        .filter(|step| step.active)
        .count();
    let source_tail_hits = source
        .tracks
        .iter()
        .flat_map(|track| &track.steps[12..16])
        .filter(|step| step.active)
        .count();
    assert!(tail_hits > source_tail_hits);
}

#[test]
fn simplify_never_increases_hit_count() {
    let source = generate(&Pattern::demo(), GrooveOperation::Complexify, &params(50));
    let result = generate(&source, GrooveOperation::Simplify, &params(51));
    assert!(active_hit_count(&result) <= active_hit_count(&source));
}

#[test]
fn generation_is_deterministic_for_the_same_seed() {
    let source = Pattern::demo();
    let first = generate(&source, GrooveOperation::Variation, &params(99));
    let second = generate(&source, GrooveOperation::Variation, &params(99));
    assert_eq!(first, second);
}

#[test]
fn candidate_fanout_uses_distinct_seeds_and_local_scores() {
    let source = Pattern::demo();
    let candidates = generate_candidates(&source, GrooveOperation::Variation, &params(200), 4);
    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].seed, 200);
    assert_eq!(candidates[3].seed, 203);
    assert!(candidates
        .iter()
        .all(|candidate| (0.0..=1.0).contains(&candidate.metrics.local_score)));
    assert!(best_local_candidate(&candidates).is_some());
}
