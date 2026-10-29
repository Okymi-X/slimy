use serde::{Deserialize, Serialize};

/// How the user wants the slime to sound; `Auto` follows its personality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Auto,
    /// Higher and a little quicker.
    Kawaii,
    /// Lower and slower.
    Calm,
}

/// The slime's personality and body, each in 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Traits {
    pub affection: f32,
    pub curiosity: f32,
    pub energy: f32,
    /// How big it is (the size gene): a small slime has a higher voice.
    pub size: f32,
}

/// A voice of the Kokoro multi-language model, with how to play it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voice {
    /// Speaker number in the model.
    pub speaker: i32,
    pub name: &'static str,
    /// Speed given to the model, 1.0 is normal.
    pub speed: f32,
    /// Playback rate applied afterwards: above 1 raises the pitch (and speeds up a little), a cheap and reliable way
    /// to make a small, cute voice.
    pub pitch: f32,
}

// Speaker numbers of `kokoro-int8-multi-lang-v1_0`.
const FRENCH: (i32, &str) = (30, "ff_siwis");
const BRIGHT: (i32, &str) = (2, "af_bella");
const WARM: (i32, &str) = (3, "af_heart");
const SOFT: (i32, &str) = (6, "af_nicole");
const AIRY: (i32, &str) = (10, "af_sky");

fn unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

/// The voice that fits a slime: the same personality always gets the same voice.
pub fn voice_for(language: &str, traits: Traits, style: Style) -> Voice {
    let (affection, curiosity, energy, size) = (
        unit(traits.affection),
        unit(traits.curiosity),
        unit(traits.energy),
        unit(traits.size),
    );
    // French has a single voice in the model; English chooses by character.
    let (speaker, name) = if language.starts_with("fr") {
        FRENCH
    } else if energy + curiosity > 1.25 {
        BRIGHT
    } else if affection > 0.65 && energy < 0.5 {
        WARM
    } else if energy < 0.35 {
        SOFT
    } else {
        AIRY
    };
    let mut pitch = 1.14 - 0.14 * size;
    let mut speed = 1.0 + 0.2 * (energy - 0.5);
    match style {
        Style::Auto => {}
        Style::Kawaii => {
            pitch += 0.05;
            speed += 0.05;
        }
        Style::Calm => {
            pitch -= 0.06;
            speed -= 0.08;
        }
    }
    Voice {
        speaker,
        name,
        speed: speed.clamp(0.85, 1.25),
        pitch: pitch.clamp(0.92, 1.22),
    }
}

impl Voice {
    /// The same voice, coloured by how the slime feels in this answer: a happy sentence is a little higher and brisker, a
    /// sleepy or sulking one lower and slower. `expression` is the face the brain chose; valence is -1..1, arousal 0..1.
    pub fn with_mood(self, expression: Option<&str>, valence: f32, arousal: f32) -> Voice {
        let (speed, pitch) = match expression.unwrap_or("calm") {
            "happy" => (1.05, 1.03),
            "joy" => (1.1, 1.05),
            "love" => (0.95, 1.02),
            "surprised" => (1.1, 1.07),
            "silly" => (1.08, 1.05),
            "sulk" => (0.9, 0.95),
            "sleepy" => (0.84, 0.94),
            _ => (1.0, 1.0),
        };
        let valence = if valence.is_finite() {
            valence.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let arousal = if arousal.is_finite() {
            arousal.clamp(0.0, 1.0)
        } else {
            0.3
        };
        // Excitement speeds speech up, a good mood lifts the pitch a touch.
        let speed = speed * (0.95 + 0.1 * arousal);
        let pitch = pitch * (1.0 + 0.025 * valence);
        Voice {
            speed: (self.speed * speed).clamp(0.8, 1.3),
            pitch: (self.pitch * pitch).clamp(0.9, 1.25),
            ..self
        }
    }
}
