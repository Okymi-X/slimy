import test from 'node:test';
import assert from 'node:assert/strict';
import { createExpressions, EXPRESSIONS } from '../expressions.js';

const base = { grabbed: false, hover: false, pointerSpeed: 0, speed: 0, airborne: false, squash: 0.9, sleepiness: 0, acc: 0 };
const run = (machine, input, seconds, start = 0) => {
  let out;
  for (let i = 0; i < seconds * 60; i++) out = machine.update(input, start + i / 60, 1 / 60);
  return out;
};
const quiet = () => createExpressions();

test('a resting slime is calm', () => {
  assert.equal(run(quiet(), base, 1).name, 'calm');
});

test('being held is a surprise, and a fast ride is joy', () => {
  assert.equal(run(quiet(), { ...base, grabbed: true }, 1).name, 'surprised');
  assert.equal(run(quiet(), { ...base, grabbed: true, speed: 900 }, 1).name, 'joy');
  assert.equal(run(quiet(), { ...base, airborne: true, speed: 900 }, 1).name, 'joy');
});

test('being petted goes from happy to love, and love lingers after the pointer leaves', () => {
  const m = quiet();
  const petting = { ...base, hover: true, pointerSpeed: 220 };
  assert.equal(run(m, petting, 1).name, 'happy');
  assert.equal(run(m, petting, 2, 1).name, 'love');
  assert.equal(run(m, base, 0.5, 3).name, 'love');
  assert.equal(run(m, base, 2, 3.6).name, 'calm');
});

test('a cursor resting on the slime never makes it fall in love', () => {
  const m = quiet();
  assert.notEqual(run(m, { ...base, hover: true }, 10).name, 'love');
});

test('a sulking slime pouts, even when the cursor rests on it, but is still surprised when grabbed', () => {
  const m = quiet();
  assert.equal(run(m, { ...base, sulking: true }, 1).name, 'sulk');
  assert.equal(run(m, { ...base, sulking: true, hover: true }, 1, 1).name, 'sulk');
  assert.equal(run(m, { ...base, sulking: true, grabbed: true }, 1, 2).name, 'surprised');
  assert.equal(run(m, base, 1, 3).name, 'calm');
});

test('what the brain wants to show wins for a moment, then the mood comes back', () => {
  const m = quiet();
  assert.equal(run(m, { ...base, hint: { name: 'joy', until: 2 } }, 1).name, 'joy');
  assert.equal(run(m, { ...base, hint: { name: 'joy', until: 2 } }, 2, 1).name, 'calm');
  assert.equal(run(quiet(), { ...base, hint: { name: 'nonsense', until: 9 } }, 1).name, 'calm');
});

test('a hard landing hurts for a moment, a shake makes it dizzy', () => {
  const m = quiet();
  assert.equal(run(m, { ...base, squash: 0.5, impact: 600 }, 0.2).name, 'ouch');
  assert.equal(run(m, base, 1.5, 0.2).name, 'calm');
  const d = quiet();
  assert.equal(run(d, { ...base, acc: 9000 }, 1).name, 'dizzy');
});

test('a sleepy slime yawns once, then dozes', () => {
  const m = quiet();
  assert.equal(run(m, { ...base, sleepiness: 0.8 }, 1).name, 'yawn');
  assert.equal(run(m, { ...base, sleepiness: 0.8 }, 3, 1).name, 'sleepy');
});

test('poses blend smoothly instead of snapping', () => {
  const m = quiet();
  run(m, base, 1);
  const first = m.update({ ...base, hover: true }, 1, 1 / 60).pose.mw;
  assert.ok(first > EXPRESSIONS.calm.mw && first < EXPRESSIONS.happy.mw, `${first}`);
});

test('preparing a hop is not a painful landing', () => {
  assert.equal(run(quiet(), { ...base, intent: 'wander', crouching: true, squash: 0.5 }, 0.2).name, 'brace');
});

test('a normal autonomous leap does not trigger dizziness', () => {
  assert.equal(run(quiet(), { ...base, intent: 'wander', airborne: true, acc: 9000 }, 1).name, 'leap');
});

test('finishing repeated walks never triggers an unrelated tongue expression', () => {
  const m = quiet();
  for (let i = 0; i < 4; i++) {
    run(m, { ...base, intent: 'wander' }, 1, i * 2);
    assert.equal(run(m, base, 1, i * 2 + 1).name, 'calm');
  }
});

test('walking to bed stays awake until resting in the corner', () => {
  assert.equal(run(quiet(), { ...base, sleepiness: 0.9, intent: 'wander' }, 3).name, 'focus');
});

test('an interrupted walk does not celebrate an arrival', () => {
  const m = quiet();
  run(m, { ...base, intent: 'wander' }, 1);
  assert.equal(run(m, base, 0.6, 1).name, 'calm');
});

test('being compressed in a corner is not a hard landing', () => {
  assert.equal(run(quiet(), { ...base, squash: 0.5, impact: 0 }, 1).name, 'calm');
});

test('faces follow what the slime is doing', () => {
  assert.equal(run(quiet(), { ...base, intent: 'wander' }, 1).name, 'focus');
  assert.equal(run(quiet(), { ...base, intent: 'follow' }, 1).name, 'fond');
  assert.equal(run(quiet(), { ...base, intent: 'peek', sulking: true }, 1).name, 'curious');
  assert.equal(run(quiet(), { ...base, intent: 'nap' }, 4).name, 'sleepy');
  assert.equal(run(quiet(), { ...base, intent: 'wander', crouching: true }, 1).name, 'brace');
  assert.equal(run(quiet(), { ...base, intent: 'wander', airborne: true }, 1).name, 'leap');
});

test('a long stillness never makes a face by chance', () => {
  const idle = quiet();
  for (let t = 0; t < 600; t += 1) assert.equal(run(idle, base, 1, t).name, 'calm');
});

test('a hard landing hurts by its measured impact, a soft hop does not', () => {
  assert.equal(run(quiet(), { ...base, impact: 600 }, 0.2).name, 'ouch');
  assert.equal(run(quiet(), { ...base, intent: 'wander', impact: 300 }, 0.5).name, 'focus');
});

test('every step of a catch has its own face, from noticing to backing off', () => {
  const face = (catchPhase, crouching = false) => run(quiet(), { ...base, intent: 'catch', catchPhase, crouching }, 1).name;
  assert.equal(face('acquire'), 'curious');
  assert.equal(face('prepare'), 'focus');
  assert.equal(face('prepare', true), 'brace');
  assert.equal(face('flight'), 'leap');
  assert.equal(face('contact'), 'brace');
  assert.equal(face('spread'), 'happy');
  assert.equal(face('hold'), 'fond');
  assert.equal(face('retreat'), 'calm');
  assert.equal(face(undefined), 'focus');
});
