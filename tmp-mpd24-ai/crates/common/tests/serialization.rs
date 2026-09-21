use mpd24_ai_common::{MidiEvent, MidiEventKind, MidiPortInfo, MidiStatus, MidiValue};
use serde_json::json;

#[test]
fn midi_status_serializes_with_frontend_field_names() {
    let status = MidiStatus {
        connected: true,
        port: Some(MidiPortInfo {
            id: "midir:0:MPD24".into(),
            index: 0,
            name: "MPD24".into(),
        }),
        message: "Connected".into(),
    };

    assert_eq!(
        serde_json::to_value(status).expect("serialize status"),
        json!({
            "connected": true,
            "port": {
                "id": "midir:0:MPD24",
                "index": 0,
                "name": "MPD24"
            },
            "message": "Connected"
        })
    );
}

#[test]
fn mmc_event_serializes_with_frontend_variant_names() {
    let event = MidiEvent {
        timestamp_micros: 5_000,
        channel: 0,
        kind: MidiEventKind::MachineControl,
        data1: 1,
        value: MidiValue::SevenBit(127),
    };

    assert_eq!(
        serde_json::to_value(event).expect("serialize event"),
        json!({
            "timestampMicros": 5000,
            "channel": 0,
            "kind": "machineControl",
            "data1": 1,
            "value": {
                "kind": "sevenBit",
                "value": 127
            }
        })
    );
}
