//! Colour of a genome and the harmony rules that keep every evolution good looking: the hue gap
//! between skin and core stays small, and the face stays readable against the skin it sits on.
//! Colours are built in OKLCH and handed out as sRGB triples in [0, 1].

use crate::genome::{Gene, Genome};

/// Largest allowed hue gap between the top of the skin and the core, in degrees.
pub const MAX_HUE_GAP: f32 = 40.0;
/// Smallest allowed contrast ratio (WCAG) of the face against the skin around it.
pub const MIN_FACE_CONTRAST: f32 = 4.5;
/// Hue travel of the core gene, in degrees each way; wider than the rule so the rule has something to do.
const CORE_TRAVEL: f32 = 55.0;
const LIGHTNESS_STEP: f32 = 0.01;

pub type Rgb = [f32; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub top: Rgb,
    pub mid: Rgb,
    pub bot: Rgb,
    pub light: Rgb,
    pub core: Rgb,
    pub face: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Harmony {
    pub hue_gap: f32,
    pub face_contrast: f32,
}

impl Harmony {
    pub fn is_good(&self) -> bool {
        self.hue_gap <= MAX_HUE_GAP + 1e-3 && self.face_contrast >= MIN_FACE_CONTRAST
    }
}

fn oklab_to_linear(l: f32, a: f32, b: f32) -> [f32; 3] {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_93 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s,
        -0.004_196_086 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

fn in_gamut(rgb: [f32; 3]) -> bool {
    rgb.iter().all(|v| (-1e-4..=1.0 + 1e-4).contains(v))
}

fn encode(linear: f32) -> f32 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// OKLCH to linear sRGB, reducing chroma until the colour fits the gamut so the hue never shifts.
fn linear_from_oklch(l: f32, c: f32, hue_deg: f32) -> [f32; 3] {
    let l = l.clamp(0.0, 1.0);
    let h = hue_deg.to_radians();
    let at = |chroma: f32| oklab_to_linear(l, chroma * h.cos(), chroma * h.sin());
    let full = at(c);
    if in_gamut(full) {
        return full;
    }
    let (mut lo, mut hi) = (0.0f32, c);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if in_gamut(at(mid)) {
            lo = mid
        } else {
            hi = mid
        }
    }
    at(lo)
}

fn srgb(l: f32, c: f32, hue_deg: f32) -> Rgb {
    let linear = linear_from_oklch(l, c, hue_deg);
    [encode(linear[0]), encode(linear[1]), encode(linear[2])]
}

fn luminance(linear: [f32; 3]) -> f32 {
    0.2126 * linear[0].clamp(0.0, 1.0)
        + 0.7152 * linear[1].clamp(0.0, 1.0)
        + 0.0722 * linear[2].clamp(0.0, 1.0)
}

fn contrast(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

struct Shades {
    hue: f32,
    core_hue: f32,
    chroma: f32,
    lightness: f32,
    core_chroma: f32,
}

fn shades(genome: &Genome) -> Shades {
    let hue = genome.get(Gene::SkinHue) * 360.0;
    let core_hue =
        (hue + (genome.get(Gene::CoreShift) * 2.0 - 1.0) * CORE_TRAVEL).rem_euclid(360.0);
    let chroma = 0.04 + 0.16 * genome.get(Gene::Saturation);
    Shades {
        hue,
        core_hue,
        chroma,
        lightness: 0.5 + 0.35 * genome.get(Gene::Lightness),
        core_chroma: chroma * (0.7 + 0.6 * genome.get(Gene::CoreVividness)),
    }
}

fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

/// Where the face sits: the middle of the skin, which is also the darkest part it overlaps.
fn face_contrast(s: &Shades) -> f32 {
    let skin = luminance(linear_from_oklch(s.lightness, s.chroma, s.hue));
    let face = luminance(linear_from_oklch(0.2, 0.04, s.hue));
    contrast(skin, face)
}

/// Offsets that turn the skin colour into the five layers of the gel, calibrated on the base slime:
/// the top is lighter and greener, the base deeper and more violet, the core darkest and most vivid.
pub fn palette(genome: &Genome) -> Palette {
    let s = shades(genome);
    Palette {
        top: srgb(s.lightness + 0.2, s.chroma * 0.55, s.hue - 23.0),
        mid: srgb(s.lightness, s.chroma, s.hue),
        bot: srgb(s.lightness - 0.16, s.chroma * 1.3, s.hue + 11.0),
        light: srgb(s.lightness + 0.28, s.chroma * 0.26, s.hue - 26.0),
        core: srgb(s.lightness - 0.226, s.core_chroma, s.core_hue),
        face: srgb(0.2, 0.04, s.hue),
    }
}

pub fn check(genome: &Genome) -> Harmony {
    let s = shades(genome);
    Harmony {
        hue_gap: hue_gap(s.hue, s.core_hue),
        face_contrast: face_contrast(&s),
    }
}

/// Brings a genome back inside the harmony rules by nudging the offending gene the shortest way.
/// Returns true when something had to move.
pub fn harmonize(genome: &mut Genome) -> bool {
    let mut moved = false;
    let limit = MAX_HUE_GAP / CORE_TRAVEL / 2.0;
    let shift = genome.get(Gene::CoreShift);
    if (shift - 0.5).abs() > limit {
        genome.set(Gene::CoreShift, 0.5 + (shift - 0.5).signum() * limit);
        moved = true;
    }
    while face_contrast(&shades(genome)) < MIN_FACE_CONTRAST && genome.get(Gene::Lightness) < 1.0 {
        genome.set(
            Gene::Lightness,
            genome.get(Gene::Lightness) + LIGHTNESS_STEP,
        );
        moved = true;
    }
    moved
}
