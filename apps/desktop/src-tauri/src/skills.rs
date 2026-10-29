//! The skills the user wrote, read from `<data>/skills/*.md`. A file that does not parse is skipped.

use slimy_skills::Skill;
use std::path::Path;

const MAX_FILE: u64 = 16 * 1024;

pub fn load(dir: &Path) -> Vec<Skill> {
    let Ok(entries) = std::fs::read_dir(dir.join("skills")) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "md")
                && p.is_file()
                && std::fs::metadata(p).is_ok_and(|m| m.len() <= MAX_FILE)
        })
        .collect();
    paths.sort();
    slimy_skills::collect(
        paths
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|t| slimy_skills::parse(&t).ok())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_valid_skill_files_are_loaded() {
        let dir = std::env::temp_dir().join(format!("slimy-skills-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("skills")).unwrap();
        std::fs::write(
            dir.join("skills/a.md"),
            "---\nname: a\ndescription: d\n---\nbody",
        )
        .unwrap();
        std::fs::write(dir.join("skills/b.md"), "garbage").unwrap();
        std::fs::write(
            dir.join("skills/c.txt"),
            "---\nname: c\ndescription: d\n---\nbody",
        )
        .unwrap();
        let skills = load(&dir);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "a");
    }
}
