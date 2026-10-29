use slimy_skills::{collect, parse, Skill, SkillError, MAX_BODY, MAX_SKILLS};

const GOOD: &str = "---\nname: standup\ndescription: Help me write my daily standup\n---\nAsk what I did, then summarize in three lines.\n";

#[test]
fn a_skill_is_read_from_its_header_and_body() {
    let s = parse(GOOD).unwrap();
    assert_eq!(s.name, "standup");
    assert_eq!(s.description, "Help me write my daily standup");
    assert!(s.body.starts_with("Ask what I did"));
}

#[test]
fn malformed_skills_are_refused() {
    assert_eq!(parse("no header"), Err(SkillError::NoHeader));
    assert_eq!(
        parse("---\nname: Bad Name\ndescription: x\n---\nbody"),
        Err(SkillError::BadName)
    );
    assert_eq!(
        parse("---\nname: ok\n---\nbody"),
        Err(SkillError::NoDescription)
    );
    assert_eq!(
        parse("---\nname: ok\ndescription: x\n---\n  \n"),
        Err(SkillError::EmptyBody)
    );
}

#[test]
fn a_long_body_is_cut() {
    let long = format!(
        "---\nname: a\ndescription: d\n---\n{}",
        "x".repeat(MAX_BODY * 2)
    );
    assert_eq!(parse(&long).unwrap().body.chars().count(), MAX_BODY);
}

#[test]
fn skills_are_deduplicated_and_capped() {
    let one = |n: usize| Skill {
        name: format!("s{n}"),
        description: "d".into(),
        body: "b".into(),
    };
    let mut list: Vec<Skill> = (0..30).map(one).collect();
    list.push(one(0));
    assert_eq!(collect(list).len(), MAX_SKILLS);
}
