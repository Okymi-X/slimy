//! When the slime speaks first. It asks little: a budget per hour, never while sulking, never twice
//! in a row, and only about what it does not know yet.

use std::collections::HashSet;

const PER_HOUR: usize = 4;
const MIN_GAP: f64 = 120.0;
const ANSWER_WAIT: f64 = 90.0;
const POSTPONE: f64 = 7200.0;
const TIMER_AFTER: f64 = 900.0;
const TIMER_EVERY: f64 = 1800.0;

pub struct Question {
    pub id: &'static str,
    /// The fact the answer is stored under.
    pub key: &'static str,
}

pub const QUESTIONS: &[Question] = &[
    Question {
        id: "color",
        key: "favorite_color",
    },
    Question {
        id: "hobby",
        key: "hobby",
    },
    Question {
        id: "job",
        key: "job",
    },
    Question {
        id: "music",
        key: "favorite_music",
    },
    Question {
        id: "wake",
        key: "wake_time",
    },
    Question {
        id: "sleep",
        key: "bed_time",
    },
    Question {
        id: "food",
        key: "favorite_food",
    },
    Question {
        id: "pet",
        key: "pet",
    },
    Question {
        id: "season",
        key: "favorite_season",
    },
    Question {
        id: "game",
        key: "favorite_game",
    },
    Question {
        id: "place",
        key: "dream_place",
    },
    Question {
        id: "fear",
        key: "fear",
    },
];

pub struct Context<'a> {
    pub now: f64,
    pub sulking: bool,
    /// The user just came back after a long absence.
    pub returned: bool,
    /// The user caressed the slime a moment ago.
    pub caressed: bool,
    pub idle_for: f64,
    pub known: &'a HashSet<String>,
}

#[derive(Default)]
pub struct Initiative {
    first_seen: Option<f64>,
    asked_at: Vec<f64>,
    pending: Option<(&'static str, f64)>,
    done: Vec<&'static str>,
    postponed: Vec<(&'static str, f64)>,
}

fn find(id: &str) -> Option<&'static Question> {
    QUESTIONS.iter().find(|q| q.id == id)
}

impl Initiative {
    pub fn decide(&mut self, c: &Context) -> Option<&'static str> {
        let first = *self.first_seen.get_or_insert(c.now);
        if c.sulking || self.pending.is_some() {
            return None;
        }
        self.asked_at.retain(|t| c.now - t < 3600.0);
        if self.asked_at.len() >= PER_HOUR
            || self.asked_at.last().is_some_and(|t| c.now - t < MIN_GAP)
        {
            return None;
        }
        let timer = c.now - first >= TIMER_AFTER
            && c.idle_for < MIN_GAP
            && self
                .asked_at
                .last()
                .is_none_or(|t| c.now - t >= TIMER_EVERY);
        if !(c.returned || c.caressed || timer) {
            return None;
        }
        let question = QUESTIONS.iter().find(|q| {
            !c.known.contains(q.key)
                && !self.done.contains(&q.id)
                && !self
                    .postponed
                    .iter()
                    .any(|(id, until)| *id == q.id && c.now < *until)
        })?;
        self.asked_at.push(c.now);
        self.pending = Some((question.id, c.now));
        Some(question.id)
    }

    pub fn answered(&mut self, id: &str, _now: f64) {
        if let Some(q) = find(id) {
            self.done.push(q.id);
        }
        self.pending = None;
    }

    /// The user said "later": no penalty, the question waits.
    pub fn declined(&mut self, id: &str, now: f64) {
        if let Some(q) = find(id) {
            self.postponed.push((q.id, now + POSTPONE));
        }
        self.pending = None;
    }

    /// A question left without any answer for too long. Returns it once, so the caller can count it as ignored.
    pub fn expired(&mut self, now: f64) -> Option<&'static str> {
        let (id, since) = self.pending?;
        if now - since < ANSWER_WAIT {
            return None;
        }
        self.pending = None;
        self.postponed.push((id, now + POSTPONE));
        Some(id)
    }

    pub fn pending(&self) -> Option<&'static str> {
        self.pending.map(|(id, _)| id)
    }
}
