//! The day as the slime lived it: raw counts kept by the page, turned into the five normalized
//! signals the night digests. Every count is bounded on the way in, so a hostile or buggy page can
//! only produce a strange day, never an out of range gene.

use serde::{Deserialize, Serialize};

use crate::night::Signals;

const MAX_SECONDS: f32 = 86_400.0;
const MAX_COUNT: u32 = 10_000;
/// One flush from the page cannot claim more than this much of a single count.
const MAX_FLUSH_SECONDS: f32 = 3_600.0;
const MAX_FLUSH_COUNT: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DayLog {
    /// Local day number (days since 1970-01-01 in the user's time zone); 0 means not started yet.
    pub day: i64,
    pub caress_seconds: f32,
    pub throws: u32,
    pub answered: u32,
    pub ignored: u32,
    pub messages: u32,
    pub active_seconds: f32,
    pub night_seconds: f32,
    /// Sulks that ended on their own, without a caress.
    pub calmed_alone: u32,
}

fn seconds(value: f32, cap: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, cap)
    }
}

fn squash(value: f32) -> f32 {
    value.tanh()
}

impl DayLog {
    pub fn started(day: i64) -> Self {
        Self {
            day,
            ..Self::default()
        }
    }

    /// True when nothing happened that day, so the night has nothing to digest.
    pub fn is_quiet(&self) -> bool {
        self.caress_seconds <= 0.0
            && self.throws == 0
            && self.answered == 0
            && self.ignored == 0
            && self.messages == 0
            && self.active_seconds <= 0.0
            && self.night_seconds <= 0.0
            && self.calmed_alone == 0
    }

    /// Adds what the page counted since its last report, bounded per report and per day.
    pub fn add(&mut self, delta: &DayLog) {
        let count = |have: u32, more: u32| {
            have.saturating_add(more.min(MAX_FLUSH_COUNT))
                .min(MAX_COUNT)
        };
        let time =
            |have: f32, more: f32| (have + seconds(more, MAX_FLUSH_SECONDS)).min(MAX_SECONDS);
        self.caress_seconds = time(self.caress_seconds, delta.caress_seconds);
        self.active_seconds = time(self.active_seconds, delta.active_seconds);
        self.night_seconds = time(self.night_seconds, delta.night_seconds);
        self.throws = count(self.throws, delta.throws);
        self.answered = count(self.answered, delta.answered);
        self.ignored = count(self.ignored, delta.ignored);
        self.messages = count(self.messages, delta.messages);
        self.calmed_alone = count(self.calmed_alone, delta.calmed_alone);
    }

    /// The day in the five signals of the night, each in [-1, 1]. A quiet day gives zeros.
    pub fn signals(&self) -> Signals {
        let answered = self.answered as f32;
        Signals {
            affection: squash(
                self.caress_seconds / 120.0 + answered * 0.4 - self.ignored as f32 * 0.5,
            ),
            energy: squash(self.throws as f32 * 0.25 + self.active_seconds / 10_800.0),
            curiosity: squash(answered * 0.3 + self.messages as f32 * 0.15),
            night_use: squash(self.night_seconds / 3_600.0),
            tenacity: squash(self.calmed_alone as f32 * 0.5),
        }
    }
}
