//! The slime's voice as pure rules: which voice suits its personality, what text is fit to be spoken, and when
//! the person has finished talking. No audio and no models here, so all of it is tested without a sound card.

mod audio;
mod endpoint;
mod hearing;
mod speech;
mod voice;

pub use audio::{finish, silence};
pub use endpoint::{level, normalize, DcBlocker, Endpointer, Heard};
pub use hearing::Hearing;
pub use speech::{sentences, speakable, speakable_in, MAX_SPOKEN};
pub use voice::{voice_for, Style, Traits, Voice};
