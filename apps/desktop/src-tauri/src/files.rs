//! Read-only access to one folder the user declared in `files.json` (`{"root": "/path"}`). The page
//! never names the folder, and a path can never leave it: no absolute paths, no `..`, and symbolic
//! links are resolved before the check.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

const FILE: &str = "files.json";
const MAX_CHARS: usize = 2000;
const MAX_BYTES: u64 = 64 * 1024;
const MAX_ENTRIES: usize = 50;
const MAX_PATH: usize = 200;

/// The declared folder, resolved; `None` when nothing is declared or it does not exist.
pub fn root(dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dir.join(FILE)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let path = PathBuf::from(value.get("root")?.as_str()?);
    path.is_absolute().then_some(())?;
    std::fs::canonicalize(path).ok().filter(|p| p.is_dir())
}

/// Resolves `relative` inside `root`, refusing anything that would land outside.
fn inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let bad = relative.len() > MAX_PATH
        || relative.contains('\0')
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir));
    if bad {
        return Err("files_path".into());
    }
    let full =
        std::fs::canonicalize(root.join(relative)).map_err(|_| "files_missing".to_string())?;
    full.starts_with(root)
        .then_some(full)
        .ok_or_else(|| "files_path".to_string())
}

/// A folder gives its entry names (folders end with a slash); a file gives its text.
pub fn read(root: &Path, relative: &str) -> Result<String, String> {
    let full = inside(root, relative)?;
    if full.is_dir() {
        let mut names: Vec<String> = std::fs::read_dir(&full)
            .map_err(|_| "files_missing".to_string())?
            .filter_map(Result::ok)
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .map(|e| {
                format!(
                    "{}{}",
                    e.file_name().to_string_lossy(),
                    if e.path().is_dir() { "/" } else { "" }
                )
            })
            .collect();
        names.sort();
        names.truncate(MAX_ENTRIES);
        return Ok(names.join("\n"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&full)
        .and_then(|f| f.take(MAX_BYTES).read_to_end(&mut bytes))
        .map_err(|_| "files_missing".to_string())?;
    if bytes.contains(&0) {
        return Err("files_binary".into());
    }
    Ok(String::from_utf8_lossy(&bytes)
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(MAX_CHARS)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("slimy-files-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("root/sub")).unwrap();
        std::fs::write(base.join("root/a.txt"), "hello").unwrap();
        std::fs::write(base.join("root/.hidden"), "x").unwrap();
        std::fs::write(base.join("root/bin"), [1u8, 0, 2]).unwrap();
        std::fs::write(base.join("root/big.txt"), "x".repeat(10_000)).unwrap();
        std::fs::write(base.join("secret.txt"), "top secret").unwrap();
        std::os::unix::fs::symlink(base.join("secret.txt"), base.join("root/link.txt")).unwrap();
        std::os::unix::fs::symlink(&base, base.join("root/up")).unwrap();
        std::fs::write(
            base.join("files.json"),
            format!(r#"{{"root":"{}"}}"#, base.join("root").display()),
        )
        .unwrap();
        base
    }

    #[test]
    fn it_reads_files_and_lists_folders_inside_the_root() {
        let base = tree("read");
        let root = root(&base).unwrap();
        assert_eq!(read(&root, "a.txt").unwrap(), "hello");
        assert_eq!(
            read(&root, "").unwrap(),
            "a.txt\nbig.txt\nbin\nlink.txt\nsub/\nup/"
        );
        assert_eq!(read(&root, "big.txt").unwrap().chars().count(), MAX_CHARS);
        assert_eq!(read(&root, "bin").unwrap_err(), "files_binary");
        assert_eq!(read(&root, "nope").unwrap_err(), "files_missing");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn nothing_leaves_the_root() {
        let base = tree("escape");
        let root = root(&base).unwrap();
        for path in [
            "../secret.txt",
            "/etc/passwd",
            "sub/../../secret.txt",
            "link.txt",
            "up/secret.txt",
            "a\0b",
        ] {
            assert!(read(&root, path).is_err(), "{path} must be refused");
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn no_declaration_means_no_folder() {
        assert!(root(Path::new("/nonexistent-slimy-dir")).is_none());
        let base = tree("decl");
        std::fs::write(base.join("files.json"), r#"{"root":"relative/dir"}"#).unwrap();
        assert!(root(&base).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}
