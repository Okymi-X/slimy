import test from 'node:test';
import assert from 'node:assert/strict';
import { createBirth, nameError, MAX_NAME } from '../birth.js';

test('an egg hatches after three touches', () => {
  const b = createBirth(0);
  assert.equal(b.poke(1), false);
  assert.equal(b.poke(2), false);
  assert.equal(b.poke(3), true);
  assert.equal(b.phase, 'hatching');
});

test('an untouched egg hatches by itself, slowly', () => {
  const b = createBirth(0);
  let hatched = false;
  for (let t = 0; t < 40; t += 0.5) hatched = b.update(t) || hatched;
  assert.equal(hatched, true);
  assert.equal(b.phase === 'hatching' || b.phase === 'askUser', true);
});

test('a touch resets the wait for the next crack', () => {
  const b = createBirth(0);
  b.poke(6.5);
  b.update(7.2);
  assert.equal(b.view(7.2).cracks, 1);
});

test('the slime asks for the user first, then for its own name', () => {
  const b = createBirth(0);
  b.poke(1); b.poke(1); b.poke(1);
  b.update(3);
  assert.equal(b.view(3).prompt, 'askUser');
  assert.deepEqual(b.submit('  Okymi ', 3), { ok: true, finished: false });
  assert.equal(b.view(3).prompt, 'askSlime');
  assert.deepEqual(b.submit('Mochi', 4), { ok: true, finished: true });
  assert.deepEqual(b.names, { userName: 'Okymi', slimeName: 'Mochi' });
  b.welcome(4);
  assert.equal(b.view(4).prompt, 'welcome');
  b.update(9);
  assert.equal(b.phase, 'done');
});

test('bad answers are refused with a reason and the question stays', () => {
  const b = createBirth(0);
  b.poke(1); b.poke(1); b.poke(1); b.update(3);
  assert.equal(b.submit('   ', 3).error, 'empty');
  assert.equal(b.submit('x'.repeat(MAX_NAME + 1), 3).error, 'too_long');
  assert.equal(b.submit('a\u0007b', 3).error, 'bad_character');
  assert.equal(b.view(3).prompt, 'askUser');
  assert.equal(b.submit('ok', 3).ok, true);
});

test('nothing can be answered before the slime is awake', () => {
  assert.equal(createBirth(0).submit('Okymi', 1).error, 'not_now');
});

test('names validation mirrors the soul rules', () => {
  assert.equal(nameError('Zoé-Lou'), null);
  assert.equal(nameError('x'.repeat(MAX_NAME)), null);
  assert.equal(nameError(undefined), 'empty');
});

test('the eyes open gradually after the hatch', () => {
  const b = createBirth(0);
  b.poke(1); b.poke(1); b.poke(1);
  assert.equal(b.view(1).awake, 0);
  assert.ok(b.view(1.8).awake > 0 && b.view(1.8).awake < 1);
  b.update(3);
  assert.equal(b.view(3).awake, 1);
});
