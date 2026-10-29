use slimy_learn::*;

/// A pretend person whose reaction to each option is `mean` plus a little noise.
fn person(
    learner: &mut Learner,
    d: &Decision,
    context: u8,
    means: &[f32],
    trials: usize,
    noise_seed: u64,
) -> Vec<usize> {
    let mut noise = Learner::new(noise_seed);
    let mut picks = Vec::new();
    for _ in 0..trials {
        let arm = learner.choose(d, context);
        let jitter = (noise.choose(&REMARK_GAP, 0) as f32 - 1.5) * 0.25;
        learner.reward(d, context, arm, means[arm] + jitter);
        picks.push(arm);
    }
    picks
}

fn share(picks: &[usize], arm: usize) -> f32 {
    picks.iter().filter(|a| **a == arm).count() as f32 / picks.len() as f32
}

#[test]
fn nothing_is_known_at_first_so_the_defaults_are_used() {
    let mut learner = Learner::new(1);
    assert_eq!(learner.choose(&REMARK_GAP, 1), REMARK_GAP.default);
    assert_eq!(learner.choose(&VOICE_PACE, 3), VOICE_PACE.default);
    assert!(learner.report().is_empty());
}

#[test]
fn it_finds_the_option_the_person_likes_best() {
    let mut learner = Learner::new(7);
    // this person reacts best to the longest gap
    let picks = person(
        &mut learner,
        &REMARK_GAP,
        2,
        &[-0.3, 0.0, 0.2, 0.7],
        250,
        11,
    );
    assert!(
        share(&picks[200..], 3) > 0.7,
        "late picks of the best option: {:.2}",
        share(&picks[200..], 3)
    );
    assert_eq!(learner.best(&REMARK_GAP, 2), 3);
    let belief = learner
        .report()
        .into_iter()
        .find(|b| b.decision == "remark_gap" && b.context == 2)
        .expect("a belief");
    assert_eq!(belief.best, 2400.0);
    assert!(
        belief.confidence > 0.6,
        "confidence {:.2}",
        belief.confidence
    );
}

#[test]
fn it_follows_the_person_when_their_habits_change() {
    let mut learner = Learner::new(5);
    person(&mut learner, &VOICE_PACE, 1, &[0.6, 0.0, -0.4], 150, 3);
    assert_eq!(learner.best(&VOICE_PACE, 1), 0);
    let later = person(&mut learner, &VOICE_PACE, 1, &[-0.5, 0.0, 0.6], 200, 4);
    assert!(
        share(&later[120..], 2) > 0.6,
        "after the change it prefers the new best: {:.2}",
        share(&later[120..], 2)
    );
}

#[test]
fn each_time_of_day_is_learned_on_its_own() {
    let mut learner = Learner::new(9);
    person(
        &mut learner,
        &REMARK_GAP,
        0,
        &[0.8, 0.0, -0.2, -0.5],
        150,
        1,
    );
    person(
        &mut learner,
        &REMARK_GAP,
        3,
        &[-0.5, -0.2, 0.0, 0.8],
        150,
        2,
    );
    assert_eq!(learner.best(&REMARK_GAP, 0), 0);
    assert_eq!(learner.best(&REMARK_GAP, 3), 3);
    assert_eq!(
        learner.best(&REMARK_GAP, 1),
        REMARK_GAP.default,
        "an unseen time of day keeps the default"
    );
    assert_eq!(
        (
            context_of_hour(3),
            context_of_hour(8),
            context_of_hour(14),
            context_of_hour(21),
            context_of_hour(24)
        ),
        (0, 1, 2, 3, 0)
    );
}

#[test]
fn it_can_only_pick_from_its_safe_options_and_ignores_nonsense() {
    let mut learner = Learner::new(2);
    for _ in 0..300 {
        let arm = learner.choose(&REMARK_GAP, 0);
        assert!(arm < REMARK_GAP.options.len());
        learner.reward(&REMARK_GAP, 0, arm, 0.1);
    }
    learner.reward(&REMARK_GAP, 0, 99, 1.0);
    learner.reward(&REMARK_GAP, 0, 0, f32::NAN);
    learner.reward(&REMARK_GAP, 0, 0, 1e9);
    learner.reward(&REMARK_GAP, 0, 0, -1e9);
    let json = learner.to_json();
    assert!(
        Learner::from_json(&json, 2) == learner,
        "still sane after absurd rewards"
    );
}

#[test]
fn it_is_deterministic_and_survives_saving() {
    let run = |seed| {
        let mut l = Learner::new(seed);
        person(&mut l, &REMARK_GAP, 1, &[0.0, 0.5, 0.0, 0.0], 60, 8);
        l
    };
    assert_eq!(run(4), run(4));
    assert_ne!(run(4).to_json(), run(5).to_json());
    let mut saved = run(4);
    let restored = Learner::from_json(&saved.to_json(), 4);
    assert_eq!(restored, saved);
    // and goes on identically after a reload
    let mut again = restored;
    assert_eq!(saved.choose(&REMARK_GAP, 1), again.choose(&REMARK_GAP, 1));
}

#[test]
fn a_damaged_or_tampered_file_gives_a_fresh_learner() {
    assert_eq!(Learner::from_json("{ nope", 3), Learner::new(3));
    assert_eq!(Learner::from_json("", 3), Learner::new(3));
    let tampered = r#"{"seed":1,"draws":0,"tables":{"remark_gap:0":[{"n":5.0,"mean":99.0,"var":0.1},{"n":0,"mean":0,"var":0},{"n":0,"mean":0,"var":0},{"n":0,"mean":0,"var":0}]}}"#;
    assert_eq!(
        Learner::from_json(tampered, 3),
        Learner::new(3),
        "an absurd average is refused"
    );
}

#[test]
fn it_can_forget_everything() {
    let mut learner = Learner::new(6);
    person(&mut learner, &REMARK_GAP, 1, &[0.0, 0.0, 0.9, 0.0], 100, 1);
    assert!(!learner.report().is_empty());
    learner.reset();
    assert!(learner.report().is_empty());
    assert_eq!(learner.best(&REMARK_GAP, 1), REMARK_GAP.default);
}

#[test]
fn walk_and_greeting_choices_are_bounded_and_learned() {
    for d in [WALK_GAP, GREETING_STYLE] {
        assert!(DECISIONS.iter().any(|k| k.name == d.name));
        assert!(d.default < d.options.len());
    }
    let mut learner = Learner::new(9);
    person(&mut learner, &WALK_GAP, 2, &[-0.4, 0.0, 0.8, 0.0], 150, 2);
    assert_eq!(learner.best(&WALK_GAP, 2), 2);
    person(&mut learner, &GREETING_STYLE, 1, &[-0.5, 0.0, 0.7], 150, 3);
    assert_eq!(learner.best(&GREETING_STYLE, 1), 2);
    assert!(learner
        .report()
        .iter()
        .any(|b| b.decision == "walk_gap" && b.best == 90.0));
}
