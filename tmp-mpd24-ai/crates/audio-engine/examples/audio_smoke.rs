use std::{error::Error, thread, time::Duration};

use mpd24_ai_audio_engine::{AudioCommand, AudioEngineHandle};

fn main() -> Result<(), Box<dyn Error>> {
    let audio = AudioEngineHandle::start_default()?;
    let device = audio.device_info();
    println!(
        "Audio ready: {} / {} Hz / {} channels",
        device.name, device.sample_rate, device.channels
    );

    audio.send(AudioCommand::Trigger {
        pad: 0,
        velocity: 112,
    })?;
    thread::sleep(Duration::from_millis(220));
    audio.send(AudioCommand::Trigger {
        pad: 1,
        velocity: 104,
    })?;
    thread::sleep(Duration::from_millis(420));
    Ok(())
}
