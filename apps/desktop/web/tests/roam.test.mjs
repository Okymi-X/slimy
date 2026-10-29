import test from 'node:test';
import assert from 'node:assert/strict';
import { createRoam } from '../roam.js';

const state = { busy: false, sulking: false, sleepiness: 0, affection: 0.5, pointer: { known: false, active: false, x: 0, y: 0 } };

function scene(angle = 0) {
  let paused = false;
  let hops = 0;
  const slime = {
    centroid_x: () => 100, centroid_y: () => 200, gravity_up: () => [0, -1],
    surface_gap: () => 0, speed: () => 0, is_grabbed: () => false,
    orientation: () => angle, crouch(d) { this.crouches.push(d); }, hop() { hops++; },
    flattened: 0, leaps: [], crouches: [], flatten_calls: 0, set_flatten(a) { this.flattened = a; this.flatten_calls++; }, leap_level(x, y, p) { this.leaps.push([x, y, p]); },
  };
  const roamer = { step(input) {
    paused = input[5] === 1;
    return paused ? [0, 0, 0, 0, 0, 0, 0, 0] : [0, 0, 0, 0, 0, -1, 1, 1];
  } };
  return { slime, roam: createRoam({ slime, roamer, radius: 64 }), paused: () => paused, hops: () => hops };
}

test('a world direction remains the same on screen when the body rotates', () => {
  for (const angle of [0, Math.PI / 2, -Math.PI / 2, Math.PI]) {
    const { roam } = scene(angle);
    const { gaze } = roam.update(1, state);
    const x = gaze.x * Math.cos(angle) - gaze.y * Math.sin(angle);
    const y = gaze.x * Math.sin(angle) + gaze.y * Math.cos(angle);
    assert.ok(Math.abs(x) < 1e-6);
    assert.ok(Math.abs(y + 1) < 1e-6);
  }
});

test('disabling roaming informs the decision engine and cancels its pending movement', () => {
  const s = scene();
  s.roam.enabled = false;
  const out = s.roam.update(1, state);
  assert.equal(s.paused(), true);
  assert.equal(out.intent, 'idle');
  assert.equal(out.gaze, null);
  assert.equal(s.hops(), 0);
});

const catchSlime = () => ({
  centroid_x: () => 100, centroid_y: () => 200, gravity_up: () => [0, -1],
  surface_gap: () => 0, speed: () => 0, is_grabbed: () => false, orientation: () => 0,
  flattened: 0, flatten_calls: 0, leaps: [], hops: [], crouches: [],
  crouch(d) { this.crouches.push(d); }, hop(...a) { this.hops.push(a); }, leap_level(...a) { this.leaps.push(a); },
  set_flatten(a) { this.flattened = a; this.flatten_calls++; },
});
// Roamer outputs: hop dx, dy, power, crouch, gaze x, gaze y, has gaze, intent, flatten, gentle, catch phase.
const outputs = (extra) => Object.assign([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], extra);

test('a catch squashes the whole body while the decider asks for it and gives it back when it stops', () => {
  const slime = catchSlime();
  let out = outputs({ 7: 6, 8: 1, 10: 5 });
  const roam = createRoam({ slime, roamer: { step: () => out }, radius: 64 });
  const first = roam.update(1, state);
  assert.equal(first.intent, 'catch');
  assert.equal(first.phase, 'spread');
  assert.equal(slime.flattened, 1);
  roam.update(1.02, state);
  roam.update(1.04, state);
  assert.equal(slime.flatten_calls, 1, 'the body is told once, not every frame');
  out = outputs({});
  roam.update(2, state);
  assert.equal(slime.flattened, 0);
});

test('the approach hop of a catch is soft and level, an ordinary hop is neither', () => {
  const slime = catchSlime();
  let out = outputs({ 3: 1, 7: 6, 9: 1, 10: 2 });
  const roam = createRoam({ slime, roamer: { step: () => out }, radius: 64 });
  roam.update(1, state, 1 / 60);
  assert.ok(Math.abs(slime.crouches[0] - 5) < 1e-9, `gentle crouch ${slime.crouches[0]} per 1/60 s`);
  out = outputs({ 0: 0.97, 1: -0.25, 2: 450, 7: 6, 9: 1, 10: 3 });
  roam.update(1.2, state);
  assert.equal(slime.leaps.length, 1);
  assert.equal(slime.hops.length, 0, 'no leaning hop in a catch');
  out = outputs({ 3: 1, 7: 1 });
  roam.update(2, state, 1 / 60);
  assert.ok(Math.abs(slime.crouches[1] - 40) < 1e-9, 'an ordinary crouch is the strong one');
  out = outputs({ 0: 0.5, 1: -0.8, 2: 300, 7: 1 });
  roam.update(2.2, state);
  assert.equal(slime.hops.length, 1);
});

test('a resting cursor is handed to the decider as known, a moving one as active', () => {
  const seen = [];
  const slime = catchSlime();
  const roam = createRoam({ slime, roamer: { step: (input) => { seen.push([input[11], input[14]]); return outputs({}); } }, radius: 64 });
  roam.update(1, { ...state, pointer: { known: true, active: false, x: 10, y: 20 } });
  roam.update(2, { ...state, pointer: { known: true, active: true, x: 10, y: 20 } });
  roam.update(3, { ...state, pointer: { known: false, active: false, x: 0, y: 0 } });
  assert.deepEqual(seen, [[0, 1], [1, 1], [0, 0]]);
});
