use mpd24_ai_common::{MidiEventKind, MidiValue};
use mpd24_ai_midi_engine::decode::decode_message;

#[test]
fn decodes_note_on_with_velocity() {
    let event = decode_message(1_500, &[0x99, 36, 112]).expect("note on");
    assert_eq!(event.channel, 10);
    assert_eq!(event.kind, MidiEventKind::NoteOn);
    assert_eq!(event.data1, 36);
    assert_eq!(event.value, MidiValue::SevenBit(112));
    assert_eq!(event.timestamp_micros, 1_500);
}

#[test]
fn normalizes_note_on_zero_velocity_to_note_off() {
    let event = decode_message(2_000, &[0x90, 60, 0]).expect("note off");
    assert_eq!(event.kind, MidiEventKind::NoteOff);
    assert_eq!(event.data1, 60);
}

#[test]
fn decodes_control_change() {
    let event = decode_message(3_000, &[0xB0, 12, 74]).expect("cc");
    assert_eq!(event.channel, 1);
    assert_eq!(event.kind, MidiEventKind::ControlChange);
    assert_eq!(event.data1, 12);
    assert_eq!(event.value, MidiValue::SevenBit(74));
}

#[test]
fn decodes_poly_aftertouch() {
    let event = decode_message(4_000, &[0xA0, 36, 55]).expect("aftertouch");
    assert_eq!(event.kind, MidiEventKind::PolyAftertouch);
    assert_eq!(event.data1, 36);
    assert_eq!(event.value, MidiValue::SevenBit(55));
}

#[test]
fn decodes_channel_aftertouch() {
    let event = decode_message(4_500, &[0xD2, 67]).expect("channel aftertouch");
    assert_eq!(event.channel, 3);
    assert_eq!(event.kind, MidiEventKind::ChannelAftertouch);
    assert_eq!(event.data1, 0);
    assert_eq!(event.value, MidiValue::SevenBit(67));
}

#[test]
fn decodes_mmc_transport_sysex() {
    let event = decode_message(5_000, &[0xF0, 0x7F, 0x7F, 0x06, 0x01, 0xF7]).expect("MMC stop");
    assert_eq!(event.channel, 0);
    assert_eq!(event.kind, MidiEventKind::MachineControl);
    assert_eq!(event.data1, 0x01);
    assert_eq!(event.value, MidiValue::SevenBit(0x7F));
}

#[test]
fn unsupported_or_truncated_messages_are_ignored() {
    assert!(decode_message(0, &[]).is_none());
    assert!(decode_message(0, &[0x90, 36]).is_none());
    assert!(decode_message(0, &[0xF8]).is_none());
}
