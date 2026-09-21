use std::f32::consts::TAU;

use crate::{PadSample, PAD_COUNT};

pub fn demo_kit(sample_rate: u32) -> [PadSample; PAD_COUNT] {
    let kick = kick(sample_rate);
    let snare = snare(sample_rate);
    let closed_hat = hat(sample_rate, 0.06);
    let open_hat = hat(sample_rate, 0.32).with_choke_group(1);
    let closed_hat = closed_hat.with_choke_group(1);
    let clap = clap(sample_rate);
    let tom_low = tom(sample_rate, 105.0);
    let tom_mid = tom(sample_rate, 145.0);
    let tom_high = tom(sample_rate, 190.0);
    let perc_low = tom(sample_rate, 260.0).with_gain(0.65);
    let perc_high = tom(sample_rate, 390.0).with_gain(0.55);

    [
        kick.clone(),
        snare.clone(),
        closed_hat.clone(),
        open_hat.clone(),
        clap.clone(),
        tom_low,
        tom_mid,
        tom_high,
        perc_low,
        perc_high,
        kick.with_gain(0.75).with_pan(-0.15),
        snare.with_gain(0.72).with_pan(0.15),
        closed_hat.with_gain(0.72).with_pan(-0.3),
        clap.with_gain(0.72).with_pan(0.3),
        noise_fx(sample_rate, 0.45).with_gain(0.5).with_pan(-0.45),
        noise_fx(sample_rate, 0.8).with_gain(0.42).with_pan(0.45),
    ]
}

fn kick(sample_rate: u32) -> PadSample {
    let duration = 0.42;
    let len = (sample_rate as f32 * duration) as usize;
    let mut phase = 0.0;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let pitch = 46.0 + 105.0 * (-t * 18.0).exp();
        phase += TAU * pitch / sample_rate as f32;
        let envelope = (-t * 9.5).exp();
        let click = if index < sample_rate as usize / 600 {
            0.18 * (1.0 - index as f32 / (sample_rate as f32 / 600.0))
        } else {
            0.0
        };
        samples.push((phase.sin() * envelope + click).clamp(-1.0, 1.0));
    }
    PadSample::new("Demo Kick", samples, sample_rate)
}

fn snare(sample_rate: u32) -> PadSample {
    let duration = 0.28;
    let len = (sample_rate as f32 * duration) as usize;
    let mut seed = 0x1234_5678_u32;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let noise = noise(&mut seed);
        let body = (TAU * 185.0 * t).sin() * (-t * 18.0).exp() * 0.34;
        let envelope = (-t * 13.5).exp();
        samples.push((noise * envelope * 0.82 + body).clamp(-1.0, 1.0));
    }
    PadSample::new("Demo Snare", samples, sample_rate)
}

fn hat(sample_rate: u32, duration: f32) -> PadSample {
    let len = (sample_rate as f32 * duration) as usize;
    let mut seed = 0x91E1_0DA5_u32;
    let mut previous = 0.0;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let raw = noise(&mut seed);
        let high_pass = raw - previous * 0.93;
        previous = raw;
        let envelope = (-t * if duration < 0.1 { 48.0 } else { 10.0 }).exp();
        samples.push((high_pass * envelope * 0.42).clamp(-1.0, 1.0));
    }
    PadSample::new(
        if duration < 0.1 {
            "Demo Closed Hat"
        } else {
            "Demo Open Hat"
        },
        samples,
        sample_rate,
    )
}

fn clap(sample_rate: u32) -> PadSample {
    let duration = 0.26;
    let len = (sample_rate as f32 * duration) as usize;
    let mut seed = 0xC1A0_2026_u32;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let burst = [0.0_f32, 0.026, 0.051]
            .iter()
            .map(|offset| {
                let local = t - offset;
                if local >= 0.0 {
                    (-local * 58.0).exp()
                } else {
                    0.0
                }
            })
            .sum::<f32>();
        samples.push((noise(&mut seed) * burst * 0.34).clamp(-1.0, 1.0));
    }
    PadSample::new("Demo Clap", samples, sample_rate)
}

fn tom(sample_rate: u32, frequency: f32) -> PadSample {
    let duration = 0.36;
    let len = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let envelope = (-t * 9.0).exp();
        let pitch_drop = frequency * (1.0 + 0.22 * (-t * 16.0).exp());
        samples.push((TAU * pitch_drop * t).sin() * envelope * 0.75);
    }
    PadSample::new(format!("Demo Tom {frequency:.0}"), samples, sample_rate)
}

fn noise_fx(sample_rate: u32, duration: f32) -> PadSample {
    let len = (sample_rate as f32 * duration) as usize;
    let mut seed = 0x0A11_CE55_u32;
    let mut samples = Vec::with_capacity(len);
    for index in 0..len {
        let t = index as f32 / sample_rate as f32;
        let envelope = (1.0 - t / duration).max(0.0).powf(2.2);
        let pulse = (TAU * (280.0 + 1400.0 * t) * t).sin() * 0.25;
        samples.push((noise(&mut seed) * 0.55 + pulse) * envelope);
    }
    PadSample::new("Demo FX", samples, sample_rate)
}

fn noise(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*seed >> 8) as f32 / 16_777_215.0) * 2.0 - 1.0
}
