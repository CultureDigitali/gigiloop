use mpd24_ai_common::{MidiEvent, MidiEventKind, MidiValue};

pub fn decode_message(timestamp_micros: u64, bytes: &[u8]) -> Option<MidiEvent> {
    let status = *bytes.first()?;

    if is_mmc_sysex(bytes) {
        return Some(MidiEvent {
            timestamp_micros,
            channel: 0,
            kind: MidiEventKind::MachineControl,
            data1: bytes[4] & 0x7F,
            value: MidiValue::SevenBit(bytes[2] & 0x7F),
        });
    }

    if !(0x80..0xF0).contains(&status) {
        return None;
    }

    let message_type = status & 0xF0;
    let channel = (status & 0x0F) + 1;

    match message_type {
        0x80 | 0x90 | 0xA0 | 0xB0 => {
            if bytes.len() < 3 {
                return None;
            }
            let data1 = bytes[1] & 0x7F;
            let data2 = bytes[2] & 0x7F;
            let kind = match message_type {
                0x80 => MidiEventKind::NoteOff,
                0x90 if data2 == 0 => MidiEventKind::NoteOff,
                0x90 => MidiEventKind::NoteOn,
                0xA0 => MidiEventKind::PolyAftertouch,
                0xB0 => MidiEventKind::ControlChange,
                _ => unreachable!(),
            };
            Some(MidiEvent {
                timestamp_micros,
                channel,
                kind,
                data1,
                value: MidiValue::SevenBit(data2),
            })
        }
        0xD0 => {
            if bytes.len() < 2 {
                return None;
            }
            Some(MidiEvent {
                timestamp_micros,
                channel,
                kind: MidiEventKind::ChannelAftertouch,
                data1: 0,
                value: MidiValue::SevenBit(bytes[1] & 0x7F),
            })
        }
        _ => None,
    }
}

fn is_mmc_sysex(bytes: &[u8]) -> bool {
    bytes.len() >= 6
        && bytes[0] == 0xF0
        && bytes[1] == 0x7F
        && bytes[3] == 0x06
        && bytes.last() == Some(&0xF7)
}
