//! What the slime may do, power by power. Everything is refused until the user grants it, every
//! access (allowed or not) is written to a bounded journal, and a revoked power stops at once.
//! Pure data and rules, no I/O: time arrives as a parameter and the caller persists the registry.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Journal entries kept; the oldest are dropped first.
pub const MAX_JOURNAL: usize = 200;
/// Longest note stored with a journal entry, in characters.
pub const MAX_NOTE: usize = 120;

/// One thing the slime can ask to do. Each is granted on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Power {
    /// Read the title of the active application.
    ActiveApp,
    /// Read the clipboard text.
    Clipboard,
    /// Read the screen.
    Screen,
    /// Read files inside a folder the user chose.
    Files,
    /// Call a tool exposed by an MCP server.
    Tool,
    /// Move the real pointer.
    Pointer,
    /// Listen through the microphone.
    Microphone,
}

impl Power {
    pub const ALL: [Power; 7] = [
        Power::ActiveApp,
        Power::Clipboard,
        Power::Screen,
        Power::Files,
        Power::Tool,
        Power::Pointer,
        Power::Microphone,
    ];

    /// Powers that can only be granted for a bounded time, because they watch or act on the user.
    pub fn is_sensitive(self) -> bool {
        matches!(
            self,
            Power::Screen | Power::Clipboard | Power::Pointer | Power::Microphone
        )
    }
}

/// Why an access was decided the way it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Allowed,
    /// Never granted.
    Denied,
    /// Was granted for a while and the time is over.
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub at: u64,
    pub power: Power,
    pub verdict: Verdict,
    pub note: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Grant {
    power: Power,
    /// Seconds since the epoch when the grant ends; `None` lasts until revoked.
    until: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PermError {
    /// A sensitive power needs an end time.
    NeedsDuration(Power),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    grants: Vec<Grant>,
    #[serde(default)]
    journal: Vec<Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants a power. Sensitive powers must carry a duration in seconds; the others may omit it.
    pub fn grant(&mut self, power: Power, now: u64, seconds: Option<u64>) -> Result<(), PermError> {
        if power.is_sensitive() && seconds.is_none() {
            return Err(PermError::NeedsDuration(power));
        }
        self.grants.retain(|g| g.power != power);
        self.grants.push(Grant {
            power,
            until: seconds.map(|s| now.saturating_add(s)),
        });
        Ok(())
    }

    /// Takes a power back. Returns whether it was granted.
    pub fn revoke(&mut self, power: Power) -> bool {
        let before = self.grants.len();
        self.grants.retain(|g| g.power != power);
        before != self.grants.len()
    }

    pub fn revoke_all(&mut self) {
        self.grants.clear();
    }

    /// Whether the power could be used right now, without writing to the journal.
    pub fn is_granted(&self, power: Power, now: u64) -> bool {
        self.grants
            .iter()
            .any(|g| g.power == power && g.until.is_none_or(|u| now < u))
    }

    /// Every power currently granted, so the UI can show the "I am watching" indicator.
    pub fn active(&self, now: u64) -> BTreeSet<Power> {
        self.grants
            .iter()
            .filter(|g| g.until.is_none_or(|u| now < u))
            .map(|g| g.power)
            .collect()
    }

    /// Asks to use a power: the decision is journaled either way. The caller must read nothing
    /// unless this returns `Allowed`.
    pub fn check(&mut self, power: Power, now: u64, note: &str) -> Verdict {
        let verdict = match self.grants.iter().find(|g| g.power == power) {
            None => Verdict::Denied,
            Some(g) if g.until.is_none_or(|u| now < u) => Verdict::Allowed,
            Some(_) => Verdict::Expired,
        };
        if verdict == Verdict::Expired {
            self.grants.retain(|g| g.power != power);
        }
        self.journal.push(Entry {
            at: now,
            power,
            verdict,
            note: note.chars().take(MAX_NOTE).collect(),
        });
        if self.journal.len() > MAX_JOURNAL {
            let extra = self.journal.len() - MAX_JOURNAL;
            self.journal.drain(..extra);
        }
        verdict
    }

    pub fn journal(&self) -> &[Entry] {
        &self.journal
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a registry always serializes")
    }

    /// Loads a registry; a damaged file gives an empty one, so everything stays refused.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }
}
