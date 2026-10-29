import test from 'node:test';
import assert from 'node:assert/strict';
import { frameStride, strideThreshold, createRefreshClock } from '../cadence.js';

test('the stride lands closest to 40 images a second on common screens', () => {
  const expected = { 50: 1, 60: 2, 75: 2, 90: 2, 120: 3, 144: 4, 165: 4, 240: 6 };
  for (const [hz, n] of Object.entries(expected)) assert.equal(frameStride(1 / hz), n, `${hz} Hz`);
});

test('a screen slower than the target, or an unknown refresh, is never slowed down', () => {
  assert.equal(frameStride(1 / 30), 1);
  assert.equal(frameStride(0), 1);
  assert.equal(frameStride(NaN), 1);
  assert.equal(frameStride(1), 1);
});

test('simulated, the walk is evenly paced and close to 40 images a second, with or without jitter', () => {
  let seed = 7;
  const random = () => ((seed = (seed * 1103515245 + 12345) % 2 ** 31) / 2 ** 31);
  for (const hz of [60, 75, 90, 120, 144, 240]) for (const jitter of [0, 0.001]) {
    const refresh = 1 / hz, n = frameStride(refresh), threshold = strideThreshold(refresh, n);
    let now = 0, last = 0, drawn = 0;
    const gaps = [];
    for (let i = 0; i < hz * 60; i++) {
      now += refresh + (random() * 2 - 1) * jitter;
      if (now - last >= threshold) { gaps.push(now - last); last = now; drawn++; }
    }
    const fps = drawn / now;
    assert.ok(Math.abs(fps - 40) <= 10, `${hz} Hz jitter ${jitter}: ${fps.toFixed(1)} images/s`);
    assert.ok(Math.max(...gaps) - Math.min(...gaps) <= 2 * refresh + 4 * jitter, `${hz} Hz gaps vary too much`);
  }
});

test('the clock finds the refresh interval and ignores pauses', () => {
  const clock = createRefreshClock();
  let now = 1;
  for (let i = 0; i < 400; i++) clock.tick((now += 1 / 144));
  assert.ok(Math.abs(clock.interval - 1 / 144) < 0.0004);
  clock.tick(now + 2);
  assert.ok(Math.abs(clock.interval - 1 / 144) < 0.0004, 'a long pause does not count as a slow screen');
});
