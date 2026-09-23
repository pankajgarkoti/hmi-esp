use hmi_core::sounds::{Cue, SoundDetector};

fn detect(samples: &[i16]) -> Vec<Cue> {
    let mut d = SoundDetector::new();
    samples.iter().filter_map(|s| d.sample(*s)).collect()
}

#[test]
fn silence_dc_and_continuous_tone_do_not_trigger_clock() {
    assert!(detect(&vec![0; 48000]).is_empty());
    assert!(detect(&vec![5000; 48000]).is_empty());
    let mut tone = vec![0; 24000];
    tone.extend((0..24000).map(|i| (5000. * (i as f32 * 0.115).sin()) as i16));
    tone.extend(vec![0; 24000]);
    assert!(detect(&tone).is_empty());
}

#[test]
fn short_broadband_transient_is_a_snap_not_a_voice_template() {
    let mut samples = vec![0; 24000];
    let mut random = 42u32;
    for i in 0..480 {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let envelope = (-i as f32 / 90.).exp();
        samples.push(((random as i16) as f32 * envelope) as i16);
    }
    samples.extend(vec![0; 24000]);
    assert_eq!(detect(&samples), [Cue::Snap]);
}

#[test]
fn arbitrary_saved_bytes_are_rejected_without_panicking() {
    let mut d = SoundDetector::new();
    for blob in [&[][..], &[1, 255], &[0, 6], &[1, 6, 4, 5], &[1, 0]] {
        assert!(!d.load_template(blob));
    }
    assert!(!d.has_learned());
}

#[test]
fn learning_times_out_without_overwriting_previous_template() {
    let mut d = SoundDetector::new();
    let mut blob = vec![1, 6];
    blob.extend([0; 48]);
    assert!(d.load_template(&blob));
    d.learn_next();
    for _ in 0..24_000 * 11 {
        d.sample(0);
    }
    assert!(!d.learning());
    assert_eq!(d.export_template(), blob);
}
