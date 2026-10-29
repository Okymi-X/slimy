import test from 'node:test';
import assert from 'node:assert/strict';
import { swallowedLevel, createSwallowed, FADE_SECONDS } from '../swallowed.js';

test('the cursor shows while held and fades out in about 1.2 s once released', () => {
  const s = createSwallowed();
  for (let i = 0; i < 30; i++) s.step(1 / 60, 'hold');
  assert.equal(s.level, 1);
  for (let i = 0; i < 30; i++) s.step(1 / 60, 'retreat');
  assert.ok(s.level > 0.3 && s.level < 0.8);
  for (let i = 0; i < Math.ceil(FADE_SECONDS * 60) + 1; i++) s.step(1 / 60, 'none');
  assert.equal(s.level, 0);
});

test('other phases never show it, and a long frame is bounded', () => {
  assert.equal(swallowedLevel(0, 1, 'flight'), 0);
  assert.equal(createSwallowed().step(5, 'hold'), 1);
});
