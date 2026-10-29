//! Soft body of a slime: a ring of points that keeps its volume, rests on a
//! floor and reacts to a pointer. No I/O and no clock: time arrives as `dt`.

use std::f32::consts::TAU;
use std::ops::{Add, AddAssign, Mul, Sub};

pub const POINTS: usize = 56;
const SUBSTEPS: usize = 5;

const SHAPE_STIFFNESS: f32 = 130.0;
const SMOOTHING: f32 = 260.0;
const PRESSURE: f32 = 2600.0;
const GRAVITY: f32 = 950.0;
const DAMPING: f32 = 5.5;
const DRIFT_DRAG: f32 = 0.35;
const GRAB_STIFFNESS: f32 = 900.0;
const GRAB_RADIUS: f32 = 0.75;
const TOUCH_RADIUS: f32 = 0.34;
const PRESS_RADIUS: f32 = 0.55;
const TOUCH_FORCE: f32 = 3400.0;
const PRESS_FORCE: f32 = 9000.0;
const FLOOR_FRICTION: f32 = 0.55;
const FLOOR_BOUNCE: f32 = 0.04;
const HARD_BOUNCE: f32 = 0.3;
const STICK_SPEED: f32 = 420.0;
const DETACH_AFTER: f32 = 0.45;
const LANDING_GRIP: f32 = 0.85;
const SURFACE_ADHESION: f32 = 7.0;
const MAX_TURN: f32 = 0.1;
const IMPACT_LOCKOUT: f32 = 0.3;
/// Pseudopod: length in radii, angular half width (radians), spring stiffness and damping of its extension.
/// How fast a metamorphosis settles: the profile closes this share of the gap per second.
const MORPH_RATE: f32 = 2.2;
/// Squashing over something: the whole outline flattens to this axis ratio (wider and lower, area kept).
const FLATTEN_Q: f32 = 0.8;
const ARM_LENGTH: f32 = 1.3;
const ARM_WIDTH: f32 = 0.5;
/// How much the rest of the body thins as the arm extends, so the gel keeps its volume.
/// How fast the lobe's length and width follow their targets, per second.
const ARM_SHAPE_RATE: f32 = 7.0;
// Measured on the real body (stiffness 90 / damping 9 overshot the tip by 43 to 62%; 60 / 16 by 0.2 to 6%).
const ARM_STIFFNESS: f32 = 60.0;
const ARM_DAMPING: f32 = 16.0;
const HOP_LEAN: f32 = 3.0;
const RIGHTING_ON_SURFACE: f32 = 0.35;
const RIGHTING_IN_AIR: f32 = 0.06;
const RIGHTING_WHEN_HELD: f32 = 0.04;
const GRAB_REACH: f32 = 1.1;
const MAX_REACH: f32 = 2.1;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        *self = *self + o;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub left: f32,
    pub right: f32,
    pub floor: f32,
    pub top: f32,
}

/// The edge of the world the slime currently clings to; gravity pulls toward it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Floor,
    Left,
    Right,
    Ceiling,
}

impl Surface {
    const ALL: [Surface; 4] = [
        Surface::Floor,
        Surface::Left,
        Surface::Right,
        Surface::Ceiling,
    ];

    fn index(self) -> usize {
        self as usize
    }

    fn gravity_dir(self) -> Vec2 {
        match self {
            Surface::Floor => Vec2::new(0.0, 1.0),
            Surface::Left => Vec2::new(-1.0, 0.0),
            Surface::Right => Vec2::new(1.0, 0.0),
            Surface::Ceiling => Vec2::new(0.0, -1.0),
        }
    }
}

/// Silhouette of the resting body, as fractions of the radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyShape {
    pub top: f32,
    pub bottom: f32,
    pub base_width: f32,
}

impl Default for BodyShape {
    fn default() -> Self {
        Self {
            top: 0.9,
            bottom: 0.5,
            base_width: 1.04,
        }
    }
}

/// Slow swelling of the resting body: angular speed in rad/s and relative depth on each axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breathing {
    pub speed: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for Breathing {
    fn default() -> Self {
        Self {
            speed: 2.1,
            width: 0.018,
            height: 0.028,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BodyParams {
    pub radius: f32,
    pub bounds: Bounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Away,
    Hover,
    Down,
}

#[derive(Clone, Copy, Debug)]
pub struct Pointer {
    pub pos: Vec2,
    pub contact: Contact,
}

impl Pointer {
    pub const AWAY: Pointer = Pointer {
        pos: Vec2::new(-1.0e6, -1.0e6),
        contact: Contact::Away,
    };
}

struct Grip {
    index: usize,
    weight: f32,
    offset: Vec2,
}

pub struct SoftBody {
    params: BodyParams,
    rest: Vec<Vec2>,
    pts: Vec<Vec2>,
    vel: Vec<Vec2>,
    rest_area: f32,
    grips: Vec<Grip>,
    was_down: bool,
    time: f32,
    shape: BodyShape,
    breathing: Breathing,
    surface: Surface,
    orient: f32,
    adrift: f32,
    touch_speed: [f32; 4],
    lockout: [f32; 4],
    impact: f32,
    arm: Arm,
    profile: Vec<f32>,
    profile_target: Vec<f32>,
    form_target: Vec<f32>,
    flatten: f32,
}

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI
}

fn polygon_area(pts: &[Vec2]) -> f32 {
    let n = pts.len();
    let twice: f32 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    twice.abs() / 2.0
}

fn rest_shape(radius: f32, shape: &BodyShape) -> Vec<Vec2> {
    rest_shape_with(radius, shape, &[])
}

/// The resting outline scaled point by point by `profile` (empty means unchanged), centred on its centroid.
fn rest_shape_with(radius: f32, shape: &BodyShape, profile: &[f32]) -> Vec<Vec2> {
    let mut outline: Vec<Vec2> = (0..POINTS)
        .map(|i| {
            let a = i as f32 / POINTS as f32 * TAU;
            let (s, c) = a.sin_cos();
            let y = radius * s * if s < 0.0 { shape.top } else { shape.bottom };
            let x = radius * c * if s < 0.0 { 1.0 } else { shape.base_width };
            let k = profile.get(i).copied().unwrap_or(1.0);
            Vec2::new(x * k, y * k)
        })
        .collect();
    let mean_x = outline.iter().map(|p| p.x).sum::<f32>() / POINTS as f32;
    let mean_y = outline.iter().map(|p| p.y).sum::<f32>() / POINTS as f32;
    for p in &mut outline {
        p.x -= mean_x;
        p.y -= mean_y;
    }
    outline
}

/// A pseudopod: the body reaches a lobe of itself toward a direction. `extent` is sprung, so it overshoots a little.
#[derive(Debug, Clone, Copy)]
struct Arm {
    angle: f32,
    target: f32,
    extent: f32,
    speed: f32,
    length: f32,
    length_target: f32,
    width: f32,
    width_target: f32,
}

impl Default for Arm {
    fn default() -> Self {
        Self {
            angle: 0.0,
            target: 0.0,
            extent: 0.0,
            speed: 0.0,
            length: ARM_LENGTH,
            length_target: ARM_LENGTH,
            width: ARM_WIDTH,
            width_target: ARM_WIDTH,
        }
    }
}

impl SoftBody {
    pub fn new(params: BodyParams, center: Vec2) -> Self {
        let shape = BodyShape::default();
        let rest = rest_shape(params.radius, &shape);
        let pts = rest.iter().map(|r| center + *r).collect();
        Self {
            rest_area: polygon_area(&rest),
            vel: vec![Vec2::ZERO; POINTS],
            rest,
            pts,
            params,
            grips: Vec::new(),
            was_down: false,
            time: 0.0,
            shape,
            breathing: Breathing::default(),
            surface: Surface::Floor,
            orient: 0.0,
            adrift: 0.0,
            touch_speed: [-1.0; 4],
            lockout: [0.0; 4],
            impact: 0.0,
            arm: Arm::default(),
            profile: vec![1.0; POINTS],
            profile_target: vec![1.0; POINTS],
            form_target: vec![1.0; POINTS],
            flatten: 0.0,
        }
    }

    pub fn surface(&self) -> Surface {
        self.surface
    }

    /// Unit vector from the surface toward the slime's top, in world coordinates.
    pub fn up(&self) -> Vec2 {
        self.surface.gravity_dir() * -1.0
    }

    /// Current rotation in radians of the body frame (0 when standing on the floor).
    pub fn orientation(&self) -> f32 {
        self.orient
    }

    /// Distance in pixels between the body and the surface it clings to (0 when touching).
    pub fn surface_gap(&self) -> f32 {
        let (min, max) = self.bounding_box();
        let b = self.params.bounds;
        let gap = match self.surface {
            Surface::Floor => b.floor - max.y,
            Surface::Ceiling => min.y - b.top,
            Surface::Left => min.x - b.left,
            Surface::Right => b.right - max.x,
        };
        gap.max(0.0)
    }

    pub fn set_shape(&mut self, shape: BodyShape) {
        self.shape = shape;
        self.rest = rest_shape_with(self.params.radius, &shape, &self.profile);
        self.rest_area = polygon_area(&self.rest);
        self.refresh_target();
    }

    pub fn set_breathing(&mut self, breathing: Breathing) {
        self.breathing = breathing;
    }

    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.params.bounds = bounds;
    }

    /// Changes the world (a screen was plugged in, a resolution changed) and brings the body inside it if it ended up outside:
    /// the whole body is moved, not squashed, and its speed is kept.
    pub fn resize_world(&mut self, bounds: Bounds) {
        self.params.bounds = bounds;
        let r = self.params.radius;
        let c = self.centroid();
        let (lo_x, hi_x) = (bounds.left + r, (bounds.right - r).max(bounds.left + r));
        let (lo_y, hi_y) = (bounds.top + r, (bounds.floor - r).max(bounds.top + r));
        let (dx, dy) = (c.x.clamp(lo_x, hi_x) - c.x, c.y.clamp(lo_y, hi_y) - c.y);
        if dx != 0.0 || dy != 0.0 {
            for p in &mut self.pts {
                p.x += dx;
                p.y += dy;
            }
        }
    }

    pub fn rest_height(&self) -> f32 {
        let (min, max) = self.rest.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        max - min
    }

    pub fn points(&self) -> &[Vec2] {
        &self.pts
    }

    pub fn centroid(&self) -> Vec2 {
        let sum = self.pts.iter().fold(Vec2::ZERO, |a, p| a + *p);
        sum * (1.0 / POINTS as f32)
    }

    pub fn area(&self) -> f32 {
        polygon_area(&self.pts)
    }

    pub fn bounding_box(&self) -> (Vec2, Vec2) {
        let mut min = Vec2::new(f32::MAX, f32::MAX);
        let mut max = Vec2::new(f32::MIN, f32::MIN);
        for p in &self.pts {
            min = Vec2::new(min.x.min(p.x), min.y.min(p.y));
            max = Vec2::new(max.x.max(p.x), max.y.max(p.y));
        }
        (min, max)
    }

    pub fn speed(&self) -> f32 {
        self.vel.iter().map(|v| v.len()).fold(0.0, f32::max)
    }

    pub fn is_grabbed(&self) -> bool {
        !self.grips.is_empty()
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let mut inside = false;
        let mut j = POINTS - 1;
        for i in 0..POINTS {
            let (a, b) = (self.pts[i], self.pts[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// A leap of the slime's own making: `dir` is a unit direction in the world, `power` a speed in px/s.
    /// Leaving the surface releases the grip so the push is not absorbed.
    pub fn hop(&mut self, dir: Vec2, power: f32) {
        self.adrift = DETACH_AFTER * 0.5;
        self.impulse(dir * power);
        // Lean into the leap: a spin about the centroid, so no net momentum is added.
        let c = self.centroid();
        let omega = dir.x * HOP_LEAN;
        for (v, p) in self.vel.iter_mut().zip(&self.pts) {
            let q = *p - c;
            *v += Vec2::new(-q.y, q.x) * omega;
        }
    }

    /// Metamorphosis: the resting outline glides toward `profile`, one multiplier of the base radius per
    /// outline point (starting at angle 0, to the right, turning toward the floor). Wrong lengths are ignored.
    pub fn set_profile(&mut self, profile: &[f32]) {
        if profile.len() == POINTS {
            self.form_target = profile.iter().map(|k| k.clamp(0.3, 2.0)).collect();
            self.refresh_target();
        }
    }

    /// Squashes the whole body toward the floor and out to the sides (0 none, 1 fully), keeping its volume: how a
    /// slime spreads over something to cover it. It glides like any metamorphosis and composes with the form's profile.
    pub fn set_flatten(&mut self, amount: f32) {
        let amount = if amount.is_finite() {
            amount.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if (amount - self.flatten).abs() > 1.0e-4 {
            self.flatten = amount;
            self.refresh_target();
        }
    }

    pub fn flatten(&self) -> f32 {
        self.flatten
    }

    /// The form's profile times the flattening profile, normalised so that the outline keeps its area.
    fn refresh_target(&mut self) {
        if self.flatten <= 1.0e-4 {
            self.profile_target = self.form_target.clone();
            return;
        }
        let q = FLATTEN_Q.powf(self.flatten);
        let flat: Vec<f32> = (0..POINTS)
            .map(|i| {
                let a = i as f32 / POINTS as f32 * TAU;
                let (s, c) = a.sin_cos();
                1.0 / (q * q * c * c + s * s / (q * q)).sqrt()
            })
            .collect();
        let squashed: Vec<f32> = self
            .form_target
            .iter()
            .zip(&flat)
            .map(|(f, k)| f * k)
            .collect();
        let before = polygon_area(&rest_shape_with(
            self.params.radius,
            &self.shape,
            &self.form_target,
        ));
        let after = polygon_area(&rest_shape_with(self.params.radius, &self.shape, &squashed));
        let keep = (before / after.max(1.0)).sqrt();
        self.profile_target = self
            .form_target
            .iter()
            .zip(&flat)
            .map(|(f, k)| (f * k * keep).clamp(0.3, 2.0))
            .collect();
    }

    fn morph(&mut self, dt: f32) {
        let ease = 1.0 - (-MORPH_RATE * dt).exp();
        let mut moving = false;
        for (k, goal) in self.profile.iter_mut().zip(&self.profile_target) {
            let gap = goal - *k;
            if gap.abs() > 1.0e-4 {
                *k += gap * ease;
                moving = true;
            }
        }
        if moving {
            self.rest = rest_shape_with(self.params.radius, &self.shape, &self.profile);
            self.rest_area = polygon_area(&self.rest);
        }
    }

    /// Extends a pseudopod toward `dir` (world direction); `amount` is 0 to 1 of the full reach.
    pub fn reach_toward(&mut self, dir: Vec2, amount: f32) {
        if dir.len() > 1.0e-3 {
            self.arm.angle = dir.y.atan2(dir.x);
            self.arm.target = amount.clamp(0.0, 1.0);
        }
    }

    /// Shapes the lobe: `length` in radii beyond the body, `width` as an angular half width in radians. A long,
    /// broad lobe is the body itself flowing toward something (a catch); a short, narrow one is an arm.
    pub fn set_arm_style(&mut self, length: f32, width: f32) {
        self.arm.length_target = length.clamp(0.4, 3.0);
        self.arm.width_target = width.clamp(0.25, 1.1);
    }

    /// Draws the pseudopod back into the body.
    pub fn retract(&mut self) {
        self.arm.target = 0.0;
    }

    /// Unit direction the pseudopod points to (world).
    pub fn arm_direction(&self) -> Vec2 {
        Vec2::new(self.arm.angle.cos(), self.arm.angle.sin())
    }

    /// Current extension of the pseudopod, 0 (none) to about 1.
    pub fn arm_extent(&self) -> f32 {
        self.arm.extent
    }

    /// Tip of the pseudopod in world coordinates, when one is out.
    pub fn arm_tip(&self) -> Option<Vec2> {
        if self.arm.extent < 0.05 {
            return None;
        }
        let dir = Vec2::new(self.arm.angle.cos(), self.arm.angle.sin());
        let c = self.centroid();
        self.pts.iter().copied().max_by(|a, b| {
            ((*a - c).x * dir.x + (*a - c).y * dir.y)
                .total_cmp(&((*b - c).x * dir.x + (*b - c).y * dir.y))
        })
    }

    /// A leap that does not turn the body: the same push as `hop`, without the lean. Used for the small, flat
    /// approach hops that must land where they were aimed.
    pub fn leap_level(&mut self, dir: Vec2, power: f32) {
        self.adrift = DETACH_AFTER * 0.5;
        self.impulse(dir * power);
    }

    /// Fastest contact speed of the last step against any surface (px/s), 0 when airborne or at rest.
    pub fn last_impact(&self) -> f32 {
        self.impact
    }

    /// Presses the body against its surface, the squash before a leap.
    pub fn crouch(&mut self, depth: f32) {
        self.impulse(self.surface.gravity_dir() * depth);
    }

    pub fn impulse(&mut self, v: Vec2) {
        for vel in &mut self.vel {
            *vel += v;
        }
    }

    pub fn step(&mut self, dt: f32, pointer: &Pointer) {
        self.update_grips(pointer);
        let h = dt / SUBSTEPS as f32;
        self.touch_speed = [-1.0; 4];
        for _ in 0..SUBSTEPS {
            self.substep(h, pointer);
            self.time += h;
        }
        self.impact = self.touch_speed.iter().fold(0.0, |a, v| a.max(*v));
        self.morph(dt);
        let shape = 1.0 - (-ARM_SHAPE_RATE * dt).exp();
        self.arm.length += (self.arm.length_target - self.arm.length) * shape;
        self.arm.width += (self.arm.width_target - self.arm.width) * shape;
        let pull =
            (self.arm.target - self.arm.extent) * ARM_STIFFNESS - self.arm.speed * ARM_DAMPING;
        self.arm.speed += pull * dt;
        self.arm.extent = (self.arm.extent + self.arm.speed * dt).max(0.0);
        self.update_surface(dt);
        self.orient = self.estimate_rotation(self.centroid());
    }

    /// Rotation that best maps the resting outline onto the current one (least squares).
    fn estimate_rotation(&self, c: Vec2) -> f32 {
        let (mut dot, mut cross) = (0.0, 0.0);
        for (r, p) in self.rest.iter().zip(&self.pts) {
            let q = *p - c;
            dot += r.x * q.x + r.y * q.y;
            cross += r.x * q.y - r.y * q.x;
        }
        cross.atan2(dot)
    }

    fn update_surface(&mut self, dt: f32) {
        for s in Surface::ALL {
            let i = s.index();
            self.lockout[i] = if self.touch_speed[i] >= STICK_SPEED {
                IMPACT_LOCKOUT
            } else {
                (self.lockout[i] - dt).max(0.0)
            };
        }
        let touching = |s: Surface| self.touch_speed[s.index()] >= 0.0;
        if touching(self.surface) {
            self.adrift = 0.0;
        } else {
            self.adrift += dt;
        }
        let grabbed = !self.grips.is_empty();
        let mean = self.vel.iter().fold(Vec2::ZERO, |a, v| a + *v) * (1.0 / POINTS as f32);
        let inward = |s: Surface| mean.x * s.gravity_dir().x + mean.y * s.gravity_dir().y;
        let new = Surface::ALL.into_iter().find(|s| {
            *s != self.surface
                && (!touching(self.surface) || inward(self.surface) < -60.0)
                && touching(*s)
                && self.lockout[s.index()] <= 0.0
                && self.touch_speed[s.index()] < STICK_SPEED
                && inward(*s) > -STICK_SPEED * 0.5
        });
        match new {
            Some(s) if !grabbed => {
                self.surface = s;
                self.adrift = 0.0;
                let n = s.gravity_dir();
                let along = mean - n * (mean.x * n.x + mean.y * n.y);
                for v in &mut self.vel {
                    *v = *v - along * LANDING_GRIP;
                }
            }
            _ if self.adrift > DETACH_AFTER && self.surface != Surface::Floor => {
                self.surface = Surface::Floor
            }
            _ => {}
        }
    }

    fn update_grips(&mut self, pointer: &Pointer) {
        let down = pointer.contact == Contact::Down;
        if !down {
            self.grips.clear();
        } else if !self.was_down && self.contains(pointer.pos) {
            let reach = self.params.radius * GRAB_RADIUS;
            let est = self.estimate_rotation(self.centroid());
            self.grips = self
                .pts
                .iter()
                .enumerate()
                .filter_map(|(index, p)| {
                    let d = (*p - pointer.pos).len();
                    (d < reach).then(|| Grip {
                        index,
                        weight: 1.0 - d / reach,
                        offset: rotate(*p - pointer.pos, -est),
                    })
                })
                .collect();
        }
        self.was_down = down;
    }

    fn substep(&mut self, h: f32, pointer: &Pointer) {
        let c = self.centroid();
        let pressure = (self.rest_area / self.area().max(1.0) - 1.0) * PRESSURE;
        let breath = (self.time * self.breathing.speed).sin();
        let r = self.params.radius;
        let est = self.estimate_rotation(c);
        let upright = self.up();
        let righting = if !self.grips.is_empty() {
            RIGHTING_WHEN_HELD
        } else if self.adrift == 0.0 {
            RIGHTING_ON_SURFACE
        } else {
            RIGHTING_IN_AIR
        };
        let turn =
            (righting * wrap_angle(upright.x.atan2(-upright.y) - est)).clamp(-MAX_TURN, MAX_TURN);
        let goal_angle = est + turn;
        let (sin_o, cos_o) = goal_angle.sin_cos();
        let pull = self.surface.gravity_dir() * GRAVITY;
        let touching = pointer.contact != Contact::Away && self.grips.is_empty();
        let (reach, force) = if pointer.contact == Contact::Down {
            (r * PRESS_RADIUS, PRESS_FORCE)
        } else {
            (r * TOUCH_RADIUS, TOUCH_FORCE)
        };

        let mut internal = [Vec2::ZERO; POINTS];
        let mut external = [Vec2::ZERO; POINTS];
        for i in 0..POINTS {
            let p = self.pts[i];
            let (prev, next) = (
                self.pts[(i + POINTS - 1) % POINTS],
                self.pts[(i + 1) % POINTS],
            );
            let mut local = Vec2::new(
                self.rest[i].x * (1.0 - self.breathing.width * breath),
                self.rest[i].y * (1.0 + self.breathing.height * breath),
            );
            if self.arm.extent > 0.001 {
                // A bigger lobe takes more of the body's volume: the rest of the gel thins with the lobe's size
                // (length times width), 1 + x - sqrt(1 + x^2) with x = 0.3 L W, fitted on the real body.
                let size = 0.3 * self.arm.length * self.arm.width;
                let thinning = 1.0 + size - (1.0 + size * size).sqrt();
                local = local * (1.0 - thinning * self.arm.extent.min(1.0));
                let outward = Vec2::new(
                    self.rest[i].x * cos_o - self.rest[i].y * sin_o,
                    self.rest[i].x * sin_o + self.rest[i].y * cos_o,
                );
                let diff = wrap_angle(outward.y.atan2(outward.x) - self.arm.angle) / self.arm.width;
                let lobe = (-(diff * diff) * (diff * diff)).exp();
                if lobe > 0.002 {
                    // Every point of the lobe moves the same way, so the arm is a finger with a rounded tip, not a cone.
                    let push = r * self.arm.length * self.arm.extent * lobe;
                    let world = Vec2::new(self.arm.angle.cos(), self.arm.angle.sin());
                    local += Vec2::new(
                        world.x * cos_o + world.y * sin_o,
                        -world.x * sin_o + world.y * cos_o,
                    ) * push;
                }
            }
            let goal = Vec2::new(
                c.x + local.x * cos_o - local.y * sin_o,
                c.y + local.x * sin_o + local.y * cos_o,
            );
            let mut a = (goal - p) * SHAPE_STIFFNESS;
            a += ((prev + next) * 0.5 - p) * SMOOTHING;

            let t = next - prev;
            let len = t.len().max(1.0e-4);
            let mut n = Vec2::new(t.y / len, -t.x / len);
            if n.x * (p.x - c.x) + n.y * (p.y - c.y) < 0.0 {
                n = n * -1.0;
            }
            a += n * pressure;
            internal[i] = a;

            external[i] = pull;
            if touching {
                let away = p - pointer.pos;
                let d = away.len();
                if d < reach && d > 0.01 {
                    external[i] += away * ((1.0 - d / reach) * force / d);
                }
            }
        }
        // Internal forces cannot push the body as a whole (Newton's third law): only gravity, contacts and the pointer do.
        let net = internal.iter().fold(Vec2::ZERO, |a, f| a + *f) * (1.0 / POINTS as f32);
        for i in 0..POINTS {
            self.vel[i] += (internal[i] - net + external[i]) * h;
        }

        for g in &self.grips {
            let mut to = pointer.pos + rotate(g.offset, est) - self.pts[g.index];
            let cap = self.params.radius * GRAB_REACH;
            if to.len() > cap {
                to = to * (cap / to.len());
            }
            self.vel[g.index] += to * (GRAB_STIFFNESS * g.weight * h);
        }

        let keep = (-DAMPING * h).exp();
        let drift_keep = (-DRIFT_DRAG * h).exp();
        let mut mean = self.vel.iter().fold(Vec2::ZERO, |a, v| a + *v) * (1.0 / POINTS as f32);
        if self.adrift == 0.0 && self.grips.is_empty() {
            let n = self.surface.gravity_dir();
            let along = mean - n * (mean.x * n.x + mean.y * n.y);
            let hold = along * (1.0 - (-SURFACE_ADHESION * h).exp());
            for v in &mut self.vel {
                *v = *v - hold;
            }
            mean = mean - hold;
        }
        let b = self.params.bounds;
        let reach = self.params.radius * MAX_REACH;
        for i in 0..POINTS {
            self.vel[i] = mean * drift_keep + (self.vel[i] - mean) * keep;
            self.pts[i] += self.vel[i] * h;
            let away = self.pts[i] - c;
            if away.len() > reach {
                let dir = away * (1.0 / away.len());
                self.pts[i] = c + dir * reach;
                let outward = self.vel[i].x * dir.x + self.vel[i].y * dir.y;
                if outward > 0.0 {
                    self.vel[i] = self.vel[i] - dir * outward;
                }
            }
            let (p, v) = (&mut self.pts[i], &mut self.vel[i]);
            let bounce = |speed: f32| {
                if speed > STICK_SPEED {
                    HARD_BOUNCE
                } else {
                    FLOOR_BOUNCE
                }
            };
            if p.y > b.floor {
                let speed = v.y.max(0.0);
                p.y = b.floor;
                v.y = if v.y > 0.0 { -v.y * bounce(speed) } else { v.y };
                v.x *= FLOOR_FRICTION;
                self.touch_speed[Surface::Floor.index()] =
                    self.touch_speed[Surface::Floor.index()].max(speed);
            }
            if p.y < b.top {
                let speed = (-v.y).max(0.0);
                p.y = b.top;
                v.y = if v.y < 0.0 { -v.y * bounce(speed) } else { v.y };
                v.x *= FLOOR_FRICTION;
                self.touch_speed[Surface::Ceiling.index()] =
                    self.touch_speed[Surface::Ceiling.index()].max(speed);
            }
            if p.x < b.left {
                let speed = (-v.x).max(0.0);
                p.x = b.left;
                v.x = if v.x < 0.0 { -v.x * bounce(speed) } else { v.x };
                v.y *= FLOOR_FRICTION;
                self.touch_speed[Surface::Left.index()] =
                    self.touch_speed[Surface::Left.index()].max(speed);
            }
            if p.x > b.right {
                let speed = v.x.max(0.0);
                p.x = b.right;
                v.x = if v.x > 0.0 { -v.x * bounce(speed) } else { v.x };
                v.y *= FLOOR_FRICTION;
                self.touch_speed[Surface::Right.index()] =
                    self.touch_speed[Surface::Right.index()].max(speed);
            }
        }
    }
}
