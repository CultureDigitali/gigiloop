use std::time::Instant;

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use mpd24_ai_common::{MidiEvent, MidiPortInfo};
use thiserror::Error;

use crate::decode::decode_message;

#[derive(Debug, Error)]
pub enum MidiError {
    #[error("failed to initialize MIDI input: {0}")]
    Init(String),
    #[error("failed to read MIDI port name: {0}")]
    PortName(String),
    #[error("MIDI input port not found: {0}")]
    PortNotFound(String),
    #[error("failed to connect MIDI input: {0}")]
    Connect(String),
}

pub fn make_port_id(index: usize, name: &str) -> String {
    format!("midir:{index}:{name}")
}

pub struct MidiService {
    connection: Option<MidiInputConnection<()>>,
}

impl Default for MidiService {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiService {
    pub fn new() -> Self {
        Self { connection: None }
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn disconnect(&mut self) {
        self.connection.take();
    }

    pub fn list_inputs(&self) -> Result<Vec<MidiPortInfo>, MidiError> {
        let input = MidiInput::new("mpd24-ai-discovery")
            .map_err(|error| MidiError::Init(error.to_string()))?;
        input
            .ports()
            .into_iter()
            .enumerate()
            .map(|(index, port)| port_info(&input, index, &port))
            .collect()
    }

    pub fn connect<F>(&mut self, port_id: &str, mut on_event: F) -> Result<MidiPortInfo, MidiError>
    where
        F: FnMut(MidiEvent) + Send + 'static,
    {
        self.disconnect();

        let mut input = MidiInput::new("mpd24-ai-monitor")
            .map_err(|error| MidiError::Init(error.to_string()))?;
        input.ignore(Ignore::None);
        let ports = input.ports();

        let selected = ports
            .iter()
            .enumerate()
            .find_map(|(index, port)| {
                let info = port_info(&input, index, port).ok()?;
                (info.id == port_id).then_some((port.clone(), info))
            })
            .ok_or_else(|| MidiError::PortNotFound(port_id.to_owned()))?;

        let (port, info) = selected;
        let started = Instant::now();
        let connection = input
            .connect(
                &port,
                "mpd24-ai-monitor",
                move |_midir_timestamp, bytes, _| {
                    let elapsed = started.elapsed().as_micros() as u64;
                    if let Some(event) = decode_message(elapsed, bytes) {
                        on_event(event);
                    }
                },
                (),
            )
            .map_err(|error| MidiError::Connect(error.to_string()))?;

        self.connection = Some(connection);
        Ok(info)
    }
}

fn port_info(
    input: &MidiInput,
    index: usize,
    port: &MidiInputPort,
) -> Result<MidiPortInfo, MidiError> {
    let name = input
        .port_name(port)
        .map_err(|error| MidiError::PortName(error.to_string()))?;
    Ok(MidiPortInfo {
        id: make_port_id(index, &name),
        index,
        name,
    })
}
