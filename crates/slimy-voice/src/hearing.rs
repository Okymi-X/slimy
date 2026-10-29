use serde::{Deserialize, Deserializer, Serialize};

/// Which Whisper model the slime hears with. `Base` is the default; `Small` hears better and weighs more.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Hearing {
    #[default]
    Base,
    Small,
}

impl Hearing {
    /// The model named by `name`; anything unknown is the base model.
    pub fn parse(name: &str) -> Hearing {
        if name == "small" {
            Hearing::Small
        } else {
            Hearing::Base
        }
    }

    /// The prefix of the model files (`base-encoder.int8.onnx`).
    pub fn prefix(self) -> &'static str {
        match self {
            Hearing::Base => "base",
            Hearing::Small => "small",
        }
    }

    /// The folder the archive unpacks to.
    pub fn dir(self) -> &'static str {
        match self {
            Hearing::Base => "sherpa-onnx-whisper-base",
            Hearing::Small => "sherpa-onnx-whisper-small",
        }
    }

    /// Download size in megabytes.
    pub fn megabytes(self) -> u64 {
        match self {
            Hearing::Base => 198,
            Hearing::Small => 610,
        }
    }
}

impl<'de> Deserialize<'de> for Hearing {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Hearing::parse(&String::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_names_fall_back_to_base() {
        assert_eq!(Hearing::parse("small"), Hearing::Small);
        assert_eq!(Hearing::parse("large"), Hearing::Base);
        assert_eq!(Hearing::default(), Hearing::Base);
        assert_eq!(Hearing::Small.prefix(), "small");
    }
}
