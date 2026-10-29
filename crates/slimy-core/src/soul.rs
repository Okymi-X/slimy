use serde::{Deserialize, Serialize};

use crate::day::DayLog;
use crate::genome::{Gene, Genome, NightRecord};
use crate::relation::Relation;

pub const MAX_NAME: usize = 24;
pub const MAX_FACTS: usize = 200;

/// The gene a night moved the most, and by how much (signed): what the journal talks about.
pub fn headline(record: &NightRecord) -> Option<(Gene, f32)> {
    record
        .deltas
        .iter()
        .filter_map(|(index, delta)| Gene::from_index(usize::from(*index)).map(|g| (g, *delta)))
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
}
pub(crate) const MAX_KEY: usize = 40;
pub(crate) const MAX_VALUE: usize = 200;
const VERSION: u32 = 1;
pub(crate) const LANGUAGES: [&str; 2] = ["fr", "en"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulError {
    EmptyName,
    NameTooLong,
    BadCharacter,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tone {
    pub affection: f32,
    pub curiosity: f32,
    pub energy: f32,
    pub tenacity: f32,
}

impl Default for Tone {
    fn default() -> Self {
        Self {
            affection: 0.5,
            curiosity: 0.5,
            energy: 0.5,
            tenacity: 0.5,
        }
    }
}

impl Tone {
    fn clamped(self) -> Self {
        let unit = |v: f32| if v.is_nan() { 0.5 } else { v.clamp(0.0, 1.0) };
        Self {
            affection: unit(self.affection),
            curiosity: unit(self.curiosity),
            energy: unit(self.energy),
            tenacity: unit(self.tenacity),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub key: String,
    pub value: String,
    pub learned_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Soul {
    pub version: u32,
    pub user_name: String,
    pub slime_name: String,
    pub language: String,
    pub born_at: i64,
    pub tone: Tone,
    #[serde(default)]
    pub relation: Relation,
    /// Souls written before the genome existed load as the base slime.
    #[serde(default)]
    pub genome: Genome,
    #[serde(default)]
    pub day: DayLog,
    pub facts: Vec<Fact>,
}

fn clean(raw: &str, max: usize) -> Result<String, SoulError> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(SoulError::EmptyName);
    }
    if text.chars().any(char::is_control) {
        return Err(SoulError::BadCharacter);
    }
    if text.chars().count() > max {
        return Err(SoulError::NameTooLong);
    }
    Ok(text.to_owned())
}

fn language(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    if LANGUAGES.contains(&lower.as_str()) {
        lower
    } else {
        "en".to_owned()
    }
}

impl Soul {
    pub fn new(
        user_name: &str,
        slime_name: &str,
        language_code: &str,
        born_at: i64,
    ) -> Result<Self, SoulError> {
        Self::with_seed(
            user_name,
            slime_name,
            language_code,
            born_at,
            born_at as u64,
        )
    }

    /// A soul whose genome comes from `seed`: the page shows the hatchling before the names are known,
    /// so the same seed has to give the same slime afterwards.
    pub fn with_seed(
        user_name: &str,
        slime_name: &str,
        language_code: &str,
        born_at: i64,
        seed: u64,
    ) -> Result<Self, SoulError> {
        Ok(Self {
            version: VERSION,
            user_name: clean(user_name, MAX_NAME)?,
            slime_name: clean(slime_name, MAX_NAME)?,
            language: language(language_code),
            born_at,
            tone: Tone::default(),
            relation: Relation::default(),
            genome: Genome::newborn(seed),
            day: DayLog::default(),
            facts: Vec::new(),
        })
    }

    /// Remembers something about the user; a known key is replaced, the oldest fact goes first when full.
    /// Forgets one remembered fact. Returns whether it was there.
    pub fn forget(&mut self, key: &str) -> bool {
        let before = self.facts.len();
        self.facts.retain(|f| f.key != key);
        before != self.facts.len()
    }

    pub fn learn(&mut self, key: &str, value: &str, at: i64) -> Result<(), SoulError> {
        let key = clean(key, MAX_KEY)?;
        let value = clean(value, MAX_VALUE)?;
        self.facts.retain(|f| f.key != key);
        self.facts.push(Fact {
            key,
            value,
            learned_at: at,
        });
        if self.facts.len() > MAX_FACTS {
            self.facts.remove(0);
        }
        Ok(())
    }

    /// Closes the previous day when a new one has begun: the day lived becomes one night of the genome.
    /// The first call of a soul only opens its first day. A clock that went backwards changes nothing.
    pub fn settle_day(&mut self, today: i64) -> Option<NightRecord> {
        if today <= self.day.day {
            return None;
        }
        let night = (self.day.day != 0).then(|| {
            let seed = (self.born_at as u64) ^ (today as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            self.genome.digest(&self.day.signals(), seed).clone()
        });
        self.day = DayLog::started(today);
        night
    }

    /// Adds what the page counted to today, closing yesterday first when needed.
    pub fn record_day(&mut self, today: i64, delta: &DayLog) -> Option<NightRecord> {
        let night = self.settle_day(today);
        self.day.add(delta);
        night
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a soul always serializes")
    }

    /// Parses and sanitizes a stored soul: names are validated, tone clamped, bad facts dropped.
    pub fn from_json(json: &str) -> Result<Self, SoulError> {
        let raw: Soul = serde_json::from_str(json).map_err(|_| SoulError::Malformed)?;
        if raw.version != VERSION {
            return Err(SoulError::Malformed);
        }
        let mut soul = Soul::new(&raw.user_name, &raw.slime_name, &raw.language, raw.born_at)?;
        soul.tone = raw.tone.clamped();
        soul.relation = raw.relation.clamped();
        soul.genome = raw.genome.checked();
        soul.day = raw.day;
        for fact in raw.facts.into_iter().take(MAX_FACTS) {
            let _ = soul.learn(&fact.key, &fact.value, fact.learned_at);
        }
        Ok(soul)
    }
}

/// A night as the page needs it: gene ids, never indices.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NightReport {
    pub night: u32,
    pub mutation: Option<&'static str>,
    pub headline: Option<&'static str>,
    pub change: f32,
}

pub fn report(record: &NightRecord) -> NightReport {
    let head = headline(record);
    NightReport {
        night: record.night,
        mutation: record
            .mutation
            .and_then(|i| Gene::from_index(usize::from(i)))
            .map(Gene::id),
        headline: head.map(|(gene, _)| gene.id()),
        change: head.map_or(0.0, |(_, delta)| delta),
    }
}
