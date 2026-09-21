use std::sync::Arc;

use mpd24_ai_audio_engine::{
    demo::demo_kit, AudioCommand, PadPlaybackParams, PadSample, SamplerCore, VelocityLayer,
};

#[test]
fn demo_kit_populates_all_sixteen_pads() {
    let sampler = SamplerCore::new(demo_kit(48_000), 48_000);
    for pad in 0..16 {
        assert!(sampler.pad_name(pad).is_some_and(|name| !name.is_empty()));
    }
}

#[test]
fn triggering_a_pad_produces_audio() {
    let mut sampler = SamplerCore::new(demo_kit(48_000), 48_000);
    sampler.apply_command(AudioCommand::Trigger {
        pad: 0,
        velocity: 127,
    });
    let mut output = vec![0.0_f32; 512 * 2];
    sampler.render(&mut output, 2);
    assert!(output.iter().any(|sample| sample.abs() > 0.001));
}

#[test]
fn velocity_changes_output_level() {
    let mut soft = SamplerCore::new(demo_kit(48_000), 48_000);
    soft.trigger(0, 32);
    let mut soft_output = vec![0.0_f32; 512 * 2];
    soft.render(&mut soft_output, 2);

    let mut hard = SamplerCore::new(demo_kit(48_000), 48_000);
    hard.trigger(0, 127);
    let mut hard_output = vec![0.0_f32; 512 * 2];
    hard.render(&mut hard_output, 2);

    let soft_peak = soft_output
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0, f32::max);
    let hard_peak = hard_output
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0, f32::max);
    assert!(hard_peak > soft_peak);
}

#[test]
fn closed_hat_chokes_open_hat() {
    let mut sampler = SamplerCore::new(demo_kit(48_000), 48_000);
    sampler.trigger(3, 110);
    assert_eq!(sampler.active_voice_count(), 1);
    sampler.trigger(2, 110);
    assert_eq!(sampler.active_voice_count(), 1);
}

#[test]
fn replacing_pad_changes_future_hits_without_breaking_active_voice() {
    let mut sampler = SamplerCore::new(demo_kit(48_000), 48_000);
    sampler.trigger(0, 127);
    assert_eq!(sampler.active_voice_count(), 1);

    sampler.apply_command(AudioCommand::ReplacePad {
        pad: 0,
        sample: PadSample::new("Replacement", vec![0.25; 128], 48_000),
    });
    assert_eq!(sampler.active_voice_count(), 1);
    assert_eq!(sampler.pad_name(0), Some("Replacement"));

    sampler.trigger(0, 127);
    assert_eq!(sampler.active_voice_count(), 2);
}

#[test]
fn per_pad_pan_changes_stereo_balance() {
    let sample = PadSample::new("Pulse", vec![1.0; 16], 48_000);
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            sample.clone()
        } else {
            PadSample::new("Silent", vec![0.0; 16], 48_000)
        }
    });
    let mut sampler = SamplerCore::new(pads, 48_000);
    sampler.apply_command(AudioCommand::SetPadPan { pad: 0, pan: -1.0 });
    sampler.trigger(0, 127);
    let mut output = vec![0.0_f32; 8];
    sampler.render(&mut output, 2);
    assert!(output[0].abs() > output[1].abs());
}

#[test]
fn reverse_starts_from_the_end_of_the_selected_region() {
    let sample = PadSample::new("Ramp", vec![0.1, 0.2, 0.3, 0.4], 48_000);
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            sample.clone()
        } else {
            PadSample::new("Silent", vec![0.0; 4], 48_000)
        }
    });

    let mut forward = SamplerCore::new(pads.clone(), 48_000);
    forward.trigger(0, 127);
    let mut forward_output = vec![0.0_f32; 2];
    forward.render(&mut forward_output, 2);

    let mut reverse = SamplerCore::new(pads, 48_000);
    reverse.apply_command(AudioCommand::SetPadPlayback {
        pad: 0,
        params: PadPlaybackParams {
            reverse: true,
            ..PadPlaybackParams::default()
        },
    });
    reverse.trigger(0, 127);
    let mut reverse_output = vec![0.0_f32; 2];
    reverse.render(&mut reverse_output, 2);

    assert!(reverse_output[0] > forward_output[0]);
}

#[test]
fn sample_start_and_end_limit_playback_region() {
    let sample = PadSample::new("Ramp", vec![0.1, 0.2, 0.3, 0.4], 48_000);
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            sample.clone()
        } else {
            PadSample::new("Silent", vec![0.0; 4], 48_000)
        }
    });
    let mut sampler = SamplerCore::new(pads, 48_000);
    sampler.apply_command(AudioCommand::SetPadPlayback {
        pad: 0,
        params: PadPlaybackParams {
            start: 0.5,
            end: 0.75,
            ..PadPlaybackParams::default()
        },
    });
    sampler.trigger(0, 127);

    let mut first = vec![0.0_f32; 2];
    sampler.render(&mut first, 2);
    assert!(first[0] > 0.15);

    let mut second = vec![0.0_f32; 2];
    sampler.render(&mut second, 2);
    assert_eq!(second[0], 0.0);
    assert_eq!(sampler.active_voice_count(), 0);
}

#[test]
fn pitch_up_reduces_voice_duration() {
    let sample = PadSample::new("Tone", vec![0.5; 64], 48_000);
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            sample.clone()
        } else {
            PadSample::new("Silent", vec![0.0; 64], 48_000)
        }
    });

    let mut normal = SamplerCore::new(pads.clone(), 48_000);
    normal.trigger(0, 127);
    let mut normal_output = vec![0.0_f32; 32 * 2];
    normal.render(&mut normal_output, 2);
    assert_eq!(normal.active_voice_count(), 1);

    let mut pitched = SamplerCore::new(pads, 48_000);
    pitched.apply_command(AudioCommand::SetPadPlayback {
        pad: 0,
        params: PadPlaybackParams {
            pitch_semitones: 12.0,
            ..PadPlaybackParams::default()
        },
    });
    pitched.trigger(0, 127);
    let mut pitched_output = vec![0.0_f32; 33 * 2];
    pitched.render(&mut pitched_output, 2);
    assert_eq!(pitched.active_voice_count(), 0);
}

#[test]
fn velocity_layers_choose_soft_and_hard_samples() {
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            PadSample::new("Fallback", vec![0.3; 8], 48_000)
        } else {
            PadSample::new("Silent", vec![0.0; 8], 48_000)
        }
    });
    let layers: Arc<[VelocityLayer]> = vec![
        VelocityLayer::new(
            1,
            63,
            vec![PadSample::new("Soft", vec![0.1; 8], 48_000)],
            false,
        ),
        VelocityLayer::new(
            64,
            127,
            vec![PadSample::new("Hard", vec![0.8; 8], 48_000)],
            false,
        ),
    ]
    .into();

    let mut soft = SamplerCore::new(pads.clone(), 48_000);
    soft.apply_command(AudioCommand::SetPadVelocityLayers {
        pad: 0,
        layers: Arc::clone(&layers),
    });
    soft.trigger(0, 63);
    let mut soft_output = vec![0.0_f32; 2];
    soft.render(&mut soft_output, 2);

    let mut hard = SamplerCore::new(pads, 48_000);
    hard.apply_command(AudioCommand::SetPadVelocityLayers { pad: 0, layers });
    hard.trigger(0, 127);
    let mut hard_output = vec![0.0_f32; 2];
    hard.render(&mut hard_output, 2);

    assert!(hard_output[0] > soft_output[0]);
}

#[test]
fn round_robin_cycles_variants_in_order() {
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            PadSample::new("Fallback", vec![0.2; 8], 48_000)
        } else {
            PadSample::new("Silent", vec![0.0; 8], 48_000)
        }
    });
    let mut sampler = SamplerCore::new(pads, 48_000);
    sampler.apply_command(AudioCommand::SetPadVelocityLayers {
        pad: 0,
        layers: vec![VelocityLayer::new(
            1,
            127,
            vec![
                PadSample::new("RR A", vec![0.1; 8], 48_000),
                PadSample::new("RR B", vec![0.8; 8], 48_000),
            ],
            true,
        )]
        .into(),
    });

    sampler.trigger(0, 127);
    let mut first = vec![0.0_f32; 2];
    sampler.render(&mut first, 2);
    sampler.apply_command(AudioCommand::StopAll);

    sampler.trigger(0, 127);
    let mut second = vec![0.0_f32; 2];
    sampler.render(&mut second, 2);
    sampler.apply_command(AudioCommand::StopAll);

    sampler.trigger(0, 127);
    let mut third = vec![0.0_f32; 2];
    sampler.render(&mut third, 2);

    assert!(second[0] > first[0]);
    assert!((third[0] - first[0]).abs() < 0.0001);
}

#[test]
fn round_robin_counters_are_independent_per_velocity_layer() {
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            PadSample::new("Fallback", vec![0.2; 8], 48_000)
        } else {
            PadSample::new("Silent", vec![0.0; 8], 48_000)
        }
    });
    let mut sampler = SamplerCore::new(pads, 48_000);
    sampler.apply_command(AudioCommand::SetPadVelocityLayers {
        pad: 0,
        layers: vec![
            VelocityLayer::new(
                1,
                63,
                vec![
                    PadSample::new("Soft A", vec![0.1; 8], 48_000),
                    PadSample::new("Soft B", vec![0.3; 8], 48_000),
                ],
                true,
            ),
            VelocityLayer::new(
                64,
                127,
                vec![
                    PadSample::new("Hard A", vec![0.6; 8], 48_000),
                    PadSample::new("Hard B", vec![0.9; 8], 48_000),
                ],
                true,
            ),
        ]
        .into(),
    });

    let render_hit = |sampler: &mut SamplerCore, velocity| {
        sampler.trigger(0, velocity);
        let mut output = vec![0.0_f32; 2];
        sampler.render(&mut output, 2);
        sampler.apply_command(AudioCommand::StopAll);
        output[0]
    };

    let soft_a = render_hit(&mut sampler, 40);
    let hard_a = render_hit(&mut sampler, 110);
    let soft_b = render_hit(&mut sampler, 40);
    let hard_b = render_hit(&mut sampler, 110);

    assert!(soft_b > soft_a);
    assert!(hard_b > hard_a);
    assert!(hard_a > soft_b);
}

#[test]
fn unmatched_velocity_falls_back_to_primary_sample() {
    let pads = std::array::from_fn(|index| {
        if index == 0 {
            PadSample::new("Fallback", vec![0.6; 8], 48_000)
        } else {
            PadSample::new("Silent", vec![0.0; 8], 48_000)
        }
    });
    let mut sampler = SamplerCore::new(pads, 48_000);
    sampler.apply_command(AudioCommand::SetPadVelocityLayers {
        pad: 0,
        layers: vec![VelocityLayer::new(
            1,
            40,
            vec![PadSample::new("Soft", vec![0.1; 8], 48_000)],
            false,
        )]
        .into(),
    });
    sampler.trigger(0, 100);
    let mut output = vec![0.0_f32; 2];
    sampler.render(&mut output, 2);
    assert!(output[0] > 0.3);
}
