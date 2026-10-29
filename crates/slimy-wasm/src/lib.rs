//! Browser binding for `slimy-body`: flat arrays in, flat arrays out.

use std::collections::HashSet;

use slimy_body::{BodyParams, BodyShape, Bounds, Breathing, Contact, Pointer, SoftBody, Vec2};
use slimy_core::{
    Autonomy as Behavior, Context, Event, Initiative, Place, Relation, Snapshot, Sulk, World,
    QUESTIONS,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Slime {
    body: SoftBody,
}

fn contact(code: u8) -> Contact {
    match code {
        1 => Contact::Hover,
        2 => Contact::Down,
        _ => Contact::Away,
    }
}

#[wasm_bindgen]
impl Slime {
    #[wasm_bindgen(constructor)]
    pub fn new(radius: f32, left: f32, right: f32, floor: f32, top: f32, x: f32, y: f32) -> Slime {
        let params = BodyParams {
            radius,
            bounds: Bounds {
                left,
                right,
                floor,
                top,
            },
        };
        Slime {
            body: SoftBody::new(params, Vec2::new(x, y)),
        }
    }

    /// The play area changed (another screen, a new resolution): the body is kept inside it.
    pub fn resize_world(&mut self, left: f32, right: f32, floor: f32, top: f32) {
        self.body.resize_world(Bounds {
            left,
            right,
            floor,
            top,
        });
    }

    pub fn step(&mut self, dt: f32, x: f32, y: f32, contact_code: u8) {
        self.body.step(
            dt,
            &Pointer {
                pos: Vec2::new(x, y),
                contact: contact(contact_code),
            },
        );
    }

    pub fn set_shape(&mut self, top: f32, bottom: f32, base_width: f32) {
        self.body.set_shape(BodyShape {
            top,
            bottom,
            base_width,
        });
    }

    pub fn set_breathing(&mut self, speed: f32, width: f32, height: f32) {
        self.body.set_breathing(Breathing {
            speed,
            width,
            height,
        });
    }

    pub fn set_bounds(&mut self, left: f32, right: f32, floor: f32, top: f32) {
        self.body.set_bounds(Bounds {
            left,
            right,
            floor,
            top,
        });
    }

    /// A leap of the slime's own making: unit direction and speed.
    pub fn hop(&mut self, dx: f32, dy: f32, power: f32) {
        self.body.hop(Vec2::new(dx, dy), power);
    }

    /// The squash before a leap.
    pub fn crouch(&mut self, depth: f32) {
        self.body.crouch(depth);
    }

    pub fn impulse(&mut self, x: f32, y: f32) {
        self.body.impulse(Vec2::new(x, y));
    }

    /// Interleaved x, y of every outline point.
    pub fn points(&self) -> Vec<f32> {
        self.body.points().iter().flat_map(|p| [p.x, p.y]).collect()
    }

    pub fn centroid_x(&self) -> f32 {
        self.body.centroid().x
    }

    pub fn centroid_y(&self) -> f32 {
        self.body.centroid().y
    }

    pub fn rest_height(&self) -> f32 {
        self.body.rest_height()
    }

    /// Unit vector pointing away from the surface the slime clings to (x, y).
    pub fn gravity_up(&self) -> Vec<f32> {
        let up = self.body.up();
        vec![up.x, up.y]
    }

    /// Rotation in radians of the body frame: 0 on the floor, quarter turns on walls, half turn on the ceiling.
    pub fn orientation(&self) -> f32 {
        self.body.orientation()
    }

    /// Fastest outline point, in pixels per second.
    pub fn speed(&self) -> f32 {
        self.body.speed()
    }

    /// Pixels between the body and the surface it clings to.
    pub fn surface_gap(&self) -> f32 {
        self.body.surface_gap()
    }

    pub fn last_impact(&self) -> f32 {
        self.body.last_impact()
    }

    /// Holds a pseudopod out toward (dx, dy); `amount` 0 to 1.
    pub fn reach_toward(&mut self, dx: f32, dy: f32, amount: f32) {
        self.body.reach_toward(Vec2::new(dx, dy), amount);
    }

    /// Metamorphosis target: one radius multiplier per outline point (56); other lengths are ignored.
    pub fn set_profile(&mut self, profile: &[f32]) {
        self.body.set_profile(profile);
    }

    /// Length (radii) and angular half width of the lobe: a long, broad one is the body flowing to its target.
    pub fn set_arm_style(&mut self, length: f32, width: f32) {
        self.body.set_arm_style(length, width);
    }

    /// Squashes the whole body flat (0 to 1) to spread over something, keeping its volume.
    pub fn set_flatten(&mut self, amount: f32) {
        self.body.set_flatten(amount);
    }

    /// A hop that does not turn the body (the small approach hops of a catch).
    pub fn leap_level(&mut self, dx: f32, dy: f32, power: f32) {
        self.body.leap_level(Vec2::new(dx, dy), power);
    }

    pub fn retract(&mut self) {
        self.body.retract();
    }

    /// Unit direction x, y of the pseudopod.
    pub fn arm_direction(&self) -> Vec<f32> {
        let d = self.body.arm_direction();
        vec![d.x, d.y]
    }

    pub fn arm_extent(&self) -> f32 {
        self.body.arm_extent()
    }

    pub fn is_grabbed(&self) -> bool {
        self.body.is_grabbed()
    }
}

/// The slime's feelings and its initiative, driven by the page with the page's clock (seconds).
#[wasm_bindgen]
pub struct Companion {
    relation: Relation,
    sulk: Sulk,
    initiative: Initiative,
    known: HashSet<String>,
}

#[wasm_bindgen]
impl Companion {
    #[wasm_bindgen(constructor)]
    pub fn new(affection: f32, trust: f32, resentment: f32) -> Companion {
        Companion {
            relation: Relation {
                affection,
                trust,
                resentment,
            }
            .clamped(),
            sulk: Sulk::default(),
            initiative: Initiative::default(),
            known: HashSet::new(),
        }
    }

    /// Registers a fact the slime already knows, so it never asks for it.
    pub fn know(&mut self, key: &str) {
        self.known.insert(key.to_owned());
    }

    pub fn caress(&mut self, seconds: f32) {
        self.relation.apply(Event::Caress { seconds });
        self.sulk.caressed();
    }

    pub fn tick(&mut self, dt: f64) {
        self.relation.drift(dt);
    }

    /// The id of a question to ask now, or an empty string.
    pub fn ask(&mut self, now: f64, idle_for: f64, returned: bool, caressed: bool) -> String {
        let context = Context {
            now,
            sulking: self.sulk.active(now),
            returned,
            caressed,
            idle_for,
            known: &self.known,
        };
        self.initiative
            .decide(&context)
            .unwrap_or_default()
            .to_owned()
    }

    pub fn answer(&mut self, id: &str, now: f64) {
        self.initiative.answered(id, now);
        self.relation.apply(Event::AnsweredQuestion);
    }

    pub fn decline(&mut self, id: &str, now: f64) {
        self.initiative.declined(id, now);
    }

    /// Counts a question left unanswered for too long as ignored. Returns its id once, else an empty string.
    pub fn check_ignored(&mut self, now: f64) -> String {
        let Some(id) = self.initiative.expired(now) else {
            return String::new();
        };
        self.relation.apply(Event::IgnoredQuestion);
        self.sulk.ignored(now);
        id.to_owned()
    }

    pub fn sulking(&self, now: f64) -> bool {
        self.sulk.active(now)
    }

    pub fn affection(&self) -> f32 {
        self.relation.affection
    }

    pub fn trust(&self) -> f32 {
        self.relation.trust
    }

    pub fn resentment(&self) -> f32 {
        self.relation.resentment
    }
}

/// The fact an answer to this question is stored under.
#[wasm_bindgen]
pub fn question_key(id: &str) -> String {
    QUESTIONS
        .iter()
        .find(|q| q.id == id)
        .map(|q| q.key.to_owned())
        .unwrap_or_default()
}

/// The slime's own initiative on the move: reads a snapshot, answers with what to do this frame.
#[wasm_bindgen]
pub struct Roamer {
    behavior: Behavior,
    world: World,
}

#[wasm_bindgen]
impl Roamer {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, left: f32, right: f32, top: f32, bottom: f32) -> Roamer {
        Roamer {
            behavior: Behavior::new(u64::from(seed)),
            world: World {
                left,
                right,
                top,
                bottom,
            },
        }
    }

    /// The play area changed: the autonomy follows it.
    pub fn set_world(&mut self, left: f32, right: f32, top: f32, bottom: f32) {
        self.world = World {
            left,
            right,
            top,
            bottom,
        };
    }

    /// Keeps the body this many pixels away from the screen edges (limbs that stick out of the body).
    pub fn set_inset(&mut self, px: f32) {
        self.behavior.set_inset(px);
    }

    /// `input`: x, y, place (0 floor, 1 left, 2 right, 3 ceiling), settled, grabbed, busy, sulking,
    /// sleepiness, affection, pointer x, pointer y, pointer moving, radius, optional surface gap, optional pointer known, optional reach (half width of the resting body). Returns hop dx, dy, power,
    /// crouch, gaze x, gaze y, has gaze, intent (0 idle, 1 wander, 2 hide, 3 peek, 4 nap, 5 follow, 6 catch), then
    /// flatten (1 while squashed over the cursor), gentle (1 for the level approach hops of a catch), catch phase
    /// (0 none, 1 acquire, 2 prepare, 3 flight, 4 contact, 5 spread, 6 hold, 7 retreat).
    pub fn step(&mut self, input: &[f32], now: f64) -> Vec<f32> {
        let at = |i: usize| input.get(i).copied().unwrap_or(0.0);
        let place = match at(2) as u8 {
            1 => Place::Left,
            2 => Place::Right,
            3 => Place::Ceiling,
            _ => Place::Floor,
        };
        let usable = at(11) > 0.5;
        // A cursor that is known but resting is still there to be caught; only a moving one is followed or avoided.
        let known = input.get(14).map_or(usable, |k| *k > 0.5);
        let snapshot = Snapshot {
            now,
            x: at(0),
            y: at(1),
            place,
            settled: at(3) > 0.5,
            supported: input.get(13).map_or(at(3) > 0.5, |gap| *gap < 3.0),
            grabbed: at(4) > 0.5,
            busy: at(5) > 0.5,
            sulking: at(6) > 0.5,
            sleepiness: at(7),
            affection: at(8),
            pointer: known.then(|| (at(9), at(10))),
            pointer_active: usable,
            radius: at(12).max(8.0),
            reach: input.get(15).copied().unwrap_or(0.0).max(at(12).max(8.0)),
            world: self.world,
        };
        let out = self.behavior.step(&snapshot);
        let (hx, hy, hp) = out.hop.unwrap_or((0.0, 0.0, 0.0));
        let (gx, gy) = out.gaze.unwrap_or((0.0, 0.0));
        vec![
            hx,
            hy,
            hp,
            f32::from(u8::from(out.crouch)),
            gx,
            gy,
            f32::from(u8::from(out.gaze.is_some())),
            out.intent as u8 as f32,
            out.flatten,
            f32::from(u8::from(out.gentle)),
            out.catching as u8 as f32,
        ]
    }
}
