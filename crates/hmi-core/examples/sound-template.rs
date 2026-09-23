use hmi_core::sounds::{Cue, SoundDetector};

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("learn or detect");
    let path = args.next().expect("mono s16le 24kHz PCM path");
    let bytes = std::fs::read(path).unwrap();
    let mut detector = SoundDetector::new();
    if mode == "learn" {
        detector.learn_next();
    }
    for _ in 0..24000 {
        detector.sample(0);
    }
    for sample in bytes
        .chunks_exact(2)
        .map(|v| i16::from_le_bytes([v[0], v[1]]))
        .chain(std::iter::repeat_n(0, 24000))
    {
        if let Some(cue) = detector.sample(sample) {
            eprintln!("{cue:?}");
        }
    }
    if mode == "learn" {
        assert!(detector.has_learned(), "no utterance detected");
        let blob = detector.export_template();
        print!("&[");
        for frame in blob[2..].chunks_exact(8) {
            print!("{:?},", frame.iter().map(|v| *v as i8).collect::<Vec<_>>());
        }
        println!("],");
    } else {
        println!("distance={:.4}", detector.last_distance);
    }
    let _ = Cue::Time;
}
