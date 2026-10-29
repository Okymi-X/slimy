use slimy_core::{hop_reach, level_hop, Autonomy, CatchPhase, Intent, Place, Snapshot, World};

const WORLD: World = World {
    left: 0.0,
    right: 1900.0,
    top: 30.0,
    bottom: 1170.0,
};

fn snapshot(now: f64, x: f32, y: f32) -> Snapshot {
    Snapshot {
        now,
        x,
        y,
        place: Place::Floor,
        settled: true,
        supported: true,
        grabbed: false,
        busy: false,
        sulking: false,
        sleepiness: 0.0,
        affection: 0.5,
        pointer: None,
        pointer_active: false,
        radius: 64.0,
        reach: 64.0,
        world: WORLD,
    }
}

/// A crude stand-in for the body: a hop lands after 0.8 s, as far as the measured hop curve says.
struct Puppet {
    x: f32,
    busy_until: f64,
    hops: u32,
}

impl Puppet {
    fn new(x: f32) -> Self {
        Self {
            x,
            busy_until: 0.0,
            hops: 0,
        }
    }

    fn tick(
        &mut self,
        ai: &mut Autonomy,
        now: f64,
        edit: impl Fn(&mut Snapshot),
    ) -> slimy_core::Output {
        let mut s = snapshot(now, self.x, 1106.0);
        s.settled = now >= self.busy_until;
        edit(&mut s);
        let out = ai.step(&s);
        if let Some((dx, _dy, power)) = out.hop {
            self.x = (self.x + dx.signum() * hop_reach(power))
                .clamp(WORLD.left + 40.0, WORLD.right - 40.0);
            self.busy_until = now + 0.8;
            self.hops += 1;
        }
        out
    }
}

fn run(
    ai: &mut Autonomy,
    puppet: &mut Puppet,
    seconds: f64,
    edit: impl Fn(&mut Snapshot) + Copy,
) -> Vec<slimy_core::Output> {
    let mut outs = Vec::new();
    let mut now = 0.0;
    while now < seconds {
        outs.push(puppet.tick(ai, now, edit));
        now += 0.1;
    }
    outs
}

#[test]
fn an_idle_slime_wanders_now_and_then() {
    let mut ai = Autonomy::new(7);
    let mut p = Puppet::new(900.0);
    run(&mut ai, &mut p, 300.0, |_| {});
    assert!(p.hops >= 3, "only {} hops in five minutes", p.hops);
}

#[test]
fn it_stays_calm_it_never_races_around() {
    let mut ai = Autonomy::new(11);
    let mut p = Puppet::new(900.0);
    run(&mut ai, &mut p, 600.0, |_| {});
    assert!(
        p.hops <= 300,
        "{} hops in ten minutes, more than one every two seconds",
        p.hops
    );
}

#[test]
fn it_does_not_move_while_a_bubble_is_open_or_while_held() {
    let mut ai = Autonomy::new(3);
    let mut p = Puppet::new(900.0);
    run(&mut ai, &mut p, 120.0, |s| s.busy = true);
    run(&mut ai, &mut p, 120.0, |s| s.grabbed = true);
    assert_eq!(p.hops, 0);
}

#[test]
fn a_hop_is_preceded_by_a_crouch() {
    let mut ai = Autonomy::new(5);
    let mut p = Puppet::new(900.0);
    let outs = run(&mut ai, &mut p, 120.0, |_| {});
    let first_hop = outs.iter().position(|o| o.hop.is_some()).expect("a hop");
    assert!(
        outs[..first_hop].iter().rev().take(3).any(|o| o.crouch),
        "no crouch before the first hop"
    );
}

#[test]
fn a_sulking_slime_goes_to_a_bottom_corner_and_faces_the_wall() {
    let mut ai = Autonomy::new(21);
    let mut p = Puppet::new(900.0);
    let outs = run(&mut ai, &mut p, 90.0, |s| s.sulking = true);
    assert!(
        p.x < 140.0 || p.x > WORLD.right - 140.0,
        "stopped at {}",
        p.x
    );
    let last = outs
        .iter()
        .rev()
        .find(|o| o.intent != Intent::Peek)
        .unwrap();
    assert_eq!(last.intent, Intent::Hide);
    let wall_side = if p.x < 950.0 { -1.0 } else { 1.0 };
    assert!(
        last.gaze.is_some_and(|(gx, _)| gx * wall_side > 0.3),
        "gaze {:?}",
        last.gaze
    );
}

#[test]
fn a_hidden_slime_peeks_out_from_time_to_time() {
    let mut ai = Autonomy::new(21);
    let mut p = Puppet::new(120.0);
    let outs = run(&mut ai, &mut p, 240.0, |s| s.sulking = true);
    assert!(
        outs.iter().any(|o| o.intent == Intent::Peek),
        "never peeked"
    );
    let hidden = outs.iter().filter(|o| o.intent == Intent::Hide).count();
    let peeks = outs.iter().filter(|o| o.intent == Intent::Peek).count();
    assert!(
        hidden > peeks * 3,
        "peeks too frequent: {peeks} vs {hidden}"
    );
}

#[test]
fn a_sleepy_slime_curls_up_in_a_corner_and_stays_there() {
    let mut ai = Autonomy::new(2);
    let mut p = Puppet::new(900.0);
    run(&mut ai, &mut p, 120.0, |s| s.sleepiness = 0.9);
    let at = p.x;
    let hops = p.hops;
    let outs = run(&mut ai, &mut p, 120.0, |s| s.sleepiness = 0.9);
    assert_eq!(p.x, at);
    assert_eq!(p.hops, hops);
    assert_eq!(outs.last().unwrap().intent, Intent::Nap);
    assert!(at < 150.0 || at > WORLD.right - 150.0);
}

#[test]
fn it_never_aims_outside_the_screen() {
    let mut p = Puppet::new(60.0);
    for seed in 0..5 {
        let mut ai = Autonomy::new(seed);
        run(&mut ai, &mut p, 200.0, |_| {});
        assert!(p.x >= WORLD.left && p.x <= WORLD.right);
    }
}

#[test]
fn a_fond_slime_walks_toward_an_active_cursor_and_stops_near_it() {
    let mut ai = Autonomy::new(4);
    let mut p = Puppet::new(300.0);
    let outs = run(&mut ai, &mut p, 400.0, |s| {
        s.affection = 0.9;
        s.pointer = Some((1400.0, 900.0));
        s.pointer_active = true;
    });
    assert!(
        outs.iter().any(|o| o.intent == Intent::Follow),
        "never followed"
    );
    assert!(p.x > 700.0, "stayed at {}", p.x);
}

#[test]
fn the_same_seed_gives_the_same_life() {
    let sample = |seed| {
        let mut ai = Autonomy::new(seed);
        let mut p = Puppet::new(900.0);
        run(&mut ai, &mut p, 120.0, |_| {});
        (p.x, p.hops)
    };
    assert_eq!(sample(8), sample(8));
}

#[test]
fn a_short_hop_does_not_overshoot_its_corner() {
    let mut ai = Autonomy::new(2);
    let mut s = snapshot(0.0, 150.0, 1106.0);
    s.sulking = true;
    assert!(ai.step(&s).crouch);
    s.now = 0.2;
    let (_, _, power) = ai.step(&s).hop.expect("short hop");
    assert!(hop_reach(power) <= 150.0 - 96.0 + 0.01);
}

#[test]
fn waking_cancels_a_prepared_hop_to_the_sleeping_corner() {
    let mut ai = Autonomy::new(2);
    let mut s = snapshot(0.0, 900.0, 1106.0);
    s.sleepiness = 0.9;
    assert!(ai.step(&s).crouch);
    s.sleepiness = 0.0;
    s.now = 0.2;
    assert!(ai.step(&s).hop.is_none());
}

#[test]
fn losing_contact_cancels_a_prepared_hop() {
    let mut ai = Autonomy::new(2);
    let mut s = snapshot(0.0, 900.0, 1106.0);
    s.sulking = true;
    assert!(ai.step(&s).crouch);
    s.settled = false;
    s.supported = false;
    s.now = 0.2;
    assert!(ai.step(&s).hop.is_none());
}

#[test]
fn a_walk_that_lands_on_a_wall_can_return_to_the_floor() {
    let mut ai = Autonomy::new(7);
    let mut s = snapshot(0.0, 900.0, 1106.0);
    for frame in 0..3000 {
        s.now = frame as f64 / 60.0;
        let out = ai.step(&s);
        if out.intent == Intent::Wander && out.crouch {
            break;
        }
    }
    s.place = Place::Right;
    s.x = 1850.0;
    s.y = 700.0;
    let mut escaped = false;
    for _ in 0..600 {
        s.now += 1.0 / 60.0;
        if let Some((dx, dy, _)) = ai.step(&s).hop {
            if dx < 0.0 && dy > 0.0 {
                escaped = true;
                break;
            }
        }
    }
    assert!(
        escaped,
        "a floor destination left the slime stuck on the wall"
    );
}

#[test]
fn a_sleepy_slime_only_naps_after_reaching_its_corner() {
    let mut ai = Autonomy::new(2);
    let mut s = snapshot(0.0, 900.0, 1106.0);
    s.sleepiness = 0.9;
    assert_eq!(ai.step(&s).intent, Intent::Wander);
    s.now = 1.0;
    s.x = 76.8;
    assert_eq!(ai.step(&s).intent, Intent::Nap);
}

#[test]
fn following_a_cursor_has_a_deadline_even_if_it_stays_far_away() {
    let mut ai = Autonomy::new(4);
    let mut s = snapshot(0.0, 300.0, 1106.0);
    s.affection = 0.9;
    s.pointer = Some((1400.0, 900.0));
    s.pointer_active = true;
    let mut following = false;
    for frame in 0..12000 {
        s.now = frame as f64 / 60.0;
        if ai.step(&s).intent == Intent::Follow {
            following = true;
            break;
        }
    }
    assert!(following);
    s.now += 20.0;
    assert!(ai.step(&s).hop.is_none());
    s.now += 0.01;
    assert_eq!(ai.step(&s).intent, Intent::Idle);
}

#[test]
fn a_prepared_follow_hop_tracks_a_cursor_that_changes_sides() {
    let mut ai = Autonomy::new(4);
    let mut s = snapshot(0.0, 900.0, 1106.0);
    s.affection = 0.9;
    s.pointer = Some((1600.0, 900.0));
    s.pointer_active = true;
    let mut prepared = false;
    for frame in 0..12000 {
        s.now = frame as f64 / 60.0;
        let out = ai.step(&s);
        if out.intent == Intent::Follow && out.crouch {
            prepared = true;
            break;
        }
    }
    assert!(prepared);
    s.pointer = Some((200.0, 900.0));
    let mut hopped = false;
    for _ in 0..60 {
        s.now += 1.0 / 60.0;
        if let Some((dx, _, _)) = ai.step(&s).hop {
            assert!(dx < 0.0, "launched toward the old cursor position");
            hopped = true;
            break;
        }
    }
    assert!(hopped);
}

#[test]
fn a_hop_lands_where_it_was_planned_without_creeping() {
    for seed in [2, 21] {
        let mut ai = Autonomy::new(seed);
        let mut p = Puppet::new(900.0);
        let corner = WORLD.left + 64.0 * 1.2;
        let mut now = 0.0;
        while now < 120.0 {
            let before = p.x;
            let out = p.tick(&mut ai, now, |s| s.sulking = true);
            if let Some((_, _, power)) = out.hop {
                let remaining = (before - corner).abs();
                let reach = hop_reach(power);
                assert!(
                    reach <= remaining + 0.5,
                    "seed {seed}: hop of {reach} px from {remaining} px away overshoots"
                );
                assert!(
                    reach >= 24.0,
                    "seed {seed}: hop of {reach} px creeps toward a target {remaining} px away"
                );
            }
            now += 0.1;
        }
        assert!(
            p.x < 140.0 || p.x > WORLD.right - 140.0,
            "seed {seed}: stopped at {}",
            p.x
        );
    }
}

#[test]
fn wall_descent_hops_soften_as_the_foot_nears_the_floor() {
    let power_at = |y: f32| {
        let mut ai = Autonomy::new(5);
        let mut s = snapshot(0.0, 1850.0, y);
        s.place = Place::Right;
        for frame in 0..3000 {
            s.now = f64::from(frame) / 60.0;
            if let Some((_, dy, power)) = ai.step(&s).hop {
                return (dy, power);
            }
        }
        panic!("no descent hop at y = {y}");
    };
    let (dy_high, high) = power_at(700.0);
    let (dy_low, low) = power_at(WORLD.bottom - 64.0 - 10.0);
    assert!(dy_high > 0.0 && high >= 500.0, "far from the floor: {high}");
    assert!(dy_low > 0.0 && low <= 340.0, "foot at the floor: {low}");
}

#[test]
fn the_hop_curve_grows_with_power() {
    let mut last = 0.0;
    for power in (120..=380).step_by(10) {
        let reach = hop_reach(power as f32);
        assert!(reach >= last, "reach shrank at {power}");
        last = reach;
    }
    assert!(hop_reach(0.0) < 1.0 && hop_reach(1000.0) > 40.0);
}

/// A fond, awake slime resting on the floor at x = 900, with the cursor `dx` px to the side and `h` px above the floor.
fn at_cursor(now: f64, x: f32, dx: f32, h: f32) -> Snapshot {
    let mut s = snapshot(now, x, WORLD.bottom - 64.0);
    s.affection = 0.8;
    s.pointer = Some((900.0 + dx, WORLD.bottom - h));
    s.pointer_active = true;
    s
}

/// Runs a whole catch at 60 frames a second, the way the page does: a gentle hop moves the slime by the distance it
/// was aimed at after a second of flight. Returns the phases met, in order, and what was seen along the way.
struct Trace {
    phases: Vec<CatchPhase>,
    hops: Vec<(f32, f32, f32)>,
    x_at_hold: Option<f32>,
    flat_outside_hold: bool,
    not_catch_while_catching: bool,
    ended_at: Option<f64>,
}

fn run_catch(dx: f32, h: f32, mutate: impl Fn(f64, &mut Snapshot)) -> Trace {
    let mut ai = Autonomy::new(5);
    let (mut x, mut now) = (900.0f32, 9.7f64);
    let mut trace = Trace {
        phases: vec![],
        hops: vec![],
        x_at_hold: None,
        flat_outside_hold: false,
        not_catch_while_catching: false,
        ended_at: None,
    };
    let mut landing: Option<(f64, f32)> = None;
    while now < 40.0 {
        let mut s = at_cursor(now, x, dx, h);
        if let Some((at, to)) = landing {
            if now >= at {
                x = to;
                landing = None;
                s.x = x;
            } else {
                s.settled = false;
            }
        }
        mutate(now, &mut s);
        let out = ai.step(&s);
        if out.catching != CatchPhase::None && trace.phases.last() != Some(&out.catching) {
            trace.phases.push(out.catching);
        }
        if out.catching != CatchPhase::None && out.intent != Intent::Catch {
            trace.not_catch_while_catching = true;
        }
        if (out.flatten > 0.5) != matches!(out.catching, CatchPhase::Spread | CatchPhase::Hold) {
            trace.flat_outside_hold = true;
        }
        if out.catching == CatchPhase::Hold && trace.x_at_hold.is_none() {
            trace.x_at_hold = Some(x);
        }
        if let Some((vx, vy, power)) = out.hop {
            if out.catching != CatchPhase::None {
                assert!(out.gentle, "a catch hop is a gentle, level one");
                trace.hops.push((vx, vy, power));
                // Invert the fitted launch to find how far it was aimed.
                let aimed = (vx * power).signum() * ((vx * power).abs() + 6.478_247) / 10.980_972;
                landing = Some((now + 0.8, x + aimed));
            }
        }
        if trace.phases.last().is_some()
            && out.catching == CatchPhase::None
            && trace.ended_at.is_none()
        {
            trace.ended_at = Some(now);
            break;
        }
        now += 1.0 / 60.0;
    }
    trace
}

#[test]
fn a_cursor_lying_beside_the_slime_on_the_floor_is_approached_covered_and_left_again() {
    use CatchPhase::*;
    let t = run_catch(70.0, 20.0, |_, _| {});
    assert_eq!(
        t.phases,
        vec![Acquire, Prepare, Flight, Contact, Spread, Hold, Retreat],
        "the whole sequence, in order"
    );
    assert!(!t.not_catch_while_catching && !t.flat_outside_hold);
    // Approach: one level hop toward the cursor, then the retreat hop away from it.
    assert!(t.hops.len() >= 2, "{} hops", t.hops.len());
    assert!(t.hops[0].0 > 0.9, "toward the cursor: {:?}", t.hops[0]);
    assert!(
        t.hops.last().unwrap().0 < -0.9,
        "back away: {:?}",
        t.hops.last()
    );
    // It stops 0.4 radius short of the cursor so that the squash covers it.
    let x = t.x_at_hold.expect("held it");
    assert!(((970.0 - x) - 0.4 * 64.0).abs() < 5.0, "stopped at {x}");
    assert!(t.ended_at.is_some());
}

#[test]
fn a_cursor_right_under_the_body_is_covered_at_once_without_a_hop() {
    use CatchPhase::*;
    let t = run_catch(20.0, 20.0, |_, _| {});
    assert_eq!(t.phases.first(), Some(&Acquire));
    assert!(t.phases.contains(&Spread) && t.phases.contains(&Hold));
    assert!(!t.phases.contains(&Prepare) && !t.phases.contains(&Flight));
    assert_eq!(t.x_at_hold, Some(900.0));
}

#[test]
fn the_fitted_level_hop_is_a_unit_direction_that_grows_with_the_distance_and_mirrors() {
    let mut last = 0.0;
    for dx in [6.4f32, 20.0, 40.0, 70.4] {
        let (x, y, power) = level_hop(dx);
        assert!((x.hypot(y) - 1.0).abs() < 1e-4 && y < 0.0 && x > 0.0);
        assert!(power > last, "{dx}: {power}");
        last = power;
        let (mx, my, mp) = level_hop(-dx);
        assert!((mx + x).abs() < 1e-5 && (my - y).abs() < 1e-5 && (mp - power).abs() < 1e-3);
    }
    let (vx, vy, p) = level_hop(40.0);
    assert!((vx * p - 432.8).abs() < 1.0 && (vy * p + 110.0).abs() < 0.01);
}

#[test]
fn it_only_catches_a_cursor_that_lies_low_near_and_still_for_a_slime_that_is_free_to() {
    let none = |name: &str, dx: f32, h: f32, f: &dyn Fn(f64, &mut Snapshot)| {
        let t = run_catch(dx, h, |n, s| f(n, s));
        assert!(
            t.phases.is_empty(),
            "{name} must not start a catch, got {:?}",
            t.phases
        );
    };
    none("a cursor high in the air", 70.0, 120.0, &|_, _| {});
    none("a cursor above the strip", 70.0, 50.0, &|_, _| {});
    none("a cursor too far to the side", 150.0, 20.0, &|_, _| {});
    none("a cursor on the very floor line", 70.0, 1.0, &|_, _| {});
    none("a cool slime", 70.0, 20.0, &|_, s| s.affection = 0.3);
    none("a sleepy slime", 70.0, 20.0, &|_, s| s.sleepiness = 0.8);
    none("a sulking slime", 70.0, 20.0, &|_, s| s.sulking = true);
    none("a slime on a wall", 70.0, 20.0, &|_, s| {
        s.place = Place::Left
    });
    none("a slime still moving", 70.0, 20.0, &|_, s| {
        s.settled = false
    });
    none("a slime that is busy", 70.0, 20.0, &|_, s| s.busy = true);
    none("a slime that is held", 70.0, 20.0, &|_, s| s.grabbed = true);
    none("a cursor that keeps moving", 70.0, 20.0, &|n, s| {
        s.pointer = Some((
            970.0 + ((n * 60.0) as i64 % 2) as f32 * 20.0,
            WORLD.bottom - 20.0,
        ))
    });
    none("no cursor known", 70.0, 20.0, &|_, s| s.pointer = None);
    none("a cursor by the wall", -840.0, 20.0, &|_, _| {});
}

#[test]
fn a_catch_gives_way_when_the_cursor_moves_away_is_lifted_or_the_slime_is_taken() {
    use CatchPhase::*;
    // The cursor slides away during the hold: the body lets go at once (flatten 0, retreat).
    let moved = run_catch(70.0, 20.0, |n, s| {
        if n > 11.6 {
            s.pointer = Some((970.0 + 30.0, WORLD.bottom - 20.0));
        }
    });
    assert!(
        moved.phases.contains(&Retreat) && !moved.flat_outside_hold,
        "{:?}",
        moved.phases
    );
    // The cursor is lifted out of the strip.
    let lifted = run_catch(70.0, 20.0, |n, s| {
        if n > 11.6 {
            s.pointer = Some((970.0, WORLD.bottom - 90.0));
        }
    });
    assert!(
        lifted.phases.contains(&Retreat) && !lifted.flat_outside_hold,
        "{:?}",
        lifted.phases
    );
    // The slime is grabbed in the middle: the catch is simply gone, with no squash left asked for.
    let mut ai = Autonomy::new(5);
    let mut now = 9.7;
    let mut flatten_after_grab = 0.0f32;
    let mut seen = false;
    while now < 16.0 {
        let mut s = at_cursor(now, 970.0 - 25.6, 70.0, 20.0);
        s.grabbed = now > 12.0;
        let out = ai.step(&s);
        seen |= out.catching != CatchPhase::None;
        if now > 12.0 {
            flatten_after_grab = flatten_after_grab.max(out.flatten);
            assert_eq!(out.catching, CatchPhase::None);
        }
        now += 1.0 / 60.0;
    }
    assert!(seen && flatten_after_grab == 0.0);
}

#[test]
fn after_a_catch_the_slime_waits_a_good_while_before_pouncing_again() {
    let mut ai = Autonomy::new(5);
    let (mut now, mut started, mut second_start) = (9.7f64, None::<f64>, None::<f64>);
    let mut x = 900.0f32;
    let mut was_catching = false;
    while now < 120.0 {
        let s = at_cursor(now, x, 70.0, 20.0);
        let out = ai.step(&s);
        let catching = out.catching != CatchPhase::None;
        if catching && !was_catching {
            if started.is_none() {
                started = Some(now);
            } else if second_start.is_none() {
                second_start = Some(now);
            }
        }
        if out.hop.is_some() {
            x = 900.0;
        } // the puppet: every hop ends back where it began, so the cursor stays in range
        was_catching = catching;
        now += 1.0 / 60.0;
    }
    let (a, b) = (
        started.expect("first catch"),
        second_start.expect("second catch"),
    );
    assert!(
        b - a > 45.0 + 5.0,
        "second catch only {:.0} s after the first",
        b - a
    );
}
