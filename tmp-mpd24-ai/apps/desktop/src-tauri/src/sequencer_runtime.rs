use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use mpd24_ai_audio_engine::{AudioCommand, AudioEngineHandle};
use mpd24_ai_sequencer::{
    LaunchQuantization, Pattern, PatternBank, PatternBankStatus, SequencerError,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPosition {
    pub step: usize,
    pub cycle: u64,
    pub active_slot: usize,
    pub queued_slot: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTick {
    pub position: StepPosition,
    pub activated_pattern: Option<Pattern>,
    pub bank_status: PatternBankStatus,
}

pub struct SequencerRuntime {
    bank: Arc<Mutex<PatternBank>>,
    stop: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
    recording: Arc<AtomicBool>,
    current_step: Arc<AtomicUsize>,
    worker: Option<JoinHandle<()>>,
}

impl Default for SequencerRuntime {
    fn default() -> Self {
        Self {
            bank: Arc::new(Mutex::new(PatternBank::default())),
            stop: Arc::new(AtomicBool::new(false)),
            playing: Arc::new(AtomicBool::new(false)),
            recording: Arc::new(AtomicBool::new(false)),
            current_step: Arc::new(AtomicUsize::new(0)),
            worker: None,
        }
    }
}

impl SequencerRuntime {
    pub fn pattern(&self) -> Result<Pattern, String> {
        self.bank
            .lock()
            .map(|bank| bank.active().clone())
            .map_err(|_| "sequencer pattern lock poisoned".to_string())
    }

    pub fn bank(&self) -> Result<PatternBank, String> {
        self.bank
            .lock()
            .map(|bank| bank.clone())
            .map_err(|_| "sequencer pattern bank lock poisoned".to_string())
    }

    pub fn bank_status(&self) -> Result<PatternBankStatus, String> {
        self.bank
            .lock()
            .map(|bank| bank.status())
            .map_err(|_| "sequencer pattern bank lock poisoned".to_string())
    }

    pub fn set_step(
        &self,
        track: usize,
        step: usize,
        active: bool,
        velocity: u8,
    ) -> Result<Pattern, String> {
        self.mutate_pattern(|pattern| pattern.set_step(track, step, active, velocity))
    }

    pub fn set_step_count(&self, total_steps: usize) -> Result<Pattern, String> {
        self.mutate_pattern(|pattern| pattern.set_step_count(total_steps))
    }

    pub fn set_bpm(&self, bpm: f64) -> Result<Pattern, String> {
        self.mutate_pattern(|pattern| pattern.set_bpm(bpm))
    }

    pub fn set_swing(&self, swing: f32) -> Result<Pattern, String> {
        self.mutate_pattern(|pattern| {
            pattern.set_swing(swing);
            Ok(())
        })
    }

    pub fn clear(&self) -> Result<Pattern, String> {
        self.mutate_pattern(|pattern| {
            pattern.clear();
            Ok(())
        })
    }

    pub fn load_demo(&self) -> Result<Pattern, String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern lock poisoned".to_string())?;
        let mut demo = Pattern::demo();
        demo.name = format!("Pattern {} · Demo", (b'A' + bank.active_slot as u8) as char);
        bank.replace_active(demo);
        Ok(bank.active().clone())
    }

    pub fn replace_pattern(&self, replacement: Pattern) -> Result<Pattern, String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern lock poisoned".to_string())?;
        bank.replace_active(replacement);
        Ok(bank.active().clone())
    }

    pub fn replace_bank(&self, replacement: PatternBank) -> Result<Pattern, String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern bank lock poisoned".to_string())?;
        *bank = replacement;
        Ok(bank.active().clone())
    }

    pub fn queue_pattern(
        &self,
        slot: usize,
        quantization: LaunchQuantization,
    ) -> Result<(Pattern, PatternBankStatus), String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern bank lock poisoned".to_string())?;
        bank.queue(slot, quantization, self.is_playing())
            .map_err(|error| error.to_string())?;
        Ok((bank.active().clone(), bank.status()))
    }

    pub fn copy_active_pattern_to(&self, slot: usize) -> Result<PatternBankStatus, String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern bank lock poisoned".to_string())?;
        bank.copy_active_to(slot)
            .map_err(|error| error.to_string())?;
        Ok(bank.status())
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Acquire)
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Acquire)
    }

    pub fn set_recording(&self, recording: bool) -> Result<bool, String> {
        if recording && !self.is_playing() {
            return Err("start the sequencer before enabling recording".into());
        }
        self.recording.store(recording, Ordering::Release);
        Ok(recording)
    }

    pub fn record_hit(&self, pad: u8, velocity: u8) -> Result<Option<Pattern>, String> {
        if !self.is_recording() {
            return Ok(None);
        }
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern lock poisoned".to_string())?;
        let step = self.current_step.load(Ordering::Acquire) % bank.active().total_steps.max(1);
        bank.active_mut()
            .set_step(pad as usize, step, true, velocity)
            .map_err(|error| error.to_string())?;
        Ok(Some(bank.active().clone()))
    }

    pub fn start<F>(&mut self, audio: AudioEngineHandle, on_step: F) -> Result<(), String>
    where
        F: Fn(RuntimeTick) + Send + 'static,
    {
        if self.is_playing() {
            return Ok(());
        }

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }

        self.stop.store(false, Ordering::Release);
        self.playing.store(true, Ordering::Release);

        let bank = Arc::clone(&self.bank);
        let stop = Arc::clone(&self.stop);
        let playing = Arc::clone(&self.playing);
        let recording = Arc::clone(&self.recording);
        let current_step = Arc::clone(&self.current_step);
        self.worker = Some(
            thread::Builder::new()
                .name("mpd24-ai-sequencer".into())
                .spawn(move || {
                    let mut step_index = 0_usize;
                    let mut cycle = 0_u64;
                    let mut absolute_step = 0_u64;
                    let mut deadline = Instant::now();

                    while !stop.load(Ordering::Acquire) {
                        let (snapshot, activated_pattern, bank_status) = match bank.lock() {
                            Ok(mut bank) => {
                                let changed = bank.commit_if_due(absolute_step);
                                (
                                    bank.active().clone(),
                                    changed.then(|| bank.active().clone()),
                                    bank.status(),
                                )
                            }
                            Err(_) => break,
                        };
                        if snapshot.total_steps == 0 {
                            break;
                        }
                        step_index %= snapshot.total_steps;
                        current_step.store(step_index, Ordering::Release);

                        for hit in snapshot.hits_for_step(step_index, cycle) {
                            let _ = audio.send(AudioCommand::Trigger {
                                pad: hit.pad,
                                velocity: hit.velocity,
                            });
                        }
                        on_step(RuntimeTick {
                            position: StepPosition {
                                step: step_index,
                                cycle,
                                active_slot: bank_status.active_slot,
                                queued_slot: bank_status.queued_slot,
                            },
                            activated_pattern,
                            bank_status,
                        });

                        let interval = snapshot.interval_after_step(step_index);
                        deadline += interval;
                        sleep_until(deadline, &stop);

                        step_index += 1;
                        absolute_step = absolute_step.wrapping_add(1);
                        if step_index >= snapshot.total_steps {
                            step_index = 0;
                            cycle = cycle.wrapping_add(1);
                            deadline = Instant::now();
                        }
                    }

                    playing.store(false, Ordering::Release);
                    recording.store(false, Ordering::Release);
                })
                .map_err(|error| error.to_string())?,
        );

        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.playing.store(false, Ordering::Release);
        self.recording.store(false, Ordering::Release);
        self.current_step.store(0, Ordering::Release);
    }

    fn mutate_pattern(
        &self,
        mutation: impl FnOnce(&mut Pattern) -> Result<(), SequencerError>,
    ) -> Result<Pattern, String> {
        let mut bank = self
            .bank
            .lock()
            .map_err(|_| "sequencer pattern lock poisoned".to_string())?;
        mutation(bank.active_mut()).map_err(|error| error.to_string())?;
        Ok(bank.active().clone())
    }
}

impl Drop for SequencerRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

fn sleep_until(deadline: Instant, stop: &AtomicBool) {
    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        let remaining = deadline.saturating_duration_since(now);
        thread::sleep(remaining.min(Duration::from_millis(3)));
    }
}
