// Turns the roamer's decisions into leaps of the real body: reads where the slime is, asks what to
// do, then crouches and hops. It never acts while the user is holding the slime or a bubble is open.

import { createWatch, walkReward, WALK_WINDOW } from './learn.js';

const SURFACE = { floor: 0, left: 1, right: 2, ceiling: 3 };
const CROUCH_DEPTH = 40;
// The soft crouch of a catch's approach hop: an impulse rate (px/s per second), applied in proportion to the step.
const GENTLE_CROUCH = 300;
const SETTLED_GAP = 3;
const SETTLED_SPEED = 60;
export const INTENT = ['idle', 'wander', 'hide', 'peek', 'nap', 'follow', 'catch'];
/** The steps of a catch, as the decider numbers them (0 is none). */
export const CATCH_PHASE = ['none', 'acquire', 'prepare', 'flight', 'contact', 'spread', 'hold', 'retreat'];

function surfaceOf([x, y]) {
  if (y < -0.5) return SURFACE.floor;
  if (y > 0.5) return SURFACE.ceiling;
  return x > 0 ? SURFACE.left : SURFACE.right;
}

/**
 * How often the slime starts a wander: after each walk it stays put for a gap picked by the learning model (a short list of
 * seconds written in the backend). The person reacting soon after a walk is a yes, shooing the slime away a no.
 */
export function createWalkUrge({ learn, window = WALK_WINDOW }) {
  const watch = createWatch(window);
  let choice = null, asked = false, walking = false, restUntil = 0, lastIntent = 'idle';
  const settle = async (done) => {
    if (!done) return;
    await learn?.reward(done.choice, walkReward(done.outcome));
    choice = await learn?.choose('walk_gap') ?? null;
  };
  return {
    /** True while the slime should keep still because the last walk was too recent. Only idle moments are held. */
    holding(now) { return now < restUntil && lastIntent === 'idle'; },
    async update(now, intent) {
      lastIntent = intent;
      if (learn && !asked) { asked = true; choice = await learn.choose('walk_gap'); }
      await settle(watch.tick(now));
      if (intent === 'wander' && !walking) { walking = true; if (!watch.active) watch.start(now, choice); }
      else if (intent !== 'wander' && walking) { walking = false; if (choice) restUntil = now + choice.value; }
    },
    /** The person touched, clicked or wrote to the slime. */
    interaction(now) { return settle(watch.interact(now)); },
    /** The person threw the slime or pushed it away. */
    shoo() { return settle(watch.cut()); },
  };
}

export function createRoam({ slime, roamer, radius, walk = null }) {
  let enabled = true;
  let reach = 1;
  let flattened = 0;
  return {
    /** Keeps the body this far from the screen edges, so limbs that stick out of it stay visible. */
    setInset(px) { roamer.set_inset(px); },
    /** The play area changed (another screen, a new resolution). */
    setWorld(w) { roamer.set_world(w.left, w.right, w.top, w.bottom); },
    /** Half the resting width of the body in radii (claws and wings included): the catch lets go of the cursor beyond it. */
    setReach(radii) { reach = Math.max(1, radii); },
    get enabled() { return enabled; },
    set enabled(value) { enabled = value; },
    /** state: { busy, sulking, sleepiness, affection, pointer: {x, y, known, active} }. Returns { gaze, intent, crouching, phase }. */
    update(now, state, dt = 1 / 60) {
      const out = roamer.step(Float32Array.of(
        slime.centroid_x(), slime.centroid_y(), surfaceOf(slime.gravity_up()),
        slime.surface_gap() < SETTLED_GAP && slime.speed() < SETTLED_SPEED ? 1 : 0,
        slime.is_grabbed() ? 1 : 0, state.busy || !enabled || walk?.holding(now) ? 1 : 0, state.sulking ? 1 : 0,
        state.sleepiness, state.affection,
        state.pointer.x, state.pointer.y, state.pointer.known && state.pointer.active ? 1 : 0, radius, slime.surface_gap(),
        state.pointer.known ? 1 : 0, radius * reach,
      ), now);
      const gentle = out[9] > 0.5;
      // Called once per physics step, so the squash is proportional to the step length.
      if (out[3] > 0.5) slime.crouch(gentle ? GENTLE_CROUCH * dt : CROUCH_DEPTH * dt * 60);
      if (out[2] > 0) {
        if (gentle) slime.leap_level(out[0], out[1], out[2]);
        else slime.hop(out[0], out[1], out[2]);
      }
      // The whole body spreads over the cursor while the decider asks for it, and comes back when it stops asking.
      if (out[8] !== flattened) { flattened = out[8]; slime.set_flatten(flattened); }
      const intent = INTENT[out[7]] ?? 'idle';
      walk?.update(now, intent);
      const phi = slime.orientation();
      const cos = Math.cos(phi), sin = Math.sin(phi);
      return {
        gaze: out[6] > 0.5 ? { x: out[4] * cos + out[5] * sin, y: -out[4] * sin + out[5] * cos } : null,
        intent, crouching: out[3] > 0.5, phase: CATCH_PHASE[out[10]] ?? 'none',
      };
    },
  };
}
