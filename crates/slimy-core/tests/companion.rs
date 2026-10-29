use std::collections::HashSet;

use slimy_core::{Context, Event, Initiative, Relation, Sulk, QUESTIONS};

fn known(keys: &[&str]) -> HashSet<String> {
    keys.iter().map(|k| (*k).to_owned()).collect()
}

fn ctx(now: f64) -> Context<'static> {
    // Leaked on purpose: tests only, keeps the helper short.
    Context {
        now,
        sulking: false,
        returned: false,
        caressed: false,
        idle_for: 0.0,
        known: Box::leak(Box::new(HashSet::new())),
    }
}

#[test]
fn a_caress_brings_affection_and_calms_resentment() {
    let mut r = Relation {
        affection: 0.5,
        trust: 0.5,
        resentment: 0.4,
    };
    r.apply(Event::Caress { seconds: 3.0 });
    assert!(r.affection > 0.5 && r.resentment < 0.4);
}

#[test]
fn feelings_stay_between_zero_and_one() {
    let mut r = Relation::default();
    for _ in 0..500 {
        r.apply(Event::Caress { seconds: 10.0 });
        r.apply(Event::IgnoredQuestion);
    }
    for v in [r.affection, r.trust, r.resentment] {
        assert!((0.0..=1.0).contains(&v));
    }
}

#[test]
fn resentment_fades_with_time() {
    let mut r = Relation {
        affection: 0.5,
        trust: 0.5,
        resentment: 0.8,
    };
    r.drift(600.0);
    assert!(r.resentment < 0.5);
}

#[test]
fn two_ignored_questions_in_half_an_hour_start_a_sulk() {
    let mut s = Sulk::default();
    assert!(!s.ignored(0.0));
    assert!(s.ignored(600.0));
}

#[test]
fn ignored_questions_far_apart_do_not_sulk() {
    let mut s = Sulk::default();
    s.ignored(0.0);
    assert!(!s.ignored(2400.0));
}

#[test]
fn a_caress_ends_the_sulk_and_so_does_time() {
    let mut s = Sulk::default();
    s.ignored(0.0);
    s.ignored(10.0);
    assert!(s.active(100.0));
    s.caressed();
    assert!(!s.active(101.0));
    s.ignored(200.0);
    s.ignored(210.0);
    assert!(s.active(300.0));
    assert!(!s.active(210.0 + 2701.0));
}

#[test]
fn a_returning_user_gets_a_question_first() {
    let mut i = Initiative::default();
    let c = Context {
        returned: true,
        ..ctx(10_000.0)
    };
    let id = i.decide(&c).expect("a question");
    assert!(QUESTIONS.iter().any(|q| q.id == id));
}

#[test]
fn nothing_is_asked_while_sulking_or_while_waiting_for_an_answer() {
    let mut i = Initiative::default();
    assert!(i
        .decide(&Context {
            returned: true,
            sulking: true,
            ..ctx(10_000.0)
        })
        .is_none());
    assert!(i
        .decide(&Context {
            returned: true,
            ..ctx(10_000.0)
        })
        .is_some());
    assert!(i
        .decide(&Context {
            returned: true,
            ..ctx(10_030.0)
        })
        .is_none());
}

#[test]
fn the_slime_never_asks_more_than_four_times_an_hour() {
    let mut i = Initiative::default();
    let mut asked = 0;
    for step in 0..60 {
        let now = 10_000.0 + step as f64 * 60.0;
        if let Some(id) = i.decide(&Context {
            returned: true,
            ..ctx(now)
        }) {
            asked += 1;
            i.answered(id, now + 1.0);
        }
    }
    assert!(asked <= 4, "asked {asked} times in an hour");
    assert!(asked >= 2);
}

#[test]
fn a_question_already_answered_is_not_asked_again() {
    let mut i = Initiative::default();
    let all: Vec<&str> = QUESTIONS.iter().map(|q| q.key).collect();
    let mut c = ctx(10_000.0);
    c.returned = true;
    let learned = known(&all);
    c.known = &learned;
    assert!(i.decide(&c).is_none());
}

#[test]
fn an_unanswered_question_expires_as_ignored() {
    let mut i = Initiative::default();
    let id = i
        .decide(&Context {
            returned: true,
            ..ctx(10_000.0)
        })
        .unwrap();
    assert!(i.expired(10_050.0).is_none());
    assert_eq!(i.expired(10_100.0), Some(id));
    assert!(i.expired(10_200.0).is_none());
}

#[test]
fn declining_gently_postpones_without_a_penalty() {
    let mut i = Initiative::default();
    let id = i
        .decide(&Context {
            returned: true,
            ..ctx(10_000.0)
        })
        .unwrap();
    i.declined(id, 10_005.0);
    assert!(i.expired(10_500.0).is_none());
    let next = i.decide(&Context {
        returned: true,
        ..ctx(10_300.0)
    });
    assert_ne!(next, Some(id));
}
