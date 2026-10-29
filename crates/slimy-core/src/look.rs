//! What a genome looks like on screen: the values the page feeds into its slime model. The mapping
//! lives here, in Rust, so it is tested and the page only applies numbers.

use serde::Serialize;

use crate::genome::{Gene, Genome, Stage};
use crate::harmony::{palette, Rgb};

/// Radius of the base slime in pixels, which a size factor of 1 reproduces.
pub const BASE_RADIUS: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Colors {
    pub top: Rgb,
    pub mid: Rgb,
    pub bot: Rgb,
    pub light: Rgb,
    pub core: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Layers {
    pub stripes: f32,
    pub honey: f32,
    pub crystal: f32,
    pub fur: f32,
    pub lava: f32,
    pub stars: f32,
}

/// Every field is already inside the range the page's model accepts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Look {
    pub stage: &'static str,
    pub nights: u32,
    pub radius: f32,
    pub colors: Colors,
    pub layers: Layers,
    pub opacity: f32,
    pub edge_opacity: f32,
    pub glow: f32,
    pub core_strength: f32,
    pub bubbles: u8,
    pub base_width: f32,
    pub breathing_speed: f32,
    pub eye_width: f32,
    pub eye_separation: f32,
    pub eye_aspect: f32,
    pub blush: bool,
}

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Baby => "baby",
        Stage::Child => "child",
        Stage::Teen => "teen",
        Stage::Adult => "adult",
        Stage::Elder => "elder",
    }
}

impl Genome {
    pub fn look(&self) -> Look {
        let g = |gene: Gene| self.get(gene);
        let p = palette(self);
        let stage = self.stage();
        Look {
            stage: stage_name(stage),
            nights: self.nights,
            radius: BASE_RADIUS * stage.size_factor() * (0.92 + 0.16 * g(Gene::Size)),
            colors: Colors {
                top: p.top,
                mid: p.mid,
                bot: p.bot,
                light: p.light,
                core: p.core,
            },
            layers: Layers {
                stripes: g(Gene::Stripes),
                honey: g(Gene::Honey),
                crystal: g(Gene::Crystal),
                fur: g(Gene::Fur),
                lava: g(Gene::Lava),
                stars: g(Gene::Stars),
            },
            opacity: 0.9 - 0.5 * g(Gene::Transparency),
            edge_opacity: (0.75 * g(Gene::Outline)).min(0.6),
            glow: g(Gene::Glow).min(0.8),
            core_strength: 0.6 * (0.18 + g(Gene::CoreVividness)),
            bubbles: (2.0 + 12.0 * g(Gene::Bubbles)).round().min(12.0) as u8,
            base_width: 0.92 + 0.24 * g(Gene::Flatness),
            breathing_speed: 1.7 * (1.3 - 0.6 * g(Gene::Viscosity)),
            eye_width: 0.055 * (0.8 + 0.4 * g(Gene::EyeSize)),
            eye_separation: 0.17 * (0.85 + 0.3 * g(Gene::EyeSpacing)),
            eye_aspect: 1.45 * (0.8 + 0.4 * g(Gene::EyeShape)),
            blush: g(Gene::Cheeks) > 0.25,
        }
    }
}
