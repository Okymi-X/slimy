import test from 'node:test';
import assert from 'node:assert/strict';
import { createFace } from '../face.js';
import { normalizeModel } from '../model.js';

const model = normalizeModel();
const body = { cx: 0, cy: 0, minX: -65, maxX: 65, minY: -50, maxY: 40, w: 130, h: 90, squash: 0.9 };

function recorder() {
  const eyes = [];
  const ctx = new Proxy({}, {
    get(target, name) {
      if (name === 'translate') return (x, y) => eyes.push({ x, y });
      return () => {};
    },
    set() { return true; },
  });
  return { ctx, eyes };
}

function settle(face, input, seconds) {
  for (let i = 0, n = Math.round(seconds * 60); i < n; i++) face.draw(body, { ...input, t: 10 + i / 60, dt: 1 / 60 });
}

function eyeCenters(rec, face, input) {
  rec.eyes.length = 0;
  face.draw(body, { ...input, t: 20, dt: 1 / 60 });
  const open = rec.eyes.slice(0, 2);
  return open.map((e) => e.x);
}

const still = { vel: { x: 0, y: 0 }, acc: { x: 0, y: 0 }, sleepiness: 0, grabbed: false, hover: false, look: { x: 0, y: 0 } };

test('the eyes follow a pointer on the right and on the left', () => {
  const rec = recorder();
  const face = createFace(rec.ctx, model);
  settle(face, still, 1);
  const centered = eyeCenters(rec, face, still);
  settle(face, { ...still, look: { x: 1, y: 0 } }, 1);
  const right = eyeCenters(rec, face, { ...still, look: { x: 1, y: 0 } });
  settle(face, { ...still, look: { x: -1, y: 0 } }, 1);
  const left = eyeCenters(rec, face, { ...still, look: { x: -1, y: 0 } });
  assert.ok(right[0] > centered[0] + 6, `right ${right[0]} vs ${centered[0]}`);
  assert.ok(left[0] < centered[0] - 6, `left ${left[0]} vs ${centered[0]}`);
});

test('the eyes trail behind a body moving fast, then catch up', () => {
  const rec = recorder();
  const face = createFace(rec.ctx, model);
  settle(face, still, 1);
  const rest = eyeCenters(rec, face, still);
  const moving = { ...still, vel: { x: 1500, y: 0 } };
  settle(face, moving, 0.6);
  const trailing = eyeCenters(rec, face, moving);
  assert.ok(trailing[0] < rest[0] - 8, `trailing ${trailing[0]} vs rest ${rest[0]}`);
  settle(face, still, 1.5);
  const after = eyeCenters(rec, face, still);
  assert.ok(Math.abs(after[0] - rest[0]) < 1.5, `did not come back: ${after[0]} vs ${rest[0]}`);
});

test('the eyes slosh when the body accelerates and stay inside the body', () => {
  const rec = recorder();
  const face = createFace(rec.ctx, model);
  settle(face, still, 1);
  const rest = eyeCenters(rec, face, still);
  const shaken = { ...still, acc: { x: 5000, y: 0 } };
  settle(face, shaken, 0.5);
  const sloshed = eyeCenters(rec, face, shaken);
  assert.ok(sloshed[0] < rest[0] - 4, `sloshed ${sloshed[0]} vs rest ${rest[0]}`);
  assert.ok(Math.abs(sloshed[0] - body.cx) < body.w * 0.5, 'eye left the body');
});

test('a hostile model cannot produce non finite eye positions', () => {
  const rec = recorder();
  const face = createFace(rec.ctx, normalizeModel({ face: { lag: { stiffness: 'x', damping: -5, velocity: 1e9 } } }));
  settle(face, { ...still, vel: { x: 1e6, y: -1e6 } }, 1);
  assert.ok(eyeCenters(rec, face, still).every(Number.isFinite));
});

test('strong motion keeps the eyes separated and inside the gel', () => {
  for (const direction of [-1, 1]) {
    const rec = recorder(), face = createFace(rec.ctx, model);
    const input = { ...still, look: { x: direction, y: 0 }, vel: { x: -direction * 1500, y: 0 }, acc: { x: -direction * 5000, y: 0 } };
    settle(face, input, 0.5);
    const [left, right] = eyeCenters(rec, face, input);
    assert.ok(right - left >= body.w * 0.24, `eyes bunched together: ${left}, ${right}`);
    assert.ok(left > body.minX + 8 && right < body.maxX - 8);
  }
});

test('the eyes respond to the cursor within a tenth of a second', () => {
  const rec = recorder(), face = createFace(rec.ctx, model);
  settle(face, still, 1);
  const centered = eyeCenters(rec, face, still)[0];
  const looking = { ...still, look: { x: 1, y: 0 } };
  for (let i = 0; i < 6; i++) face.draw(body, { ...looking, t: 20 + i / 60, dt: 1 / 60 });
  const early = eyeCenters(rec, face, looking)[0];
  settle(face, looking, 1);
  const final = eyeCenters(rec, face, looking)[0];
  assert.ok(early - centered > (final - centered) * 0.6, `early ${early - centered}, final ${final - centered}`);
});
