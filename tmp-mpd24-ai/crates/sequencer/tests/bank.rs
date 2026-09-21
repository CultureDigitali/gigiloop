use mpd24_ai_sequencer::{LaunchQuantization, Pattern, PatternBank, SequencerError};

#[test]
fn bank_contains_eight_named_slots() {
    let bank = PatternBank::default();
    assert_eq!(bank.slots.len(), 8);
    assert_eq!(bank.active_slot, 0);
    assert!(bank.slots[0].name.starts_with("Pattern A"));
    assert_eq!(bank.slots[7].name, "Pattern H");
}

#[test]
fn selecting_while_stopped_switches_immediately() {
    let mut bank = PatternBank::default();
    assert!(bank
        .queue(2, LaunchQuantization::NextBar, false)
        .expect("queue"));
    assert_eq!(bank.active_slot, 2);
    assert!(bank.queued.is_none());
}

#[test]
fn next_bar_waits_until_sixteenth_step_boundary() {
    let mut bank = PatternBank::default();
    assert!(!bank
        .queue(1, LaunchQuantization::NextBar, true)
        .expect("queue"));
    assert!(!bank.commit_if_due(7));
    assert!(!bank.commit_if_due(15));
    assert!(bank.commit_if_due(16));
    assert_eq!(bank.active_slot, 1);
}

#[test]
fn next_two_bars_waits_for_thirty_second_step_boundary() {
    let mut bank = PatternBank::default();
    bank.queue(3, LaunchQuantization::NextTwoBars, true)
        .expect("queue");
    assert!(!bank.commit_if_due(16));
    assert!(bank.commit_if_due(32));
    assert_eq!(bank.active_slot, 3);
}

#[test]
fn next_beat_commits_on_next_runtime_boundary() {
    let mut bank = PatternBank::default();
    bank.queue(4, LaunchQuantization::NextBeat, true)
        .expect("queue");
    assert!(bank.commit_if_due(9));
    assert_eq!(bank.active_slot, 4);
}

#[test]
fn copying_active_pattern_preserves_music_but_renames_slot() {
    let mut bank = PatternBank::default();
    bank.slots[0] = Pattern::demo();
    let source_hits = bank.active().hits_for_step(0, 0);
    bank.copy_active_to(5).expect("copy");
    assert_eq!(bank.slots[5].hits_for_step(0, 0), source_hits);
    assert_eq!(bank.slots[5].name, "Pattern F");
}

#[test]
fn rejects_invalid_pattern_slot() {
    let mut bank = PatternBank::default();
    assert_eq!(
        bank.queue(8, LaunchQuantization::Immediate, false),
        Err(SequencerError::InvalidPatternSlot)
    );
}
