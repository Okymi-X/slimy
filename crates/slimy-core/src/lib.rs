//! The slime's inner life: its soul (who it is, what it learned), how it feels about its human,
//! and when it speaks first. Pure data and rules, no I/O, time arrives as a parameter.

mod behavior;
mod catching;
mod day;
mod genome;
mod harmony;
mod initiative;
mod look;
mod night;
mod relation;
mod rng;
mod slimefile;
mod soul;

pub use behavior::{hop_reach, Autonomy, Intent, Output, Place, Snapshot, World};
pub use catching::{level_hop, CatchPhase};
pub use day::DayLog;
pub use genome::{
    gene_distance, Gene, Genome, NightRecord, Stage, MAX_HISTORY, MAX_NIGHTLY_STEP, MUTATION_LEVEL,
};
pub use harmony::{check, harmonize, palette, Harmony, Palette, MAX_HUE_GAP, MIN_FACE_CONTRAST};
pub use initiative::{Context, Initiative, Question, QUESTIONS};
pub use look::{Colors, Look, BASE_RADIUS};
pub use night::{Signals, MUTATION_CHANCE};
pub use relation::{Event, Relation, Sulk};
pub use slimefile::{
    export, import, summary, SlimeFileError, Summary, FORMAT_VERSION, MAX_FILE_BYTES,
};
pub use soul::{headline, report, Fact, NightReport, Soul, SoulError, Tone, MAX_FACTS, MAX_NAME};
