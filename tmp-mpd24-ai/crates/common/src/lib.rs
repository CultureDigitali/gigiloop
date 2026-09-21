#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "MPD24-AI";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiPortInfo {
    pub id: String,
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MidiEventKind {
    NoteOn,
    NoteOff,
    ControlChange,
    PolyAftertouch,
    ChannelAftertouch,
    MachineControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum MidiValue {
    SevenBit(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiEvent {
    pub timestamp_micros: u64,
    pub channel: u8,
    pub kind: MidiEventKind,
    pub data1: u8,
    pub value: MidiValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileVerification {
    Unverified,
    HardwareVerified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerMapping {
    pub control_id: String,
    pub message_kind: MidiEventKind,
    pub channel: u8,
    pub data1: u8,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProfile {
    pub id: String,
    pub display_name: String,
    pub name_matches: Vec<String>,
    pub verification: ProfileVerification,
    pub mappings: Vec<ControllerMapping>,
}

impl ControllerProfile {
    pub fn unverified(
        id: impl Into<String>,
        display_name: impl Into<String>,
        name_matches: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            name_matches,
            verification: ProfileVerification::Unverified,
            mappings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiStatus {
    pub connected: bool,
    pub port: Option<MidiPortInfo>,
    pub message: String,
}
