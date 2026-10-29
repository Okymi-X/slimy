//! What the slime does on its own: wander, climb a wall, follow a fond glance of the cursor, hide in a
//! corner when it sulks, curl up when it is sleepy. Pure decisions; the page turns them into hops.
//! The pace is deliberately slow and sparse: a companion, not a screensaver.

use crate::catching::{level_hop, CatchPhase, Catching};
use crate::rng::Rng;

/// Longest ordinary floor hop, in pixels actually travelled.
const HOP_MAX: f32 = 80.0;
/// Real travel of a floor hop on the body (radius 64, after the crouch), as (pixels, launch speed in px/s).
/// Measured on `slimy-body`; the curve is steep and not a square root, so it is tabulated, not fitted.
const HOP_CURVE: [(f32, f32); 10] = [
    (0.0, 120.0),
    (3.2, 150.0),
    (6.8, 200.0),
    (12.0, 250.0),
    (19.6, 300.0),
    (31.4, 350.0),
    (42.0, 380.0),
    (77.0, 400.0),
    (195.0, 500.0),
    (286.0, 600.0),
];
const HOP_RADIUS: f32 = 64.0;
const CROUCH_FOR: f64 = 0.16;
const NEAR_USER: f32 = 260.0;
const FOLLOW_GAP: f32 = 150.0;
const ARRIVED: f32 = 26.0;
/// Distance of a resting spot from a screen edge, in radii: the body is about 1.1 radii wide on each side.
const EDGE_MARGIN: f32 = 1.5;
const WALL_SPEED: f32 = 400.0;
const MAX_CLIMB_TRIES: u8 = 3;
/// Wall descent: launch speeds and the body-to-floor clearances (px) that choose between them.
const DESCENT_FAST: f32 = 560.0;
const DESCENT_SLOW: f32 = 380.0;
const DESCENT_FAR: f32 = 60.0;
const DESCENT_FOOT: f32 = 25.0;
const STEP_OFF_SPEED: f32 = 330.0;

/// Launch speed (px/s) of a floor hop that travels `distance` pixels, read off the measured curve.
fn hop_power(distance: f32) -> f32 {
    let distance = distance.clamp(0.0, HOP_CURVE[HOP_CURVE.len() - 1].0);
    let at = HOP_CURVE
        .iter()
        .position(|(d, _)| *d >= distance)
        .unwrap_or(HOP_CURVE.len() - 1)
        .max(1);
    let ((d0, p0), (d1, p1)) = (HOP_CURVE[at - 1], HOP_CURVE[at]);
    p0 + (p1 - p0) * (distance - d0) / (d1 - d0)
}

/// Pixels a floor hop of this launch speed travels on a body of radius 64 (inverse of the hop planning).
pub fn hop_reach(power: f32) -> f32 {
    let power = power.clamp(HOP_CURVE[0].1, HOP_CURVE[HOP_CURVE.len() - 1].1);
    let at = HOP_CURVE
        .iter()
        .position(|(_, p)| *p >= power)
        .unwrap_or(HOP_CURVE.len() - 1)
        .max(1);
    let ((d0, p0), (d1, p1)) = (HOP_CURVE[at - 1], HOP_CURVE[at]);
    d0 + (d1 - d0) * (power - p0) / (p1 - p0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Floor,
    Left,
    Right,
    Ceiling,
}

#[derive(Debug, Clone, Copy)]
pub struct World {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub now: f64,
    pub x: f32,
    pub y: f32,
    pub place: Place,
    /// Touching its surface and nearly still: ready for the next move.
    pub settled: bool,
    /// Still touching the surface, including while squashing before a leap.
    pub supported: bool,
    pub grabbed: bool,
    /// A bubble is open or the birth is running: the slime keeps still.
    pub busy: bool,
    pub sulking: bool,
    pub sleepiness: f32,
    pub affection: f32,
    pub pointer: Option<(f32, f32)>,
    pub pointer_active: bool,
    pub radius: f32,
    /// Half the width of the resting body, claws or wings included (at least the radius): how far from a cursor it must be
    /// to stop covering it.
    pub reach: f32,
    pub world: World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Idle,
    Wander,
    Hide,
    Peek,
    Nap,
    Follow,
    /// The slime is catching the cursor with its whole body (see `CatchPhase`).
    Catch,
}

#[derive(Debug, Clone, Copy)]
pub struct Output {
    /// Unit direction (x, y) and speed of a leap to make now.
    pub hop: Option<(f32, f32, f32)>,
    /// Press against the surface: the squash before a leap.
    pub crouch: bool,
    /// Where to look, each axis from -1 to 1.
    pub gaze: Option<(f32, f32)>,
    pub intent: Intent,
    /// 1 while the whole body is squashed flat over the cursor, 0 otherwise: the page flattens the body (volume kept).
    pub flatten: f32,
    /// A gentle crouch and a level hop (no turn): the small approach hops of a catch.
    pub gentle: bool,
    /// Which step of a catch the slime is in, for the face.
    pub catching: CatchPhase,
}

#[derive(Debug, Clone, Copy)]
enum Goal {
    Idle,
    Wander { tx: f32 },
    Climb { left: bool, ty: f32 },
    Descend,
    Hide { corner: f32 },
    Nap { corner: f32 },
    Follow,
}

#[derive(Debug, Clone, Copy)]
struct Leap {
    launch_at: f64,
    dir: (f32, f32),
    power: f32,
    gentle: bool,
}

pub struct Autonomy {
    rng: Rng,
    goal: Goal,
    until: f64,
    next_hop: f64,
    leap: Option<Leap>,
    next_peek: f64,
    peek_until: f64,
    started: bool,
    climb_tries: u8,
    catching: Catching,
    inset: f32,
}

impl Autonomy {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            goal: Goal::Idle,
            until: 0.0,
            next_hop: 0.0,
            leap: None,
            next_peek: 0.0,
            peek_until: -1.0,
            started: false,
            climb_tries: 0,
            catching: Catching::default(),
            inset: 0.0,
        }
    }

    fn unit(&mut self) -> f32 {
        self.rng.unit()
    }

    fn between(&mut self, lo: f32, hi: f32) -> f32 {
        self.rng.between(lo, hi)
    }

    fn corner(s: &Snapshot) -> f32 {
        let margin = s.radius * EDGE_MARGIN;
        if s.x < (s.world.left + s.world.right) / 2.0 {
            s.world.left + margin
        } else {
            s.world.right - margin
        }
    }

    fn rest(&mut self, now: f64, seconds: (f32, f32)) {
        self.goal = Goal::Idle;
        self.leap = None;
        self.until = now + f64::from(self.between(seconds.0, seconds.1));
    }

    fn decide(&mut self, s: &Snapshot) {
        let margin = s.radius * EDGE_MARGIN;
        let roll = self.unit();
        if s.place != Place::Floor {
            self.goal = Goal::Descend;
        } else if s.pointer_active && s.pointer.is_some() && s.affection > 0.55 && roll < 0.35 {
            self.goal = Goal::Follow;
            self.until = s.now + f64::from(self.between(8.0, 16.0));
        } else if roll < 0.6 {
            let mut step = self.between(140.0, 520.0);
            if self.unit() < 0.5 {
                step = -step;
            }
            let mut tx = (s.x + step).clamp(s.world.left + margin, s.world.right - margin);
            if (tx - s.x).abs() < 100.0 {
                tx = (s.x - step).clamp(s.world.left + margin, s.world.right - margin);
            }
            self.goal = Goal::Wander { tx };
        } else if roll < 0.78 && self.inset == 0.0 {
            let left = s.x < (s.world.left + s.world.right) / 2.0;
            let ty = s.world.bottom - self.between(120.0, 320.0);
            self.goal = Goal::Climb { left, ty };
            self.climb_tries = 0;
        } else {
            self.rest(s.now, (6.0, 20.0));
        }
    }

    /// Prepares a floor hop toward `tx`; true when already there.
    fn floor_hop(&mut self, s: &Snapshot, tx: f32) -> bool {
        let dx = tx - s.x;
        if dx.abs() < ARRIVED {
            return s.settled;
        }
        if self.leap.is_none() && s.settled && s.now >= self.next_hop && s.place == Place::Floor {
            let dist = dx.abs().min(HOP_MAX * self.between(0.7, 1.15));
            let power = hop_power(dist) * (s.radius / HOP_RADIUS).sqrt();
            self.leap = Some(Leap {
                launch_at: s.now + CROUCH_FOR,
                dir: (dx.signum() * 0.66, -0.75),
                power,
                gentle: false,
            });
        }
        false
    }

    fn wall_hop(&mut self, s: &Snapshot, dir: (f32, f32), power: f32) {
        if self.leap.is_none() && s.settled && s.now >= self.next_hop {
            self.leap = Some(Leap {
                launch_at: s.now + CROUCH_FOR,
                dir,
                power,
                gentle: false,
            });
        }
    }

    /// One step down a wall: firm hops while far above the floor, then a diagonal step off the wall once
    /// the body's foot is near it. A downward hop there would drive the body into the floor hard enough to hurt.
    fn descend_hop(&mut self, s: &Snapshot) {
        let inward = if s.place == Place::Left { 1.0 } else { -1.0 };
        let clearance = s.world.bottom - (s.y + s.radius);
        if clearance <= DESCENT_FOOT {
            self.wall_hop(s, (inward * 0.8, 0.6), STEP_OFF_SPEED);
        } else {
            let power = if clearance > DESCENT_FAR {
                DESCENT_FAST
            } else {
                DESCENT_SLOW
            };
            self.wall_hop(s, (inward * 0.2, 0.95), power);
        }
    }

    /// Keeps the body this far (px) from the screen edges: limbs that stick out of it (a tail, claws) must
    /// stay on screen. A slime with limbs does not climb walls.
    pub fn set_inset(&mut self, px: f32) {
        self.inset = if px.is_finite() {
            px.clamp(0.0, 200.0)
        } else {
            0.0
        };
    }

    /// Crouches until the planned leap is due, then launches it.
    fn resolve_leap(&mut self, s: &Snapshot, out: &mut Output) {
        if let Some(leap) = self.leap {
            out.gentle = leap.gentle;
            if s.now >= leap.launch_at {
                out.hop = Some((leap.dir.0, leap.dir.1, leap.power));
                self.leap = None;
                self.next_hop = s.now + f64::from(self.between(0.5, 1.1));
            } else {
                out.crouch = true;
            }
        }
    }

    pub fn step(&mut self, snapshot: &Snapshot) -> Output {
        let inset = Snapshot {
            world: World {
                left: snapshot.world.left + self.inset,
                right: snapshot.world.right - self.inset,
                ..snapshot.world
            },
            ..*snapshot
        };
        let s = &inset;
        let mut out = Output {
            hop: None,
            crouch: false,
            gaze: None,
            intent: Intent::Idle,
            flatten: 0.0,
            gentle: false,
            catching: CatchPhase::None,
        };
        if !self.started {
            self.started = true;
            self.until = s.now + f64::from(self.between(4.0, 10.0));
            self.next_peek = s.now + f64::from(self.between(6.0, 14.0));
        }
        if s.grabbed || s.busy {
            self.catching.abort(s.now);
            self.leap = None;
            self.rest(s.now, (2.0, 4.0));
            return out;
        }
        if !s.supported {
            self.leap = None;
        }
        if s.place == Place::Ceiling {
            self.leap = None;
            if s.settled && s.now >= self.next_hop {
                self.next_hop = s.now + 2.0;
                out.hop = Some((0.0, 1.0, 320.0));
            }
            return out;
        }

        if s.sulking {
            self.catching.abort(s.now);
        }
        if let Some(step) = self.catching.update(s, self.leap.is_some()) {
            out.intent = Intent::Catch;
            out.catching = step.phase;
            out.flatten = step.flatten;
            out.gaze = Some(step.gaze);
            self.goal = Goal::Idle;
            self.until = s.now + 2.0;
            if let Some(dx) = step.hop {
                if self.leap.is_none() {
                    let (x, y, power) = level_hop(dx);
                    let scale = s.radius / HOP_RADIUS;
                    self.leap = Some(Leap {
                        launch_at: s.now + CROUCH_FOR,
                        dir: (x, y),
                        power: power * scale.sqrt(),
                        gentle: true,
                    });
                }
            }
            self.resolve_leap(s, &mut out);
            return out;
        }

        let corner = Self::corner(s);
        match (s.sulking, s.sleepiness > 0.6) {
            (true, _) => {
                if !matches!(self.goal, Goal::Hide { .. }) {
                    self.leap = None;
                    self.goal = Goal::Hide { corner };
                    self.next_peek = s.now + f64::from(self.between(8.0, 18.0));
                }
            }
            (false, true) => {
                if !matches!(self.goal, Goal::Nap { .. }) {
                    self.leap = None;
                    self.goal = Goal::Nap { corner };
                }
            }
            (false, false) => {
                if matches!(self.goal, Goal::Hide { .. } | Goal::Nap { .. }) {
                    self.rest(s.now, (3.0, 8.0));
                }
            }
        }

        if s.place != Place::Floor && matches!(self.goal, Goal::Wander { .. } | Goal::Follow) {
            self.leap = None;
            self.goal = Goal::Descend;
        }
        let near_user = s.pointer_active
            && s.pointer
                .is_some_and(|(px, py)| (px - s.x).hypot(py - s.y) < NEAR_USER);
        if near_user
            && s.place == Place::Floor
            && matches!(self.goal, Goal::Wander { .. } | Goal::Climb { .. })
        {
            self.leap = None;
        }
        match self.goal {
            Goal::Idle => {
                if s.now >= self.until {
                    self.decide(s);
                }
            }
            Goal::Wander { tx } => {
                out.intent = Intent::Wander;
                out.gaze = Some(((tx - s.x).signum() * 0.7, -0.1));
                if !near_user && self.floor_hop(s, tx) {
                    self.rest(s.now, (5.0, 20.0));
                }
            }
            Goal::Follow => {
                out.intent = Intent::Follow;
                match s.pointer.filter(|_| s.pointer_active) {
                    Some((px, _)) => {
                        let side = (px - s.x).signum();
                        let tx = (px - side * FOLLOW_GAP)
                            .clamp(s.world.left + s.radius, s.world.right - s.radius);
                        out.gaze = Some((side * 0.8, -0.2));
                        if s.now >= self.until {
                            self.rest(s.now, (4.0, 10.0));
                        } else if (px - s.x).abs() < FOLLOW_GAP + ARRIVED {
                            self.leap = None;
                        } else {
                            if self
                                .leap
                                .is_some_and(|leap| leap.dir.0.signum() != (tx - s.x).signum())
                            {
                                self.leap = None;
                            }
                            self.floor_hop(s, tx);
                        }
                    }
                    None => self.rest(s.now, (4.0, 10.0)),
                }
            }
            Goal::Climb { left, ty } => {
                out.intent = Intent::Wander;
                let base = if left {
                    s.world.left + s.radius * 1.2
                } else {
                    s.world.right - s.radius * 1.2
                };
                let toward = if left { -1.0 } else { 1.0 };
                let inward = -toward;
                out.gaze = Some(if s.place == Place::Floor {
                    (toward * 0.7, -0.3)
                } else {
                    (inward * 0.2, (ty - s.y).signum() * 0.8)
                });
                if s.place == Place::Floor {
                    if self.climb_tries >= MAX_CLIMB_TRIES {
                        self.rest(s.now, (6.0, 16.0));
                    } else if !near_user
                        && self.floor_hop(s, base)
                        && self.leap.is_none()
                        && s.settled
                        && s.now >= self.next_hop
                    {
                        self.climb_tries += 1;
                        self.wall_hop(s, (toward * 0.7, -0.7), WALL_SPEED);
                    }
                } else {
                    let dy = ty - s.y;
                    if dy.abs() < 30.0 {
                        self.rest(s.now, (8.0, 25.0));
                    } else {
                        self.wall_hop(
                            s,
                            (inward * 0.3, dy.signum() * 0.95),
                            (dy.abs().min(120.0) * 7.5).clamp(300.0, 560.0),
                        );
                    }
                }
            }
            Goal::Descend => {
                out.intent = Intent::Wander;
                out.gaze = Some((0.0, 0.8));
                if s.place == Place::Floor {
                    self.rest(s.now, (6.0, 16.0));
                } else {
                    self.descend_hop(s);
                }
            }
            Goal::Hide { corner } | Goal::Nap { corner } => {
                let napping = matches!(self.goal, Goal::Nap { .. });
                let wall_side = if corner < (s.world.left + s.world.right) / 2.0 {
                    -1.0
                } else {
                    1.0
                };
                out.intent = if napping {
                    Intent::Wander
                } else {
                    Intent::Hide
                };
                if s.place != Place::Floor {
                    self.descend_hop(s);
                } else if self.floor_hop(s, corner) {
                    self.leap = None;
                    if napping {
                        out.intent = Intent::Nap;
                        out.gaze = Some((wall_side * 0.4, 0.3));
                    } else {
                        if s.now >= self.next_peek {
                            self.peek_until = s.now + 1.6;
                            self.next_peek = s.now + f64::from(self.between(12.0, 32.0));
                        }
                        if s.now < self.peek_until {
                            out.intent = Intent::Peek;
                            out.gaze = Some((-wall_side * 0.9, -0.1));
                        } else {
                            out.gaze = Some((wall_side * 0.9, 0.35));
                        }
                    }
                } else {
                    out.gaze = Some(((corner - s.x).signum() * 0.7, 0.2));
                }
            }
        }

        self.resolve_leap(s, &mut out);
        out
    }
}
