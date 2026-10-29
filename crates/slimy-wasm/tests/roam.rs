//! The slime's initiative driving the real body, the way the page does it.

use slimy_body::{BodyParams, Bounds, Pointer, SoftBody, Surface, Vec2};
use slimy_core::{Autonomy, Place, Snapshot, World};

const WORLD: World = World {
    left: 0.0,
    right: 1800.0,
    top: 30.0,
    bottom: 1100.0,
};
const STEP: f32 = 1.0 / 120.0;

struct Scene {
    body: SoftBody,
    ai: Autonomy,
    now: f64,
    crouch_calls: u32,
    hops: u32,
    cursor: Option<Vec2>,
    affection: f32,
    radius: f32,
    reach: f32,
}

impl Scene {
    fn new(seed: u64, x: f32) -> Self {
        Self::sized(seed, x, 64.0)
    }

    fn sized(seed: u64, x: f32, radius: f32) -> Self {
        let params = BodyParams {
            radius,
            bounds: Bounds {
                left: WORLD.left,
                right: WORLD.right,
                floor: WORLD.bottom,
                top: WORLD.top,
            },
        };
        let mut body = SoftBody::new(params, Vec2::new(x, WORLD.bottom - 200.0));
        for _ in 0..600 {
            body.step(STEP, &Pointer::AWAY);
        }
        Scene {
            body,
            ai: Autonomy::new(seed),
            now: 0.0,
            crouch_calls: 0,
            hops: 0,
            cursor: None,
            affection: 0.5,
            radius,
            reach: radius,
        }
    }

    fn place(&self) -> Place {
        match self.body.surface() {
            Surface::Floor => Place::Floor,
            Surface::Left => Place::Left,
            Surface::Right => Place::Right,
            Surface::Ceiling => Place::Ceiling,
        }
    }

    /// One frame of 1/60 s, with the page's wiring.
    fn frame(&mut self, sulking: bool, sleepiness: f32) -> slimy_core::Output {
        self.frame_at(sulking, sleepiness, 60)
    }

    fn frame_at(&mut self, sulking: bool, sleepiness: f32, fps: u32) -> slimy_core::Output {
        let mut out = self.tick(sulking, sleepiness);
        for _ in 1..120 / fps {
            out = self.tick(sulking, sleepiness);
        }
        out
    }

    fn tick(&mut self, sulking: bool, sleepiness: f32) -> slimy_core::Output {
        let c = self.body.centroid();
        let snap = Snapshot {
            now: self.now,
            x: c.x,
            y: c.y,
            place: self.place(),
            supported: self.body.surface_gap() < 3.0,
            settled: self.body.surface_gap() < 3.0 && self.body.speed() < 60.0,
            grabbed: false,
            busy: false,
            sulking,
            sleepiness,
            affection: self.affection,
            pointer: self.cursor.map(|c| (c.x, c.y)),
            pointer_active: self.cursor.is_some(),
            radius: self.radius,
            reach: self.reach,
            world: WORLD,
        };
        let out = self.ai.step(&snap);
        // The page's wiring: a catch crouches softly and hops level, and asks for the whole body to flatten.
        if out.crouch {
            let depth = if out.gentle { 5.0 } else { 40.0 };
            self.body.crouch(depth * STEP * 60.0);
            self.crouch_calls += 1;
        }
        if let Some((dx, dy, power)) = out.hop {
            if out.gentle {
                self.body.leap_level(Vec2::new(dx, dy), power);
            } else {
                self.body.hop(Vec2::new(dx, dy), power);
            }
            self.hops += 1;
        }
        self.body.set_flatten(out.flatten);
        self.body.step(STEP, &Pointer::AWAY);
        self.now += f64::from(STEP);
        out
    }

    fn run(&mut self, seconds: f64, sulking: bool, sleepiness: f32) {
        let end = self.now + seconds;
        while self.now < end {
            self.frame(sulking, sleepiness);
        }
    }
}

#[test]
fn left_alone_the_slime_explores_the_floor_and_stays_on_screen() {
    let mut scene = Scene::new(3, 900.0);
    let mut seen = (f32::MAX, f32::MIN);
    for _ in 0..(600.0 * 60.0) as usize {
        scene.frame(false, 0.0);
        let c = scene.body.centroid();
        seen = (seen.0.min(c.x), seen.1.max(c.x));
        let (min, max) = scene.body.bounding_box();
        assert!(
            min.x >= WORLD.left - 1.0 && max.x <= WORLD.right + 1.0 && max.y <= WORLD.bottom + 1.0
        );
    }
    assert!(scene.hops >= 5, "only {} hops in ten minutes", scene.hops);
    assert!(
        seen.1 - seen.0 > 300.0,
        "explored only {} px",
        seen.1 - seen.0
    );
}

#[test]
fn a_sulking_slime_ends_up_in_a_bottom_corner() {
    let mut scene = Scene::new(21, 900.0);
    scene.run(120.0, true, 0.0);
    let x = scene.body.centroid().x;
    assert!(x < 200.0 || x > WORLD.right - 200.0, "sulking at {x}");
    assert_eq!(scene.body.surface(), Surface::Floor);
}

#[test]
fn a_sleepy_slime_curls_up_in_a_corner() {
    let mut scene = Scene::new(2, 900.0);
    scene.run(120.0, false, 0.9);
    let x = scene.body.centroid().x;
    assert!(x < 200.0 || x > WORLD.right - 200.0, "napping at {x}");
}

#[test]
fn the_slime_sometimes_climbs_a_wall_and_comes_back_down() {
    let mut climbed = false;
    for seed in 1..=8u64 {
        let mut scene = Scene::new(seed, 300.0);
        for _ in 0..(900.0 * 60.0) as usize {
            scene.frame(false, 0.0);
            if matches!(scene.place(), Place::Left | Place::Right) {
                climbed = true;
            }
        }
        assert!(matches!(
            scene.place(),
            Place::Floor | Place::Left | Place::Right
        ));
        if climbed {
            break;
        }
    }
    assert!(
        climbed,
        "never reached a wall in eight lives of fifteen minutes"
    );
}

#[test]
fn both_sleeping_corners_remain_still_at_resting_frame_rates() {
    for fps in [6, 12, 60] {
        for x in [300.0, 1500.0] {
            let mut scene = Scene::new(2, x);
            for _ in 0..120 * fps {
                scene.frame_at(false, 0.9, fps);
            }
            let start = scene.body.centroid();
            let hops = scene.hops;
            assert!(
                start.x < 200.0 || start.x > WORLD.right - 200.0,
                "{fps} fps: did not arrive from {x}, at {start:?}"
            );
            for _ in 0..30 * fps {
                let out = scene.frame_at(false, 0.9, fps);
                assert_eq!(
                    out.intent,
                    slimy_core::Intent::Nap,
                    "{fps} fps: still travelling from {x}"
                );
                assert_eq!(scene.body.surface(), Surface::Floor);
                assert!((scene.body.centroid().x - start.x).abs() < 15.0);
            }
            assert_eq!(scene.hops, hops, "{fps} fps: repeated hops in the corner");
        }
    }
}

#[test]
fn walking_has_the_same_path_at_every_render_rate() {
    let sample = |fps| {
        let mut scene = Scene::new(3, 900.0);
        for _ in 0..60 * fps {
            scene.frame_at(false, 0.0, fps);
        }
        (scene.body.centroid(), scene.hops)
    };
    let (expected, hops) = sample(60);
    assert!(hops > 0);
    for fps in [6, 12, 30, 120] {
        let (actual, actual_hops) = sample(fps);
        assert!((actual.x - expected.x).abs() < 0.01 && (actual.y - expected.y).abs() < 0.01);
        assert_eq!(actual_hops, hops);
    }
}

#[test]
fn wandering_never_hurts_the_slime_and_never_creeps() {
    let (mut floor_hops, mut tiny) = (0u32, 0u32);
    for seed in [3, 4, 5] {
        let mut scene = Scene::new(seed, 900.0);
        let mut launched: Option<f32> = None;
        while scene.now < 300.0 {
            let out = scene.tick(false, 0.0);
            let impact = scene.body.last_impact();
            if !out.crouch && impact > 420.0 {
                eprintln!(
                    "DBG surface {:?} y={:.0} x={:.0} gap={:.1} hops={}",
                    scene.body.surface(),
                    scene.body.centroid().y,
                    scene.body.centroid().x,
                    scene.body.surface_gap(),
                    scene.hops
                );
            }
            assert!(
                out.crouch || impact <= 420.0,
                "seed {seed}: hit a surface at {impact} px/s on its own at {:.1} s",
                scene.now
            );
            if let Some((dx, dy, _)) = out.hop {
                launched = (scene.place() == Place::Floor && dy < 0.0 && dx.abs() > 0.5)
                    .then(|| scene.body.centroid().x);
            } else if let Some(x0) = launched {
                if scene.body.surface_gap() < 3.0
                    && scene.body.speed() < 60.0
                    && scene.body.surface() == Surface::Floor
                {
                    floor_hops += 1;
                    tiny += u32::from((scene.body.centroid().x - x0).abs() < 12.0);
                    launched = None;
                }
            }
        }
    }
    assert!(floor_hops > 100, "only {floor_hops} floor hops measured");
    assert!(
        tiny * 20 <= floor_hops,
        "{tiny} of {floor_hops} floor hops moved less than 12 px"
    );
}

#[test]
fn hiding_in_either_corner_never_climbs_the_wall_or_jitters() {
    for seed in 1..=12u64 {
        for x in [250.0, 900.0, 1550.0] {
            let mut scene = Scene::new(seed, x);
            scene.run(90.0, true, 0.0);
            let hops = scene.hops;
            let start = scene.body.centroid();
            for _ in 0..30 * 60 {
                scene.frame(true, 0.0);
                assert_eq!(
                    scene.body.surface(),
                    Surface::Floor,
                    "seed {seed} from {x}: on a wall while hiding"
                );
            }
            let (min, max) = scene.body.bounding_box();
            assert!(
                min.x > WORLD.left + 4.0 && max.x < WORLD.right - 4.0,
                "seed {seed} from {x}: squeezed against the wall ({:.0}..{:.0})",
                min.x,
                max.x
            );
            assert!(
                start.x < 220.0 || start.x > WORLD.right - 220.0,
                "seed {seed} from {x}: not in a corner at {}",
                start.x
            );
            assert!(
                scene.hops - hops <= 1,
                "seed {seed} from {x}: {} extra hops in the corner",
                scene.hops - hops
            );
        }
    }
}

#[test]
fn a_walk_reaches_its_target_instead_of_pacing_around_it() {
    for seed in 1..=10u64 {
        let mut scene = Scene::new(seed, 900.0);
        let (mut run_start, mut longest) = (None::<f64>, 0.0f64);
        for _ in 0..(900.0 * 60.0) as usize {
            let out = scene.frame(false, 0.0);
            match (out.intent == slimy_core::Intent::Wander, run_start) {
                (true, None) => run_start = Some(scene.now),
                (false, Some(t)) => {
                    longest = longest.max(scene.now - t);
                    run_start = None;
                }
                _ => {}
            }
        }
        assert!(
            longest < 45.0,
            "seed {seed}: one walk lasted {longest:.0} s"
        );
    }
}

#[test]
fn a_slime_with_limbs_hides_far_enough_from_the_edge_and_never_climbs() {
    for seed in 1..=8u64 {
        for x in [250.0, 1550.0] {
            let mut scene = Scene::new(seed, x);
            scene.ai.set_inset(48.0);
            let mut walls = false;
            for _ in 0..(120.0 * 60.0) as usize {
                scene.frame(true, 0.0);
                walls |= scene.body.surface() != Surface::Floor;
            }
            let (min, max) = scene.body.bounding_box();
            let room = (min.x - WORLD.left).min(WORLD.right - max.x);
            assert!(
                room >= 48.0 - 6.0,
                "seed {seed} from {x}: only {room:.0} px from the edge"
            );
            assert!(!walls, "seed {seed} from {x}: climbed a wall");
        }
    }
}

/// How much of the vertical line through `x` lies inside the body (px).
fn thickness_at(body: &SoftBody, x: f32) -> f32 {
    (0..700)
        .filter(|y| body.contains(Vec2::new(x, WORLD.bottom - *y as f32)))
        .count() as f32
}

/// Runs one whole catch on a body of the given radius and silhouette, with the cursor lying `dx` px aside (in units of the
/// radius divided by 64) 20 px above the floor, scaled likewise. Returns what went wrong, if anything.
fn run_catch(
    seed: u64,
    cursor_dx: f32,
    radius: f32,
    profile: Option<&[f32]>,
) -> Result<(), String> {
    let scale = radius / 64.0;
    let cursor_dx = cursor_dx * scale;
    {
        let mut scene = Scene::sized(seed, 900.0, radius);
        if let Some(p) = profile {
            scene.body.set_profile(p);
            scene.reach = radius * p.iter().copied().fold(1.0f32, f32::max);
        }
        scene.body.set_shape(slimy_body::BodyShape {
            top: 0.9,
            bottom: 0.5,
            base_width: 1.04,
        });
        scene.body.set_breathing(slimy_body::Breathing {
            speed: 1.7,
            width: 0.016,
            height: 0.026,
        });
        for _ in 0..900 {
            scene.body.step(STEP, &Pointer::AWAY);
        }
        let rest_area = scene.body.area();
        let cursor = Vec2::new(900.0 + cursor_dx, WORLD.bottom - 20.0 * scale);
        scene.affection = 0.8;
        scene.cursor = Some(cursor);
        let (mut began, mut held, mut covered, mut ended_at) = (false, 0u32, 0u32, None::<f64>);
        let (mut thinnest, mut area_lo, mut area_hi, mut impact) =
            (f32::MAX, f32::MAX, 0.0f32, 0.0f32);
        let mut hold_frames = 0u32;
        for _ in 0..(40.0 * 60.0) as usize {
            let out = scene.frame(false, 0.0);
            let phase = out.catching;
            began |= phase != slimy_core::CatchPhase::None;
            if began && ended_at.is_none() {
                impact = impact.max(scene.body.last_impact());
                if scene.body.surface() != Surface::Floor {
                    return Err(format!("seed {seed}: left the floor"));
                }
            }
            if phase == slimy_core::CatchPhase::Hold {
                hold_frames += 1;
                if hold_frames > 20 {
                    held += 1;
                    covered += u32::from(scene.body.contains(cursor));
                    thinnest = thinnest.min(thickness_at(&scene.body, cursor.x));
                    area_lo = area_lo.min(scene.body.area() / rest_area);
                    area_hi = area_hi.max(scene.body.area() / rest_area);
                }
            }
            if began && phase == slimy_core::CatchPhase::None && ended_at.is_none() {
                ended_at = Some(scene.now);
            }
            if let Some(end) = ended_at {
                if scene.now > end + 6.0 {
                    break;
                }
            }
        }
        if !(began && held > 20) {
            return Err(format!(
                "seed {seed} dx {cursor_dx}: began {began}, held {held} frames"
            ));
        }
        if covered != held {
            return Err(format!("seed {seed} dx {cursor_dx}: the cursor was inside the body in {covered} of {held} held frames"));
        }
        if thinnest < 45.0 * scale {
            return Err(format!(
                "seed {seed} dx {cursor_dx}: only {thinnest:.0} px of gel over the cursor, a blade"
            ));
        }
        if !(area_lo > 0.85 && area_hi < 1.2) {
            return Err(format!("seed {seed}: area {area_lo:.2} to {area_hi:.2}"));
        }
        if impact > 150.0 {
            return Err(format!("seed {seed}: hit the floor at {impact:.0} px/s"));
        }
        if scene.body.contains(cursor) {
            return Err(format!(
                "seed {seed} dx {cursor_dx}: still on top of the cursor after letting go"
            ));
        }
        if (scene.body.area() / rest_area - 1.0).abs() >= 0.08 {
            return Err(format!("seed {seed}: not back to its volume"));
        }
    }
    Ok(())
}

#[test]
fn the_whole_body_covers_a_cursor_lying_on_the_floor_holds_it_and_lets_it_go() {
    for (seed, cursor_dx) in [(7u64, 70.0f32), (11, -70.0), (13, 40.0), (17, -25.0)] {
        run_catch(seed, cursor_dx, 64.0, None).unwrap_or_else(|e| panic!("{e}"));
    }
}

/// A radial profile like the page's forms: a base of 1 with gaussian lobes `(angle, width, height)`.
fn lobes(list: &[(f32, f32, f32)]) -> Vec<f32> {
    (0..slimy_body::POINTS)
        .map(|i| {
            let a = i as f32 / slimy_body::POINTS as f32 * std::f32::consts::TAU;
            let wrap = |d: f32| {
                (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
            };
            1.0 + list
                .iter()
                .map(|(at, w, h)| h * (-(wrap(a - at) / w).powi(2)).exp())
                .sum::<f32>()
        })
        .collect()
}

#[test]
fn the_catch_also_works_for_other_sizes_and_for_wide_forms() {
    let crab = lobes(&[
        (0.12, 0.3, 0.5),
        (std::f32::consts::PI - 0.12, 0.3, 0.5),
        (0.42, 0.14, 0.34),
        (0.95, 0.14, 0.34),
    ]);
    let bat = lobes(&[(0.2, 0.5, 0.45), (std::f32::consts::PI - 0.2, 0.5, 0.45)]);
    for radius in [52.0f32, 80.0] {
        for (name, profile) in [("plain", None), ("crab", Some(&crab)), ("bat", Some(&bat))] {
            for (seed, dx) in [(7u64, 70.0f32), (11, -70.0), (13, 40.0)] {
                run_catch(seed, dx, radius, profile.map(Vec::as_slice))
                    .unwrap_or_else(|e| panic!("radius {radius} {name} [{seed} {dx}]: {e}"));
            }
        }
    }
}

#[test]
#[ignore = "survey: prints which sizes and silhouettes the catch handles"]
fn survey_of_sizes_and_silhouettes() {
    let crab = lobes(&[
        (0.12, 0.3, 0.5),
        (std::f32::consts::PI - 0.12, 0.3, 0.5),
        (0.42, 0.14, 0.34),
        (0.95, 0.14, 0.34),
    ]);
    let bat = lobes(&[(0.2, 0.5, 0.45), (std::f32::consts::PI - 0.2, 0.5, 0.45)]);
    let tall = lobes(&[(-std::f32::consts::FRAC_PI_2, 0.5, 0.4)]);
    let forms: [(&str, Option<Vec<f32>>); 4] = [
        ("plain", None),
        ("crab", Some(crab)),
        ("bat", Some(bat)),
        ("tall", Some(tall)),
    ];
    for radius in [40.0f32, 52.0, 64.0, 80.0, 96.0] {
        for (name, profile) in &forms {
            let fails: Vec<String> = [(7u64, 70.0f32), (11, -70.0), (13, 40.0), (17, -25.0)]
                .iter()
                .filter_map(|(seed, dx)| {
                    run_catch(*seed, *dx, radius, profile.as_deref())
                        .err()
                        .map(|e| format!("[{seed} {dx}] {e}"))
                })
                .collect();
            println!(
                "radius {radius:>3} {name:<5}: {}",
                if fails.is_empty() {
                    "ok".to_string()
                } else {
                    fails.join(" | ")
                }
            );
        }
    }
}
