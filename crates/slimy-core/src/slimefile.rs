//! The `.slime` file: the soul of a slime (identity, tone, relation, genome, history, facts) as one
//! versioned JSON document. It never carries keys, permissions, voice models or logs. Importing is
//! strict: anything out of range or unknown is refused, nothing is repaired.

use serde::{Deserialize, Serialize};

use crate::genome::Genome;
use crate::relation::Relation;
use crate::soul::{Fact, Soul, Tone, LANGUAGES, MAX_FACTS, MAX_KEY, MAX_VALUE};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_FILE_BYTES: usize = 256 * 1024;
const FORMAT: &str = "slime";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlimeFileError {
    TooLarge,
    Malformed,
    BadFormat,
    BadVersion,
    OutOfRange,
}

impl SlimeFileError {
    /// Short code the page turns into a sentence.
    pub fn code(self) -> &'static str {
        match self {
            Self::TooLarge => "slime_too_large",
            Self::Malformed => "slime_malformed",
            Self::BadFormat => "slime_bad_format",
            Self::BadVersion => "slime_bad_version",
            Self::OutOfRange => "slime_out_of_range",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    user_name: String,
    slime_name: String,
    language: String,
    born_at: i64,
    tone: Tone,
    relation: Relation,
    genome: Genome,
    facts: Vec<Fact>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    format: String,
    version: u32,
    exported_at: i64,
    soul: Body,
}

/// What the confirmation shows before anything is replaced.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub slime_name: String,
    pub nights: u32,
    pub facts: usize,
}

pub fn summary(soul: &Soul) -> Summary {
    Summary {
        slime_name: soul.slime_name.clone(),
        nights: soul.genome.nights,
        facts: soul.facts.len(),
    }
}

pub fn export(soul: &Soul, exported_at: i64) -> String {
    let file = File {
        format: FORMAT.to_owned(),
        version: FORMAT_VERSION,
        exported_at,
        soul: Body {
            user_name: soul.user_name.clone(),
            slime_name: soul.slime_name.clone(),
            language: soul.language.clone(),
            born_at: soul.born_at,
            tone: soul.tone,
            relation: soul.relation,
            genome: soul.genome.clone(),
            facts: soul.facts.clone(),
        },
    };
    serde_json::to_string_pretty(&file).expect("a slime file always serializes")
}

fn unit(v: f32) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}

pub fn import(text: &str) -> Result<Soul, SlimeFileError> {
    if text.len() > MAX_FILE_BYTES {
        return Err(SlimeFileError::TooLarge);
    }
    let file: File = serde_json::from_str(text).map_err(|_| SlimeFileError::Malformed)?;
    if file.format != FORMAT {
        return Err(SlimeFileError::BadFormat);
    }
    if file.version != FORMAT_VERSION {
        return Err(SlimeFileError::BadVersion);
    }
    let body = file.soul;
    let t = body.tone;
    let r = body.relation;
    let ranges = [
        t.affection,
        t.curiosity,
        t.energy,
        t.tenacity,
        r.affection,
        r.trust,
        r.resentment,
    ];
    if !ranges.into_iter().all(unit)
        || !body.genome.is_strictly_valid()
        || body.facts.len() > MAX_FACTS
        || !LANGUAGES.contains(&body.language.as_str())
    {
        return Err(SlimeFileError::OutOfRange);
    }
    let mut soul = Soul::new(
        &body.user_name,
        &body.slime_name,
        &body.language,
        body.born_at,
    )
    .map_err(|_| SlimeFileError::OutOfRange)?;
    if soul.user_name != body.user_name || soul.slime_name != body.slime_name {
        return Err(SlimeFileError::OutOfRange);
    }
    soul.tone = t;
    soul.relation = r;
    soul.genome = body.genome;
    for fact in body.facts {
        let fits = fact.key.chars().count() <= MAX_KEY && fact.value.chars().count() <= MAX_VALUE;
        if !fits || soul.facts.iter().any(|f| f.key == fact.key) {
            return Err(SlimeFileError::OutOfRange);
        }
        soul.learn(&fact.key, &fact.value, fact.learned_at)
            .map_err(|_| SlimeFileError::OutOfRange)?;
    }
    Ok(soul)
}
