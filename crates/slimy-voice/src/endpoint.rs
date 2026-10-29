/// Loudness of a chunk of samples (root mean square, 0..=1).
pub fn level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Removes the slowly drifting part of a microphone signal. Many sound cards start with a loud offset that takes about a
/// second to settle; left in, it looks like a very noisy room.
pub struct DcBlocker {
    previous_in: f32,
    previous_out: f32,
}

impl DcBlocker {
    pub fn new() -> Self {
        DcBlocker {
            previous_in: 0.0,
            previous_out: 0.0,
        }
    }

    /// First-order high-pass (cut-off about 25 Hz at 16 kHz and 80 Hz at 48 kHz is not needed here: R = 0.995).
    pub fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            let out = *s - self.previous_in + 0.995 * self.previous_out;
            self.previous_in = *s;
            self.previous_out = out;
            *s = out;
        }
    }
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::new()
    }
}

/// What the endpointer has decided so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heard {
    /// Nothing said yet.
    Waiting,
    /// Speech is going on.
    Speaking,
    /// The person has spoken and then fallen silent: stop recording.
    Finished,
    /// Nobody spoke for too long.
    GaveUp,
    /// Talking for too long: stop anyway.
    TooLong,
}

/// The first moments of a microphone are often a click or an offset settling: they are ignored.
const SKIP_FOR: f32 = 0.25;
const NOISE_FOR: f32 = 0.3;
const MIN_THRESHOLD: f32 = 0.02;
/// Speech must be this many times louder than the room to count.
const OVER_ROOM: f32 = 2.5;
const START_FOR: f32 = 0.12;
/// A sound shorter than this in total is a click or a cough, not something said.
const MIN_SPEECH: f32 = 0.45;
const SILENCE_FOR: f32 = 1.0;
/// How long the microphone waits for the first word: a person needs a moment after pressing the button.
const WAIT_FOR: f32 = 10.0;
const LONGEST: f32 = 25.0;

/// Decides from the stream of microphone chunks when speech starts and ends. The first fraction of a second after the
/// start-up sets the noise floor, so a noisy room does not count as speech.
pub struct Endpointer {
    rate: f32,
    elapsed: f32,
    floor: f32,
    floor_samples: f32,
    warmup: Vec<f32>,
    loud_for: f32,
    quiet_for: f32,
    spoken_for: f32,
    began: Option<f32>,
    spoke: bool,
    state: Heard,
}

impl Endpointer {
    pub fn new(sample_rate: u32) -> Self {
        Endpointer {
            rate: sample_rate.max(1) as f32,
            elapsed: 0.0,
            floor: 0.0,
            floor_samples: 0.0,
            warmup: Vec::new(),
            loud_for: 0.0,
            quiet_for: 0.0,
            spoken_for: 0.0,
            began: None,
            spoke: false,
            state: Heard::Waiting,
        }
    }

    pub fn state(&self) -> Heard {
        self.state
    }

    /// Seconds from the start of the stream at which the current speech began, once it is confirmed.
    pub fn speech_began(&self) -> Option<f32> {
        self.began
    }

    /// Feeds the next chunk and returns the decision so far; once finished it stays so.
    pub fn feed(&mut self, chunk: &[f32]) -> Heard {
        if matches!(self.state, Heard::Finished | Heard::GaveUp | Heard::TooLong)
            || chunk.is_empty()
        {
            return self.state;
        }
        let seconds = chunk.len() as f32 / self.rate;
        let loudness = level(chunk);
        self.elapsed += seconds;
        if self.elapsed <= SKIP_FOR {
            return self.state;
        }
        if self.floor_samples < NOISE_FOR {
            // The median of what is heard first: one click cannot raise the floor much, and a hissing microphone is taken as it is.
            self.warmup.push(loudness);
            let mut sorted = self.warmup.clone();
            sorted.sort_by(|a, b| a.total_cmp(b));
            self.floor = sorted[sorted.len() / 2];
            self.floor_samples += seconds;
            return self.state;
        }
        let threshold = (self.floor * OVER_ROOM).max(MIN_THRESHOLD);
        if loudness > threshold {
            self.loud_for += seconds;
            self.quiet_for = 0.0;
            if self.spoke {
                self.spoken_for += seconds;
            }
            if self.loud_for >= START_FOR && !self.spoke {
                self.spoke = true;
                self.spoken_for = self.loud_for;
                self.began = Some((self.elapsed - self.loud_for).max(0.0));
                self.state = Heard::Speaking;
            }
        } else {
            self.loud_for = 0.0;
            if self.spoke {
                self.quiet_for += seconds;
            }
        }
        if self.spoke && self.quiet_for >= SILENCE_FOR && self.spoken_for < MIN_SPEECH {
            // Too short to be words: forget it and keep waiting.
            self.spoke = false;
            self.began = None;
            self.spoken_for = 0.0;
            self.quiet_for = 0.0;
        }
        self.state = if self.elapsed >= LONGEST {
            Heard::TooLong
        } else if self.spoke && self.quiet_for >= SILENCE_FOR {
            Heard::Finished
        } else if !self.spoke && self.elapsed >= WAIT_FOR {
            Heard::GaveUp
        } else if self.spoke {
            Heard::Speaking
        } else {
            Heard::Waiting
        };
        self.state
    }
}

/// Brings a quiet recording up to a comfortable level (peak about 0.9), by at most 15 times, so that a microphone set low
/// still gives the recognizer something to work with. Silence and loud recordings are left alone.
pub fn normalize(samples: &mut [f32]) {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak < 0.02 || peak >= 0.9 {
        return;
    }
    let gain = (0.9 / peak).min(15.0);
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
}
