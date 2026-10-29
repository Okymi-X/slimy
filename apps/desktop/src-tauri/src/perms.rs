//! Keeps the permission registry on disk and exposes it to the page. The page can grant, revoke and
//! read; the decision to allow an access is always taken here, never by the page.

use std::path::Path;
use std::sync::Mutex;

use slimy_perms::{Power, Registry};

const FILE: &str = "perms.json";

/// One registry change at a time, so two commands never overwrite each other's file.
static LOCK: Mutex<()> = Mutex::new(());

pub fn load(dir: &Path) -> Registry {
    std::fs::read_to_string(dir.join(FILE))
        .map(|t| Registry::from_json(&t))
        .unwrap_or_default()
}

fn save(dir: &Path, registry: &Registry) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let staging = dir.join("perms.json.tmp");
    std::fs::write(&staging, registry.to_json())?;
    std::fs::rename(staging, dir.join(FILE))
}

pub fn power(name: &str) -> Result<Power, String> {
    serde_json::from_value(serde_json::Value::String(name.to_string()))
        .map_err(|_| "power".to_string())
}

/// Loads, changes, stores. Returns what the closure returns.
pub fn change<T>(dir: &Path, edit: impl FnOnce(&mut Registry) -> T) -> Result<T, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = load(dir);
    let out = edit(&mut registry);
    save(dir, &registry).map_err(|_| "io".to_string())?;
    Ok(out)
}

/// What the page needs to draw the panel and the "watching" indicator.
pub fn view(dir: &Path, now: u64) -> serde_json::Value {
    let registry = load(dir);
    let active: Vec<Power> = registry.active(now).into_iter().collect();
    let sensitive: Vec<Power> = Power::ALL
        .iter()
        .copied()
        .filter(|p| p.is_sensitive())
        .collect();
    let journal: Vec<_> = registry.journal().iter().rev().take(30).collect();
    serde_json::json!({ "powers": Power::ALL, "sensitive": sensitive, "active": active, "journal": journal })
}

/// Journals the request and says whether the power may be used right now. Reading must not start before this passes.
pub fn allow(dir: &Path, power: Power, now: u64, note: &str) -> Result<(), String> {
    match change(dir, |r| r.check(power, now, note))? {
        slimy_perms::Verdict::Allowed => Ok(()),
        _ => Err("no_permission".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "slimy-perms-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_grant_survives_a_reload_and_a_missing_file_refuses_everything() {
        let dir = temp();
        assert!(load(&dir).active(0).is_empty());
        change(&dir, |r| r.grant(Power::Tool, 0, None))
            .unwrap()
            .unwrap();
        assert!(load(&dir).is_granted(Power::Tool, 5));
        change(&dir, |r| r.revoke_all()).unwrap();
        assert!(!load(&dir).is_granted(Power::Tool, 5));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn allow_refuses_without_a_grant_and_stops_after_the_time() {
        let dir = temp();
        assert_eq!(
            allow(&dir, Power::Clipboard, 5, "read").unwrap_err(),
            "no_permission"
        );
        change(&dir, |r| r.grant(Power::Clipboard, 10, Some(60)))
            .unwrap()
            .unwrap();
        assert!(allow(&dir, Power::Clipboard, 20, "read").is_ok());
        assert_eq!(
            allow(&dir, Power::Screen, 20, "read").unwrap_err(),
            "no_permission",
            "another power stays refused"
        );
        assert_eq!(
            allow(&dir, Power::Clipboard, 70, "read").unwrap_err(),
            "no_permission"
        );
        assert_eq!(load(&dir).journal().len(), 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_known_power_names_are_accepted() {
        assert_eq!(power("screen"), Ok(Power::Screen));
        assert_eq!(power("active_app"), Ok(Power::ActiveApp));
        assert!(power("root").is_err());
        assert!(power("").is_err());
    }
}
