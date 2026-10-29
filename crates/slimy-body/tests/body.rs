use slimy_body::{
    BodyParams, BodyShape, Bounds, Breathing, Contact, Pointer, SoftBody, Surface, Vec2,
};

const DT: f32 = 1.0 / 120.0;

fn params() -> BodyParams {
    BodyParams {
        radius: 100.0,
        bounds: Bounds {
            left: 0.0,
            right: 900.0,
            floor: 500.0,
            top: -10_000.0,
        },
    }
}

fn run(body: &mut SoftBody, seconds: f32, pointer: &Pointer) {
    for _ in 0..(seconds / DT) as usize {
        body.step(DT, pointer);
    }
}

fn settled() -> SoftBody {
    let mut body = SoftBody::new(params(), Vec2::new(450.0, 150.0));
    run(&mut body, 4.0, &Pointer::AWAY);
    body
}

fn extent(body: &SoftBody) -> (f32, f32) {
    let (min, max) = body.bounding_box();
    (max.x - min.x, max.y - min.y)
}

#[test]
fn falls_and_rests_on_the_floor_with_only_breathing_left() {
    let body = settled();
    let (_, max) = body.bounding_box();
    assert!(max.y <= 500.01, "body went under the floor: {}", max.y);
    assert!(
        max.y >= 498.0,
        "body is not resting on the floor: {}",
        max.y
    );
    assert!(
        body.speed() < 10.0,
        "only breathing should be left: {}",
        body.speed()
    );
}

#[test]
fn keeps_its_volume_when_pressed_from_above() {
    let mut body = settled();
    let at_rest = body.area();
    let top = Pointer {
        pos: Vec2::new(450.0, 500.0 - extent(&body).1 + 5.0),
        contact: Contact::Down,
    };
    run(&mut body, 1.0, &top);
    let ratio = body.area() / at_rest;
    assert!((0.85..=1.1).contains(&ratio), "area ratio {ratio}");
}

#[test]
fn stretches_when_grabbed_and_recovers_after_release() {
    let mut body = settled();
    let (_, rest_height) = extent(&body);
    let grab_at = Vec2::new(450.0, 500.0 - rest_height * 0.7);
    run(
        &mut body,
        0.05,
        &Pointer {
            pos: grab_at,
            contact: Contact::Down,
        },
    );
    assert!(body.is_grabbed());
    for i in 1..=60 {
        let lift = i as f32 / 60.0 * 180.0;
        let pos = Vec2::new(grab_at.x, grab_at.y - lift);
        body.step(
            DT * 2.0,
            &Pointer {
                pos,
                contact: Contact::Down,
            },
        );
    }
    assert!(
        extent(&body).1 > rest_height * 1.3,
        "not stretched: {}",
        extent(&body).1
    );
    run(&mut body, 4.0, &Pointer::AWAY);
    assert!(!body.is_grabbed());
    let (_, height) = extent(&body);
    assert!(
        (height / rest_height - 1.0).abs() < 0.1,
        "did not recover: {height} vs {rest_height}"
    );
}

#[test]
fn same_inputs_give_identical_points() {
    let a = settled();
    let b = settled();
    assert_eq!(a.points(), b.points());
}

#[test]
fn stays_inside_the_side_walls() {
    let mut body = SoftBody::new(params(), Vec2::new(450.0, 400.0));
    body.impulse(Vec2::new(4000.0, -300.0));
    run(&mut body, 3.0, &Pointer::AWAY);
    let (min, max) = body.bounding_box();
    assert!(
        min.x >= -0.01 && max.x <= 900.01,
        "left {} right {}",
        min.x,
        max.x
    );
}

#[test]
fn a_settled_body_is_slightly_squashed_against_its_rest_height() {
    let body = settled();
    let (_, height) = extent(&body);
    let ratio = height / body.rest_height();
    assert!((0.7..1.0).contains(&ratio), "settled/rest = {ratio}");
}

#[test]
fn moving_the_floor_up_lifts_the_body() {
    let mut body = settled();
    body.set_bounds(Bounds {
        left: 0.0,
        right: 900.0,
        floor: 400.0,
        top: -10_000.0,
    });
    run(&mut body, 3.0, &Pointer::AWAY);
    let (_, max) = body.bounding_box();
    assert!((max.y - 400.0).abs() < 2.0, "bottom at {}", max.y);
}

#[test]
fn falls_freely_instead_of_floating() {
    let high = BodyParams {
        radius: 100.0,
        bounds: Bounds {
            left: 0.0,
            right: 900.0,
            floor: 5000.0,
            top: -10_000.0,
        },
    };
    let mut body = SoftBody::new(high, Vec2::new(450.0, 100.0));
    run(&mut body, 1.0, &Pointer::AWAY);
    let fallen = body.centroid().y - 100.0;
    assert!(fallen > 350.0, "fell only {fallen} px in one second");
}

fn near_left_wall() -> SoftBody {
    let mut body = SoftBody::new(params(), Vec2::new(115.0, 300.0));
    body.impulse(Vec2::new(-150.0, 0.0));
    run(&mut body, 3.0, &Pointer::AWAY);
    body
}

#[test]
fn clings_to_a_wall_it_touches_slowly() {
    let body = near_left_wall();
    assert_eq!(body.surface(), Surface::Left);
    let (min, _) = body.bounding_box();
    assert!(min.x < 1.0, "not against the wall: {}", min.x);
    assert!(
        body.centroid().y < 400.0,
        "slid down the wall: {}",
        body.centroid().y
    );
}

#[test]
fn points_its_top_away_from_the_surface_it_clings_to() {
    let body = near_left_wall();
    assert!(body.up().x > 0.99, "up = {:?}", body.up());
    assert!(
        body.orientation() > 1.4,
        "not turned yet: {}",
        body.orientation()
    );
}

#[test]
fn clings_to_the_ceiling() {
    let mut body = SoftBody::new(params(), Vec2::new(450.0, 130.0));
    body.impulse(Vec2::new(0.0, -250.0));
    body.set_bounds(Bounds {
        top: 40.0,
        ..params().bounds
    });
    run(&mut body, 3.0, &Pointer::AWAY);
    assert_eq!(body.surface(), Surface::Ceiling);
    assert!(body.up().y > 0.99, "up = {:?}", body.up());
}

#[test]
fn falls_back_to_the_floor_when_the_wall_goes_away() {
    let mut body = near_left_wall();
    body.set_bounds(Bounds {
        left: -600.0,
        ..params().bounds
    });
    run(&mut body, 3.0, &Pointer::AWAY);
    assert_eq!(body.surface(), Surface::Floor);
    let (_, max) = body.bounding_box();
    assert!(max.y > 495.0, "did not land: {}", max.y);
}

#[test]
fn bounces_off_a_wall_when_thrown_fast() {
    let mut body = SoftBody::new(params(), Vec2::new(300.0, 300.0));
    body.impulse(Vec2::new(-2500.0, 0.0));
    run(&mut body, 0.35, &Pointer::AWAY);
    assert_ne!(body.surface(), Surface::Left);
}

fn farthest_from_centroid(body: &SoftBody) -> f32 {
    let c = body.centroid();
    body.points()
        .iter()
        .map(|p| (*p - c).len())
        .fold(0.0, f32::max)
}

#[test]
fn a_violent_drag_stretches_the_gel_without_tearing_it() {
    let mut body = settled();
    let (_, rest_height) = extent(&body);
    let start = Vec2::new(450.0, 500.0 - rest_height * 0.6);
    run(
        &mut body,
        0.05,
        &Pointer {
            pos: start,
            contact: Contact::Down,
        },
    );
    let mut worst: f32 = 0.0;
    for i in 1..=15 {
        let pos = Vec2::new(start.x + i as f32 * 60.0, start.y - i as f32 * 20.0);
        body.step(
            1.0 / 60.0,
            &Pointer {
                pos,
                contact: Contact::Down,
            },
        );
        worst = worst.max(farthest_from_centroid(&body));
        assert!(body
            .points()
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite()));
    }
    assert!(
        worst < 2.6 * 100.0,
        "outline flew {worst} px from its center"
    );
    run(&mut body, 5.0, &Pointer::AWAY);
    let (width, _) = extent(&body);
    assert!(width < 2.6 * 100.0, "did not pull back together: {width}");
}

#[test]
fn the_gap_to_the_surface_is_zero_at_rest_and_grows_in_the_air() {
    let mut body = settled();
    assert!(
        body.surface_gap() < 1.0,
        "resting gap {}",
        body.surface_gap()
    );
    let grab_at = Vec2::new(450.0, 500.0 - extent(&body).1 * 0.6);
    run(
        &mut body,
        0.05,
        &Pointer {
            pos: grab_at,
            contact: Contact::Down,
        },
    );
    for i in 1..=90 {
        let pos = Vec2::new(grab_at.x, grab_at.y - i as f32 * 3.0);
        body.step(
            DT * 2.0,
            &Pointer {
                pos,
                contact: Contact::Down,
            },
        );
    }
    assert!(
        body.surface_gap() > 60.0,
        "lifted gap {}",
        body.surface_gap()
    );
}

#[test]
fn the_gap_is_measured_toward_the_wall_it_clings_to() {
    let body = near_left_wall();
    assert!(
        body.surface_gap() < 1.5,
        "gap to the wall {}",
        body.surface_gap()
    );
}

#[test]
fn a_flatter_shape_gives_a_shorter_rest_height() {
    let mut body = settled();
    let before = body.rest_height();
    body.set_shape(BodyShape {
        top: 0.6,
        ..BodyShape::default()
    });
    assert!(
        body.rest_height() < before * 0.85,
        "{} vs {before}",
        body.rest_height()
    );
}

#[test]
fn without_breathing_a_settled_body_is_perfectly_still() {
    let mut body = SoftBody::new(params(), Vec2::new(450.0, 150.0));
    body.set_breathing(Breathing {
        speed: 0.0,
        width: 0.0,
        height: 0.0,
    });
    run(&mut body, 6.0, &Pointer::AWAY);
    assert!(body.speed() < 1.0, "still moving at {}", body.speed());
}

fn lift(body: &mut SoftBody, at: Vec2, rise: f32, seconds: f32) {
    run(
        body,
        0.05,
        &Pointer {
            pos: at,
            contact: Contact::Down,
        },
    );
    let steps = (seconds / (DT * 2.0)) as usize;
    for i in 1..=steps {
        let pos = Vec2::new(at.x, at.y - rise * i as f32 / steps as f32);
        body.step(
            DT * 2.0,
            &Pointer {
                pos,
                contact: Contact::Down,
            },
        );
    }
    run(
        body,
        1.5,
        &Pointer {
            pos: Vec2::new(at.x, at.y - rise),
            contact: Contact::Down,
        },
    );
}

#[test]
fn a_body_lifted_by_its_side_hangs_and_tilts() {
    let mut body = settled();
    let c = body.centroid();
    lift(&mut body, Vec2::new(c.x + 60.0, c.y), 260.0, 1.2);
    assert!(body.is_grabbed());
    assert!(
        body.orientation().abs() > 0.45,
        "stayed upright: {}",
        body.orientation()
    );
}

#[test]
fn a_body_lifted_by_its_top_hangs_without_tilting() {
    let mut body = settled();
    let c = body.centroid();
    lift(&mut body, Vec2::new(c.x, c.y - 50.0), 260.0, 1.2);
    assert!(
        body.orientation().abs() < 0.35,
        "tilted: {}",
        body.orientation()
    );
}

#[test]
fn a_tilted_body_rights_itself_once_released_on_the_floor() {
    let mut body = settled();
    let c = body.centroid();
    lift(&mut body, Vec2::new(c.x + 60.0, c.y), 120.0, 0.8);
    run(&mut body, 6.0, &Pointer::AWAY);
    assert!(
        body.orientation().abs() < 0.12,
        "still tilted: {}",
        body.orientation()
    );
    let (_, max) = body.bounding_box();
    assert!(max.y > 495.0, "not back on the floor: {}", max.y);
}

#[test]
fn a_hop_carries_the_body_across_the_floor() {
    let mut body = settled();
    let start = body.centroid();
    body.hop(Vec2::new(0.66, -0.75), 520.0);
    run(&mut body, 0.12, &Pointer::AWAY);
    assert!(
        body.surface_gap() > 15.0,
        "did not leave the floor: {}",
        body.surface_gap()
    );
    run(&mut body, 3.0, &Pointer::AWAY);
    let moved = body.centroid().x - start.x;
    assert!((70.0..260.0).contains(&moved), "hopped {moved} px");
    assert_eq!(body.surface(), Surface::Floor);
}

#[test]
fn a_stronger_hop_goes_farther() {
    let mut short = settled();
    let mut long = settled();
    short.hop(Vec2::new(0.66, -0.75), 400.0);
    long.hop(Vec2::new(0.66, -0.75), 650.0);
    run(&mut short, 3.0, &Pointer::AWAY);
    run(&mut long, 3.0, &Pointer::AWAY);
    assert!(long.centroid().x > short.centroid().x + 40.0);
}

#[test]
fn a_crouch_squashes_the_body_before_the_leap() {
    let mut body = settled();
    let (_, rest) = extent(&body);
    for _ in 0..8 {
        body.crouch(40.0);
        body.step(DT, &Pointer::AWAY);
    }
    let (_, squashed) = extent(&body);
    assert!(squashed < rest * 0.93, "{squashed} vs {rest}");
}

#[test]
fn hopping_up_a_wall_climbs_it() {
    let mut body = near_left_wall();
    assert_eq!(body.surface(), Surface::Left);
    let before = body.centroid().y;
    for _ in 0..3 {
        body.hop(Vec2::new(0.3, -0.95), 480.0);
        run(&mut body, 1.4, &Pointer::AWAY);
    }
    assert!(
        body.centroid().y < before - 60.0,
        "climbed from {before} to {}",
        body.centroid().y
    );
    assert_eq!(body.surface(), Surface::Left);
}

#[test]
fn the_impact_of_a_landing_is_reported_and_a_resting_body_reports_none() {
    let high = BodyParams {
        radius: 64.0,
        bounds: Bounds {
            left: 0.0,
            right: 900.0,
            floor: 800.0,
            top: -10_000.0,
        },
    };
    let mut body = SoftBody::new(high, Vec2::new(450.0, 100.0));
    let mut hardest: f32 = 0.0;
    for _ in 0..(2.5 / DT) as usize {
        body.step(DT, &Pointer::AWAY);
        hardest = hardest.max(body.last_impact());
    }
    assert!(hardest > 450.0, "a long fall hit at {hardest}");
    run(&mut body, 3.0, &Pointer::AWAY);
    assert!(
        body.last_impact() < 30.0,
        "resting impact {}",
        body.last_impact()
    );
}

#[test]
fn an_ordinary_hop_lands_softly() {
    let mut body = settled();
    let mut hardest: f32 = 0.0;
    body.hop(Vec2::new(0.66, -0.75), 480.0);
    for _ in 0..(2.0 / DT) as usize {
        body.step(DT, &Pointer::AWAY);
        hardest = hardest.max(body.last_impact());
    }
    assert!(hardest < 450.0, "a hop landed at {hardest}");
}

#[test]
fn a_hop_leans_the_body_into_its_direction() {
    let mut right = settled();
    let mut left = settled();
    right.hop(Vec2::new(0.66, -0.75), 480.0);
    left.hop(Vec2::new(-0.66, -0.75), 480.0);
    let (mut lean_right, mut lean_left) = (0.0f32, 0.0f32);
    for _ in 0..(0.25 / DT) as usize {
        right.step(DT, &Pointer::AWAY);
        left.step(DT, &Pointer::AWAY);
        lean_right = lean_right.max(right.orientation());
        lean_left = lean_left.min(left.orientation());
    }
    assert!(lean_right > 0.06, "right lean {lean_right}");
    assert!(lean_left < -0.06, "left lean {lean_left}");
}

#[test]
fn resting_in_a_bottom_corner_does_not_flip_between_floor_and_wall() {
    for x in [45.0, 855.0] {
        let mut body = SoftBody::new(params(), Vec2::new(x, 420.0));
        run(&mut body, 3.0, &Pointer::AWAY);
        let mut previous = body.surface();
        let mut changes = 0;
        for _ in 0..1200 {
            body.step(DT, &Pointer::AWAY);
            if body.surface() != previous {
                changes += 1;
            }
            previous = body.surface();
        }
        assert!(
            changes <= 1,
            "corner {x}: {changes} surface changes while resting"
        );
    }
}

#[test]
#[ignore]
fn sweep_hop_reach() {
    for dir in [(0.66f32, -0.75f32), (0.8, -0.6), (0.9, -0.44)] {
        for power in [300.0, 400.0, 500.0, 600.0, 700.0, 800.0] {
            let mut body = settled();
            let start = body.centroid();
            body.crouch(40.0 * 10.0);
            run(&mut body, 0.16, &Pointer::AWAY);
            body.hop(Vec2::new(dir.0, dir.1), power);
            let mut hardest: f32 = 0.0;
            for _ in 0..(3.0 / DT) as usize {
                body.step(DT, &Pointer::AWAY);
                hardest = hardest.max(body.last_impact());
            }
            println!(
                "dir {dir:?} power {power}: reach {:.0} px, impact {:.0}, surface {:?}",
                body.centroid().x - start.x,
                hardest,
                body.surface()
            );
        }
    }
}

#[test]
fn a_pseudopod_reaches_toward_a_direction_and_comes_back() {
    let mut body = settled();
    let (rest_min, rest_max) = body.bounding_box();
    let rest_area = body.area();
    body.reach_toward(Vec2::new(1.0, -0.2), 1.0);
    run(&mut body, 1.0, &Pointer::AWAY);
    let (_, max) = body.bounding_box();
    assert!(
        max.x - rest_max.x > 35.0,
        "arm only reached {} px",
        max.x - rest_max.x
    );
    let tip = body.arm_tip().expect("tip");
    assert!(tip.x > body.centroid().x + 70.0);
    assert!(
        (body.area() / rest_area - 1.0).abs() < 0.25,
        "area {} vs {}",
        body.area(),
        rest_area
    );
    assert_eq!(body.surface(), Surface::Floor);
    body.retract();
    run(&mut body, 2.0, &Pointer::AWAY);
    let (min, max) = body.bounding_box();
    assert!(
        (max.x - min.x) - (rest_max.x - rest_min.x) < 6.0,
        "still wide: {}",
        max.x - min.x
    );
    assert!(body.arm_tip().is_none());
}

#[test]
fn a_pseudopod_does_not_push_the_body_along() {
    let mut body = settled();
    let start = body.centroid().x;
    body.reach_toward(Vec2::new(-1.0, -0.4), 1.0);
    run(&mut body, 1.5, &Pointer::AWAY);
    assert!(
        (body.centroid().x - start).abs() < 25.0,
        "drifted {}",
        body.centroid().x - start
    );
}

#[test]
fn a_pseudopod_aimed_up_keeps_the_body_stable() {
    let mut body = settled();
    body.reach_toward(Vec2::new(0.0, -1.0), 1.0);
    run(&mut body, 2.0, &Pointer::AWAY);
    assert!(
        body.orientation().abs() < 0.2,
        "tilted {}",
        body.orientation()
    );
    assert!(body.arm_extent() > 0.5);
}

fn wide_profile() -> Vec<f32> {
    // Twice as wide at the sides (angle 0 and pi), unchanged above and below.
    (0..56)
        .map(|i| {
            1.0 + 0.6
                * (2.0 * i as f32 / 56.0 * std::f32::consts::TAU)
                    .cos()
                    .powi(2)
        })
        .collect()
}

#[test]
fn a_metamorphosis_reshapes_the_body_and_settles_without_drifting_or_tearing() {
    let mut body = settled();
    let (min, max) = body.bounding_box();
    let before = (max.x - min.x, body.centroid().x);
    body.set_profile(&wide_profile());
    run(&mut body, 0.3, &Pointer::AWAY);
    let midway = body.bounding_box();
    run(&mut body, 4.0, &Pointer::AWAY);
    let (min, max) = body.bounding_box();
    assert!(
        (max.x - min.x) > before.0 * 1.25,
        "width {} vs {}",
        max.x - min.x,
        before.0
    );
    assert!(
        ((midway.1.x - midway.0.x) - before.0) < (max.x - min.x) - before.0,
        "no gradual change"
    );
    assert!(
        (body.centroid().x - before.1).abs() < 10.0,
        "drifted {}",
        body.centroid().x - before.1
    );
    assert_eq!(body.surface(), Surface::Floor);
    assert!(body.surface_gap() < 3.0);
    assert!(body.speed() < 30.0, "still moving at {}", body.speed());
}

#[test]
fn an_asymmetric_form_does_not_push_itself_along_nor_tilt() {
    let mut body = settled();
    let tail: Vec<f32> = (0..56)
        .map(|i| 1.0 + 0.7 * (-((i as f32 - 49.0) / 2.5).powi(2)).exp())
        .collect();
    body.set_profile(&tail);
    let start = body.centroid().x;
    run(&mut body, 5.0, &Pointer::AWAY);
    assert!(
        (body.centroid().x - start).abs() < 12.0,
        "walked {}",
        body.centroid().x - start
    );
    assert!(
        body.orientation().abs() < 0.25,
        "tilted {}",
        body.orientation()
    );
}

#[test]
fn a_wrong_sized_profile_is_ignored() {
    let (mut touched, mut control) = (settled(), settled());
    touched.set_profile(&[3.0, 3.0]);
    run(&mut touched, 2.0, &Pointer::AWAY);
    run(&mut control, 2.0, &Pointer::AWAY);
    let width = |b: &SoftBody| {
        let (lo, hi) = b.bounding_box();
        hi.x - lo.x
    };
    assert!((width(&touched) - width(&control)).abs() < 0.01);
}

#[test]
#[ignore]
fn probe_forms() {
    for name in ["slime", "crab", "cloud", "whale"] {
        let text =
            std::fs::read_to_string(format!("/tmp/claude-1000/profile_{name}.json")).unwrap();
        let profile: Vec<f32> = text
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|v| v.parse().unwrap())
            .collect();
        let mut body = settled();
        let rest_area = body.area();
        body.set_profile(&profile);
        run(&mut body, 6.0, &Pointer::AWAY);
        let (lo, hi) = body.bounding_box();
        println!(
            "{name}: w {:.0} h {:.0} area {:.2}x orient {:.2} surface {:?} gap {:.1}",
            hi.x - lo.x,
            hi.y - lo.y,
            body.area() / rest_area,
            body.orientation(),
            body.surface(),
            body.surface_gap()
        );
    }
}

#[test]
#[ignore]
fn probe_preview_scene() {
    let params = BodyParams {
        radius: 64.0,
        bounds: Bounds {
            left: 0.0,
            right: 560.0,
            floor: 323.0,
            top: 0.0,
        },
    };
    let mut body = SoftBody::new(params, Vec2::new(280.0, 120.0));
    body.set_shape(BodyShape {
        top: 0.9,
        bottom: 0.5,
        base_width: 1.04,
    });
    body.set_breathing(Breathing {
        speed: 1.7,
        width: 0.016,
        height: 0.026,
    });
    run(&mut body, 4.0, &Pointer::AWAY);
    run(&mut body, 6.0, &Pointer::AWAY);
    let (lo, hi) = body.bounding_box();
    println!(
        "PROBE bbox x {:.0}..{:.0} y {:.0}..{:.0} orient {:.3} surface {:?}",
        lo.x,
        hi.x,
        lo.y,
        hi.y,
        body.orientation(),
        body.surface()
    );
    let pts = body.points();
    println!("PROBE top-left {:?} top-right {:?}", pts[42], pts[14]);
}

#[test]
fn a_long_broad_lobe_lets_the_body_flow_far_toward_a_target_and_keeps_its_volume() {
    let mut narrow = settled();
    let mut flow = settled();
    let rest = flow.area();
    let (_, rest_max) = flow.bounding_box();
    narrow.reach_toward(Vec2::new(1.0, -0.2), 1.0);
    flow.reach_toward(Vec2::new(1.0, -0.2), 1.0);
    flow.set_arm_style(2.2, 0.85);
    run(&mut narrow, 1.5, &Pointer::AWAY);
    run(&mut flow, 1.5, &Pointer::AWAY);
    let reached = |b: &SoftBody| b.bounding_box().1.x - rest_max.x;
    assert!(
        reached(&flow) > reached(&narrow) * 1.25 && reached(&flow) > 120.0,
        "flow {} vs arm {}",
        reached(&flow),
        reached(&narrow)
    );
    assert!(
        (flow.area() / rest - 1.0).abs() < 0.3,
        "volume {} vs {}",
        flow.area(),
        rest
    );
    assert_eq!(flow.surface(), Surface::Floor);
    assert!(
        flow.surface_gap() < 3.0,
        "lifted off: {}",
        flow.surface_gap()
    );
    flow.retract();
    run(&mut flow, 3.0, &Pointer::AWAY);
    assert!(flow.arm_tip().is_none());
}

#[test]
fn the_lobe_style_changes_gradually_and_is_clamped() {
    let mut body = settled();
    body.reach_toward(Vec2::new(1.0, 0.0), 1.0);
    body.set_arm_style(99.0, -5.0);
    run(&mut body, 2.0, &Pointer::AWAY);
    let (_, max) = body.bounding_box();
    assert!(max.x.is_finite() && body.area().is_finite() && body.area() < 4.0 * settled().area());
}

#[test]
fn the_fitted_lobe_rule_puts_the_tip_on_a_cursor_one_and_a_half_to_two_radii_away() {
    let radius = params().radius;
    for x in [1.5f32, 1.6, 1.75, 1.9, 2.0] {
        for side in [-1.0f32, 1.0] {
            let mut body = settled();
            let c = body.centroid();
            let length = 1.95 * (x - 1.0);
            body.set_arm_style(length, 0.65 + 0.125 * length);
            body.reach_toward(Vec2::new(side, 0.0), 1.0);
            run(&mut body, 2.0, &Pointer::AWAY);
            let tip = body.arm_tip().expect("a tip");
            let reach = (tip.x - c.x).abs();
            assert!(
                (reach - x * radius).abs() < 12.0,
                "x {x} side {side}: tip at {reach:.0} px, wanted {:.0}",
                x * radius
            );
            assert!(body.surface_gap() < 3.0 && body.orientation().abs() < 0.2);
        }
    }
}

/// The calibration scene of the catch: radius 64, the model's shape and breathing, settled on a floor at y = 1100.
fn model_body() -> SoftBody {
    let params = BodyParams {
        radius: 64.0,
        bounds: Bounds {
            left: 0.0,
            right: 1900.0,
            floor: 1100.0,
            top: 0.0,
        },
    };
    let mut body = SoftBody::new(params, Vec2::new(900.0, 700.0));
    body.set_shape(BodyShape {
        top: 0.9,
        bottom: 0.5,
        base_width: 1.04,
    });
    body.set_breathing(Breathing {
        speed: 1.7,
        width: 0.016,
        height: 0.026,
    });
    run(&mut body, 7.5, &Pointer::AWAY);
    body
}

#[test]
fn a_level_approach_hop_lands_on_its_target_without_turning_or_rising() {
    // Fitted on the real body: vx = sign(dx) * (10.980972 |dx| - 6.478247), vy = -110, after a soft crouch of 19 steps.
    for dx in [-70.4f32, -40.0, -20.0, -6.4, 6.4, 20.0, 40.0, 70.4] {
        let mut body = model_body();
        let start = body.centroid();
        for _ in 0..19 {
            body.crouch(5.0 * DT * 60.0);
            body.step(DT, &Pointer::AWAY);
        }
        let vx = dx.signum() * (10.980972 * dx.abs() - 6.478247);
        let power = vx.hypot(110.0);
        body.leap_level(Vec2::new(vx / power, -110.0 / power), power);
        let mut highest = f32::MAX;
        let mut turned = 0.0f32;
        for _ in 0..(1.0 / DT) as usize {
            body.step(DT, &Pointer::AWAY);
            highest = highest.min(body.centroid().y);
            turned = turned.max(body.orientation().abs());
        }
        let moved = body.centroid().x - start.x;
        assert!((moved - dx).abs() < 6.0, "dx {dx}: moved {moved:.1}");
        assert!(
            start.y - highest < 9.0,
            "dx {dx}: rose {:.1} px",
            start.y - highest
        );
        assert!(turned < 0.45, "dx {dx}: turned {turned:.3} rad in flight");
        assert!(
            body.orientation().abs() < 0.08,
            "dx {dx}: ends tilted by {:.3}",
            body.orientation()
        );
        assert_eq!(body.surface(), Surface::Floor);
    }
}

#[test]
fn a_hop_with_the_lean_turns_where_the_level_leap_does_not() {
    let mut leaning = model_body();
    let mut level = model_body();
    leaning.hop(Vec2::new(0.9, -0.4), 300.0);
    level.leap_level(Vec2::new(0.9, -0.4), 300.0);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    for _ in 0..(0.3 / DT) as usize {
        leaning.step(DT, &Pointer::AWAY);
        level.step(DT, &Pointer::AWAY);
        a = a.max(leaning.orientation().abs());
        b = b.max(level.orientation().abs());
    }
    assert!(a > b * 1.3 + 0.01, "lean {a:.3} vs level {b:.3}");
}

#[test]
fn flattening_spreads_the_body_wide_and_low_and_keeps_its_volume_then_gives_it_back() {
    let mut body = model_body();
    let (w0, h0) = extent(&body);
    let area0 = body.area();
    body.set_flatten(1.0);
    run(&mut body, 3.0, &Pointer::AWAY);
    let (w, h) = extent(&body);
    assert!(
        h < h0 * 0.92 && w > w0 * 1.1,
        "flat: {w:.0}x{h:.0} from {w0:.0}x{h0:.0}"
    );
    assert!(
        (0.9..1.16).contains(&(body.area() / area0)),
        "area {:.2}",
        body.area() / area0
    );
    assert!(body.surface_gap() < 3.0 && body.orientation().abs() < 0.1);
    body.set_flatten(0.0);
    run(&mut body, 4.0, &Pointer::AWAY);
    let (w2, h2) = extent(&body);
    assert!(
        (w2 / w0 - 1.0).abs() < 0.05 && (h2 / h0 - 1.0).abs() < 0.05,
        "back: {w2:.0}x{h2:.0} vs {w0:.0}x{h0:.0}"
    );
}

#[test]
fn flattening_composes_with_a_form_profile_and_ignores_nonsense() {
    let mut body = model_body();
    let wide: Vec<f32> = (0..56)
        .map(|i| {
            1.0 + 0.3
                * (2.0 * i as f32 / 56.0 * std::f32::consts::TAU)
                    .cos()
                    .powi(2)
        })
        .collect();
    body.set_profile(&wide);
    run(&mut body, 3.0, &Pointer::AWAY);
    let (w_form, _) = extent(&body);
    body.set_flatten(1.0);
    run(&mut body, 3.0, &Pointer::AWAY);
    let (w_both, _) = extent(&body);
    assert!(
        w_both > w_form,
        "flat on top of a wide form: {w_both:.0} vs {w_form:.0}"
    );
    for bad in [f32::NAN, -3.0, 9.0] {
        body.set_flatten(bad);
        assert!((0.0..=1.0).contains(&body.flatten()));
    }
}

#[test]
fn a_smaller_world_brings_the_body_inside_and_a_bigger_one_leaves_it_alone() {
    let params = BodyParams {
        radius: 64.0,
        bounds: Bounds {
            left: 0.0,
            right: 2560.0,
            floor: 1440.0,
            top: 0.0,
        },
    };
    let mut body = SoftBody::new(params, Vec2::new(2300.0, 1300.0));
    for _ in 0..300 {
        body.step(1.0 / 120.0, &Pointer::AWAY);
    }
    let before = body.centroid();
    body.resize_world(Bounds {
        left: 0.0,
        right: 4000.0,
        floor: 2000.0,
        top: 0.0,
    });
    assert_eq!(
        (body.centroid().x, body.centroid().y),
        (before.x, before.y),
        "a bigger world changes nothing"
    );
    let area = body.area();
    // the second screen is unplugged: the world shrinks to 1920 x 1080
    body.resize_world(Bounds {
        left: 0.0,
        right: 1920.0,
        floor: 1080.0,
        top: 0.0,
    });
    let c = body.centroid();
    assert!(
        c.x <= 1920.0 - 64.0 + 0.1 && c.y <= 1080.0 - 64.0 + 0.1,
        "inside the new world: {c:?}"
    );
    assert!((body.area() - area).abs() < 1.0, "moved, not squashed");
    for _ in 0..600 {
        body.step(1.0 / 120.0, &Pointer::AWAY);
    }
    let c = body.centroid();
    assert!(
        c.x < 1920.0 && c.y < 1080.0,
        "and it stays inside while it lives on"
    );
}
