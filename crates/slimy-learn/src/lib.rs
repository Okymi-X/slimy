//! A tiny model of the slime's own habits that improves by itself. For each small choice the slime makes (how long to wait
//! before speaking again, how fast to talk) it keeps, per time of day, what happened after each option and, with Thompson
//! sampling, leans toward the options its human reacted well to while still trying the others now and then. Everything is
//! local, a few numbers per option, bounded to a short list of safe options written in code, and forgets slowly so it follows
//! the person when their habits change.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A choice with a fixed, safe list of options. The learner can only ever pick from `options`.
#[derive(Clone, Copy, Debug)]
pub struct Decision {
    pub name: &'static str,
    pub options: &'static [f32],
    /// The option used before anything is learned.
    pub default: usize,
}

/// Seconds the slime waits before speaking up on its own again.
pub const REMARK_GAP: Decision = Decision {
    name: "remark_gap",
    options: &[600.0, 900.0, 1500.0, 2400.0],
    default: 1,
};
/// How fast the slime talks, as a multiplier of its usual pace.
pub const VOICE_PACE: Decision = Decision {
    name: "voice_pace",
    options: &[0.92, 1.0, 1.08],
    default: 1,
};

/// Seconds the slime stays still after a walk before it may wander again.
pub const WALK_GAP: Decision = Decision {
    name: "walk_gap",
    options: &[15.0, 40.0, 90.0, 180.0],
    default: 0,
};
/// How it greets the person on their return: 0 not at all, 1 quietly, 2 cheerfully.
pub const GREETING_STYLE: Decision = Decision {
    name: "greeting_style",
    options: &[0.0, 1.0, 2.0],
    default: 1,
};

/// Every decision the learner knows.
pub const DECISIONS: [Decision; 4] = [REMARK_GAP, VOICE_PACE, WALK_GAP, GREETING_STYLE];

/// Times of day the habits are learned for separately: night, morning, afternoon, evening.
pub const CONTEXTS: u8 = 4;
/// The time-of-day bucket of an hour (0..24).
pub fn context_of_hour(hour: u32) -> u8 {
    match hour % 24 {
        0..=5 => 0,
        6..=11 => 1,
        12..=17 => 2,
        _ => 3,
    }
}

/// How many recent results each option remembers at most: older ones fade, so habits can change.
const MEMORY: f32 = 30.0;
/// Before an option has been tried it is assumed a bit better than average, so every option gets a fair first try.
const OPTIMISM: f32 = 0.25;
const PRIOR_SPREAD: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Stat {
    n: f32,
    mean: f32,
    var: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Learner {
    seed: u64,
    draws: u64,
    /// "decision:context" to the statistics of each option.
    tables: BTreeMap<String, Vec<Stat>>,
}

/// What the learner currently believes about one decision, for showing to the user.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Belief {
    pub decision: &'static str,
    pub context: u8,
    pub best: f32,
    pub tries: u32,
    /// 0..1: how sure it is that the best option is better than the next.
    pub confidence: f32,
}

fn key(d: &Decision, context: u8) -> String {
    format!("{}:{}", d.name, context % CONTEXTS)
}

impl Learner {
    pub fn new(seed: u64) -> Self {
        Learner {
            seed,
            draws: 0,
            tables: BTreeMap::new(),
        }
    }

    fn uniform(&mut self) -> f32 {
        // splitmix64 over the seed and a counter: deterministic, and the counter is saved
        self.draws += 1;
        let mut z = self
            .seed
            .wrapping_add(self.draws.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }

    fn normal(&mut self) -> f32 {
        let (a, b) = (self.uniform(), self.uniform());
        (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos()
    }

    fn table(&mut self, d: &Decision, context: u8) -> &mut Vec<Stat> {
        let table = self.tables.entry(key(d, context)).or_default();
        if table.len() != d.options.len() {
            *table = vec![Stat::default(); d.options.len()];
        }
        table
    }

    /// Picks an option (its index) for this time of day.
    pub fn choose(&mut self, d: &Decision, context: u8) -> usize {
        let stats = self.table(d, context).clone();
        if stats.iter().all(|s| s.n == 0.0) {
            return d.default.min(d.options.len() - 1);
        }
        let mut best = (d.default.min(d.options.len() - 1), f32::NEG_INFINITY);
        for (i, s) in stats.iter().enumerate() {
            let (mean, spread) = if s.n < 1.0 {
                (OPTIMISM, PRIOR_SPREAD)
            } else {
                (s.mean, ((s.var + 0.05) / s.n).sqrt())
            };
            let sample = mean + spread * self.normal();
            if sample > best.1 {
                best = (i, sample);
            }
        }
        best.0
    }

    /// Tells the learner how well option `arm` went, from -1 (badly) to 1 (very well). Unknown options are ignored.
    pub fn reward(&mut self, d: &Decision, context: u8, arm: usize, reward: f32) {
        if arm >= d.options.len() || !reward.is_finite() {
            return;
        }
        let reward = reward.clamp(-1.0, 1.0);
        let s = &mut self.table(d, context)[arm];
        s.n = (s.n + 1.0).min(MEMORY);
        let rate = 1.0 / s.n;
        let delta = reward - s.mean;
        s.mean += rate * delta;
        s.var += rate * (delta * (reward - s.mean) - s.var);
        s.var = s.var.max(0.0);
    }

    /// The option with the best average so far, or the default when nothing is known.
    pub fn best(&self, d: &Decision, context: u8) -> usize {
        match self.tables.get(&key(d, context)) {
            Some(t) if t.len() == d.options.len() && t.iter().any(|s| s.n > 0.0) => t
                .iter()
                .enumerate()
                .filter(|(_, s)| s.n > 0.0)
                .max_by(|a, b| a.1.mean.total_cmp(&b.1.mean))
                .map_or(d.default, |(i, _)| i),
            _ => d.default.min(d.options.len() - 1),
        }
    }

    /// What it has learned, for the user to see.
    pub fn report(&self) -> Vec<Belief> {
        let mut out = Vec::new();
        for d in DECISIONS {
            for context in 0..CONTEXTS {
                let Some(t) = self.tables.get(&key(&d, context)) else {
                    continue;
                };
                let tries: f32 = t.iter().map(|s| s.n).sum();
                if tries < 1.0 || t.len() != d.options.len() {
                    continue;
                }
                let best = self.best(&d, context);
                let rival = t
                    .iter()
                    .enumerate()
                    .filter(|(i, s)| *i != best && s.n > 0.0)
                    .map(|(_, s)| s.mean)
                    .fold(f32::NEG_INFINITY, f32::max);
                let gap = if rival.is_finite() {
                    t[best].mean - rival
                } else {
                    0.0
                };
                let spread = ((t[best].var + 0.05) / t[best].n.max(1.0)).sqrt();
                out.push(Belief {
                    decision: d.name,
                    context,
                    best: d.options[best],
                    tries: tries as u32,
                    confidence: (0.5 + gap / (4.0 * spread + 1e-3)).clamp(0.0, 1.0),
                });
            }
        }
        out
    }

    /// Forgets everything learned.
    pub fn reset(&mut self) {
        self.tables.clear();
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a learner serializes")
    }

    /// A damaged file gives a fresh learner with the same seed: the slime starts from its defaults, never from garbage.
    pub fn from_json(text: &str, seed: u64) -> Self {
        serde_json::from_str::<Learner>(text)
            .ok()
            .filter(|l| l.sane())
            .unwrap_or_else(|| Learner::new(seed))
    }

    fn sane(&self) -> bool {
        self.tables.values().flatten().all(|s| {
            s.n.is_finite()
                && s.mean.is_finite()
                && s.var.is_finite()
                && (0.0..=MEMORY + 0.5).contains(&s.n)
                && s.mean.abs() <= 1.0001
                && s.var >= 0.0
        }) && self.tables.len() <= DECISIONS.len() * CONTEXTS as usize
    }
}
