//! Small acoustic template matcher, not a speech-to-text engine.
//! 24 kHz PCM is averaged down to 8 kHz, then analysed in 20 ms frames.
//! Stores only spectral fingerprints (never recordings) for the learned cue.
use alloc::{vec, vec::Vec};
#[allow(unused_imports)]
use micromath::F32Ext;

const FRAME: usize = 160;
const BANDS: usize = 8;
const MAX_FRAMES: usize = 80;
type Feature = [i8; BANDS];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Snap,
    Time,
    Learned,
}

pub struct SoundDetector {
    buffer: [f32; FRAME],
    used: usize,
    sum: i32,
    decimation: u8,
    noise: f32,
    active: bool,
    silence: usize,
    frames: Vec<Feature>,
    peak: f32,
    max_rms: f32,
    broadband: bool,
    previous_rms: f32,
    cooldown: usize,
    enrollment: usize,
    learned: Vec<Feature>,
    pub last_distance: f32,
    pub segments: u32,
    pub last_frames: usize,
}

impl Default for SoundDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundDetector {
    pub fn new() -> Self {
        Self {
            buffer: [0.; FRAME],
            used: 0,
            sum: 0,
            decimation: 0,
            noise: 80.,
            active: false,
            silence: 0,
            frames: Vec::with_capacity(MAX_FRAMES),
            peak: 0.,
            max_rms: 0.,
            broadband: false,
            previous_rms: 0.,
            cooldown: 0,
            enrollment: 0,
            learned: Vec::new(),
            last_distance: 1.,
            segments: 0,
            last_frames: 0,
        }
    }

    pub fn learn_next(&mut self) {
        self.enrollment = 500; // ten seconds to say the cue
        self.cooldown = 15; // ignore the button click itself
        self.active = false;
        self.frames.clear();
    }
    pub fn learning(&self) -> bool {
        self.enrollment > 0
    }
    pub fn has_learned(&self) -> bool {
        !self.learned.is_empty()
    }

    /// Versioned, bounded NVS blob; signed features encoded as raw bytes.
    pub fn export_template(&self) -> Vec<u8> {
        let mut bytes = vec![1, self.learned.len() as u8];
        bytes.extend(self.learned.iter().flatten().map(|v| *v as u8));
        bytes
    }
    pub fn load_template(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < 2
            || bytes[0] != 1
            || !(6..=MAX_FRAMES).contains(&(bytes[1] as usize))
            || bytes.len() != 2 + bytes[1] as usize * BANDS
        {
            return false;
        }
        self.learned = bytes[2..]
            .chunks_exact(BANDS)
            .map(|chunk| {
                let mut f = [0; BANDS];
                for (out, value) in f.iter_mut().zip(chunk) {
                    *out = *value as i8;
                }
                f
            })
            .collect();
        true
    }

    /// First microphone channel from the board's interleaved stereo stream.
    pub fn stereo(&mut self, pcm: &[i16]) -> Option<Cue> {
        let mut event = None;
        for &sample in pcm.iter().step_by(2) {
            if let Some(cue) = self.sample(sample) {
                event = Some(cue);
            }
        }
        event
    }
    pub fn sample(&mut self, sample: i16) -> Option<Cue> {
        self.sum += sample as i32;
        self.decimation += 1;
        if self.decimation != 3 {
            return None;
        }
        self.buffer[self.used] = self.sum as f32 / 3.;
        self.decimation = 0;
        self.sum = 0;
        self.used += 1;
        if self.used != FRAME {
            return None;
        }
        self.used = 0;
        self.analyse()
    }

    fn analyse(&mut self) -> Option<Cue> {
        self.enrollment = self.enrollment.saturating_sub(1);
        let mean = self.buffer.iter().sum::<f32>() / FRAME as f32;
        let mut power = 0.;
        let mut peak: f32 = 0.;
        for v in &mut self.buffer {
            *v -= mean;
            power += *v * *v;
            peak = peak.max(v.abs());
        }
        let rms = (power / FRAME as f32).sqrt();
        let loud = rms > (self.noise * 3.).max(60.);
        if self.cooldown > 0 {
            self.cooldown -= 1;
            self.previous_rms = rms;
            return None;
        }
        if !self.active && !loud {
            self.noise = (0.98 * self.noise + 0.02 * rms).clamp(20., 3000.);
            self.previous_rms = rms;
            return None;
        }
        let (feature, high_fraction) = fingerprint(&self.buffer);
        if !self.active {
            self.active = true;
            self.frames.clear();
            self.silence = 0;
            self.peak = 0.;
            self.max_rms = 0.;
            self.broadband = rms > (self.previous_rms * 4.).max(500.) && high_fraction > 0.20;
        }
        self.previous_rms = rms;
        self.peak = self.peak.max(peak);
        self.max_rms = self.max_rms.max(rms);
        if self.frames.len() < MAX_FRAMES {
            self.frames.push(feature);
        }
        self.silence = if loud { 0 } else { self.silence + 1 };
        if self.silence < 6 && self.frames.len() < MAX_FRAMES {
            return None;
        }
        self.active = false;
        let length = self.frames.len().saturating_sub(self.silence);
        self.segments = self.segments.saturating_add(1);
        self.last_frames = length;
        self.frames.truncate(length);
        let cue = if self.enrollment > 0 && (6..=65).contains(&length) {
            self.learned.clone_from(&self.frames);
            self.enrollment = 0;
            Some(Cue::Learned)
        } else if length <= 4
            && self.broadband
            && self.peak > 2200.
            && self.peak > self.max_rms * 2.2
        {
            Some(Cue::Snap)
        } else if (6..=65).contains(&length) {
            let mut distance = 1.;
            if !self.learned.is_empty() {
                distance = template_distance(&self.frames, &self.learned);
            }
            for preset in presets::TIME {
                distance = distance.min(template_distance(&self.frames, preset));
            }
            self.last_distance = distance;
            if distance < 0.22 {
                Some(Cue::Time)
            } else {
                None
            }
        } else {
            None
        };
        if cue.is_some() {
            self.cooldown = 75;
        }
        cue
    }
}

fn fingerprint(samples: &[f32; FRAME]) -> (Feature, f32) {
    let mut powers = [0f32; BANDS];
    for (band, frequency) in [200., 350., 550., 850., 1250., 1800., 2500., 3400.]
        .iter()
        .enumerate()
    {
        let coefficient = 2. * (2. * core::f32::consts::PI * frequency / 8000.).cos();
        let (mut a, mut b) = (0., 0.);
        for (i, &sample) in samples.iter().enumerate() {
            let window = 0.5 - 0.5 * (2. * core::f32::consts::PI * i as f32 / 159.).cos();
            let next = sample * window + coefficient * a - b;
            b = a;
            a = next;
        }
        powers[band] = (a * a + b * b - coefficient * a * b).max(1.);
    }
    let high = powers[5..].iter().sum::<f32>() / powers.iter().sum::<f32>();
    let logs = powers.map(|p| p.log2());
    let mean = logs.iter().sum::<f32>() / BANDS as f32;
    let norm = (logs.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / BANDS as f32)
        .sqrt()
        .max(1.);
    (
        logs.map(|v| ((v - mean) / norm * 32.).clamp(-100., 100.) as i8),
        high,
    )
}

fn template_distance(a: &[Feature], b: &[Feature]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() > b.len() * 3 || b.len() > a.len() * 3 {
        return 1.;
    }
    let mut previous = [f32::INFINITY; MAX_FRAMES + 1];
    let mut current = previous;
    previous[0] = 0.;
    for row in a {
        current[0] = f32::INFINITY;
        for (j, other) in b.iter().enumerate() {
            let cost = row
                .iter()
                .zip(other)
                .map(|(&x, &y)| (x as f32 - y as f32).abs())
                .sum::<f32>()
                / (BANDS as f32 * 64.);
            current[j + 1] = cost + previous[j].min(previous[j + 1]).min(current[j]);
        }
        core::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()] / a.len().max(b.len()) as f32
}

mod presets {
    include!("time_templates.rs");
}
