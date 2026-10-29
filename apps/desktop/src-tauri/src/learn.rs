//! Keeps what the slime has learned about its human's habits in `learn.json` and answers the page's questions about it.
//! Only the fixed decisions of `slimy-learn` exist: the page names one, never a value.

use std::path::Path;
use std::sync::Mutex;

use slimy_learn::{Belief, Decision, Learner, DECISIONS};

const FILE: &str = "learn.json";
static LEARNER: Mutex<Option<Learner>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<Learner>> {
    LEARNER.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn decision(name: &str) -> Result<Decision, String> {
    DECISIONS
        .iter()
        .copied()
        .find(|d| d.name == name)
        .ok_or_else(|| "learn_unknown".to_string())
}

fn save(dir: &Path, learner: &Learner) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let staging = dir.join("learn.json.tmp");
    std::fs::write(&staging, learner.to_json())?;
    std::fs::rename(staging, dir.join(FILE))
}

/// Runs `work` on the learner, loading it first (from disk, or fresh) and saving afterwards when `keep` says so.
fn with<T>(
    dir: &Path,
    seed: u64,
    keep: bool,
    work: impl FnOnce(&mut Learner) -> T,
) -> Result<T, String> {
    let mut slot = lock();
    if slot.is_none() {
        *slot = Some(Learner::from_json(
            &std::fs::read_to_string(dir.join(FILE)).unwrap_or_default(),
            seed,
        ));
    }
    let learner = slot.as_mut().expect("loaded above");
    let out = work(learner);
    if keep {
        save(dir, learner).map_err(|_| "io".to_string())?;
    }
    Ok(out)
}

/// The option to use now for a decision: (index, value).
pub fn choose(dir: &Path, seed: u64, name: &str, hour: u32) -> Result<(usize, f32), String> {
    let d = decision(name)?;
    let context = slimy_learn::context_of_hour(hour);
    with(dir, seed, true, |l| {
        let arm = l.choose(&d, context);
        (arm, d.options[arm])
    })
}

pub fn reward(
    dir: &Path,
    seed: u64,
    name: &str,
    hour: u32,
    arm: usize,
    reward: f32,
) -> Result<(), String> {
    let d = decision(name)?;
    if arm >= d.options.len() {
        return Err("learn_unknown".into());
    }
    with(dir, seed, true, |l| {
        l.reward(&d, slimy_learn::context_of_hour(hour), arm, reward)
    })
}

pub fn report(dir: &Path, seed: u64) -> Result<Vec<Belief>, String> {
    with(dir, seed, false, |l| l.report())
}

pub fn reset(dir: &Path, seed: u64) -> Result<(), String> {
    with(dir, seed, true, Learner::reset)
}

#[cfg(test)]
pub fn forget_loaded() {
    *lock() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_learned_survives_a_restart_and_only_known_decisions_exist() {
        let dir = std::env::temp_dir().join(format!("slimy-learn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        forget_loaded();
        assert!(decision("make_coffee").is_err());
        assert_eq!(
            choose(&dir, 1, "remark_gap", 14).unwrap().1,
            900.0,
            "the default at first"
        );
        for _ in 0..60 {
            let (arm, _) = choose(&dir, 1, "remark_gap", 14).unwrap();
            reward(
                &dir,
                1,
                "remark_gap",
                14,
                arm,
                if arm == 3 { 0.9 } else { -0.2 },
            )
            .unwrap();
        }
        assert!(!report(&dir, 1).unwrap().is_empty());
        forget_loaded();
        let after = report(&dir, 1).unwrap();
        assert_eq!(
            after
                .iter()
                .find(|b| b.decision == "remark_gap")
                .map(|b| b.best),
            Some(2400.0),
            "learned and reloaded from disk"
        );
        assert!(reward(&dir, 1, "remark_gap", 14, 99, 1.0).is_err());
        reset(&dir, 1).unwrap();
        assert!(report(&dir, 1).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
