use std::f32::consts::TAU;

use mpd24_ai_audio_engine::load_wav_mono;
use tempfile::tempdir;

#[test]
fn loads_and_downmixes_stereo_pcm_wav() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("tone.wav");
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).expect("writer");
    for index in 0..441 {
        let value = ((TAU * 440.0 * index as f32 / 44_100.0).sin() * i16::MAX as f32 * 0.5) as i16;
        writer.write_sample(value).expect("left");
        writer.write_sample(value).expect("right");
    }
    writer.finalize().expect("finalize");

    let sample = load_wav_mono(&path).expect("load wav");
    assert_eq!(sample.sample_rate, 44_100);
    assert_eq!(sample.samples.len(), 441);
    assert!(sample.samples.iter().any(|value| value.abs() > 0.1));
}
