use slimy_remind::*;

const NOW: i64 = 1_800_000_000;

#[test]
fn a_reminder_is_added_listed_and_fires_once() {
    let mut r = Reminders::new();
    let id = r.add(NOW, NOW + 600, "  Appeler   maman\n", false).unwrap();
    assert_eq!(r.list()[0].text, "Appeler maman");
    assert!(r.due(NOW + 599).is_empty(), "not yet");
    let due = r.due(NOW + 601);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].id, id);
    assert!(
        r.due(NOW + 700).is_empty() && r.list().is_empty(),
        "a one-off is gone after it fired"
    );
}

#[test]
fn a_daily_reminder_comes_back_the_next_day_and_never_piles_up() {
    let mut r = Reminders::new();
    r.add(NOW, NOW + 100, "Boire de l'eau", true).unwrap();
    assert_eq!(r.due(NOW + 200).len(), 1);
    assert_eq!(r.list()[0].at, NOW + 100 + 86_400, "set again for tomorrow");
    // the computer was off for three days (and a bit): announced once at most, then set for the next future time
    let mut late = Reminders::new();
    late.add(NOW, NOW + 100, "Quotidien", true).unwrap();
    let after = NOW + 100 + 3 * 86_400 + 600;
    assert!(
        late.due(after).is_empty(),
        "far too late to announce, but not lost"
    );
    assert!(late.list()[0].at > after && late.list()[0].at <= after + 86_400);
}

#[test]
fn old_one_offs_are_dropped_quietly_and_a_burst_is_limited() {
    let mut r = Reminders::new();
    r.add(NOW, NOW + 10, "Trop vieux", false).unwrap();
    assert!(r.due(NOW + 10 + 13 * 3600).is_empty());
    assert!(r.list().is_empty());
    let mut many = Reminders::new();
    for i in 0..6 {
        many.add(NOW, NOW + 10 + i, &format!("n{i}"), false)
            .unwrap();
    }
    assert_eq!(
        many.due(NOW + 100).len(),
        MAX_AT_ONCE,
        "three at once, in order"
    );
    assert_eq!(many.list().len(), 3, "the rest wait for the next look");
}

#[test]
fn bad_requests_are_refused() {
    let mut r = Reminders::new();
    assert_eq!(r.add(NOW, NOW + 60, "   ", false), Err(RemindError::Empty));
    assert_eq!(
        r.add(NOW, NOW - 3600, "Hier", false),
        Err(RemindError::InThePast)
    );
    assert!(
        r.add(NOW, NOW - 30, "Il y a un instant", false).is_ok(),
        "a little lateness is tolerated"
    );
    assert_eq!(
        r.add(NOW, NOW + MAX_AHEAD + 10, "Dans deux ans", false),
        Err(RemindError::TooFar)
    );
    for i in 0..MAX_REMINDERS {
        let _ = r.add(NOW, NOW + 60 + i as i64, "x", false);
    }
    assert_eq!(
        r.add(NOW, NOW + 60, "En trop", false),
        Err(RemindError::Full)
    );
    assert_eq!(tidy(&"é".repeat(500)).chars().count(), MAX_TEXT);
    assert_eq!(tidy("a\u{0}b\u{1b}c"), "a b c");
}

#[test]
fn a_reminder_can_be_removed_and_the_file_survives_a_restart() {
    let mut r = Reminders::new();
    let a = r.add(NOW, NOW + 100, "A", false).unwrap();
    let b = r.add(NOW, NOW + 200, "B", true).unwrap();
    assert!(r.remove(a) && !r.remove(a));
    let back = Reminders::from_json(&r.to_json());
    assert_eq!(back, r);
    assert_eq!(back.list()[0].id, b);
    let mut grown = back;
    assert!(
        grown.add(NOW, NOW + 300, "C", false).unwrap() > b,
        "ids never repeat"
    );
    assert_eq!(Reminders::from_json("{ nope"), Reminders::new());
    assert_eq!(
        Reminders::from_json(r#"{"next_id":1,"items":[{"id":1,"at":5,"text":""}]}"#),
        Reminders::new(),
        "an empty text is refused"
    );
}
