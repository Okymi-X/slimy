//! The night: the day's signals become small, bounded moves of the genome. The model may propose a
//! summary, but only this code decides how far any gene moves.

use crate::genome::{
    gene_distance, Gene, Genome, NightRecord, Stage, MAX_HISTORY, MAX_NIGHTLY_STEP, MUTATION_LEVEL,
};
use crate::harmony::harmonize;
use crate::rng::Rng;

/// A normalized signal of 1 pushes a gene by this much before the life stage and the cap apply.
const PUSH: f32 = 0.02;
/// Chance that a night adds a material or an accessory.
pub const MUTATION_CHANCE: f32 = 0.03;
/// Skin hue the slime drifts toward when it is cared for (a warm orange) and when it is used at night (blue violet).
const WARM_HUE: f32 = 0.08;
const NIGHT_HUE: f32 = 0.72;

/// The day, normalized. Each value is in [-1, 1]: negative means the opposite (ignored questions
/// count against affection). Out of range or NaN values are bounded on the way in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Signals {
    pub affection: f32,
    pub energy: f32,
    pub curiosity: f32,
    pub night_use: f32,
    pub tenacity: f32,
}

impl Signals {
    fn bounded(self) -> Self {
        let b = |v: f32| if v.is_nan() { 0.0 } else { v.clamp(-1.0, 1.0) };
        Self {
            affection: b(self.affection),
            energy: b(self.energy),
            curiosity: b(self.curiosity),
            night_use: b(self.night_use),
            tenacity: b(self.tenacity),
        }
    }
}

fn signed_toward(gene: Gene, from: f32, target: f32) -> f32 {
    if gene.is_circular() {
        let d = (target - from).rem_euclid(1.0);
        if d <= 0.5 {
            1.0
        } else {
            -1.0
        }
    } else {
        (target - from).signum()
    }
}

/// The pushes of a day, before life stage and cap: (gene, signed amount).
fn pushes(genome: &Genome, s: &Signals) -> Vec<(Gene, f32)> {
    let hue = genome.get(Gene::SkinHue);
    let toward = |target: f32, strength: f32| {
        strength.abs() * signed_toward(Gene::SkinHue, hue, target) * strength.signum()
    };
    vec![
        (Gene::SkinHue, toward(WARM_HUE, s.affection)),
        (Gene::Cheeks, s.affection),
        (Gene::Viscosity, -s.affection),
        (Gene::Bounce, s.energy),
        (Gene::Size, -s.energy),
        (Gene::CoreVividness, s.energy),
        (Gene::EyeSize, s.curiosity),
        (Gene::Transparency, s.curiosity),
        (Gene::Bubbles, s.curiosity),
        (Gene::SkinHue, toward(NIGHT_HUE, s.night_use)),
        (Gene::Stars, s.night_use),
        (Gene::Outline, s.tenacity),
        (Gene::Crystal, s.tenacity),
        (Gene::Honey, s.tenacity),
    ]
}

/// Signed move of a gene between two values (the short way round for the hue).
fn signed_change(gene: Gene, before: f32, after: f32) -> f32 {
    let d = after - before;
    if gene.is_circular() {
        if d > 0.5 {
            d - 1.0
        } else if d < -0.5 {
            d + 1.0
        } else {
            d
        }
    } else {
        d
    }
}

impl Genome {
    /// One night: applies the day's signals, maybe a mutation, then the harmony rules. Returns the record.
    /// `seed` is stored so the night can be explained; it is the only source of chance.
    pub fn digest(&mut self, signals: &Signals, seed: u64) -> &NightRecord {
        let signals = signals.bounded();
        let before: Vec<f32> = Gene::ALL.iter().map(|g| self.get(*g)).collect();
        let plasticity = Stage::from_nights(self.nights).plasticity();

        let mut total = [0.0f32; Gene::ALL.len()];
        for (gene, amount) in pushes(self, &signals) {
            total[gene.index()] += amount * PUSH * plasticity;
        }
        for gene in Gene::ALL {
            let step = total[gene.index()].clamp(-MAX_NIGHTLY_STEP, MAX_NIGHTLY_STEP);
            if step != 0.0 {
                self.set(gene, before[gene.index()] + step);
            }
        }

        let mut rng = Rng::new(seed);
        let mutation = (rng.unit() < MUTATION_CHANCE).then(|| {
            let absent: Vec<Gene> = Gene::MUTABLE
                .into_iter()
                .filter(|g| self.get(*g) < MUTATION_LEVEL)
                .collect();
            (!absent.is_empty())
                .then(|| absent[(rng.unit() * absent.len() as f32) as usize % absent.len()])
        });
        let mutation = mutation.flatten();
        if let Some(gene) = mutation {
            self.set(gene, MUTATION_LEVEL);
        }

        harmonize(self);

        let deltas = Gene::ALL
            .iter()
            .filter_map(|g| {
                let d = signed_change(*g, before[g.index()], self.get(*g));
                (d.abs() > 1e-7).then_some((g.index() as u8, d))
            })
            .collect();
        self.nights += 1;
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(NightRecord {
            night: self.nights,
            seed,
            deltas,
            mutation: mutation.map(|g| g.index() as u8),
        });
        self.history.last().expect("a record was just pushed")
    }

    /// Rebuilds a genome by applying the recorded deltas to its starting point: the evolution, replayed.
    pub fn replayed(start: &Genome, history: &[NightRecord]) -> Genome {
        let mut genome = start.clone();
        for record in history {
            for (index, delta) in &record.deltas {
                if let Some(gene) = Gene::from_index(usize::from(*index)) {
                    genome.set(gene, genome.get(gene) + delta);
                }
            }
            genome.nights += 1;
        }
        genome.history = history.to_vec();
        genome
    }

    /// Largest distance between two genomes over all genes, for tests and for explaining a night.
    pub fn farthest_gene(&self, other: &Genome) -> f32 {
        Gene::ALL
            .iter()
            .map(|g| gene_distance(*g, self.get(*g), other.get(*g)))
            .fold(0.0, f32::max)
    }
}
