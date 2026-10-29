//! How the slime feels about its human, and when it sulks. Pure state, time comes in as a parameter.

use serde::{Deserialize, Serialize};

const SULK_WINDOW: f64 = 1800.0;
const SULK_LENGTH: f64 = 2700.0;
const SULK_AFTER: usize = 2;
const RESENTMENT_HALF_LIFE: f64 = 800.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub affection: f32,
    pub trust: f32,
    pub resentment: f32,
}

impl Default for Relation {
    fn default() -> Self {
        Self {
            affection: 0.5,
            trust: 0.4,
            resentment: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Caress { seconds: f32 },
    AnsweredQuestion,
    IgnoredQuestion,
}

impl Relation {
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Caress { seconds } => {
                self.affection += 0.012 * seconds;
                self.trust += 0.003 * seconds;
                self.resentment -= 0.06 * seconds;
            }
            Event::AnsweredQuestion => {
                self.trust += 0.03;
                self.affection += 0.02;
                self.resentment -= 0.1;
            }
            Event::IgnoredQuestion => {
                self.resentment += 0.15;
                self.trust -= 0.01;
            }
        }
        self.clamp();
    }

    /// Time passing: resentment melts away on its own.
    pub fn drift(&mut self, seconds: f64) {
        self.resentment *= (-seconds / RESENTMENT_HALF_LIFE).exp() as f32;
        self.clamp();
    }

    pub fn clamped(mut self) -> Self {
        self.clamp();
        self
    }

    fn clamp(&mut self) {
        for v in [&mut self.affection, &mut self.trust, &mut self.resentment] {
            *v = if v.is_nan() { 0.5 } else { v.clamp(0.0, 1.0) };
        }
    }
}

/// The slime sulks after two unanswered questions in half an hour; a caress or time ends it.
#[derive(Debug, Clone, Default)]
pub struct Sulk {
    ignored_at: Vec<f64>,
    since: Option<f64>,
}

impl Sulk {
    /// Registers an ignored question and says whether the slime now sulks.
    pub fn ignored(&mut self, now: f64) -> bool {
        self.ignored_at.retain(|t| now - t < SULK_WINDOW);
        self.ignored_at.push(now);
        if self.ignored_at.len() >= SULK_AFTER && self.since.is_none() {
            self.since = Some(now);
        }
        self.since.is_some()
    }

    pub fn caressed(&mut self) {
        self.since = None;
        self.ignored_at.clear();
    }

    pub fn active(&self, now: f64) -> bool {
        self.since.is_some_and(|s| now - s <= SULK_LENGTH)
    }
}
