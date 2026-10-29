//! Catching the cursor with the whole body, the way a slime would: it never reaches into the air. A cursor lying
//! near the floor is approached by a small level hop, then the slime squashes flat over it, holds it a moment and
//! draws back. The numbers come from calculations run on the real body (radius 64): the hop speed is fitted on the
//! hop that lands where it was aimed, the squash is an axis ratio of 0.8 with the volume kept.

use crate::behavior::{Place, Snapshot};

/// The cursor must lie this high above the floor (px): a strip, not a direction. Height wins over angle.
const HEIGHT_MIN: f32 = 4.0;
const HEIGHT_MAX: f32 = 48.0;
/// A running catch is dropped when the cursor leaves this band.
const HEIGHT_DROP: f32 = 56.0;
/// Directly under the body: spread at once. Farther: approach first. Beyond: only look.
const DIRECT: f32 = 32.0;
const NEAR: f32 = 96.0;
const GIVE_UP: f32 = 112.0;
/// The slime stops this far (in radii) before the cursor, so that the squash covers it.
const STOP_SHORT: f32 = 0.4;
/// Validated approach hops move the centre by this much (px, radius 64).
const MOVE_MIN: f32 = 6.4;
const MOVE_MAX: f32 = 70.4;
/// The cursor must hold still within this (px) for this long (s) before a catch begins, and may not drift more
/// than `MOVED` (px) from where it was found while the catch runs.
const STABLE_PX: f32 = 6.4;
const STABLE_FOR: f64 = 0.15;
const MOVED: f32 = 12.8;
const ACQUIRE_FOR: f64 = 0.35;
/// After a hop: at least this long, and at most this long waiting for the body to settle.
const SETTLE_MIN: f64 = 1.0;
const SETTLE_MAX: f64 = 2.5;
const CONTACT_FOR: f64 = 0.2;
/// Squash: transition, then hold, then the profile comes back.
const SPREAD_FOR: f64 = 1.0;
const HOLD_FOR: f64 = 1.2;
const RESTORE_FOR: f64 = 1.4;
/// The slime backs off to this far (px) on its own side of the cursor.
const EXIT_GAP: f32 = 80.0;
/// A retreat ends this many radii beyond the body's own reach, so even wide claws or wings let go of the cursor.
const EXIT_CLEAR: f32 = 0.4;
const MAX_RETREAT_HOPS: u8 = 3;
const ABORT_REST: f64 = 10.0;
/// A companion that pounces on a resting cursor every ten seconds would be a tic: it waits a good while.
const DONE_REST: f64 = 45.0;
const AFFECTION: f32 = 0.55;
const RADIUS: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CatchPhase {
    None = 0,
    Acquire = 1,
    Prepare = 2,
    Flight = 3,
    Contact = 4,
    Spread = 5,
    Hold = 6,
    Retreat = 7,
}

/// What the catch wants this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CatchStep {
    pub phase: CatchPhase,
    /// Where to look, each axis from -1 to 1.
    pub gaze: (f32, f32),
    /// 1 while the body is squashed over the cursor, 0 otherwise.
    pub flatten: f32,
    /// A level hop to start now: how far to move the centre (signed px, already scaled for the radius).
    pub hop: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
struct Run {
    phase: CatchPhase,
    since: f64,
    side: f32,
    cursor: (f32, f32),
    exit_x: f32,
    asked: bool,
    cancelled: bool,
    retreat_hops: u8,
}

#[derive(Debug, Default)]
pub(crate) struct Catching {
    run: Option<Run>,
    anchor: Option<((f32, f32), f64)>,
    rest_until: f64,
}

/// Launch velocity (px/s) of a level approach hop that moves the centre by `dx` pixels, fitted on the real body.
pub fn level_hop(dx: f32) -> (f32, f32, f32) {
    let moved = dx.abs();
    let vx = dx.signum() * (10.980_972 * moved - 6.478_247);
    let power = vx.hypot(110.0);
    (vx / power, -110.0 / power, power)
}

impl Catching {
    /// The slime was grabbed, or something else took over: the catch is gone (the profile is restored by the page,
    /// since no step asks for the squash any more).
    pub fn abort(&mut self, now: f64) {
        if self.run.take().is_some() {
            self.rest_until = now + ABORT_REST;
        }
    }

    fn finish(&mut self, now: f64, rest: f64) {
        self.run = None;
        self.rest_until = now + rest;
    }

    /// `leaping` is true while a hop of the slime is crouching or in preparation.
    pub fn update(&mut self, s: &Snapshot, leaping: bool) -> Option<CatchStep> {
        let scale = s.radius / RADIUS;
        let floor_h = s.pointer.map(|p| s.world.bottom - p.1);
        match s.pointer {
            Some(p) => match self.anchor {
                Some((a, _)) if (a.0 - p.0).hypot(a.1 - p.1) <= STABLE_PX * scale => {}
                _ => self.anchor = Some((p, s.now)),
            },
            None => self.anchor = None,
        }
        if self.run.is_some() {
            return self.step(s, leaping, scale, floor_h);
        }
        self.try_start(s, scale, floor_h)
    }

    fn try_start(&mut self, s: &Snapshot, scale: f32, floor_h: Option<f32>) -> Option<CatchStep> {
        let willing = s.place == Place::Floor
            && s.supported
            && s.settled
            && !s.sulking
            && s.sleepiness < 0.5
            && s.affection > AFFECTION
            && s.now >= self.rest_until;
        let ((px, py), since) = self.anchor?;
        let h = floor_h?;
        let dx = px - s.x;
        let margin = s.radius;
        let inside = |x: f32| x > s.world.left + margin && x < s.world.right - margin;
        let side = if dx.abs() < 1.0 {
            if s.x < (s.world.left + s.world.right) / 2.0 {
                1.0
            } else {
                -1.0
            }
        } else {
            dx.signum()
        };
        let exit_x =
            px - side * (EXIT_GAP * scale).max(s.reach.max(s.radius) + EXIT_CLEAR * s.radius);
        if !willing
            || s.now - since < STABLE_FOR
            || !(HEIGHT_MIN * scale..=HEIGHT_MAX * scale).contains(&h)
            || dx.abs() > NEAR * scale
            || !inside(px)
            || !inside(exit_x)
        {
            return None;
        }
        self.run = Some(Run {
            phase: CatchPhase::Acquire,
            since: s.now,
            side,
            cursor: (px, py),
            exit_x,
            asked: false,
            cancelled: false,
            retreat_hops: 0,
        });
        Some(self.look(CatchPhase::Acquire, s, 0.0))
    }

    fn look(&self, phase: CatchPhase, s: &Snapshot, flatten: f32) -> CatchStep {
        let toward = self.run.map_or(1.0, |r| r.side);
        let aim = self.run.map_or(s.x, |r| r.cursor.0) - s.x;
        let side = if aim.abs() < 1.0 {
            toward
        } else {
            aim.signum()
        };
        CatchStep {
            phase,
            gaze: (side * 0.8, 0.45),
            flatten,
            hop: None,
        }
    }

    fn step(
        &mut self,
        s: &Snapshot,
        leaping: bool,
        scale: f32,
        floor_h: Option<f32>,
    ) -> Option<CatchStep> {
        let mut run = self.run?;
        let elapsed = s.now - run.since;
        // The cursor left, moved away or was lifted off the floor strip: the catch ends gently.
        let gone = match (s.pointer, floor_h) {
            (Some(p), Some(h)) => {
                (p.0 - run.cursor.0).hypot(p.1 - run.cursor.1) > MOVED * scale
                    || !(0.0..=HEIGHT_DROP * scale).contains(&h)
            }
            _ => true,
        };
        if gone && !run.cancelled {
            match run.phase {
                CatchPhase::Acquire | CatchPhase::Prepare if !run.asked && !leaping => {
                    self.finish(s.now, ABORT_REST);
                    return None;
                }
                CatchPhase::Contact | CatchPhase::Spread | CatchPhase::Hold => {
                    run = Run {
                        phase: CatchPhase::Retreat,
                        since: s.now,
                        asked: false,
                        ..run
                    };
                }
                _ => run.cancelled = true,
            }
        }
        if run.phase == CatchPhase::Acquire && (s.x - run.cursor.0).abs() > GIVE_UP * scale {
            self.finish(s.now, ABORT_REST);
            return None;
        }
        let out = match run.phase {
            CatchPhase::Acquire => {
                if elapsed >= ACQUIRE_FOR {
                    let dx = run.cursor.0 - s.x;
                    let target = run.cursor.0 - run.side * STOP_SHORT * s.radius;
                    let go = target - s.x;
                    if dx.abs() <= DIRECT * scale || go.abs() < MOVE_MIN * scale {
                        run = Run {
                            phase: CatchPhase::Contact,
                            since: s.now,
                            ..run
                        };
                        self.look(CatchPhase::Contact, s, 0.0)
                    } else {
                        run = Run {
                            phase: CatchPhase::Prepare,
                            since: s.now,
                            ..run
                        };
                        let mut step = self.look(CatchPhase::Prepare, s, 0.0);
                        step.hop = Some(go.clamp(-MOVE_MAX * scale, MOVE_MAX * scale) / scale);
                        run.asked = true;
                        step
                    }
                } else {
                    self.look(CatchPhase::Acquire, s, 0.0)
                }
            }
            CatchPhase::Prepare => {
                if !leaping && run.asked {
                    run = Run {
                        phase: CatchPhase::Flight,
                        since: s.now,
                        ..run
                    };
                }
                self.look(run.phase, s, 0.0)
            }
            CatchPhase::Flight => {
                if elapsed >= SETTLE_MIN && s.settled {
                    let next = if run.cancelled {
                        CatchPhase::Retreat
                    } else {
                        CatchPhase::Contact
                    };
                    run = Run {
                        phase: next,
                        since: s.now,
                        asked: false,
                        ..run
                    };
                } else if elapsed > SETTLE_MAX {
                    self.finish(s.now, ABORT_REST);
                    return None;
                }
                self.look(CatchPhase::Flight, s, 0.0)
            }
            CatchPhase::Contact => {
                if elapsed >= CONTACT_FOR {
                    run = Run {
                        phase: CatchPhase::Spread,
                        since: s.now,
                        ..run
                    };
                }
                self.look(CatchPhase::Contact, s, 0.0)
            }
            CatchPhase::Spread => {
                if elapsed >= SPREAD_FOR {
                    run = Run {
                        phase: CatchPhase::Hold,
                        since: s.now,
                        ..run
                    };
                }
                self.look(CatchPhase::Spread, s, 1.0)
            }
            CatchPhase::Hold => {
                if elapsed >= HOLD_FOR {
                    run = Run {
                        phase: CatchPhase::Retreat,
                        since: s.now,
                        asked: false,
                        ..run
                    };
                }
                self.look(CatchPhase::Hold, s, 1.0)
            }
            CatchPhase::Retreat => {
                let mut step = self.look(CatchPhase::Retreat, s, 0.0);
                if elapsed >= RESTORE_FOR && !leaping {
                    let away = run.exit_x - s.x;
                    if run.asked {
                        // A hop was asked: wait for it to land and settle before looking again.
                        if elapsed >= RESTORE_FOR + SETTLE_MIN && s.settled {
                            run.asked = false;
                            run.since = s.now - RESTORE_FOR;
                        }
                    } else if away.abs() <= MOVE_MIN * scale || run.retreat_hops >= MAX_RETREAT_HOPS
                    {
                        self.finish(s.now, DONE_REST);
                        return None;
                    } else {
                        step.hop = Some(away.clamp(-MOVE_MAX * scale, MOVE_MAX * scale) / scale);
                        run.asked = true;
                        run.retreat_hops += 1;
                        run.since = s.now - RESTORE_FOR;
                    }
                }
                step
            }
            CatchPhase::None => return None,
        };
        self.run = Some(run);
        Some(out)
    }
}
