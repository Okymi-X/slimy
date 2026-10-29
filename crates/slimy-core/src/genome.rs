//! The slime's genome: continuous genes in [0, 1] that shape its body, colour and face, plus the
//! life stage that sets how fast it can still change. Pure data: the seed comes in as a parameter,
//! and every value is bounded on the way in and after every change.

use serde::{Deserialize, Serialize};

use crate::rng::Rng;

/// Nights kept in the history, enough to replay a long life without growing a save file forever.
pub const MAX_HISTORY: usize = 400;
/// Largest move of a single gene in one night.
pub const MAX_NIGHTLY_STEP: f32 = 0.03;
/// Value a material or accessory gene takes when a mutation adds it.
pub const MUTATION_LEVEL: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Gene {
    SkinHue,
    Saturation,
    Lightness,
    Transparency,
    CoreShift,
    HighlightPosition,
    Glow,
    Viscosity,
    Size,
    Flatness,
    Bounce,
    Outline,
    EyeSize,
    EyeSpacing,
    EyeShape,
    Cheeks,
    CoreVividness,
    Bubbles,
    Stripes,
    Honey,
    Crystal,
    Fur,
    Lava,
    Stars,
    AccessoryGrowth,
}

impl Gene {
    pub const ALL: [Gene; 25] = [
        Gene::SkinHue,
        Gene::Saturation,
        Gene::Lightness,
        Gene::Transparency,
        Gene::CoreShift,
        Gene::HighlightPosition,
        Gene::Glow,
        Gene::Viscosity,
        Gene::Size,
        Gene::Flatness,
        Gene::Bounce,
        Gene::Outline,
        Gene::EyeSize,
        Gene::EyeSpacing,
        Gene::EyeShape,
        Gene::Cheeks,
        Gene::CoreVividness,
        Gene::Bubbles,
        Gene::Stripes,
        Gene::Honey,
        Gene::Crystal,
        Gene::Fur,
        Gene::Lava,
        Gene::Stars,
        Gene::AccessoryGrowth,
    ];

    /// Genes a mutation may switch on: the six materials and the accessories.
    pub const MUTABLE: [Gene; 7] = [
        Gene::Stripes,
        Gene::Honey,
        Gene::Crystal,
        Gene::Fur,
        Gene::Lava,
        Gene::Stars,
        Gene::AccessoryGrowth,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Option<Gene> {
        Gene::ALL.get(index).copied()
    }

    /// A circular gene wraps around: 0 and 1 are the same hue.
    pub fn is_circular(self) -> bool {
        self == Gene::SkinHue
    }

    /// Stable name of the gene, for the page and the journal.
    pub fn id(self) -> &'static str {
        match self {
            Gene::SkinHue => "skin_hue",
            Gene::Saturation => "saturation",
            Gene::Lightness => "lightness",
            Gene::Transparency => "transparency",
            Gene::CoreShift => "core_shift",
            Gene::HighlightPosition => "highlight_position",
            Gene::Glow => "glow",
            Gene::Viscosity => "viscosity",
            Gene::Size => "size",
            Gene::Flatness => "flatness",
            Gene::Bounce => "bounce",
            Gene::Outline => "outline",
            Gene::EyeSize => "eye_size",
            Gene::EyeSpacing => "eye_spacing",
            Gene::EyeShape => "eye_shape",
            Gene::Cheeks => "cheeks",
            Gene::CoreVividness => "core_vividness",
            Gene::Bubbles => "bubbles",
            Gene::Stripes => "stripes",
            Gene::Honey => "honey",
            Gene::Crystal => "crystal",
            Gene::Fur => "fur",
            Gene::Lava => "lava",
            Gene::Stars => "stars",
            Gene::AccessoryGrowth => "accessory_growth",
        }
    }

    /// Value of an untouched base slime.
    fn base(self) -> f32 {
        match self {
            Gene::SkinHue => 0.699,
            Gene::Saturation => 0.79,
            Gene::Lightness => 0.45,
            Gene::Transparency => 0.6,
            Gene::CoreShift => 0.61,
            Gene::HighlightPosition => 0.5,
            Gene::Glow => 0.3,
            Gene::Viscosity
            | Gene::Size
            | Gene::Flatness
            | Gene::Bounce
            | Gene::EyeSize
            | Gene::EyeSpacing
            | Gene::EyeShape => 0.5,
            Gene::Outline => 0.4,
            Gene::Cheeks => 0.5,
            Gene::CoreVividness => 0.82,
            Gene::Bubbles => 0.8,
            Gene::Stripes
            | Gene::Honey
            | Gene::Crystal
            | Gene::Fur
            | Gene::Lava
            | Gene::Stars
            | Gene::AccessoryGrowth => 0.0,
        }
    }
}

/// Distance between two values of a gene, going the short way round for circular genes.
pub fn gene_distance(gene: Gene, a: f32, b: f32) -> f32 {
    let d = (a - b).abs();
    if gene.is_circular() {
        d.min(1.0 - d)
    } else {
        d
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Baby,
    Child,
    Teen,
    Adult,
    Elder,
}

impl Stage {
    pub fn from_nights(nights: u32) -> Stage {
        match nights {
            0..=2 => Stage::Baby,
            3..=9 => Stage::Child,
            10..=24 => Stage::Teen,
            25..=99 => Stage::Adult,
            _ => Stage::Elder,
        }
    }

    /// How much of a night's push a gene still takes: the young change fast, the old slowly.
    pub fn plasticity(self) -> f32 {
        match self {
            Stage::Baby => 1.0,
            Stage::Child => 0.9,
            Stage::Teen => 0.8,
            Stage::Adult => 0.5,
            Stage::Elder => 0.3,
        }
    }

    /// Body scale of the stage: babies are small and round, never a different creature.
    pub fn size_factor(self) -> f32 {
        match self {
            Stage::Baby => 0.7,
            Stage::Child => 0.85,
            Stage::Teen => 0.95,
            Stage::Adult | Stage::Elder => 1.0,
        }
    }
}

/// One night of change, kept so the evolution can be replayed and explained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NightRecord {
    pub night: u32,
    pub seed: u64,
    /// (gene index, signed change) for every gene that moved.
    pub deltas: Vec<(u8, f32)>,
    /// Gene index switched on by a mutation, if any.
    pub mutation: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    genes: Vec<f32>,
    pub nights: u32,
    pub history: Vec<NightRecord>,
}

fn unit(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else {
        value.clamp(0.0, 1.0)
    }
}

impl Default for Genome {
    fn default() -> Self {
        Self {
            genes: Gene::ALL.iter().map(|g| g.base()).collect(),
            nights: 0,
            history: Vec::new(),
        }
    }
}

impl Genome {
    /// A newborn: the base slime nudged a little by the seed, so no two are identical.
    pub fn newborn(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut genome = Genome::default();
        for gene in Gene::ALL {
            if !Gene::MUTABLE.contains(&gene) {
                let value = genome.get(gene) + rng.between(-0.08, 0.08);
                genome.set(gene, value);
            }
        }
        genome.set(Gene::SkinHue, rng.unit());
        genome
    }

    pub fn get(&self, gene: Gene) -> f32 {
        self.genes[gene.index()]
    }

    /// Sets a gene, bounded to [0, 1]; a circular gene wraps instead.
    pub fn set(&mut self, gene: Gene, value: f32) {
        let value = if gene.is_circular() && value.is_finite() {
            value.rem_euclid(1.0)
        } else {
            unit(value, gene.base())
        };
        self.genes[gene.index()] = value;
    }

    /// Brings a genome of unknown origin into range: gene count, every value, history length and steps.
    pub fn sanitized(mut self) -> Self {
        let mut genes: Vec<f32> = Gene::ALL.iter().map(|g| g.base()).collect();
        for (slot, value) in genes.iter_mut().zip(self.genes.drain(..)) {
            *slot = unit(value, *slot);
        }
        self.genes = genes;
        if self.history.len() > MAX_HISTORY {
            let extra = self.history.len() - MAX_HISTORY;
            self.history.drain(..extra);
        }
        let known = |index: u8| Gene::from_index(usize::from(index)).is_some();
        for record in &mut self.history {
            record.deltas.retain(|(index, _)| known(*index));
            for (_, delta) in &mut record.deltas {
                *delta = if delta.is_nan() {
                    0.0
                } else {
                    delta.clamp(-1.0, 1.0)
                };
            }
            record.mutation = record.mutation.filter(|index| known(*index));
        }
        self
    }

    /// True only when nothing needs repair: right gene count, every gene in [0,1], bounded history
    /// with known gene indices and deltas in [-1,1]. What an imported genome must satisfy as is.
    pub fn is_strictly_valid(&self) -> bool {
        let known = |index: u8| Gene::from_index(usize::from(index)).is_some();
        self.genes.len() == Gene::ALL.len()
            && self
                .genes
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            && self.history.len() <= MAX_HISTORY
            && self.history.iter().all(|r| {
                r.deltas
                    .iter()
                    .all(|(i, d)| known(*i) && d.is_finite() && (-1.0..=1.0).contains(d))
                    && r.mutation.is_none_or(known)
            })
    }

    pub fn stage(&self) -> Stage {
        Stage::from_nights(self.nights)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a genome always serializes")
    }

    /// Parses a stored genome, bounds every value and applies the harmony rules.
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str::<Genome>(json)
            .ok()
            .map(Genome::checked)
    }

    /// Bounds every value and applies the harmony rules: what any genome from outside goes through.
    pub fn checked(self) -> Self {
        let mut genome = self.sanitized();
        crate::harmony::harmonize(&mut genome);
        genome
    }
}
