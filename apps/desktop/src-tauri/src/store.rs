//! Where the soul lives on disk: one JSON file in the app data directory, written atomically,
//! the previous version kept as a backup.

use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;
use slimy_core::{report, DayLog, Look, NightReport, Soul};

const FILE: &str = "soul.json";
const BACKUP: &str = "soul.json.bak";

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Corrupt,
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

pub fn load(dir: &Path) -> Result<Option<Soul>, StoreError> {
    match fs::read_to_string(dir.join(FILE)) {
        Ok(text) => Soul::from_json(&text)
            .map(Some)
            .map_err(|_| StoreError::Corrupt),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save(dir: &Path, soul: &Soul) -> Result<(), StoreError> {
    fs::create_dir_all(dir)?;
    let target = dir.join(FILE);
    let staging = dir.join("soul.json.tmp");
    fs::write(&staging, soul.to_json())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o600))?;
    }
    if target.exists() {
        fs::copy(&target, dir.join(BACKUP))?;
    }
    fs::rename(&staging, &target)?;
    Ok(())
}

/// What the page needs to dress the slime: its look today and, when a night just passed, what changed.
#[derive(Debug, Serialize)]
pub struct GenomeState {
    pub look: Look,
    pub night: Option<NightReport>,
}

/// Opens the day `today`: a night that has just ended is digested into the genome and the soul saved.
/// None when there is no soul yet.
pub fn genome_state(dir: &Path, today: i64) -> Result<Option<GenomeState>, StoreError> {
    let Some(mut soul) = load(dir)? else {
        return Ok(None);
    };
    let opened = soul.day.day;
    let night = soul.settle_day(today);
    if soul.day.day != opened {
        save(dir, &soul)?;
    }
    Ok(Some(GenomeState {
        look: soul.genome.look(),
        night: night.as_ref().map(report),
    }))
}

/// Adds what the page counted to today. Returns the night report when the day changed while the app ran.
pub fn record_day(
    dir: &Path,
    today: i64,
    delta: &DayLog,
) -> Result<Option<NightReport>, StoreError> {
    let Some(mut soul) = load(dir)? else {
        return Ok(None);
    };
    let night = soul.record_day(today, delta);
    save(dir, &soul)?;
    Ok(night.as_ref().map(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("slimy-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn soul() -> Soul {
        Soul::new("Okymi", "Mochi", "fr", 42).unwrap()
    }

    #[test]
    fn a_missing_file_means_a_slime_that_is_not_born_yet() {
        assert!(load(&scratch("missing")).unwrap().is_none());
    }

    #[test]
    fn a_saved_soul_comes_back() {
        let dir = scratch("roundtrip");
        save(&dir, &soul()).unwrap();
        assert_eq!(load(&dir).unwrap().unwrap(), soul());
    }

    #[test]
    fn saving_again_keeps_the_previous_version() {
        let dir = scratch("backup");
        save(&dir, &soul()).unwrap();
        let mut next = soul();
        next.learn("job", "dev", 1).unwrap();
        save(&dir, &next).unwrap();
        let backup = fs::read_to_string(dir.join(BACKUP)).unwrap();
        assert_eq!(Soul::from_json(&backup).unwrap(), soul());
        assert_eq!(load(&dir).unwrap().unwrap(), next);
    }

    #[test]
    fn a_corrupt_file_is_reported_not_overwritten_silently() {
        let dir = scratch("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE), "{ nope").unwrap();
        assert!(matches!(load(&dir), Err(StoreError::Corrupt)));
        assert!(dir.join(FILE).exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("private");
        save(&dir, &soul()).unwrap();
        let mode = fs::metadata(dir.join(FILE)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn no_soul_means_no_genome_state() {
        assert!(genome_state(&scratch("no-genome"), 20_000)
            .unwrap()
            .is_none());
    }

    #[test]
    fn the_first_launch_opens_the_day_without_a_night() {
        let dir = scratch("first-day");
        save(&dir, &soul()).unwrap();
        let state = genome_state(&dir, 20_000).unwrap().unwrap();
        assert!(state.night.is_none());
        assert_eq!(load(&dir).unwrap().unwrap().day.day, 20_000);
    }

    #[test]
    fn a_busy_day_becomes_a_saved_night_on_the_next_launch() {
        let dir = scratch("night");
        save(&dir, &soul()).unwrap();
        genome_state(&dir, 20_000).unwrap();
        record_day(
            &dir,
            20_000,
            &DayLog {
                caress_seconds: 300.0,
                answered: 2,
                ..DayLog::default()
            },
        )
        .unwrap();
        let state = genome_state(&dir, 20_001).unwrap().unwrap();
        assert_eq!(state.night.expect("a night").night, 1);
        let reloaded = load(&dir).unwrap().unwrap();
        assert_eq!(reloaded.genome.nights, 1);
        assert_eq!(reloaded.day.day, 20_001);
        assert!(
            genome_state(&dir, 20_001).unwrap().unwrap().night.is_none(),
            "the same day digests only once"
        );
    }

    #[test]
    fn counting_the_day_never_loses_the_soul_facts() {
        let dir = scratch("keeps-facts");
        let mut with_fact = soul();
        with_fact.learn("job", "dev", 1).unwrap();
        save(&dir, &with_fact).unwrap();
        record_day(
            &dir,
            20_000,
            &DayLog {
                throws: 3,
                ..DayLog::default()
            },
        )
        .unwrap();
        let reloaded = load(&dir).unwrap().unwrap();
        assert_eq!(reloaded.facts.len(), 1);
        assert_eq!(reloaded.day.throws, 3);
    }
}
