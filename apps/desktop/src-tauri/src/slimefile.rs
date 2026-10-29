//! Export and import of `.slime` files. Both live in one fixed folder (`exports/` in the data
//! directory): the page never names a path, only a plain file name that is checked here. Import is
//! two steps (preview, then apply after the user's Yes) and keeps a timestamped backup of the soul.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use slimy_core::{export, import, summary, SlimeFileError, Summary, MAX_FILE_BYTES};
use tauri::Manager;

use crate::store;

const FOLDER: &str = "exports";
const EXTENSION: &str = ".slime";
const MAX_NAME: usize = 80;

#[derive(Debug, Serialize)]
pub struct Preview {
    pub file: String,
    #[serde(flatten)]
    pub summary: Summary,
}

fn folder(dir: &Path) -> PathBuf {
    dir.join(FOLDER)
}

/// A bare file name: letters, digits, dot, dash, underscore, ending in `.slime`, no separators.
fn valid_name(name: &str) -> bool {
    name.len() > EXTENSION.len()
        && name.len() <= MAX_NAME
        && name.ends_with(EXTENSION)
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Writes the soul to a new file in the exports folder and returns its full path.
pub fn export_soul(dir: &Path, now: i64) -> Result<String, String> {
    let soul = store::load(dir)
        .map_err(|_| "io".to_string())?
        .ok_or("no_soul")?;
    let target = folder(dir);
    fs::create_dir_all(&target).map_err(|_| "io".to_string())?;
    let path = target.join(format!("slimy-{now}{EXTENSION}"));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|_| "io".to_string())?;
    file.write_all(export(&soul, now).as_bytes())
        .map_err(|_| "io".to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn read_capped(dir: &Path, name: &str) -> Result<String, String> {
    if !valid_name(name) {
        return Err("slime_bad_name".into());
    }
    let path = folder(dir).join(name);
    let meta = fs::symlink_metadata(&path).map_err(|_| "slime_missing".to_string())?;
    if !meta.is_file() {
        return Err("slime_missing".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|_| "io".to_string())?
        .take(MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "io".to_string())?;
    String::from_utf8(bytes).map_err(|_| SlimeFileError::Malformed.code().to_string())
}

/// The most recently modified `.slime` file of the exports folder, if any.
fn newest(dir: &Path) -> Option<String> {
    let mut found: Option<(std::time::SystemTime, String)> = None;
    for entry in fs::read_dir(folder(dir)).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !valid_name(&name) || !meta.is_file() {
            continue;
        }
        let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        if found.as_ref().is_none_or(|(best, _)| modified > *best) {
            found = Some((modified, name));
        }
    }
    found.map(|(_, name)| name)
}

/// Reads and validates the newest file so the page can ask Yes or No. Nothing is changed.
pub fn preview(dir: &Path) -> Result<Preview, String> {
    let file = newest(dir).ok_or("slime_missing")?;
    let soul = import(&read_capped(dir, &file)?).map_err(|e| e.code().to_string())?;
    Ok(Preview {
        file,
        summary: summary(&soul),
    })
}

/// Replaces the soul with the validated file, after copying the current soul to a timestamped backup.
pub fn apply(dir: &Path, name: &str, now: i64) -> Result<(), String> {
    let soul = import(&read_capped(dir, name)?).map_err(|e| e.code().to_string())?;
    if let Some(current) = store::load(dir).map_err(|_| "io".to_string())? {
        let backup = dir.join(format!("soul-before-import-{now}.json"));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&backup)
            .and_then(|mut f| f.write_all(current.to_json().as_bytes()))
            .map_err(|_| "io".to_string())?;
    }
    store::save(dir, &soul).map_err(|_| "io".to_string())
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Writes the soul to the exports folder and returns the file's path.
#[tauri::command]
pub fn slime_export(app: tauri::AppHandle) -> Result<String, String> {
    export_soul(&data_dir(&app)?, now_secs())
}

/// Validates the newest file of the exports folder and describes it. Changes nothing.
#[tauri::command]
pub fn slime_import_preview(app: tauri::AppHandle) -> Result<Preview, String> {
    preview(&data_dir(&app)?)
}

/// Replaces the soul with the named file once the user said Yes (the soul is backed up first).
#[tauri::command]
pub fn slime_import_apply(app: tauri::AppHandle, file: String) -> Result<(), String> {
    apply(&data_dir(&app)?, &file, now_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slimy_core::Soul;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("slimy-test-sf-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn born(dir: &Path, name: &str) {
        store::save(dir, &Soul::new("Okymi", name, "fr", 42).unwrap()).unwrap();
    }

    #[test]
    fn names_with_separators_or_dots_are_refused() {
        for bad in [
            "../a.slime",
            "a/b.slime",
            "..slime",
            ".hidden.slime",
            "a.json",
            ".slime",
            "a b.slime",
            "a\\b.slime",
        ] {
            assert!(!valid_name(bad), "{bad}");
        }
        assert!(valid_name("slimy-1700000000.slime"));
    }

    #[test]
    fn export_then_import_replaces_the_soul_and_keeps_a_backup() {
        let dir = scratch("roundtrip");
        born(&dir, "Mochi");
        let path = export_soul(&dir, 100).unwrap();
        assert!(path.ends_with("slimy-100.slime"));
        born(&dir, "Other");
        let seen = preview(&dir).unwrap();
        assert_eq!(seen.summary.slime_name, "Mochi");
        apply(&dir, &seen.file, 200).unwrap();
        assert_eq!(store::load(&dir).unwrap().unwrap().slime_name, "Mochi");
        let backup = fs::read_to_string(dir.join("soul-before-import-200.json")).unwrap();
        assert!(backup.contains("Other"));
        assert!(
            apply(&dir, &seen.file, 200).is_err(),
            "a backup is never overwritten"
        );
    }

    #[test]
    fn a_malformed_or_huge_file_changes_nothing() {
        let dir = scratch("refuse");
        born(&dir, "Mochi");
        fs::create_dir_all(folder(&dir)).unwrap();
        fs::write(folder(&dir).join("bad.slime"), "{ not json").unwrap();
        assert_eq!(apply(&dir, "bad.slime", 1).unwrap_err(), "slime_malformed");
        fs::write(
            folder(&dir).join("big.slime"),
            vec![b' '; MAX_FILE_BYTES + 10],
        )
        .unwrap();
        assert_eq!(apply(&dir, "big.slime", 1).unwrap_err(), "slime_too_large");
        assert_eq!(
            apply(&dir, "../soul.json", 1).unwrap_err(),
            "slime_bad_name"
        );
        assert_eq!(store::load(&dir).unwrap().unwrap().slime_name, "Mochi");
        assert!(!dir.join("soul-before-import-1.json").exists());
    }

    #[test]
    fn export_needs_a_soul() {
        assert_eq!(export_soul(&scratch("none"), 1).unwrap_err(), "no_soul");
    }
}
