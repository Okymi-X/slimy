//! Skills the user writes for the slime: short Markdown files with a name and a description in a small header,
//! then instructions. Pure parsing and limits; reading the folder is the application's job.

pub const MAX_BODY: usize = 2000;
pub const MAX_SKILLS: usize = 20;
const MAX_NAME: usize = 40;
const MAX_DESCRIPTION: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SkillError {
    NoHeader,
    BadName,
    NoDescription,
    EmptyBody,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parses `---\nname: x\ndescription: y\n---\nbody`. The body is cut to `MAX_BODY` characters.
pub fn parse(text: &str) -> Result<Skill, SkillError> {
    let rest = text.strip_prefix("---").ok_or(SkillError::NoHeader)?;
    let (header, body) = rest.split_once("\n---").ok_or(SkillError::NoHeader)?;
    let (mut name, mut description) = (None, None);
    for line in header.lines() {
        match line.split_once(':') {
            Some(("name", v)) => name = Some(v.trim().to_owned()),
            Some(("description", v)) => {
                description = Some(v.trim().chars().take(MAX_DESCRIPTION).collect::<String>())
            }
            _ => {}
        }
    }
    let name = name.filter(|n| valid_name(n)).ok_or(SkillError::BadName)?;
    let description = description
        .filter(|d| !d.is_empty())
        .ok_or(SkillError::NoDescription)?;
    let body: String = body.trim().chars().take(MAX_BODY).collect();
    if body.is_empty() {
        return Err(SkillError::EmptyBody);
    }
    Ok(Skill {
        name,
        description,
        body,
    })
}

/// Keeps the first `MAX_SKILLS` skills with distinct names.
pub fn collect(skills: Vec<Skill>) -> Vec<Skill> {
    let mut out: Vec<Skill> = Vec::new();
    for s in skills {
        if out.len() < MAX_SKILLS && !out.iter().any(|o| o.name == s.name) {
            out.push(s);
        }
    }
    out
}
