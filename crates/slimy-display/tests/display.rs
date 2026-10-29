use slimy_display::*;

fn screen(x: i32, y: i32, w: u32, h: u32, bar: u32, scale: f64, primary: bool) -> Screen {
    Screen {
        rect: Rect { x, y, w, h },
        work: Rect {
            x,
            y: y + bar as i32,
            w,
            h: h - bar,
        },
        scale,
        primary,
    }
}

fn two_screens() -> Vec<Screen> {
    vec![
        screen(0, 0, 1920, 1080, 32, 1.0, false),
        screen(1920, 0, 2560, 1440, 0, 1.5, true),
    ]
}

#[test]
fn the_play_area_is_the_work_area_of_the_chosen_screen() {
    let s = two_screens();
    let w = world_of(&s[0]);
    assert_eq!(
        (w.left, w.top, w.right, w.bottom),
        (0.0, 32.0, 1920.0, 1080.0),
        "below the top bar"
    );
    let second = world_of(&s[1]);
    assert_eq!(
        (second.left, second.top, second.right, second.bottom),
        (1920.0, 0.0, 4480.0, 1440.0),
        "absolute desktop coordinates"
    );
    // a nonsense work area (empty, or outside the screen) falls back to the whole screen
    let mut odd = s[0];
    odd.work = Rect {
        x: -50,
        y: 0,
        w: 10,
        h: 10,
    };
    assert_eq!(world_of(&odd).bottom, 1080.0);
    odd.work = Rect::default();
    assert_eq!(world_of(&odd).top, 0.0);
}

#[test]
fn the_right_screen_is_chosen_for_every_preference() {
    let s = two_screens();
    assert_eq!(
        choose(&s, Preference::Primary, None, None),
        1,
        "the main screen is not always the first"
    );
    assert_eq!(choose(&s, Preference::Index(0), None, None), 0);
    assert_eq!(
        choose(&s, Preference::Index(7), None, None),
        1,
        "an unplugged screen falls back to the main one"
    );
    assert_eq!(
        choose(&s, Preference::Follow, Some((100.0, 100.0)), Some(1)),
        0
    );
    assert_eq!(
        choose(&s, Preference::Follow, Some((3000.0, 700.0)), Some(0)),
        1
    );
    assert_eq!(
        choose(&s, Preference::Follow, Some((-9000.0, 0.0)), Some(0)),
        0,
        "pointer nowhere: stays"
    );
    assert_eq!(choose(&s, Preference::Follow, None, None), 1);
    assert_eq!(choose(&[], Preference::Primary, None, None), 0);
    let none_primary = vec![screen(0, 0, 800, 600, 0, 1.0, false)];
    assert_eq!(choose(&none_primary, Preference::Primary, None, None), 0);
}

#[test]
fn the_saved_preference_round_trips_and_nonsense_means_primary() {
    for p in [
        Preference::Primary,
        Preference::Follow,
        Preference::Index(2),
    ] {
        assert_eq!(Preference::parse(&p.text()), p);
    }
    assert_eq!(Preference::parse("cursor"), Preference::Follow);
    assert_eq!(Preference::parse("  FOLLOW "), Preference::Follow);
    assert_eq!(Preference::parse("banana"), Preference::Primary);
    assert_eq!(Preference::parse("-3"), Preference::Primary);
    assert_eq!(Preference::parse(""), Preference::Primary);
}

#[test]
fn any_change_of_the_setup_changes_the_signature_and_nothing_else_does() {
    let base = two_screens();
    assert_eq!(signature(&base), signature(&two_screens()));
    let mut resized = two_screens();
    resized[1].rect.w = 3840;
    assert_ne!(signature(&base), signature(&resized));
    let mut rescaled = two_screens();
    rescaled[0].scale = 2.0;
    assert_ne!(signature(&base), signature(&rescaled));
    let mut barred = two_screens();
    barred[1].work.y = 40;
    assert_ne!(signature(&base), signature(&barred), "a bar appeared");
    let mut moved = two_screens();
    moved[1].rect.x = -2560;
    assert_ne!(signature(&base), signature(&moved));
    let mut unplugged = two_screens();
    unplugged.pop();
    assert_ne!(signature(&base), signature(&unplugged));
    let mut swapped = two_screens();
    swapped[0].primary = true;
    swapped[1].primary = false;
    assert_ne!(signature(&base), signature(&swapped));
}

#[test]
fn the_slime_is_sized_to_the_screen_in_steps() {
    assert_eq!(size_scale(1000.0), 1.0);
    assert_eq!(size_scale(2160.0), 1.4, "capped on a very tall screen");
    assert_eq!(size_scale(400.0), 0.8, "never tiny");
    assert!(size_scale(1200.0) > size_scale(900.0));
    assert_eq!(size_scale(f64::NAN), 1.0);
    assert_eq!(size_scale(-5.0), 1.0);
    // steps of a twentieth: a few pixels more or less change nothing
    assert_eq!(size_scale(1001.0), size_scale(1000.0));
}

#[test]
fn the_slime_follows_the_pointer_to_another_screen_only_after_a_pause() {
    let mut follower = Follower::default();
    assert_eq!(follower.update(0.0, 0, Some(1)), 0, "just crossed");
    assert_eq!(follower.update(1.0, 0, Some(1)), 0);
    assert_eq!(follower.update(1.5, 0, Some(0)), 0, "came back: forgotten");
    assert_eq!(follower.update(2.0, 0, Some(1)), 0, "starts again");
    assert_eq!(follower.update(4.6, 0, Some(1)), 1, "stayed long enough");
    assert_eq!(follower.update(5.0, 1, Some(1)), 1);
    assert_eq!(follower.update(6.0, 1, None), 1, "pointer unknown: stays");
}

#[test]
fn only_the_slime_takes_the_mouse_and_the_rest_lets_it_through() {
    let region = HitRegion {
        x: 400,
        y: 300,
        w: 120,
        h: 100,
    };
    assert!(region.takes(450.0, 350.0));
    assert!(!region.takes(399.0, 350.0) && !region.takes(450.0, 400.0));
    assert!(
        !HitRegion::default().takes(0.0, 0.0),
        "an empty region takes nothing"
    );
    let scaled = region.scaled(2.0);
    assert_eq!(
        scaled,
        HitRegion {
            x: 800,
            y: 600,
            w: 240,
            h: 200
        }
    );
    assert_eq!(region.scaled(f64::NAN), region);

    let mut gate = ClickThrough::default();
    assert_eq!(
        gate.update(&region, Some((10.0, 10.0))),
        Some(true),
        "far away: clicks go through"
    );
    assert_eq!(
        gate.update(&region, Some((20.0, 10.0))),
        None,
        "no change, no call"
    );
    assert_eq!(
        gate.update(&region, Some((396.0, 350.0))),
        Some(false),
        "the margin makes the edge easy to grab"
    );
    assert_eq!(gate.update(&region, Some((450.0, 350.0))), None);
    assert_eq!(
        gate.update(&region, None),
        None,
        "an unknown pointer never hides the slime"
    );
    assert_eq!(gate.update(&region, Some((900.0, 900.0))), Some(true));
    assert_eq!(
        gate.update(&region, None),
        Some(false),
        "and takes the clicks back"
    );
}
