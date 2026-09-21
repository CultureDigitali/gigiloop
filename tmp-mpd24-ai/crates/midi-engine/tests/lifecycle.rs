use mpd24_ai_midi_engine::service::{make_port_id, MidiService};

#[test]
fn port_id_contains_index_and_name() {
    assert_eq!(make_port_id(2, "MPD24"), "midir:2:MPD24");
}

#[test]
fn duplicate_port_names_still_have_distinct_session_ids() {
    assert_ne!(make_port_id(1, "USB MIDI"), make_port_id(2, "USB MIDI"));
}

#[test]
fn disconnect_without_connection_is_safe_and_idempotent() {
    let mut service = MidiService::new();
    service.disconnect();
    service.disconnect();
    assert!(!service.is_connected());
}
