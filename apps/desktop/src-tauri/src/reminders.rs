//! The reminders the slime promised, kept in `reminders.json`. The rules (limits, when one is due, daily ones) are in
//! `slimy-remind`; this stores them and turns its errors into short codes for the page.

use std::path::Path;
use std::sync::Mutex;

use slimy_remind::{RemindError, Reminder, Reminders};

const FILE: &str = "reminders.json";
static LOCK: Mutex<()> = Mutex::new(());

pub fn load(dir: &Path) -> Reminders {
    std::fs::read_to_string(dir.join(FILE))
        .map(|t| Reminders::from_json(&t))
        .unwrap_or_default()
}

fn save(dir: &Path, reminders: &Reminders) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let staging = dir.join("reminders.json.tmp");
    std::fs::write(&staging, reminders.to_json())?;
    std::fs::rename(staging, dir.join(FILE))
}

/// One change at a time, stored right away.
fn change<T>(dir: &Path, edit: impl FnOnce(&mut Reminders) -> T) -> Result<T, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut reminders = load(dir);
    let out = edit(&mut reminders);
    save(dir, &reminders).map_err(|_| "io".to_string())?;
    Ok(out)
}

pub fn code(e: RemindError) -> String {
    match e {
        RemindError::Empty => "reminder_empty",
        RemindError::InThePast => "reminder_past",
        RemindError::TooFar => "reminder_far",
        RemindError::Full => "reminder_full",
    }
    .to_string()
}

pub fn add(dir: &Path, now: i64, at: i64, text: &str, every_day: bool) -> Result<u32, String> {
    change(dir, |r| r.add(now, at, text, every_day))?.map_err(code)
}

pub fn due(dir: &Path, now: i64) -> Result<Vec<Reminder>, String> {
    change(dir, |r| r.due(now))
}

pub fn remove(dir: &Path, id: u32) -> Result<bool, String> {
    change(dir, |r| r.remove(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_are_stored_fire_once_and_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("slimy-remind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load(&dir).list().is_empty());
        let id = add(&dir, 1000, 1600, "Appeler maman", false).unwrap();
        assert_eq!(
            load(&dir).list()[0].text,
            "Appeler maman",
            "read back from disk"
        );
        assert_eq!(
            add(&dir, 1000, 10, "Hier", false).unwrap_err(),
            "reminder_past"
        );
        assert_eq!(
            add(&dir, 1000, 1700, " ", false).unwrap_err(),
            "reminder_empty"
        );
        assert!(due(&dir, 1500).unwrap().is_empty());
        assert_eq!(due(&dir, 1700).unwrap().len(), 1);
        assert!(due(&dir, 1800).unwrap().is_empty(), "announced once");
        assert!(!remove(&dir, id).unwrap(), "already gone");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
