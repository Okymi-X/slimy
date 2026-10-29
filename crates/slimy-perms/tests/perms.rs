use slimy_perms::*;

#[test]
fn everything_is_refused_by_default_and_the_refusal_is_journaled() {
    let mut r = Registry::new();
    for p in Power::ALL {
        assert!(!r.is_granted(p, 0));
        assert_eq!(r.check(p, 10, "test"), Verdict::Denied);
    }
    assert_eq!(r.journal().len(), Power::ALL.len());
    assert!(r.active(0).is_empty());
}

#[test]
fn a_power_is_granted_alone_and_revoked_at_once() {
    let mut r = Registry::new();
    r.grant(Power::Tool, 0, None).unwrap();
    assert_eq!(r.check(Power::Tool, 5, "mcp"), Verdict::Allowed);
    assert_eq!(r.check(Power::Files, 5, ""), Verdict::Denied);
    assert!(r.revoke(Power::Tool));
    assert!(!r.revoke(Power::Tool));
    assert_eq!(r.check(Power::Tool, 6, ""), Verdict::Denied);
}

#[test]
fn sensitive_powers_need_a_duration_and_expire() {
    let mut r = Registry::new();
    assert_eq!(
        r.grant(Power::Screen, 0, None),
        Err(PermError::NeedsDuration(Power::Screen))
    );
    assert_eq!(r.check(Power::Screen, 1, ""), Verdict::Denied);
    r.grant(Power::Screen, 100, Some(60)).unwrap();
    assert!(r.active(159).contains(&Power::Screen));
    assert_eq!(r.check(Power::Screen, 159, ""), Verdict::Allowed);
    assert_eq!(r.check(Power::Screen, 160, ""), Verdict::Expired);
    assert_eq!(r.check(Power::Screen, 161, ""), Verdict::Denied);
}

#[test]
fn the_journal_is_bounded_and_notes_are_cut() {
    let mut r = Registry::new();
    let long = "x".repeat(1000);
    for i in 0..(MAX_JOURNAL as u64 + 50) {
        r.check(Power::Files, i, &long);
    }
    assert_eq!(r.journal().len(), MAX_JOURNAL);
    assert_eq!(r.journal()[0].at, 50);
    assert!(r
        .journal()
        .iter()
        .all(|e| e.note.chars().count() == MAX_NOTE));
}

#[test]
fn a_damaged_file_leaves_everything_refused_and_a_good_one_round_trips() {
    let mut r = Registry::new();
    r.grant(Power::ActiveApp, 0, None).unwrap();
    r.check(Power::ActiveApp, 3, "title");
    assert_eq!(Registry::from_json(&r.to_json()), r);
    let broken = Registry::from_json("{ not json");
    assert!(!broken.is_granted(Power::ActiveApp, 0));
}

#[test]
fn granting_again_replaces_the_old_grant() {
    let mut r = Registry::new();
    r.grant(Power::Clipboard, 0, Some(10)).unwrap();
    r.grant(Power::Clipboard, 0, Some(100)).unwrap();
    assert!(r.is_granted(Power::Clipboard, 50));
    r.revoke_all();
    assert!(!r.is_granted(Power::Clipboard, 50));
}

#[test]
fn the_microphone_is_sensitive_so_it_needs_a_duration_and_expires() {
    assert!(Power::Microphone.is_sensitive());
    let mut r = Registry::new();
    assert_eq!(
        r.grant(Power::Microphone, 0, None),
        Err(PermError::NeedsDuration(Power::Microphone))
    );
    r.grant(Power::Microphone, 10, Some(300)).unwrap();
    assert_eq!(r.check(Power::Microphone, 100, "listen"), Verdict::Allowed);
    assert_eq!(r.check(Power::Microphone, 311, "listen"), Verdict::Expired);
    assert_eq!(Power::ALL.len(), 7);
}
