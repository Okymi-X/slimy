use slimy_core::{Soul, SoulError, MAX_FACTS, MAX_NAME};

fn soul() -> Soul {
    Soul::new("  Okymi ", "Mochi", "fr", 1_700_000_000).expect("valid names")
}

#[test]
fn names_are_trimmed_and_kept() {
    let s = soul();
    assert_eq!(s.user_name, "Okymi");
    assert_eq!(s.slime_name, "Mochi");
    assert_eq!(s.born_at, 1_700_000_000);
}

#[test]
fn empty_names_are_refused() {
    assert_eq!(
        Soul::new("   ", "Mochi", "fr", 0).unwrap_err(),
        SoulError::EmptyName
    );
    assert_eq!(
        Soul::new("Okymi", "", "fr", 0).unwrap_err(),
        SoulError::EmptyName
    );
}

#[test]
fn long_names_are_refused() {
    let long = "x".repeat(MAX_NAME + 1);
    assert_eq!(
        Soul::new(&long, "Mochi", "fr", 0).unwrap_err(),
        SoulError::NameTooLong
    );
    assert!(Soul::new(&"x".repeat(MAX_NAME), "Mochi", "fr", 0).is_ok());
}

#[test]
fn control_characters_are_refused() {
    assert_eq!(
        Soul::new("Oky\nmi", "Mochi", "fr", 0).unwrap_err(),
        SoulError::BadCharacter
    );
    assert_eq!(
        Soul::new("Okymi", "Mo\u{7}chi", "fr", 0).unwrap_err(),
        SoulError::BadCharacter
    );
}

#[test]
fn accents_and_symbols_are_welcome() {
    let s = Soul::new("Zoé-Lou", "Mochi 2", "fr", 0).unwrap();
    assert_eq!(s.user_name, "Zoé-Lou");
}

#[test]
fn a_soul_survives_a_round_trip() {
    let mut s = soul();
    s.learn("favorite_color", "bleu", 1_700_000_100).unwrap();
    let back = Soul::from_json(&s.to_json()).unwrap();
    assert_eq!(back, s);
}

#[test]
fn a_fact_is_replaced_by_its_key() {
    let mut s = soul();
    s.learn("job", "dev", 1).unwrap();
    s.learn("job", "artist", 2).unwrap();
    assert_eq!(s.facts.len(), 1);
    assert_eq!(s.facts[0].value, "artist");
    assert_eq!(s.facts[0].learned_at, 2);
}

#[test]
fn the_memory_is_capped() {
    let mut s = soul();
    for i in 0..MAX_FACTS + 20 {
        s.learn(&format!("k{i}"), "v", i as i64).unwrap();
    }
    assert_eq!(s.facts.len(), MAX_FACTS);
    assert_eq!(s.facts.last().unwrap().key, format!("k{}", MAX_FACTS + 19));
}

#[test]
fn a_hostile_file_is_clamped_or_refused() {
    let json = r#"{"version":1,"user_name":"A","slime_name":"B","language":"fr","born_at":5,
        "tone":{"affection":9.0,"curiosity":-4.0,"energy":0.3,"tenacity":"x"},"facts":[]}"#;
    assert_eq!(Soul::from_json(json).unwrap_err(), SoulError::Malformed);
    let json = r#"{"version":1,"user_name":"A","slime_name":"B","language":"fr","born_at":5,
        "tone":{"affection":9.0,"curiosity":-4.0,"energy":0.3,"tenacity":0.5},"facts":[]}"#;
    let s = Soul::from_json(json).unwrap();
    assert_eq!(s.tone.affection, 1.0);
    assert_eq!(s.tone.curiosity, 0.0);
    assert_eq!(
        Soul::from_json("not json").unwrap_err(),
        SoulError::Malformed
    );
    assert_eq!(
        Soul::from_json(r#"{"version":1,"user_name":"","slime_name":"B","language":"fr","born_at":0,"tone":{"affection":0.5,"curiosity":0.5,"energy":0.5,"tenacity":0.5},"facts":[]}"#).unwrap_err(),
        SoulError::EmptyName
    );
}

#[test]
fn an_unknown_language_falls_back_to_english() {
    let s = Soul::new("A", "B", "klingon", 0).unwrap();
    assert_eq!(s.language, "en");
}

#[test]
fn an_old_soul_without_feelings_gets_neutral_ones() {
    let json = r#"{"version":1,"user_name":"A","slime_name":"B","language":"fr","born_at":5,
        "tone":{"affection":0.5,"curiosity":0.5,"energy":0.5,"tenacity":0.5},"facts":[]}"#;
    let s = Soul::from_json(json).unwrap();
    assert_eq!(s.relation, slimy_core::Relation::default());
}

#[test]
fn stored_feelings_are_clamped() {
    let json = r#"{"version":1,"user_name":"A","slime_name":"B","language":"fr","born_at":5,
        "tone":{"affection":0.5,"curiosity":0.5,"energy":0.5,"tenacity":0.5},
        "relation":{"affection":7.0,"trust":-1.0,"resentment":0.2},"facts":[]}"#;
    let s = Soul::from_json(json).unwrap();
    assert_eq!(
        (
            s.relation.affection,
            s.relation.trust,
            s.relation.resentment
        ),
        (1.0, 0.0, 0.2)
    );
}
