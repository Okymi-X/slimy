//! Things the slime promised to remind its human of, as pure rules: how many, how far ahead, when one is due, what happens
//! to a daily one, and what is dropped when the computer was off for a long time. Times are Unix seconds; the page turns
//! "in 20 minutes" or "at 18:30" into one. No clock and no files here.

use serde::{Deserialize, Serialize};

pub const MAX_REMINDERS: usize = 50;
pub const MAX_TEXT: usize = 120;
/// Furthest ahead a reminder can be set, in seconds (a year).
pub const MAX_AHEAD: i64 = 366 * 86_400;
/// A reminder this late (the computer was off) is dropped instead of announced out of the blue.
pub const STALE_AFTER: i64 = 12 * 3600;
/// At most this many are announced at once.
pub const MAX_AT_ONCE: usize = 3;
const DAY: i64 = 86_400;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    pub id: u32,
    pub at: i64,
    pub text: String,
    #[serde(default)]
    pub every_day: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemindError {
    Empty,
    InThePast,
    TooFar,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminders {
    #[serde(default)]
    next_id: u32,
    #[serde(default)]
    items: Vec<Reminder>,
}

/// The text of a reminder: control characters out, spaces tidy, cut to the limit.
pub fn tidy(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flat.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TEXT)
        .collect()
}

impl Reminders {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a reminder for `at`. A minute of lateness is tolerated (the clock of the page and of the backend differ a little).
    pub fn add(
        &mut self,
        now: i64,
        at: i64,
        text: &str,
        every_day: bool,
    ) -> Result<u32, RemindError> {
        let text = tidy(text);
        if text.is_empty() {
            return Err(RemindError::Empty);
        }
        if at < now - 60 {
            return Err(RemindError::InThePast);
        }
        if at > now.saturating_add(MAX_AHEAD) {
            return Err(RemindError::TooFar);
        }
        if self.items.len() >= MAX_REMINDERS {
            return Err(RemindError::Full);
        }
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.items.push(Reminder {
            id: self.next_id,
            at,
            text,
            every_day,
        });
        Ok(self.next_id)
    }

    /// The reminders that are due now, oldest first, at most `MAX_AT_ONCE`. A one-off is removed; a daily one is set again
    /// for its next future time; one that is far too late is dropped quietly.
    pub fn due(&mut self, now: i64) -> Vec<Reminder> {
        let mut out = Vec::new();
        let mut keep = Vec::new();
        let mut items = std::mem::take(&mut self.items);
        items.sort_by_key(|r| (r.at, r.id));
        for mut r in items {
            if r.at > now || out.len() >= MAX_AT_ONCE {
                keep.push(r);
                continue;
            }
            let late = now - r.at;
            if late <= STALE_AFTER {
                out.push(r.clone());
            }
            if r.every_day {
                // the next time in the future that is a whole number of days later
                let days = late / DAY + 1;
                r.at += days * DAY;
                keep.push(r);
            }
        }
        self.items = keep;
        out
    }

    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.items.len();
        self.items.retain(|r| r.id != id);
        before != self.items.len()
    }

    /// All reminders, soonest first.
    pub fn list(&self) -> Vec<Reminder> {
        let mut items = self.items.clone();
        items.sort_by_key(|r| (r.at, r.id));
        items
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("reminders serialize")
    }

    /// A damaged file gives an empty list: nothing is announced from garbage.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str::<Reminders>(text)
            .ok()
            .filter(|r| {
                r.items.len() <= MAX_REMINDERS
                    && r.items
                        .iter()
                        .all(|i| !i.text.is_empty() && i.text.chars().count() <= MAX_TEXT)
            })
            .unwrap_or_default()
    }
}
